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

    pub fn initialize_windows_with_layouts(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        attrs: &winit::window::WindowAttributes,
        monitor_layouts: Option<&[crate::MonitorLayout]>,
    ) {
        if self.has_windows() {
            return;
        }

        // 如果提供了显示器布局信息，使用它；否则使用 winit 的信息
        if let Some(layouts) = monitor_layouts {
            // 使用来自 platform_mac 的精确布局信息
            for layout in layouts {
                let attrs = attrs.clone();
                let physical_size = winit::dpi::PhysicalSize::new(layout.width, layout.height);

                match event_loop.create_window(attrs) {
                    Ok(window) => {
                        let _ = window.request_inner_size(physical_size);
                        window.set_outer_position(winit::dpi::Position::Physical(
                            winit::dpi::PhysicalPosition::new(layout.x, layout.y),
                        ));

                        let info = WindowInfo::new(
                            window,
                            physical_size,
                            layout.scale_factor,
                            layout.x,
                            layout.y,
                        );
                        self.windows.push(info);
                    }
                    Err(e) => {
                        eprintln!("无法为显示器创建窗口：{}", e);
                    }
                }
            }
        } else {
            // 回退到 winit 的显示器信息
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
