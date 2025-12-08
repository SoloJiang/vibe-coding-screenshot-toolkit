/// 选择器应用程序模块
///
/// 包含 SelectionApp 的核心逻辑和事件处理，使用 egui 进行渲染
use crate::event_handler::{EventHandler, EventResult, SelectionState};
use crate::frame_timer::FrameTimer;
use crate::platform;
use crate::selection_render::{EguiSelectionRenderer, WindowRenderer};
use crate::window_manager::WindowManager;
use crate::Region;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

const TARGET_FPS: u32 = 60; // 目标帧率：60 FPS

/// 选择器应用程序
pub struct SelectionApp {
    pub attrs: WindowAttributes,
    pub window_manager: WindowManager,
    pub pres_guard: Option<platform::PresentationGuard>,
    pub bg: Option<Vec<u8>>,
    pub bg_w: u32,
    pub bg_h: u32,
    pub state: SelectionState,
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
            state: SelectionState::new(virtual_bounds),
            frame_timer: FrameTimer::new(TARGET_FPS),
        }
    }

    pub fn render_window_by_index(&mut self, window_index: usize) {
        if window_index >= self.window_manager.windows.len() {
            return;
        }

        // 初始化 WGPU 渲染器
        if let Err(e) = self.window_manager.windows[window_index].init_wgpu() {
            eprintln!("⚠️  Failed to init WGPU: {}", e);
            return;
        }

        // 确保背景和图像缓存已初始化（需要 egui context）
        let egui_ctx = self.window_manager.windows[window_index].egui_ctx.clone();
        self.ensure_backgrounds_and_cache(&egui_ctx, window_index);

        let render_data = self.prepare_render_data(window_index);

        // 渲染当前帧
        if let Err(e) = self.render(window_index, &render_data) {
            eprintln!("⚠️  Render error: {}", e);
        }
    }

    fn prepare_render_data(&mut self, window_index: usize) -> RenderData {
        let (x0c, y0c, x1c, y1c) = self.state.calculate_selection_rect();
        // 只要有有效的选择尺寸，就应该渲染选择框
        let selection_exists = (x1c - x0c).abs() > 1.0 && (y1c - y0c).abs() > 1.0;

        if selection_exists {
            tracing::debug!(
                "🔍 窗口 {} 准备渲染数据: 选择框=({:.0}, {:.0}) -> ({:.0}, {:.0}), 尺寸=({:.0}x{:.0})",
                window_index,
                x0c, y0c, x1c, y1c,
                (x1c - x0c).abs(),
                (y1c - y0c).abs()
            );
        }

        let window_info = &self.window_manager.windows[window_index];
        let window_needs_selection = WindowRenderer::should_render_selection(
            selection_exists,
            &mut self.state,
            window_info.virtual_x,
            window_info.virtual_y,
            window_info.size_px.width,
            window_info.size_px.height,
        );

        if selection_exists {
            tracing::debug!(
                "   └─ 窗口 {} 虚拟位置=({}, {}), 尺寸={}x{}, 需要渲染={}",
                window_index,
                window_info.virtual_x,
                window_info.virtual_y,
                window_info.size_px.width,
                window_info.size_px.height,
                window_needs_selection
            );
        }

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
        let window_size_px = self.window_manager.windows[window_index].size_px;
        let window_scale = self.window_manager.windows[window_index].scale;

        #[cfg(debug_assertions)]
        if render_data.selection_exists && window_index == 0 {
            // 只在第一个窗口打印，避免刷屏
            let (x0, y0, x1, y1) = render_data.selection_rect;
            tracing::debug!("=== 渲染坐标转换 [Window {}] ===", window_index);
            tracing::debug!("选择框虚拟坐标: ({}, {}) 到 ({}, {})", x0, y0, x1, y1);
            tracing::debug!("窗口虚拟位置: ({}, {})", window_virtual_x, window_virtual_y);
            tracing::debug!(
                "窗口物理尺寸: {}x{}, scale: {}",
                window_size_px.width,
                window_size_px.height,
                window_scale
            );

            if let Some((vx, vy, vw, vh)) = self.state.virtual_bounds {
                tracing::debug!("虚拟边界: ({}, {}) 尺寸 {}x{}", vx, vy, vw, vh);
                let offset_x = -(window_virtual_x - vx);
                let offset_y = -(window_virtual_y - vy);
                tracing::debug!("背景偏移: ({}, {})", offset_x, offset_y);
            }
        }

        // 注意：背景现在由 wgpu 直接渲染，egui 层只绘制 UI 元素

        let virtual_bounds = self.state.virtual_bounds;

        // 计算逻辑坐标下的选择框（用于 egui 渲染）
        let selection_rect_logical =
            if render_data.selection_exists && render_data.window_needs_selection {
                let (x0, y0, x1, y1) = render_data.selection_rect;

                tracing::debug!(
                    "🎯 窗口 {} 选择框转换: 虚拟坐标=({}, {}) -> ({}, {})",
                    window_index,
                    x0,
                    y0,
                    x1,
                    y1
                );
                tracing::debug!(
                    "   └─ 窗口虚拟位置=({}, {}), scale={}",
                    window_virtual_x,
                    window_virtual_y,
                    window_scale
                );

                // 将虚拟坐标转换为窗口本地坐标
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

                tracing::debug!(
                    "   └─ 窗口本地坐标=({}, {}) -> ({}, {})",
                    win_x0,
                    win_y0,
                    win_x1,
                    win_y1
                );

                // 转换为逻辑坐标（egui 使用逻辑坐标）
                let scale = window_scale as f32;
                let logical_rect = (
                    win_x0 as f32 / scale,
                    win_y0 as f32 / scale,
                    (win_x1 - win_x0) as f32 / scale,
                    (win_y1 - win_y0) as f32 / scale,
                );

                tracing::debug!(
                    "   └─ egui逻辑坐标=({:.1}, {:.1}), 尺寸=({:.1}, {:.1})",
                    logical_rect.0,
                    logical_rect.1,
                    logical_rect.2,
                    logical_rect.3
                );

                Some(logical_rect)
            } else {
                if render_data.selection_exists {
                    tracing::debug!("窗口 {} 无需绘制选择框（不在范围内）", window_index);
                }
                None
            };

        // 窗口逻辑尺寸
        let window_logical_size = (
            window_size_px.width as f32 / window_scale as f32,
            window_size_px.height as f32 / window_scale as f32,
        );

        let window_info = &mut self.window_manager.windows[window_index];

        // 执行 egui 渲染（只绘制 UI 层：选择框、文本等）
        // 背景已经由 wgpu 在底层渲染
        tracing::trace!(
            "渲染窗口 {} UI 层，选择框={:?}",
            window_index,
            selection_rect_logical
        );

        window_info.render(|ctx| {
            // 使用全屏 CentralPanel，透明背景
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    let painter = ui.painter();

                    // 只绘制选择框和 UI 元素（不绘制背景）
                    EguiSelectionRenderer::render_selection(
                        painter,
                        selection_rect_logical,
                        window_logical_size,
                    );
                });
        })
    }

    /// 确保背景暗化和图像缓存都已初始化
    ///
    /// 性能优化：只在首次调用时处理，后续调用直接返回
    fn ensure_backgrounds_and_cache(&mut self, _ctx: &egui::Context, window_index: usize) {
        // 检查这个窗口是否已上传背景
        if self.window_manager.windows[window_index].has_background_texture() {
            tracing::debug!("窗口 {} 背景已上传，跳过", window_index);
            return;
        }

        // 为这个窗口上传原始背景（不暗化）
        // 暗化效果将由 egui 层的遮罩实现
        if let Some(ref bg) = self.bg {
            if !bg.is_empty() && self.bg_w > 0 && self.bg_h > 0 {
                // 获取虚拟坐标系统的最小值
                let (virtual_min_x, virtual_min_y) = self
                    .state
                    .virtual_bounds
                    .map(|(min_x, min_y, _, _)| (min_x, min_y))
                    .unwrap_or((0, 0));

                let window_info = &self.window_manager.windows[window_index];
                tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                tracing::info!("[背景上传] 窗口 {} 准备上传背景:", window_index);
                tracing::info!("  └─ 完整背景: {}x{} (RGBA)", self.bg_w, self.bg_h);
                tracing::info!(
                    "  └─ 虚拟桌面最小值: ({}, {})",
                    virtual_min_x,
                    virtual_min_y
                );
                tracing::info!(
                    "  └─ 窗口虚拟坐标 (物理): ({}, {})",
                    window_info.virtual_x,
                    window_info.virtual_y
                );
                tracing::info!(
                    "  └─ 窗口物理尺寸: {}x{}",
                    window_info.size_px.width,
                    window_info.size_px.height
                );
                tracing::info!("  └─ 窗口 scale: {}", window_info.scale);

                self.window_manager.windows[window_index].upload_background_texture(
                    bg,
                    self.bg_w,
                    self.bg_h,
                    virtual_min_x,
                    virtual_min_y,
                );
            }
        }
    }

    pub fn request_redraw_all(&mut self) {
        // 在请求重绘时检查帧率限制
        if !self.frame_timer.should_render() {
            tracing::trace!("🚫 帧率限制：跳过重绘请求");
            return;
        }

        if self.state.should_throttle_redraw() {
            tracing::trace!("🚫 重绘节流：跳过重绘请求");
            return;
        }

        tracing::debug!("✅ 请求所有窗口重绘");
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

        // 首先让 egui 处理事件
        let window_info = &mut self.window_manager.windows[window_index];
        let response = window_info
            .egui_state
            .on_window_event(&window_info.window, &event);

        // 如果 egui 消费了事件，可能需要重绘
        if response.consumed {
            self.request_redraw_all();
        }

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
            WindowEvent::Moved(new_position) => {
                // 忽略 Moved 事件，因为 macOS 报告的位置是错误的
                // 窗口位置在创建时已经正确设置，不应该改变
                tracing::debug!(
                    "[窗口 {}] 忽略 Moved 事件: 报告位置=({}, {})，保持原位置=({}, {})",
                    window_index,
                    new_position.x,
                    new_position.y,
                    self.window_manager.windows[window_index].virtual_x,
                    self.window_manager.windows[window_index].virtual_y
                );
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
        let (vx, vy) = (window_info.virtual_x, window_info.virtual_y);

        // 将窗口本地坐标转换为虚拟桌面坐标
        let virtual_x = position.x + vx as f64;
        let virtual_y = position.y + vy as f64;

        // 更新状态，渲染由 about_to_wait 触发
        EventHandler::handle_cursor_moved(&mut self.state, (virtual_x, virtual_y));
    }

    fn handle_scale_factor_changed(&mut self, window_index: usize, scale_factor: f64) {
        // 从 window 获取实际的 scale，而不是完全信任事件参数
        let window_info = &self.window_manager.windows[window_index];
        let actual_scale = window_info.window.scale_factor();

        tracing::debug!(
            "[窗口 {}] ScaleFactorChanged 事件: 事件scale={}, 实际window.scale={}",
            window_index,
            scale_factor,
            actual_scale
        );

        self.window_manager.windows[window_index].update_scale(actual_scale);
    }

    fn handle_resized(&mut self, window_index: usize, new_size: winit::dpi::PhysicalSize<u32>) {
        // 重要：winit 在 macOS 上经常发送错误的尺寸和 scale
        // 我们检测异常并使用创建时保存的正确值
        let window_info = &self.window_manager.windows[window_index];
        let reported_size = window_info.window.inner_size();
        let reported_scale = window_info.window.scale_factor();
        let initial_size = window_info.initial_size_px;
        let initial_scale = window_info.initial_scale;

        tracing::debug!(
            "[窗口 {}] Resized 事件: 事件={}x{}, window报告={}x{} scale={}, 初始={}x{} scale={}",
            window_index,
            new_size.width,
            new_size.height,
            reported_size.width,
            reported_size.height,
            reported_scale,
            initial_size.width,
            initial_size.height,
            initial_scale
        );

        // 检测是否是 winit 的bug：报告的尺寸是初始尺寸的一半（逻辑尺寸）
        let is_bug = (reported_size.width as f64) < (initial_size.width as f64 * 0.6)
            || (reported_size.height as f64) < (initial_size.height as f64 * 0.6)
            || reported_scale < initial_scale * 0.6;

        if is_bug {
            tracing::warn!(
                "[窗口 {}] ⚠️  检测到 winit bug！忽略错误的 Resized 事件，保持初始值",
                window_index
            );
            // 不更新，保持原值
        } else {
            // 正常更新
            self.window_manager.windows[window_index].update_size(reported_size);
            self.window_manager.windows[window_index].update_scale(reported_scale);
        }
    }

    fn handle_redraw_requested(&mut self, window_index: usize) {
        tracing::trace!("🖼️  窗口 {} 收到重绘请求", window_index);
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
            EventResult::Continue(_need_redraw) => {
                // 渲染由 about_to_wait 触发，不再直接渲染
            }
            EventResult::Finish => {
                if let Some(region) = self.create_region(window_index) {
                    self.state.result = Some(region);
                    event_loop.exit();
                }
            }
            EventResult::Exit => {
                self.state.result = None;
                event_loop.exit();
            }
        }
    }

    fn create_region(&mut self, window_index: usize) -> Option<Region> {
        // calculate_selection_rect 返回的是虚拟桌面物理坐标（像素）
        let (x0_phys, y0_phys, x1_phys, y1_phys) = self.state.calculate_selection_rect();
        if (x1_phys - x0_phys).abs() < 1.0 || (y1_phys - y0_phys).abs() < 1.0 {
            return None;
        }

        // 计算选择区域中心点（物理坐标），找到对应的显示器 scale
        let center_x_phys = (x0_phys + x1_phys) / 2.0;
        let center_y_phys = (y0_phys + y1_phys) / 2.0;

        // 查找中心点所在的窗口（显示器）
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!(
            "[创建 Region] 选择框物理坐标: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
            x0_phys,
            y0_phys,
            x1_phys,
            y1_phys
        );
        tracing::info!(
            "[创建 Region] 中心点物理坐标: ({:.1}, {:.1})",
            center_x_phys,
            center_y_phys
        );

        tracing::info!("[创建 Region] 查找中心点所在的显示器:");
        for (idx, w) in self.window_manager.windows.iter().enumerate() {
            let wx = w.virtual_x as f64;
            let wy = w.virtual_y as f64;
            let ww = w.size_px.width as f64;
            let wh = w.size_px.height as f64;
            let contains = center_x_phys >= wx
                && center_x_phys < wx + ww
                && center_y_phys >= wy
                && center_y_phys < wy + wh;
            tracing::info!(
                "  窗口 {}: virtual=({}, {}) size={}x{} scale={} contains={}",
                idx,
                wx,
                wy,
                ww,
                wh,
                w.scale,
                contains
            );
        }

        let scale = self
            .window_manager
            .windows
            .iter()
            .find(|w| {
                let wx = w.virtual_x as f64;
                let wy = w.virtual_y as f64;
                let ww = w.size_px.width as f64;
                let wh = w.size_px.height as f64;
                center_x_phys >= wx
                    && center_x_phys < wx + ww
                    && center_y_phys >= wy
                    && center_y_phys < wy + wh
            })
            .map(|w| w.scale)
            .unwrap_or_else(|| {
                // 如果中心点不在任何窗口内（理论上不应该），使用触发窗口的 scale
                tracing::warn!(
                    "选择区域中心点 ({:.1}, {:.1}) 不在任何窗口内，使用窗口 {} 的 scale",
                    center_x_phys,
                    center_y_phys,
                    window_index
                );
                self.window_manager.windows[window_index].scale
            });

        tracing::info!("[创建 Region] 选定的 scale: {}", scale);

        // 将物理坐标转换为逻辑坐标（Region 应该保存逻辑坐标）
        let x_phys = x0_phys.min(x1_phys);
        let y_phys = y0_phys.min(y1_phys);
        let w_phys = (x1_phys - x0_phys).abs();
        let h_phys = (y1_phys - y0_phys).abs();

        let x_logical = (x_phys / scale) as f32;
        let y_logical = (y_phys / scale) as f32;
        let w_logical = (w_phys / scale) as f32;
        let h_logical = (h_phys / scale) as f32;
        let scale_f32 = scale as f32;

        tracing::info!("[创建 Region] 坐标转换:");
        tracing::info!(
            "  └─ 物理: ({:.1}, {:.1}) 尺寸 {:.1}x{:.1}",
            x_phys,
            y_phys,
            w_phys,
            h_phys
        );
        tracing::info!(
            "  └─ 逻辑: ({:.1}, {:.1}) 尺寸 {:.1}x{:.1} (除以 scale={})",
            x_logical,
            y_logical,
            w_logical,
            h_logical,
            scale
        );
        tracing::info!(
            "  └─ Region 保存: x={:.1}, y={:.1}, w={:.1}, h={:.1}, scale={}",
            x_logical,
            y_logical,
            w_logical,
            h_logical,
            scale_f32
        );

        Some(Region::new(
            x_logical, y_logical, w_logical, h_logical, scale_f32,
        ))
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window_manager
            .initialize_windows(event_loop, &self.attrs);

        #[cfg(target_os = "macos")]
        {
            if let Some(first_win) = self.window_manager.windows.first() {
                platform::apply_overlay_window_appearance(&first_win.window, [0, 0, 0, 0]);
            }
        }

        // 对所有窗口应用平台特定的外观
        #[cfg(target_os = "macos")]
        {
            for window_info in &self.window_manager.windows {
                platform::apply_overlay_window_appearance(&window_info.window, [0, 0, 0, 0]);
            }
        }

        // 显示窗口前进行预热渲染
        for i in 0..self.window_manager.windows.len() {
            if let Err(e) = self.window_manager.windows[i].init_wgpu() {
                eprintln!("⚠️  Failed to init WGPU for window {}: {}", i, e);
                continue;
            }

            // 预热渲染（确保缓存初始化）
            let egui_ctx = self.window_manager.windows[i].egui_ctx.clone();
            self.ensure_backgrounds_and_cache(&egui_ctx, i);

            let render_data = self.prepare_render_data(i);
            if let Err(e) = self.render(i, &render_data) {
                eprintln!("⚠️  Warmup render error for window {}: {}", i, e);
            }
        }

        // 现在显示所有窗口
        for window_info in &self.window_manager.windows {
            window_info.window.set_visible(true);
            window_info.window.request_redraw();
        }

        self.pres_guard = platform::start_presentation();
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
        // Winit 推荐的渲染模式：在事件循环空闲时请求重绘
        // 拖拽时或有选择框时，持续请求重绘所有窗口
        if self.state.dragging {
            self.window_manager.request_redraw_all();
        }
    }
}
