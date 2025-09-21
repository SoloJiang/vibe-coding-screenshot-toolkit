use crate::event_handler::{EventHandler, EventResult, SelectionState};
use crate::platform;
use crate::renderer::{Background, RenderContext, SelectionRenderer};
use crate::window_manager::WindowManager;
use crate::{OverlayError, Region, RegionSelector, Result as OverlayResult};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowLevel};

pub struct WinitRegionSelector {
    /// 复用的RGBA缓冲区，避免重复分配
    rgba_buffer: parking_lot::Mutex<Vec<u8>>,
}

pub struct SelectionApp {
    attrs: WindowAttributes,
    window_manager: WindowManager,
    pres_guard: Option<platform::PresentationGuard>,
    bg: Option<Vec<u8>>,
    bg_w: u32,
    bg_h: u32,
    bg_dim: Option<Vec<u8>>,
    state: SelectionState,
}

impl WinitRegionSelector {
    pub fn new() -> Self {
        Self {
            rgba_buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// 高效的RGB到RGBA转换，复用缓冲区
    fn convert_rgb_to_rgba(&self, rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
        let required_size = (width as usize) * (height as usize) * 4;
        let mut buffer = self.rgba_buffer.lock();

        // 只有在需要更大空间时才重新分配
        if buffer.len() < required_size {
            buffer.resize(required_size, 0);
        }

        // 就地转换，复用现有内存
        for (i, chunk) in rgb.chunks_exact(3).enumerate() {
            let base = i * 4;
            if base + 3 < buffer.len() {
                buffer[base] = chunk[0];
                buffer[base + 1] = chunk[1];
                buffer[base + 2] = chunk[2];
                buffer[base + 3] = 255;
            }
        }

        // 返回所需大小的切片副本
        buffer[..required_size].to_vec()
    }

    fn run_selector(
        &self,
        bg_rgb: Option<&[u8]>,
        bg_w: u32,
        bg_h: u32,
        virtual_bounds: Option<(i32, i32, u32, u32)>, // (min_x, min_y, width, height)
    ) -> crate::MaybeRegion {
        // 使用优化的RGB到RGBA转换
        let bg_rgba: Option<Vec<u8>> = bg_rgb.map(|rgb| self.convert_rgb_to_rgba(rgb, bg_w, bg_h));

        let event_loop =
            EventLoop::new().map_err(|e| OverlayError::Internal(format!("event loop: {e}")))?;
        let attrs = WindowAttributes::default()
            // 置顶，防止被其他窗口遮挡
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(false)
            // 先隐藏窗口，预热渲染后再显示，避免首次交互卡顿
            .with_visible(false);
        let mut app = SelectionApp {
            attrs,
            window_manager: WindowManager::new(),
            pres_guard: None,
            bg: bg_rgba,
            bg_w,
            bg_h,
            bg_dim: None,
            state: SelectionState::new(virtual_bounds),
        };

        if let Err(e) = event_loop.run_app(&mut app) {
            return Err(OverlayError::Internal(format!("event loop run: {e}")));
        }

        Ok(app.state.result)
    }
}

impl RegionSelector for WinitRegionSelector {
    fn select(&self) -> OverlayResult<Region> {
        match self.run_selector(None, 0, 0, None)? {
            Some(r) => Ok(r),
            None => Err(OverlayError::Cancelled),
        }
    }

    fn select_with_background(&self, rgb: &[u8], width: u32, height: u32) -> crate::MaybeRegion {
        self.run_selector(Some(rgb), width, height, None)
    }

    fn select_with_virtual_background(
        &self,
        rgb: &[u8],
        width: u32,
        height: u32,
        virtual_bounds: (i32, i32, u32, u32),
        _display_offset: (i32, i32),
    ) -> crate::MaybeRegion {
        self.run_selector(Some(rgb), width, height, Some(virtual_bounds))
    }
}

impl SelectionApp {
    fn render_window_by_index(&mut self, window_index: usize) {
        if window_index >= self.window_manager.windows.len() {
            return;
        }

        // 先检查是否需要创建 Pixels
        if self.window_manager.windows[window_index].pixels.is_none() {
            let size_px = self.window_manager.windows[window_index].size_px;
            if size_px.width == 0 || size_px.height == 0 {
                return;
            }

            // 为窗口创建 Pixels
            let window_ref: &'static Window = unsafe {
                &*(self.window_manager.windows[window_index].window.as_ref() as *const Window)
            };
            let surface = SurfaceTexture::new(size_px.width, size_px.height, window_ref);
            match Pixels::new(size_px.width, size_px.height, surface) {
                Ok(p) => {
                    self.window_manager.windows[window_index].pixels = Some(p);
                }
                Err(_) => return,
            }
        }

        // 提取需要的数据以避免借用冲突
        let size_px = self.window_manager.windows[window_index].size_px;
        let virtual_x = self.window_manager.windows[window_index].virtual_x;
        let virtual_y = self.window_manager.windows[window_index].virtual_y;
        let virtual_bounds = self.state.virtual_bounds;
        let bg_w = self.bg_w;
        let bg_h = self.bg_h;
        // 优化：使用引用而非克隆大数据
        let bg_dim_ref = self.bg_dim.as_ref();

        // 优化：提前计算选择区域
        let (x0c, y0c, x1c, y1c) = self.state.calculate_selection_rect();

        // 早期退出：如果选择区域很小且不在拖动状态，跳过部分渲染
        let selection_exists = (self.state.dragging || self.state.alt_down)
            && ((x1c - x0c).abs() > 1.0 && (y1c - y0c).abs() > 1.0);

        let window_needs_selection = if virtual_bounds.is_some() {
            // 使用事件处理器检查交集
            EventHandler::selection_intersects_window(
                &self.state,
                virtual_x,
                virtual_y,
                size_px.width,
                size_px.height,
            )
        } else {
            true
        };

        // 现在可以安全地访问 pixels
        let window_info = &mut self.window_manager.windows[window_index];
        let pixels = window_info.pixels.as_mut().unwrap();
        let frame = pixels.frame_mut();

        // 创建渲染上下文
        let mut ctx = RenderContext {
            frame,
            size_px,
            virtual_x,
            virtual_y,
            virtual_bounds,
        };

        // 渲染背景
        if let Some(bg_data) = bg_dim_ref {
            let bg = Background {
                data: bg_data,
                width: bg_w,
                height: bg_h,
            };
            SelectionRenderer::render_virtual_background(&mut ctx, &bg);
        } else {
            // 黑色背景
            SelectionRenderer::render_solid_background(ctx.frame, 0, 0, 0, 255);
        }

        // 在选择区域内恢复原始背景（如果有选择且有原始背景）
        if selection_exists && window_needs_selection {
            if let Some(original_bg_data) = &self.bg {
                let original_bg = Background {
                    data: original_bg_data,
                    width: bg_w,
                    height: bg_h,
                };
                let selection = (x0c as i32, y0c as i32, x1c as i32, y1c as i32);
                SelectionRenderer::render_selection_background(&mut ctx, &original_bg, selection);
            }

            // 渲染选择框边框
            let selection = (x0c as i32, y0c as i32, x1c as i32, y1c as i32);
            SelectionRenderer::render_selection_border(&mut ctx, selection);
        }
        let _ = pixels.render();
    }

    fn request_redraw_all(&mut self) {
        if self.state.should_throttle_redraw() {
            return;
        }

        self.state.mark_redraw_requested();
        self.window_manager.request_redraw_all();
    }

    fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // 找到对应的窗口
        let window_index = self.window_manager.find_window_index(window_id);
        let Some(window_index) = window_index else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                self.state.result = None;
                if let Some(g) = self.pres_guard.take() {
                    platform::end_presentation(g);
                }
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => match EventHandler::handle_keyboard_event(&mut self.state, key_event) {
                EventResult::Continue(need_redraw) => {
                    if need_redraw {
                        self.request_redraw_all();
                    }
                }
                EventResult::Exit => {
                    event_loop.exit();
                }
                EventResult::Finish => {
                    if let Some(region) = self.create_region(window_index) {
                        self.state.result = Some(region);
                        event_loop.exit();
                    }
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
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
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.window_manager.windows[window_index].update_scale(scale_factor);
                self.request_redraw_all();
            }
            WindowEvent::Resized(new_size) => {
                self.window_manager.windows[window_index].update_size(new_size);
                self.request_redraw_all();
            }
            WindowEvent::RedrawRequested => {
                self.state.clear_redraw_pending();
                self.render_window_by_index(window_index);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => match EventHandler::handle_mouse_input(&mut self.state, button, button_state) {
                EventResult::Continue(need_redraw) => {
                    if need_redraw {
                        self.request_redraw_all();
                    }
                }
                EventResult::Exit => {
                    event_loop.exit();
                }
                EventResult::Finish => {
                    if let Some(region) = self.create_region(window_index) {
                        self.state.result = Some(region);
                        event_loop.exit();
                    }
                }
            },
            _ => {}
        }
    }

    fn create_region(&self, window_index: usize) -> Option<Region> {
        let scale_out = if self.state.virtual_bounds.is_some() {
            1.0
        } else {
            self.window_manager.windows[window_index].scale as f32
        };

        let region = self.state.to_region(scale_out)?;

        // 添加调试信息
        if let Some((virt_min_x, virt_min_y, virt_w, virt_h)) = self.state.virtual_bounds {
            let window_info = &self.window_manager.windows[window_index];
            println!(
                "🐛 调试：虚拟桌面模式 边界=({},{},{},{})",
                virt_min_x, virt_min_y, virt_w, virt_h
            );
            println!(
                "🐛 调试：当前窗口虚拟位置=({},{}) 尺寸=({},{})",
                window_info.virtual_x,
                window_info.virtual_y,
                window_info.size_px.width,
                window_info.size_px.height
            );
            println!(
                "🐛 调试：选择区域 x={}, y={}, w={}, h={}",
                region.x, region.y, region.w, region.h
            );
        }

        println!(
            "📏 UI层返回Region: x={}, y={}, w={}, h={}, scale={}",
            region.x, region.y, region.w, region.h, region.scale
        );
        Some(region)
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.window_manager.windows.is_empty() {
            return;
        }

        self.window_manager
            .initialize_windows(event_loop, &self.attrs);

        // 创建演示守护程序（只需要一次）
        if !self.window_manager.windows.is_empty() {
            self.pres_guard = platform::start_presentation();

            // 预计算变暗背景
            if self.bg_dim.is_none() {
                if let Some(bg) = &self.bg {
                    let mut dim = vec![0u8; bg.len()];
                    let a = 90u8 as u16;
                    for (i, chunk) in bg.chunks_exact(4).enumerate() {
                        let r = chunk[0] as u16;
                        let g = chunk[1] as u16;
                        let b = chunk[2] as u16;
                        let base = i * 4;
                        dim[base] = ((r * (255 - a)) / 255) as u8;
                        dim[base + 1] = ((g * (255 - a)) / 255) as u8;
                        dim[base + 2] = ((b * (255 - a)) / 255) as u8;
                        dim[base + 3] = 255;
                    }
                    self.bg_dim = Some(dim);
                }
            }

            // 为每个窗口初始化 Pixels，预热渲染，然后再显示
            for i in 0..self.window_manager.windows.len() {
                // 初始化 Pixels
                if self.window_manager.windows[i].pixels.is_none() {
                    let size_px = self.window_manager.windows[i].size_px;
                    if size_px.width > 0 && size_px.height > 0 {
                        let window_ref: &'static Window = unsafe {
                            &*(self.window_manager.windows[i].window.as_ref() as *const Window)
                        };
                        let surface =
                            SurfaceTexture::new(size_px.width, size_px.height, window_ref);
                        if let Ok(p) = Pixels::new(size_px.width, size_px.height, surface) {
                            self.window_manager.windows[i].pixels = Some(p);
                        }
                    }
                }

                // 预热渲染 - 先渲染一帧再显示窗口，避免闪动
                self.render_window_by_index(i);
                self.window_manager.windows[i].window.set_visible(true);
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
        // 空闲时不强制重绘，按需在输入或尺寸变化时 request_redraw
    }
}
