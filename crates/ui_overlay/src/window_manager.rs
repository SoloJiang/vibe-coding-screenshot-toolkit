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
        base_attrs: &winit::window::WindowAttributes,
    ) {
        if self.has_windows() {
            return;
        }

        // 使用 winit 作为唯一的显示器信息来源
        let monitors: Vec<_> = event_loop.available_monitors().collect();

        #[cfg(debug_assertions)]
        {
            tracing::debug!("=== Winit 显示器列表 ===");
            tracing::debug!("检测到 {} 个显示器", monitors.len());

            for (i, monitor) in monitors.iter().enumerate() {
                let size = monitor.size();
                let pos = monitor.position();
                let scale = monitor.scale_factor();
                let name = monitor.name().unwrap_or_else(|| format!("Unknown {}", i));

                tracing::debug!("[Winit Monitor {}] name: {}", i, name);
                tracing::debug!(
                    "[Winit Monitor {}] 物理位置: ({}, {}), 物理尺寸: {}x{}",
                    i,
                    pos.x,
                    pos.y,
                    size.width,
                    size.height
                );
                tracing::debug!("[Winit Monitor {}] scale_factor: {}", i, scale);
            }
        }

        for (window_index, monitor) in monitors.iter().enumerate() {
            let physical_size = monitor.size();
            let physical_position = monitor.position();
            let scale = monitor.scale_factor();

            // 在创建时就指定窗口的尺寸和位置
            let attrs = base_attrs
                .clone()
                .with_inner_size(physical_size)
                .with_position(winit::dpi::Position::Physical(physical_position));

            match event_loop.create_window(attrs) {
                Ok(window) => {
                    // 验证窗口实际尺寸
                    let actual_inner_size = window.inner_size();

                    #[cfg(debug_assertions)]
                    {
                        let actual_outer_size = window.outer_size();
                        let actual_position = window.outer_position().ok();

                        tracing::debug!("[Window {}] 创建成功", window_index);
                        tracing::debug!(
                            "[Window {}] Monitor报告: size={}x{}, pos=({}, {}), scale={}",
                            window_index,
                            physical_size.width,
                            physical_size.height,
                            physical_position.x,
                            physical_position.y,
                            scale
                        );
                        tracing::debug!(
                            "[Window {}] 实际inner: {}x{}",
                            window_index,
                            actual_inner_size.width,
                            actual_inner_size.height
                        );
                        tracing::debug!(
                            "[Window {}] 实际outer: {}x{}",
                            window_index,
                            actual_outer_size.width,
                            actual_outer_size.height
                        );
                        if let Some(pos) = actual_position {
                            tracing::debug!(
                                "[Window {}] 实际位置: ({}, {})",
                                window_index,
                                pos.x,
                                pos.y
                            );
                        }

                        // 检查尺寸差异，考虑 HiDPI 缩放
                        // 在 Retina 显示器上，实际窗口尺寸可能是逻辑尺寸 * backing_scale_factor
                        if actual_inner_size != physical_size {
                            let size_ratio =
                                actual_inner_size.width as f64 / physical_size.width as f64;

                            // 如果比例接近整数（如 2.0），这是 HiDPI 的正常行为
                            if (size_ratio - size_ratio.round()).abs() < 0.01 && size_ratio >= 1.0 {
                                tracing::debug!(
                                    "[Window {}] HiDPI 缩放: Monitor报告 {}x{}, 窗口实际 {}x{} (backing_scale ≈ {:.1}x)",
                                    window_index,
                                    physical_size.width,
                                    physical_size.height,
                                    actual_inner_size.width,
                                    actual_inner_size.height,
                                    size_ratio
                                );
                            } else {
                                // 只有在比例异常时才输出警告
                                tracing::warn!(
                                    "[Window {}] ⚠️  尺寸异常! Monitor报告 {}x{}, 窗口实际 {}x{} (比例: {:.2})",
                                    window_index,
                                    physical_size.width,
                                    physical_size.height,
                                    actual_inner_size.width,
                                    actual_inner_size.height,
                                    size_ratio
                                );
                            }
                        }
                    }

                    let info = WindowInfo::new(
                        window,
                        actual_inner_size,
                        scale,
                        physical_position.x,
                        physical_position.y,
                    );
                    self.windows.push(info);
                }
                Err(e) => {
                    eprintln!("[Window {}] ⚠️  无法为显示器创建窗口：{}", window_index, e);
                    #[cfg(debug_assertions)]
                    tracing::error!("[Window {}] 创建失败: {}", window_index, e);
                }
            }
        }

        #[cfg(debug_assertions)]
        {
            tracing::debug!("=== 窗口创建完成 ===");
            tracing::debug!("成功创建 {} 个窗口", self.windows.len());
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
