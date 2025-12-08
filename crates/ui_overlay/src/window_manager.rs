/// 窗口管理器模块
///
/// 负责管理多显示器环境下的窗口创建、布局和生命周期。
///
/// # 多显示器支持
///
/// 该模块自动检测所有可用显示器，并为每个显示器创建一个全屏覆盖窗口。
/// 窗口坐标使用虚拟桌面坐标系，支持跨显示器的区域选择。
///
/// # HiDPI 支持
///
/// 正确处理不同 scale_factor 的显示器：
/// - scale = 1.0: 使用 LogicalSize（避免 macOS winit 的尺寸缩小问题）
/// - scale ≠ 1.0: 使用 PhysicalSize（Retina 等高 DPI 显示器）
pub use crate::window_info::WindowInfo;

/// 窗口管理器
///
/// 维护所有显示器窗口的集合，提供统一的窗口操作接口。
pub struct WindowManager {
    /// 所有显示器的窗口信息列表，按显示器索引排序
    pub windows: Vec<WindowInfo>,
}

/// 窗口尺寸验证容差（像素）- 仅在 debug 模式下使用
#[cfg(debug_assertions)]
const SIZE_VALIDATION_TOLERANCE: i32 = 1;

impl WindowManager {
    /// 创建新的窗口管理器实例
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    /// 根据窗口 ID 查找窗口索引
    ///
    /// # Arguments
    /// * `window_id` - winit 窗口标识符
    ///
    /// # Returns
    /// 窗口在列表中的索引，如果未找到则返回 None
    pub fn find_window_index(&self, window_id: winit::window::WindowId) -> Option<usize> {
        self.windows.iter().position(|w| w.window.id() == window_id)
    }

    /// 检查是否已有窗口
    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    /// 根据显示器特性选择合适的窗口尺寸类型
    ///
    /// # Arguments
    /// * `physical_size` - 显示器的物理尺寸
    /// * `scale_factor` - 显示器的缩放因子
    ///
    /// # Returns
    /// 适合该显示器的窗口尺寸规格
    fn determine_window_size(
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> winit::dpi::Size {
        // 关键修复：必须使用 LogicalSize，让 winit 自己处理 scale
        //
        // 问题分析：
        // 1. monitor.size() 返回的是实际物理像素尺寸（如 1080x1920）
        // 2. 如果我们用 PhysicalSize(1080, 1920)，winit 会创建 1080x1920 像素的窗口
        // 3. 但如果窗口的 scale 是某个值，最终显示的"逻辑点"会被按 scale 缩放
        // 4. 我们需要的是逻辑尺寸 = 物理尺寸 / scale
        //
        // 解决方案：使用 LogicalSize，逻辑尺寸 = 物理尺寸 / monitor.scale
        //           这样 winit 会根据窗口的实际 scale 计算出正确的物理尺寸
        let logical_width = physical_size.width as f64 / scale_factor;
        let logical_height = physical_size.height as f64 / scale_factor;

        winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(logical_width, logical_height))
    }

