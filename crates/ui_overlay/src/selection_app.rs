/// 选择器应用程序模块
///
/// 包含 SelectionApp 的核心逻辑和事件处理
use crate::event_handler::{EventHandler, EventResult, SelectionState};
use crate::frame_timer::FrameTimer;
use crate::image_cache::ImageCache;
use crate::platform;
use crate::selection_render::{BackgroundProcessor, WindowRenderer};
use crate::window_manager::WindowManager;
use crate::Region;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

const OVERLAY_BG_COLOR: [u8; 4] = [0, 0, 0, 128];
const TARGET_FPS: u32 = 60; // 目标帧率：60 FPS

/// 选择器应用程序
pub struct SelectionApp {
    pub attrs: WindowAttributes,
    pub window_manager: WindowManager,
    pub pres_guard: Option<platform::PresentationGuard>,
    pub bg: Option<Vec<u8>>,
    pub bg_w: u32,
    pub bg_h: u32,
    pub bg_tinted: Option<Vec<u8>>,
    pub overlay_color: [u8; 4],
    pub state: SelectionState,
    /// 图像缓存，避免每帧重新创建 Skia Image
    image_cache: ImageCache,
    /// 帧率控制器，限制到 60 FPS
    frame_timer: FrameTimer,
}

/// 渲染数据结构
struct RenderData {
    selection_rect: (i32, i32, i32, i32),
    selection_exists: bool,
    window_needs_selection: bool,
}

impl SelectionApp {
    pub fn new(
        attrs: WindowAttributes,
        bg_rgba: Option<Vec<u8>>,
        bg_w: u32,
        bg_h: u32,
        virtual_bounds: Option<(i32, i32, u32, u32)>,
    ) -> Self {
        Self {
            attrs,
            window_manager: WindowManager::new(),
            pres_guard: None,
            bg: bg_rgba,
            bg_w,
            bg_h,
            bg_tinted: None,
            overlay_color: OVERLAY_BG_COLOR,
            state: SelectionState::new(virtual_bounds),
            image_cache: ImageCache::new(),
            frame_timer: FrameTimer::new(TARGET_FPS),
        }
    }

    pub fn render_window_by_index(&mut self, window_index: usize) {
        if window_index >= self.window_manager.windows.len() {
            return;
        }

        // 初始化渲染后端
        if let Err(e) = self.window_manager.windows[window_index].init_backend() {
            eprintln!("⚠️  Failed to init render backend: {}", e);
            return;
        }

        // 确保背景和图像缓存已初始化
        self.ensure_backgrounds_and_cache();

        let render_data = self.prepare_render_data(window_index);

        // 渲染当前帧
        if let Err(e) = self.render(window_index, &render_data) {
            eprintln!("⚠️  Render error: {}", e);
        }
    }

    fn prepare_render_data(&mut self, window_index: usize) -> RenderData {
        let (x0c, y0c, x1c, y1c) = self.state.calculate_selection_rect();
        // 只要有有效的选择尺寸，就应该渲染选择框
        // 无论是否正在拖动，这样在拖动完成后选择框依然显示
        let selection_exists = (x1c - x0c).abs() > 1.0 && (y1c - y0c).abs() > 1.0;

        let window_info = &self.window_manager.windows[window_index];
        let window_needs_selection = WindowRenderer::should_render_selection(
            selection_exists,
            &mut self.state,
            window_info.virtual_x,
            window_info.virtual_y,
            window_info.size_px.width,
            window_info.size_px.height,
        );

        RenderData {
            selection_rect: (x0c as i32, y0c as i32, x1c as i32, y1c as i32),
            selection_exists,
            window_needs_selection,
        }
    }

