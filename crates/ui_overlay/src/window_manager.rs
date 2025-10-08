/// 窗口管理器模块
///
/// 提供多窗口的统一管理功能
pub use crate::window_info::WindowInfo;

/// 窗口管理器
pub struct WindowManager {
    pub windows: Vec<WindowInfo>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    pub fn find_window_index(&self, window_id: winit::window::WindowId) -> Option<usize> {
        self.windows.iter().position(|w| w.window.id() == window_id)
    }

    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    pub fn initialize_windows(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        attrs: &winit::window::WindowAttributes,
    ) {
        if self.has_windows() {
            return;
        }

        let monitors: Vec<_> = event_loop.available_monitors().collect();

        for monitor in monitors {
            let physical_size = monitor.size();
            let scale = monitor.scale_factor();
            let position = monitor.position();
            let attrs = attrs.clone();

            match event_loop.create_window(attrs) {
                Ok(window) => {
                    let _ = window.request_inner_size(physical_size);
                    window.set_outer_position(winit::dpi::Position::Physical(
                        winit::dpi::PhysicalPosition::new(position.x, position.y),
                    ));

                    let info =
                        WindowInfo::new(window, physical_size, scale, position.x, position.y);
                    self.windows.push(info);
                }
                Err(e) => {
                    eprintln!("无法为显示器创建窗口：{}", e);
                }
            }
        }
    }

    pub fn request_redraw_all(&self) {
        for window_info in &self.windows {
            window_info.window.request_redraw();
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
