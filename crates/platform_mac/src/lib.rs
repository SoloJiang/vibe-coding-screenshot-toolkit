#![allow(unexpected_cfgs)]
#[macro_use]
extern crate objc;
use anyhow::{Context, Result};
use chrono::Utc;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSData, NSString};
use objc::rc::autoreleasepool;
use objc::{class, msg_send, sel};
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
        autoreleasepool(|| {
            unsafe {
                let cls = objc::runtime::Class::get("NSPasteboard").unwrap();
                let pb: id = msg_send![cls, generalPasteboard];
                let _: () = msg_send![pb, clearContents];
                // NSData from bytes
                let data: id = NSData::dataWithBytes_length_(
                    nil,
                    bytes.as_ptr() as *const _,
                    bytes.len() as u64,
                );
                if data == nil {
                    return Ok(());
                }
                // UTI for png: public.png
                let uti = NSString::alloc(nil).init_str("public.png");
                let arr: id = msg_send![class!(NSArray), arrayWithObject:data];
                // writeObjects: expects NSArray of NSPasteboardWriting; NSData conforms
                let ok: bool = msg_send![pb, writeObjects: arr];
                if !ok {
                    return Err(screenshot_core::Error::new(
                        screenshot_core::ErrorKind::Clipboard,
                        "write clipboard failed",
                    ));
                }
                // Also set data for type explicitly (some apps rely on setData:forType:)
                let _: () = msg_send![pb, setData:data forType:uti];
            }
            Ok(())
        })
    }
}

/// macOS 捕获器：优先使用 xcap（基于 CG），若失败再回退系统 screencapture 命令。
/// 回退逻辑：xcap 报错或返回空 -> screencapture；二者都失败则返回错误。
/// 注意：首次运行需在“系统设置 -> 隐私与安全性 -> 屏幕录制”授予终端权限。
pub struct MacCapturer;
impl MacCapturer {
    /// 捕获主显示器截图（仅 xcap）。
    pub fn capture_full() -> Result<Screenshot> {
        Self::capture_with_xcap()
    }

    /// 捕获所有显示器；对单个显示器失败仅记录 warning，若全部失败返回错误。
    pub fn capture_all() -> Result<Vec<Screenshot>> {
        let mut shots = Vec::new();
        let monitors = xcap::Monitor::all().context("列出显示器失败")?;
        for (idx, m) in monitors.into_iter().enumerate() {
            match m.capture_image() {
                Ok(img) => {
                    let (w, h) = (img.width(), img.height());
                    let rgba = img.into_raw();
                    shots.push(Self::build_screenshot(w, h, rgba));
                }
                Err(e) => {
                    tracing::warn!(display_index = idx, error = %e, "捕获显示器失败，跳过该屏");
                }
            }
        }
        if shots.is_empty() {
            anyhow::bail!("所有显示器捕获均失败");
        }
        Ok(shots)
    }

    fn capture_with_xcap() -> Result<Screenshot> {
        // 遍历找到主显示器
        let monitors = xcap::Monitor::all().context("列出显示器失败 (xcap)")?;
        let display = monitors
            .into_iter()
            .find(|m| m.is_primary())
            .ok_or_else(|| anyhow::anyhow!("未找到主显示器"))?;
        let img = display
            .capture_image()
            .context("xcap 主显示器图像捕获失败")?; // RgbaImage
        let (width, height) = (img.width(), img.height());
        let rgba = img.into_raw();
        Ok(Self::build_screenshot(width, height, rgba))
    }

    // 移除 screencapture 相关路径：区域/交互均改为基于一次全屏 + 内存裁剪

    /// 非交互式区域截图：先获取一次全屏，再在内存中按像素裁剪。
    /// 注意：x,y,w,h 均为像素坐标（主屏）。越界会自动截断，空区域返回错误。
    pub fn capture_region(x: u32, y: u32, w: u32, h: u32) -> Result<Screenshot> {
        let full = Self::capture_full()?;
        let frame = &full.raw.primary;
        if w == 0 || h == 0 {
            anyhow::bail!("empty crop");
        }
        let x2 = (x.saturating_add(w)).min(frame.width);
        let y2 = (y.saturating_add(h)).min(frame.height);
        let cw = x2.saturating_sub(x);
        let ch = y2.saturating_sub(y);
        if cw == 0 || ch == 0 {
            anyhow::bail!("empty crop");
        }
        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for row in 0..ch {
            let src_row_start = (((y + row) * frame.width + x) * 4) as usize;
            let src_row_end = src_row_start + (cw * 4) as usize;
            let dst_row_start = (row * cw * 4) as usize;
            let dst_row_end = dst_row_start + (cw * 4) as usize;
            bytes[dst_row_start..dst_row_end]
                .copy_from_slice(&frame.bytes[src_row_start..src_row_end]);
        }
        Ok(Self::build_screenshot(cw, ch, bytes))
    }

    /// 未来自研框选 UI 接口：通过 ui_overlay 的 RegionSelector 获取矩形，
    /// 然后基于一次全屏截图在内存裁剪出区域，避免多次系统调用。
    /// 当前：若 selector 返回 Some(Rect) => 裁剪；None => 返回交互取消错误 (anyhow::bail)。
    pub fn capture_region_interactive_custom(
        selector: &dyn ui_overlay::RegionSelector,
    ) -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "interactive_duration_us",
            &[1000, 5000, 10000, 50000, 100000, 500000, 1000000],
        );
        metrics::counter("interactive_start").inc();

        // 首先获取全屏截图作为背景
        let full = Self::capture_full()?;
        let frame = &full.raw.primary;

        // 将RGBA转换为RGB用于ui_overlay
        let rgb_data: Vec<u8> = frame
            .bytes
            .chunks_exact(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]]) // 跳过alpha通道
            .collect();

        // 使用带背景的选择方法
        let rect_opt = selector
            .select_with_background(&rgb_data, frame.width, frame.height)
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
        let x2 = (x + w).min(frame.width);
        let y2 = (y + h).min(frame.height);
        let cw = x2.saturating_sub(x);
        let ch = y2.saturating_sub(y);
        if cw == 0 || ch == 0 {
            anyhow::bail!("empty crop");
        }
        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for row in 0..ch {
            let src_row_start = (((y + row) * frame.width + x) * 4) as usize;
            let src_row_end = src_row_start + (cw * 4) as usize;
            let dst_row_start = (row * cw * 4) as usize;
            let dst_row_end = dst_row_start + (cw * 4) as usize;
            bytes[dst_row_start..dst_row_end]
                .copy_from_slice(&frame.bytes[src_row_start..src_row_end]);
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