    /// 渲染当前帧
    fn render(&mut self, window_index: usize, render_data: &RenderData) -> Result<(), String> {
        // 获取窗口信息
        let window_virtual_x = self.window_manager.windows[window_index].virtual_x;
        let window_virtual_y = self.window_manager.windows[window_index].virtual_y;

        #[cfg(debug_assertions)]
        if render_data.selection_exists && window_index == 0 {
            // 只在第一个窗口打印，避免刷屏
            let window_info = &self.window_manager.windows[window_index];
            let (x0, y0, x1, y1) = render_data.selection_rect;
            tracing::debug!("=== 渲染坐标转换 [Window {}] ===", window_index);
            tracing::debug!("选择框虚拟坐标: ({}, {}) 到 ({}, {})", x0, y0, x1, y1);
            tracing::debug!("窗口虚拟位置: ({}, {})", window_virtual_x, window_virtual_y);
            tracing::debug!(
                "窗口物理尺寸: {}x{}",
                window_info.size_px.width,
                window_info.size_px.height
            );

            if let Some((vx, vy, vw, vh)) = self.state.virtual_bounds {
                tracing::debug!("虚拟边界: ({}, {}) 尺寸 {}x{}", vx, vy, vw, vh);
                let offset_x = -(window_virtual_x - vx);
                let offset_y = -(window_virtual_y - vy);
                tracing::debug!("背景偏移: ({}, {})", offset_x, offset_y);
            }
        }

        // 获取缓存的图像（不再需要每次创建）
        let bg_tinted_image = self.image_cache.get_tinted_image();
        let original_image = self.image_cache.get_original_image();

        let window_info = &mut self.window_manager.windows[window_index];
        let virtual_bounds = self.state.virtual_bounds;

        // 执行渲染
        window_info.render(|canvas| {
            use skia_safe::Color;

            // 1. 清空背景
            canvas.clear(Color::from_argb(0, 0, 0, 0));

            // 2. 渲染暗化背景 - 使用旧的渲染器（已有正确的坐标转换）
            if let Some(ref tinted_image) = bg_tinted_image {
                // 手动实现 render_virtual_cached 的逻辑
                if let Some((vx, vy, _, _)) = virtual_bounds {
                    let offset_x = -(window_virtual_x - vx) as f32;
                    let offset_y = -(window_virtual_y - vy) as f32;
                    canvas.draw_image(tinted_image, (offset_x, offset_y), None);
                } else {
                    canvas.draw_image(tinted_image, (0, 0), None);
                }
            }

            // 3. 渲染选择区域 - 只在 window_needs_selection 为 true 时渲染
            if render_data.selection_exists && render_data.window_needs_selection {
                let (x0, y0, x1, y1) = render_data.selection_rect;

                // 渲染选择区域的原始背景（未暗化）
                if let Some(ref image) = original_image {
                    // 手动实现 render_original_background_cached 的逻辑
                    canvas.save();

                    // 计算窗口坐标（虚拟桌面坐标 -> 窗口本地坐标）
                    let (win_x0, win_y0, win_x1, win_y1) = if virtual_bounds.is_some() {
                        (
                            x0 - window_virtual_x,
                            y0 - window_virtual_y,
                            x1 - window_virtual_x,
                            y1 - window_virtual_y,
                        )
                    } else {
                        (x0, y0, x1, y1)
                    };

                    // 裁剪到选择区域
                    let clip_rect = skia_safe::Rect::from_ltrb(
                        win_x0 as f32,
                        win_y0 as f32,
                        win_x1 as f32,
                        win_y1 as f32,
                    );
                    canvas.clip_rect(clip_rect, skia_safe::ClipOp::Intersect, true);

                    // 绘制原始图像
                    if let Some((vx, vy, _, _)) = virtual_bounds {
                        let offset_x = -(window_virtual_x - vx) as f32;
                        let offset_y = -(window_virtual_y - vy) as f32;
                        canvas.draw_image(image, (offset_x, offset_y), None);
                    } else {
                        canvas.draw_image(image, (0, 0), None);
                    }

                    canvas.restore();
                }

                // 手动实现 render_border 的逻辑
                let (x0, y0, x1, y1) = render_data.selection_rect;
                let (win_x0, win_y0, win_x1, win_y1) = if virtual_bounds.is_some() {
                    (
                        x0 - window_virtual_x,
                        y0 - window_virtual_y,
                        x1 - window_virtual_x,
                        y1 - window_virtual_y,
                    )
                } else {
                    (x0, y0, x1, y1)
                };

                let mut paint = skia_safe::Paint::default();
                paint.set_color(skia_safe::Color::WHITE);
                paint.set_style(skia_safe::paint::Style::Stroke);
                paint.set_stroke_width(2.0);
                paint.set_anti_alias(true);

                let rect = skia_safe::Rect::from_ltrb(
                    win_x0 as f32,
                    win_y0 as f32,
                    win_x1 as f32,
                    win_y1 as f32,
                );
                canvas.draw_rect(rect, &paint);
            }
        })
    }

    /// 确保背景暗化和图像缓存都已初始化
    ///
    /// 性能优化：只在首次调用时处理，后续调用直接返回
    fn ensure_backgrounds_and_cache(&mut self) {
        // 生成暗化背景（如果尚未生成）
        if self.bg_tinted.is_none() {
            if let Some(bg) = &self.bg {
                if !bg.is_empty() && self.bg_w > 0 && self.bg_h > 0 {
                    self.bg_tinted =
                        Some(BackgroundProcessor::tint_background(bg, self.overlay_color));
                }
            }
        }

        // 一次性初始化 Skia Image 缓存
        if let (Some(ref bg), Some(ref bg_tinted)) = (&self.bg, &self.bg_tinted) {
            self.image_cache
                .ensure_images_cached(bg, bg_tinted, self.bg_w, self.bg_h);
        }
    }

