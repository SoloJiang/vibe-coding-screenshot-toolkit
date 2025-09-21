// Windows platform implementation for screenshot toolkit
#![allow(unexpected_cfgs)]

use anyhow::Result;
use chrono::Utc;
use screenshot_core::Result as CoreResult;
use screenshot_core::{Frame, FrameSet, PixelFormat, Screenshot};
use services::Clipboard;
use std::sync::Arc;
use ui_overlay as _; // 引入 crate 以便泛型约束解析
use uuid::Uuid;

/// Windows 剪贴板实现：写入 PNG 数据到 Windows 剪贴板
pub struct WinClipboard;

impl Clipboard for WinClipboard {
    fn write_image(&self, _bytes: &[u8]) -> CoreResult<()> {
        // TODO: 实现 Windows 剪贴板 PNG 写入
        // 可以使用 clipboard-win 或类似的 crate
        Err(screenshot_core::Error::new(
            screenshot_core::ErrorKind::Clipboard,
            "Windows clipboard not implemented yet",
        ))
    }
}

/// Windows 捕获器：专注于交互式框选截图，支持多显示器和跨显示器选择
pub struct WinCapturer;

impl WinCapturer {
    /// 交互式截图捕获 - Windows 实现
    /// 使用 Windows API 进行屏幕捕获，集成 ui_overlay 进行区域选择
    pub fn capture_region_interactive_custom(
        _selector: &dyn ui_overlay::RegionSelector,
    ) -> Result<Screenshot> {
        use infra::metrics;
        let _timer = metrics::start_timer(
            "interactive_duration_us",
            &[1000, 5000, 10000, 50000, 100000, 500000, 1000000],
        );
        metrics::counter("interactive_start").inc();

        // TODO: 实现 Windows 屏幕捕获
        // 1. 使用 Windows API (GetDisplayMonitors, BitBlt 等) 获取显示器信息
        // 2. 捕获屏幕图像作为背景
        // 3. 调用 ui_overlay 进行区域选择
        // 4. 根据选择结果裁剪图像

        anyhow::bail!("Windows interactive capture not implemented yet")
    }

    #[allow(dead_code)]
    fn build_screenshot(width: u32, height: u32, rgba: Vec<u8>) -> Screenshot {
        let frame = Frame {
            width,
            height,
            pixel_format: PixelFormat::Rgba8,
            bytes: Arc::from(rgba.into_boxed_slice()),
        };
        let fs = FrameSet {
            primary: frame.clone(),
            all: vec![frame],
        };
        Screenshot {
            id: Uuid::now_v7(),
            raw: Arc::new(fs),
            scale: 1.0,
            created_at: Utc::now(),
        }
    }
}

/// Windows 下的全屏截图实现 (备用功能)
impl WinCapturer {
    pub fn capture_full() -> Result<Screenshot> {
        // TODO: 实现 Windows 全屏截图
        anyhow::bail!("Windows full screen capture not implemented yet")
    }

    pub fn capture_region(_x: u32, _y: u32, _width: u32, _height: u32) -> Result<Screenshot> {
        // TODO: 实现 Windows 区域截图
        anyhow::bail!("Windows region capture not implemented yet")
    }
}
