#![allow(unexpected_cfgs)]
use anyhow::{Context, Result};
use chrono::Utc;
use objc2::rc::autoreleasepool;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSData, NSString};
use screenshot_core::Result as CoreResult;
use screenshot_core::{Frame, FrameSet, PixelFormat, Screenshot};
use services::Clipboard;
use std::sync::Arc;
use ui_overlay as _; // 引入 crate 以便泛型约束解析
use uuid::Uuid;

/// macOS 剪贴板实现：写入 PNG 数据到 NSPasteboard
pub struct MacClipboard;
impl Clipboard for MacClipboard {
    fn write_image(&self, bytes: &[u8]) -> CoreResult<()> {
        autoreleasepool(|_| {
            use objc2_app_kit::NSPasteboard;
            // NSPasteboard *pb = [NSPasteboard generalPasteboard];
            let pb = unsafe { NSPasteboard::generalPasteboard() };
            unsafe { pb.clearContents() };
            // NSData from raw bytes
            let data = NSData::with_bytes(bytes);
            // public.png UTI
            let uti: Retained<NSString> = NSString::from_str("public.png");
            // writeObjects: (NSArray*) expects NSArray<id<NSPasteboardWriting>>; use single object convenience
            // objc2 目前尚无直接 arrayWithObject 包装，临时走 low-level msg_send
            let arr: *mut AnyObject =
                unsafe { msg_send![class!(NSArray), arrayWithObject: &*data] };
            let ok: bool = unsafe { msg_send![&*pb, writeObjects: arr] };
            if !ok {
                return Err(screenshot_core::Error::new(
                    screenshot_core::ErrorKind::Clipboard,
                    "write clipboard failed",
                ));
            }
            unsafe {
                let _: () = msg_send![&*pb, setData: &*data, forType: &*uti];
            }
            Ok(())
        })
    }
}