    /// 验证窗口尺寸是否符合预期
    ///
    /// 检查窗口实际尺寸是否与显示器报告的尺寸匹配。
    /// 根据平台和运行模式的不同，winit 可能返回以下几种尺寸：
    /// - 物理尺寸（physical）
    /// - 逻辑尺寸（logical = physical / scale）
    /// - 缩放物理尺寸（scaled physical = physical × scale）
    ///
    /// # Arguments
    /// * `window_index` - 窗口索引（用于日志）
    /// * `actual_size` - 窗口实际尺寸
    /// * `expected_size` - 显示器报告的尺寸
    /// * `scale_factor` - 显示器缩放因子
    #[cfg(debug_assertions)]
    fn validate_window_size(
        window_index: usize,
        actual_size: winit::dpi::PhysicalSize<u32>,
        expected_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        // 计算期望的逻辑尺寸
        let expected_logical_w = (expected_size.width as f64 / scale_factor) as u32;
        let expected_logical_h = (expected_size.height as f64 / scale_factor) as u32;

        // 检查是否匹配物理尺寸
        let width_diff = (actual_size.width as i32 - expected_size.width as i32).abs();
        let height_diff = (actual_size.height as i32 - expected_size.height as i32).abs();
        let matches_physical =
            width_diff <= SIZE_VALIDATION_TOLERANCE && height_diff <= SIZE_VALIDATION_TOLERANCE;

        // 检查是否匹配逻辑尺寸
        let width_diff_logical = (actual_size.width as i32 - expected_logical_w as i32).abs();
        let height_diff_logical = (actual_size.height as i32 - expected_logical_h as i32).abs();
        let matches_logical = width_diff_logical <= SIZE_VALIDATION_TOLERANCE
            && height_diff_logical <= SIZE_VALIDATION_TOLERANCE;

        // 检查是否匹配 scaled physical（物理尺寸 × scale_factor）
        let expected_scaled_w = (expected_size.width as f64 * scale_factor).round() as u32;
        let expected_scaled_h = (expected_size.height as f64 * scale_factor).round() as u32;
        let width_diff_scaled = (actual_size.width as i32 - expected_scaled_w as i32).abs();
        let height_diff_scaled = (actual_size.height as i32 - expected_scaled_h as i32).abs();
        let matches_scaled = width_diff_scaled <= SIZE_VALIDATION_TOLERANCE
            && height_diff_scaled <= SIZE_VALIDATION_TOLERANCE;

        // 检查是否为预期尺寸的整数倍（处理隐式缩放）
        // 某些情况下 winit 报告的 scale_factor 不准确，窗口会按实际显示器缩放创建
        let mut matches_multiple = false;
        let mut detected_scale = 1.0;
        for test_scale in [2.0, 1.5, 2.5, 3.0] {
            let test_w = (expected_size.width as f64 * test_scale).round() as u32;
            let test_h = (expected_size.height as f64 * test_scale).round() as u32;
            let w_diff = (actual_size.width as i32 - test_w as i32).abs();
            let h_diff = (actual_size.height as i32 - test_h as i32).abs();
            // 允许更大的容差（可能有菜单栏等）
            if w_diff <= SIZE_VALIDATION_TOLERANCE * 2 && h_diff <= SIZE_VALIDATION_TOLERANCE * 2 {
                matches_multiple = true;
                detected_scale = test_scale;
                break;
            }
        }

        if matches_physical {
            tracing::debug!(
                "[Window {}] ✅ 窗口尺寸匹配物理尺寸 ({}x{})",
                window_index,
                actual_size.width,
                actual_size.height
            );
        } else if matches_logical {
            tracing::debug!(
                "[Window {}] ✅ 窗口尺寸匹配逻辑尺寸 ({}x{}, physical={}x{})",
                window_index,
                actual_size.width,
                actual_size.height,
                expected_size.width,
                expected_size.height
            );
        } else if matches_scaled {
            tracing::debug!(
                "[Window {}] ✅ 窗口尺寸匹配 scaled physical ({}x{}, scale={:.2})",
                window_index,
                actual_size.width,
                actual_size.height,
                scale_factor
            );
        } else if matches_multiple {
            tracing::debug!(
                "[Window {}] ✅ 窗口尺寸匹配隐式缩放 ({}x{}, 检测到实际scale={:.1}x，报告scale={:.2})",
                window_index,
                actual_size.width,
                actual_size.height,
                detected_scale,
                scale_factor
            );
        } else {
            // 只有在所有尺寸都不匹配时才警告
            tracing::warn!(
                "[Window {}] ⚠️  窗口尺寸异常! Monitor={}x{} (scale={:.2}), 实际={}x{}, 预期physical={}x{} 或 logical={}x{} 或 scaled={}x{}",
                window_index,
                expected_size.width,
                expected_size.height,
                scale_factor,
                actual_size.width,
                actual_size.height,
                expected_size.width,
                expected_size.height,
                expected_logical_w,
                expected_logical_h,
                expected_scaled_w,
                expected_scaled_h
            );
        }
    }

