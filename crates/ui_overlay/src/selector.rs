/// 选择器入口模块
///
/// 提供基于 winit 的区域选择器实现
use crate::selection_app::SelectionApp;
use crate::{OverlayError, Region, RegionSelector, Result as OverlayResult};
use winit::event_loop::EventLoop;
use winit::window::{WindowAttributes, WindowLevel};

/// Winit 区域选择器
pub struct WinitRegionSelector {
    /// 复用的RGBA缓冲区，避免重复分配
    rgba_buffer: parking_lot::Mutex<Vec<u8>>,
}

impl WinitRegionSelector {
    pub fn new() -> Self {
        Self {
            rgba_buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// 高效的RGB到RGBA转换，复用缓冲区
    ///
    /// 性能优化：使用 rayon 并行处理转换
    /// - 小图像（< 512KB RGB）使用单线程
    /// - 大图像使用多线程并行处理
    fn convert_rgb_to_rgba(&self, rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
        use rayon::prelude::*;

        let required_size = (width as usize) * (height as usize) * 4;
        let mut buffer = self.rgba_buffer.lock();

        // 只有在需要更大空间时才重新分配
        if buffer.len() < required_size {
            buffer.resize(required_size, 0);
        }

        // 对于小图像，使用单线程避免并行开销
        const PARALLEL_THRESHOLD: usize = 512 * 1024; // 512KB RGB data

        if rgb.len() < PARALLEL_THRESHOLD {
            // 单线程处理小图像
            for (i, chunk) in rgb.chunks_exact(3).enumerate() {
                let base = i * 4;
                if base + 3 < buffer.len() {
                    buffer[base] = chunk[0];
                    buffer[base + 1] = chunk[1];
                    buffer[base + 2] = chunk[2];
                    buffer[base + 3] = 255;
                }
            }
        } else {
            // 并行处理大图像
            // 使用 par_chunks_exact 并行迭代 RGB 数据
            let buffer_slice = &mut buffer[..required_size];
            buffer_slice
                .par_chunks_exact_mut(4)
                .zip(rgb.par_chunks_exact(3))
                .for_each(|(dst, src)| {
                    dst[0] = src[0];
                    dst[1] = src[1];
                    dst[2] = src[2];
                    dst[3] = 255;
                });
        }

        // 返回所需大小的切片副本
        buffer[..required_size].to_vec()
    }

    fn run_selector(
        &self,
        bg_rgb: Option<&[u8]>,
        bg_w: u32,
        bg_h: u32,
        virtual_bounds: Option<(i32, i32, u32, u32)>,
        monitor_layouts: Option<&[crate::MonitorLayout]>,
    ) -> crate::MaybeRegion {
        // 转换背景数据
        let bg_rgba: Option<Vec<u8>> = bg_rgb.map(|rgb| self.convert_rgb_to_rgba(rgb, bg_w, bg_h));

        // 创建事件循环
        let event_loop =
            EventLoop::new().map_err(|e| OverlayError::Internal(format!("event loop: {e}")))?;

        // 配置窗口属性
        let attrs = WindowAttributes::default()
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_visible(false);

        // 创建应用程序
        let mut app =
            SelectionApp::new(attrs, bg_rgba, bg_w, bg_h, virtual_bounds, monitor_layouts);

        // 运行事件循环
        if let Err(e) = event_loop.run_app(&mut app) {
            return Err(OverlayError::Internal(format!("event loop run: {e}")));
        }

        Ok(app.state.result)
    }
}

impl RegionSelector for WinitRegionSelector {
    fn select(&self) -> OverlayResult<Region> {
        match self.run_selector(None, 0, 0, None, None)? {
            Some(r) => Ok(r),
            None => Err(OverlayError::Cancelled),
        }
    }

    fn select_with_background(&self, rgb: &[u8], width: u32, height: u32) -> crate::MaybeRegion {
        self.run_selector(Some(rgb), width, height, None, None)
    }

    fn select_with_virtual_background(
        &self,
        rgb: &[u8],
        width: u32,
        height: u32,
        virtual_bounds: (i32, i32, u32, u32),
        _display_offset: (i32, i32),
        monitor_layouts: Option<&[crate::MonitorLayout]>,
    ) -> crate::MaybeRegion {
        self.run_selector(
            Some(rgb),
            width,
            height,
            Some(virtual_bounds),
            monitor_layouts,
        )
    }
}
