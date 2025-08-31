#![allow(unexpected_cfgs)]
#[macro_use]
extern crate objc;
use anyhow::{Context, Result};
use chrono::Utc;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSData, NSString};
use image::GenericImageView;
use objc::rc::autoreleasepool;
use objc::{class, msg_send, sel};
use screenshot_core::Result as CoreResult;
use screenshot_core::{Frame, FrameSet, PixelFormat, Screenshot};
use services::Clipboard;
use std::io::Read;
use std::process::Command;
use std::sync::Arc;
use tempfile::NamedTempFile;
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
    /// 捕获主显示器截图。优先 xcap，失败回退 screencapture。
    pub fn capture_full() -> Result<Screenshot> {
        match Self::capture_with_xcap() {
            Ok(s) => Ok(s),
            Err(e) => {
                tracing::warn!(error = %e, "xcap 捕获失败, 回退 screencapture");
                Self::capture_with_screencapture()
                    .with_context(|| format!("fallback screencapture 也失败，原始 xcap 错误: {e}"))
            }
        }
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

    fn capture_with_screencapture() -> Result<Screenshot> {
        let tmp = NamedTempFile::new()?;
        let path = tmp.path().to_path_buf();
        let status = Command::new("screencapture")
            .arg("-x")
            .arg(&path)
            .status()
            .context("执行 screencapture 命令失败")?;
        if !status.success() {
            anyhow::bail!("screencapture 退出状态非 0 (可能未授权)");
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .context("打开临时截图文件失败")?
            .read_to_end(&mut bytes)
            .context("读取临时截图文件失败")?;
        let img = image::load_from_memory(&bytes).context("解析截图为图像失败")?;
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8().into_raw();
        Ok(Self::build_screenshot(w, h, rgba))
    }

    /// 使用系统 screencapture -x -R x,y,w,h 截取指定矩形区域（以主显示器为基准）。
    pub fn capture_region(x: u32, y: u32, w: u32, h: u32) -> Result<Screenshot> {
        let tmp = NamedTempFile::new()?;
        let path = tmp.path().to_path_buf();
        let region = format!("{},{},{},{}", x, y, w, h);
        let status = Command::new("screencapture")
            .arg("-x")
            .arg("-R")
            .arg(&region)
            .arg(&path)
            .status()
            .context("执行 screencapture 区域命令失败")?;
        if !status.success() {
            anyhow::bail!("screencapture 区域模式退出状态非 0");
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .context("打开区域临时截图失败")?
            .read_to_end(&mut bytes)
            .context("读取区域临时截图失败")?;
        let img = image::load_from_memory(&bytes).context("解析区域截图失败")?;
        let (rw, rh) = img.dimensions();
        let rgba = img.to_rgba8().into_raw();
        Ok(Self::build_screenshot(rw, rh, rgba))
    }

    /// 交互式框选：调用 screencapture -i -s 让用户框选（或 -i 单次交互），返回截图。
    /// 注意：自动化测试中不可用；需人工授权并交互。此函数不加 -x 以便系统出声音/动画可选。
    pub fn capture_region_interactive() -> Result<Screenshot> {
        let tmp = NamedTempFile::new()?;
        let path = tmp.path().to_path_buf();
        let status = Command::new("screencapture")
            .arg("-i")
            .arg(&path)
            .status()
            .context("执行 screencapture 交互命令失败")?;
        if !status.success() {
            anyhow::bail!("交互框选被取消或失败");
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .context("打开交互临时截图失败")?
            .read_to_end(&mut bytes)
            .context("读取交互临时截图失败")?;
        let img = image::load_from_memory(&bytes).context("解析交互截图失败")?;
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8().into_raw();
        Ok(Self::build_screenshot(w, h, rgba))
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

        // 裁剪（边界钳制）
        let x = rect.x.max(0.0) as u32;
        let y = rect.y.max(0.0) as u32;
        let w = rect.w.max(0.0) as u32;
        let h = rect.h.max(0.0) as u32;
        let x2 = (x + w).min(frame.width);
        let y2 = (y + h).min(frame.height);
        let cw = x2.saturating_sub(x);
        let ch = y2.saturating_sub(y);
        if cw == 0 || ch == 0 {
            anyhow::bail!("empty crop");
        }
        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for row in 0..ch {
            for col in 0..cw {
                let src_i = (((y + row) * frame.width + (x + col)) * 4) as usize;
                let dst_i = ((row * cw + col) * 4) as usize;
                bytes[dst_i..dst_i + 4].copy_from_slice(&frame.bytes[src_i..src_i + 4]);
            }
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