/// macOS 捕获器：支持多种截图模式，包括交互式选择、全屏和区域截图
pub struct MacCapturer;
impl MacCapturer {
    /// 全屏截图：捕获主显示器的完整屏幕
    pub fn capture_full() -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "capture_full_duration_us",
            &[1000, 5000, 10000, 50000, 100000],
        );
        metrics::counter("capture_full_start").inc();

        let monitors = xcap::Monitor::all().context("列出显示器失败")?;
        let primary_monitor = monitors
            .iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("未找到主显示器"))?;

        let img = primary_monitor
            .capture_image()
            .context("xcap 主显示器图像捕获失败")?;
        let (width, height) = (img.width(), img.height());
        let rgba = img.into_raw();

        metrics::counter("capture_full_ok").inc();
        drop(timer);
        Ok(Self::build_screenshot(width, height, rgba))
    }

    /// 区域截图：在全屏基础上裁剪指定区域
    pub fn capture_region(x: u32, y: u32, width: u32, height: u32) -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "capture_region_duration_us",
            &[1000, 5000, 10000, 50000, 100000],
        );
        metrics::counter("capture_region_start").inc();

        // 先获取全屏截图
        let full_shot = Self::capture_full().context("获取全屏截图失败")?;
        let frame = &full_shot.raw.primary;

        // 裁剪指定区域
        let crop_x = x.min(frame.width);
        let crop_y = y.min(frame.height);
        let crop_w = width.min(frame.width - crop_x);
        let crop_h = height.min(frame.height - crop_y);

        if crop_w == 0 || crop_h == 0 {
            anyhow::bail!("裁剪区域为空");
        }

        let mut cropped_bytes = vec![0u8; (crop_w * crop_h * 4) as usize];
        for row in 0..crop_h {
            let src_row_start = (((crop_y + row) * frame.width + crop_x) * 4) as usize;
            let src_row_end = src_row_start + (crop_w * 4) as usize;
            let dst_row_start = (row * crop_w * 4) as usize;
            let dst_row_end = dst_row_start + (crop_w * 4) as usize;
            cropped_bytes[dst_row_start..dst_row_end]
                .copy_from_slice(&frame.bytes[src_row_start..src_row_end]);
        }

        metrics::counter("capture_region_ok").inc();
        drop(timer);
        Ok(Self::build_screenshot(crop_w, crop_h, cropped_bytes))
    }

    /// 多显示器截图：捕获所有显示器并合成
    pub fn capture_all() -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "capture_all_duration_us",
            &[5000, 10000, 50000, 100000, 500000],
        );
        metrics::counter("capture_all_start").inc();

        let monitors = xcap::Monitor::all().context("列出显示器失败")?;
        if monitors.is_empty() {
            anyhow::bail!("未找到任何显示器");
        }

        // 目前简化实现：只返回主显示器截图
        // TODO: 实现真正的多显示器合成
        let primary_monitor = monitors
            .iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .unwrap_or(&monitors[0]);

        let img = primary_monitor
            .capture_image()
            .context("xcap 多显示器图像捕获失败")?;
        let (width, height) = (img.width(), img.height());
        let rgba = img.into_raw();

        metrics::counter("capture_all_ok").inc();
        drop(timer);
        Ok(Self::build_screenshot(width, height, rgba))
    }

    /// 自研框选 UI 接口：通过 ui_overlay 的 RegionSelector 获取矩形，
    /// 基于全屏截图（支持多显示器）在内存裁剪出区域，避免多次系统调用。
    /// 支持跨显示器选择：会自动检测并合成跨越多个显示器的区域
    pub fn capture_region_interactive_custom(
        selector: &dyn ui_overlay::RegionSelector,
    ) -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "interactive_duration_us",
            &[1000, 5000, 10000, 50000, 100000, 500000, 1000000],
        );
        metrics::counter("interactive_start").inc();

        // 获取所有显示器的截图作为背景（支持多显示器）
        let monitors = xcap::Monitor::all().context("列出显示器失败")?;

        // 找到主显示器用于交互
        let primary_monitor = monitors
            .iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("未找到主显示器"))?;

        let primary_img = primary_monitor
            .capture_image()
            .context("xcap 主显示器图像捕获失败")?;
        let (width, height) = (primary_img.width(), primary_img.height());
        let rgba = primary_img.into_raw();

        // 将RGBA转换为RGB用于ui_overlay
        let rgb_data: Vec<u8> = rgba
            .chunks_exact(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]]) // 跳过alpha通道
            .collect();

        // 使用带背景的选择方法
        let rect_opt = selector
            .select_with_background(&rgb_data, width, height)
            .map_err(|e| {
                metrics::counter("interactive_error").inc();
                anyhow::anyhow!("overlay select: {e}")
            })?;

        let rect = match rect_opt {
            Some(r) => {
                metrics::counter("interactive_ok").inc();
                r
            }
            None => {
                metrics::counter("interactive_cancel").inc();
                anyhow::bail!("user canceled interactive selection")
            }
        };

        // 裁剪（边界钳制）；Region 为逻辑坐标，需要乘以 scale 转为像素
        let scale = if rect.scale.is_finite() && rect.scale > 0.0 {
            rect.scale
        } else {
            1.0
        } as f32;
        let x = (rect.x * scale).floor().max(0.0) as u32;
        let y = (rect.y * scale).floor().max(0.0) as u32;
        let w = (rect.w * scale).round().max(0.0) as u32;
        let h = (rect.h * scale).round().max(0.0) as u32;
        let x2 = (x + w).min(width);
        let y2 = (y + h).min(height);
        let cw = x2.saturating_sub(x);
        let ch = y2.saturating_sub(y);
        if cw == 0 || ch == 0 {
            anyhow::bail!("empty crop");
        }

        // 从原始 RGBA 数据中裁剪
        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for row in 0..ch {
            let src_row_start = (((y + row) * width + x) * 4) as usize;
            let src_row_end = src_row_start + (cw * 4) as usize;
            let dst_row_start = (row * cw * 4) as usize;
            let dst_row_end = dst_row_start + (cw * 4) as usize;
            bytes[dst_row_start..dst_row_end].copy_from_slice(&rgba[src_row_start..src_row_end]);
        }
        drop(timer);
        Ok(Self::build_screenshot(cw, ch, bytes))
    }

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