    pub fn request_redraw_all(&mut self) {
        // 在请求重绘时检查帧率限制
        // 如果不应该渲染，就不发起重绘请求
        if !self.frame_timer.should_render() {
            return;
        }

        if self.state.should_throttle_redraw() {
            return;
        }

        self.state.mark_redraw_requested();
        self.window_manager.request_redraw_all();
    }

    pub fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let window_index = match self.window_manager.find_window_index(window_id) {
            Some(idx) => idx,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                self.handle_close_requested(event_loop);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.handle_keyboard_input(event_loop, window_index, key_event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(window_index, position);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.handle_scale_factor_changed(window_index, scale_factor);
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(window_index, new_size);
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw_requested(window_index);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                self.handle_mouse_input(event_loop, window_index, button, button_state);
            }
            _ => {}
        }
    }

    fn handle_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        self.state.result = None;
        if let Some(g) = self.pres_guard.take() {
            platform::end_presentation(g);
        }
        event_loop.exit();
    }

    fn handle_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_index: usize,
        key_event: winit::event::KeyEvent,
    ) {
        match EventHandler::handle_keyboard_event(&mut self.state, key_event) {
            EventResult::Continue(need_redraw) => {
                if need_redraw {
                    self.request_redraw_all();
                }
            }
            EventResult::Exit => event_loop.exit(),
            EventResult::Finish => {
                if let Some(region) = self.create_region(window_index) {
                    self.state.result = Some(region);
                    event_loop.exit();
                }
            }
        }
    }

    fn handle_cursor_moved(
        &mut self,
        window_index: usize,
        position: winit::dpi::PhysicalPosition<f64>,
    ) {
        let window_info = &self.window_manager.windows[window_index];
        let new_pos = EventHandler::convert_cursor_position(
            position,
            window_info.virtual_x,
            window_info.virtual_y,
            self.state.virtual_bounds,
            window_info.scale,
        );

        if EventHandler::handle_cursor_moved(&mut self.state, new_pos) {
            self.request_redraw_all();
        }
    }

    fn handle_scale_factor_changed(&mut self, window_index: usize, scale_factor: f64) {
        self.window_manager.windows[window_index].update_scale(scale_factor);
        self.request_redraw_all();
    }

    fn handle_resized(&mut self, window_index: usize, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_manager.windows[window_index].update_size(new_size);
        self.request_redraw_all();
    }

    fn handle_redraw_requested(&mut self, window_index: usize) {
        self.state.clear_redraw_pending();
        self.render_window_by_index(window_index);
    }

    fn handle_mouse_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_index: usize,
        button: winit::event::MouseButton,
        button_state: winit::event::ElementState,
    ) {
        match EventHandler::handle_mouse_input(&mut self.state, button, button_state) {
            EventResult::Continue(need_redraw) => {
                if need_redraw {
                    self.request_redraw_all();
                }
            }
            EventResult::Exit => event_loop.exit(),
            EventResult::Finish => {
                if let Some(region) = self.create_region(window_index) {
                    self.state.result = Some(region);
                    event_loop.exit();
                }
            }
        }
    }

    fn create_region(&mut self, window_index: usize) -> Option<Region> {
        let scale_out = if self.state.virtual_bounds.is_some() {
            1.0
        } else {
            self.window_manager.windows[window_index].scale as f32
        };

        self.state.build_region(scale_out)
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.window_manager.windows.is_empty() {
            return;
        }

        // 使用 winit 的显示器信息创建窗口
        self.window_manager
            .initialize_windows(event_loop, &self.attrs);

        #[cfg(debug_assertions)]
        {
            tracing::debug!("=== SelectionApp 窗口信息 ===");
            tracing::debug!("创建了 {} 个窗口", self.window_manager.windows.len());
            for (i, window_info) in self.window_manager.windows.iter().enumerate() {
                tracing::debug!(
                    "[Window {}] virtual_pos=({}, {}), size_px={}x{}, scale={}",
                    i,
                    window_info.virtual_x,
                    window_info.virtual_y,
                    window_info.size_px.width,
                    window_info.size_px.height,
                    window_info.scale
                );
            }
        }

        if !self.window_manager.windows.is_empty() {
            self.pres_guard = platform::start_presentation();

            // 一次性初始化背景和图像缓存
            self.ensure_backgrounds_and_cache();

            #[cfg(target_os = "macos")]
            {
                let color = self.overlay_color;
                for window_info in &self.window_manager.windows {
                    platform::apply_overlay_window_appearance(window_info.window.as_ref(), color);
                }
            }

            // 先渲染所有窗口的首帧，避免显示时出现粉色背景
            for i in 0..self.window_manager.windows.len() {
                self.render_window_by_index(i);
            }

            // 首帧渲染完成后再显示窗口
            for i in 0..self.window_manager.windows.len() {
                self.window_manager.windows[i].window.set_visible(true);
                self.window_manager.windows[i].window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.on_window_event(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 空闲时不强制重绘
    }
}