    /// 为所有可用显示器初始化窗口
    ///
    /// 自动检测所有显示器并为每个显示器创建一个全屏覆盖窗口。
    /// 窗口会根据显示器的 DPI 设置正确的尺寸类型。
    ///
    /// # Arguments
    /// * `event_loop` - winit 事件循环的活动实例
    /// * `base_attrs` - 基础窗口属性（将被克隆并为每个显示器定制）
    ///
    /// # Note
    /// 如果已有窗口，该方法会直接返回，避免重复初始化。
    pub fn initialize_windows(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        base_attrs: &winit::window::WindowAttributes,
    ) {
        if self.has_windows() {
            return;
        }

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
                    "[Winit Monitor {}] position() 返回: ({}, {}), size() 返回: {}x{} (类型声称是 Physical)",
                    i,
                    pos.x,
                    pos.y,
                    size.width,
                    size.height
                );
                tracing::debug!("[Winit Monitor {}] scale_factor: {}", i, scale);
                tracing::debug!(
                    "[Winit Monitor {}] 如果 position 是逻辑坐标，物理坐标应该是: ({}, {})",
                    i,
                    (pos.x as f64 * scale).round() as i32,
                    (pos.y as f64 * scale).round() as i32
                );
                tracing::debug!(
                    "[Winit Monitor {}] 如果 size 是逻辑尺寸，物理尺寸应该是: {}x{}",
                    i,
                    (size.width as f64 * scale).round() as u32,
                    (size.height as f64 * scale).round() as u32
                );
            }
        }

        for (window_index, monitor) in monitors.iter().enumerate() {
            let physical_size = monitor.size();
            let physical_position = monitor.position();
            let scale = monitor.scale_factor();

            #[cfg(debug_assertions)]
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::info!(
                "[创建窗口 {}] Monitor 信息: physical_size={}x{}, physical_position=({}, {}), scale={}",
                window_index,
                physical_size.width,
                physical_size.height,
                physical_position.x,
                physical_position.y,
                scale
            );

            // 根据显示器特性选择合适的窗口尺寸类型
            let size_spec = Self::determine_window_size(physical_size, scale);

            #[cfg(debug_assertions)]
            {
                let logical_w = physical_size.width as f64 / scale;
                let logical_h = physical_size.height as f64 / scale;
                tracing::info!(
                    "[创建窗口 {}] 窗口尺寸类型: Logical (物理尺寸={}x{} / monitor.scale={} = 逻辑尺寸={:.0}x{:.0})",
                    window_index,
                    physical_size.width,
                    physical_size.height,
                    scale,
                    logical_w,
                    logical_h
                );
            }

            // 创建窗口属性：指定尺寸和位置
            let attrs = base_attrs
                .clone()
                .with_inner_size(size_spec)
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

                        // 验证窗口尺寸是否符合预期
                        Self::validate_window_size(
                            window_index,
                            actual_inner_size,
                            physical_size,
                            scale,
                        );
                    }

                    // 重要：从 window 本身获取实际的 scale_factor 和位置
                    // 因为 winit 在 macOS 上 monitor 返回的值可能不准确
                    let actual_scale = window.scale_factor();

                    // 检查窗口的实际位置
                    let actual_position = window.outer_position().ok();

                    tracing::info!(
                        "[创建窗口 {}] Scale 对比: monitor.scale={}, window.scale={}",
                        window_index,
                        scale,
                        actual_scale
                    );

                    // 决定使用哪个位置
                    // 注意：在 macOS 上，monitor.position() 和 window.outer_position() 可能使用不同的坐标系统
                    // monitor.position() 在 high DPI 下可能返回逻辑坐标
                    // window.outer_position() 也可能返回逻辑坐标
                    // 为了保持一致性，始终使用 monitor.position() 并根据 scale 转换
                    let final_position = if let Some(pos) = actual_position {
                        tracing::info!(
                            "[创建窗口 {}] 位置对比: monitor.position=({}, {}), window.outer_position=({}, {}), scale={}",
                            window_index,
                            physical_position.x,
                            physical_position.y,
                            pos.x,
                            pos.y,
                            actual_scale
                        );

                        // 始终使用 monitor.position()，以保证一致性
                        // 因为背景画布是基于 monitor 信息构建的
                        tracing::info!(
                            "[创建窗口 {}] 使用 monitor.position 保持坐标系统一致",
                            window_index
                        );
                        (physical_position.x, physical_position.y)
                    } else {
                        (physical_position.x, physical_position.y)
                    };

                    let info = WindowInfo::new(
                        window,
                        actual_inner_size,
                        actual_scale, // 使用 window 的实际 scale
                        final_position.0,
                        final_position.1,
                    );

                    tracing::info!("[创建窗口 {}] ✅ WindowInfo 创建完成:", window_index);
                    tracing::info!(
                        "  └─ size_px (物理尺寸): {}x{}",
                        info.size_px.width,
                        info.size_px.height
                    );
                    tracing::info!(
                        "  └─ virtual_x, virtual_y (物理坐标): ({}, {})",
                        info.virtual_x,
                        info.virtual_y
                    );
                    tracing::info!("  └─ scale: {}", info.scale);
                    tracing::info!(
                        "  └─ 逻辑尺寸计算: {}x{} / {} = {}x{}",
                        info.size_px.width,
                        info.size_px.height,
                        info.scale,
                        (info.size_px.width as f64 / info.scale) as u32,
                        (info.size_px.height as f64 / info.scale) as u32
                    );
                    tracing::info!(
                        "  └─ 逻辑坐标计算: ({}, {}) / {} = ({}, {})",
                        info.virtual_x,
                        info.virtual_y,
                        info.scale,
                        (info.virtual_x as f64 / info.scale) as i32,
                        (info.virtual_y as f64 / info.scale) as i32
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
