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

/// 显示器信息：包含位置、尺寸、DPI等元数据
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// 显示器ID（在xcap中的唯一标识）
    pub id: u32,
    /// 显示器名称
    pub name: String,
    /// 是否为主显示器
    pub is_primary: bool,
    /// 在虚拟桌面中的位置（左上角坐标）
    pub x: i32,
    pub y: i32,
    /// 显示器的像素尺寸
    pub width: u32,
    pub height: u32,
    /// DPI缩放因子
    pub scale_factor: f64,
}

/// 虚拟桌面坐标系统：管理多显示器的统一坐标空间
#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    /// 所有显示器信息
    pub displays: Vec<DisplayInfo>,
    /// 虚拟桌面的总边界框
    pub total_bounds: VirtualBounds,
}

/// 虚拟桌面边界框
#[derive(Debug, Clone)]
pub struct VirtualBounds {
    /// 最小X坐标（可能为负数）
    pub min_x: i32,
    /// 最小Y坐标（可能为负数）
    pub min_y: i32,
    /// 最大X坐标
    pub max_x: i32,
    /// 最大Y坐标
    pub max_y: i32,
    /// 总宽度
    pub width: u32,
    /// 总高度
    pub height: u32,
}

impl VirtualDesktop {
    /// 检测并构建虚拟桌面坐标系统
    pub fn detect() -> Result<Self> {
        let monitors = xcap::Monitor::all().context("列出显示器失败")?;
        if monitors.is_empty() {
            anyhow::bail!("未找到任何显示器");
        }

        let mut displays = Vec::new();
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for monitor in monitors {
            let id = monitor.id().unwrap_or(0);
            let name = monitor.name().unwrap_or_else(|_| format!("Display {}", id));
            let is_primary = monitor.is_primary().unwrap_or(false);
            let x = monitor.x().unwrap_or(0);
            let y = monitor.y().unwrap_or(0);
            let width = monitor.width().unwrap_or(1920);
            let height = monitor.height().unwrap_or(1080);
            let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;

            // 更新边界
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + width as i32);
            max_y = max_y.max(y + height as i32);

            displays.push(DisplayInfo {
                id,
                name,
                is_primary,
                x,
                y,
                width,
                height,
                scale_factor,
            });
        }

        let total_bounds = VirtualBounds {
            min_x,
            min_y,
            max_x,
            max_y,
            width: (max_x - min_x) as u32,
            height: (max_y - min_y) as u32,
        };

        Ok(VirtualDesktop {
            displays,
            total_bounds,
        })
    }

    /// 获取主显示器信息
    pub fn primary_display(&self) -> Option<&DisplayInfo> {
        self.displays.iter().find(|d| d.is_primary)
    }

    /// 根据虚拟坐标找到对应的显示器
    pub fn find_display_at(&self, x: i32, y: i32) -> Option<&DisplayInfo> {
        self.displays
            .iter()
            .find(|d| x >= d.x && x < d.x + d.width as i32 && y >= d.y && y < d.y + d.height as i32)
    }

    /// 获取跨越指定区域的所有显示器
    pub fn displays_in_region(&self, x: i32, y: i32, width: u32, height: u32) -> Vec<&DisplayInfo> {
        let x2 = x + width as i32;
        let y2 = y + height as i32;

        self.displays
            .iter()
            .filter(|d| {
                let dx2 = d.x + d.width as i32;
                let dy2 = d.y + d.height as i32;
                // 检查矩形是否有重叠
                !(x2 <= d.x || x >= dx2 || y2 <= d.y || y >= dy2)
            })
            .collect()
    }

    /// 将虚拟坐标转换为显示器内的相对坐标
    pub fn virtual_to_display_coords(&self, display: &DisplayInfo, vx: i32, vy: i32) -> (i32, i32) {
        (vx - display.x, vy - display.y)
    }

    /// 将显示器相对坐标转换为虚拟坐标
    pub fn display_to_virtual_coords(&self, display: &DisplayInfo, dx: i32, dy: i32) -> (i32, i32) {
        (dx + display.x, dy + display.y)
    }
}

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

    /// 多显示器截图：捕获所有显示器并合成为虚拟桌面
    pub fn capture_all() -> Result<Screenshot> {
        use infra::metrics;
        let timer = metrics::start_timer(
            "capture_all_duration_us",
            &[5000, 10000, 50000, 100000, 500000],
        );
        metrics::counter("capture_all_start").inc();

        // 获取虚拟桌面信息
        let virtual_desktop = VirtualDesktop::detect().context("检测虚拟桌面失败")?;
        let bounds = &virtual_desktop.total_bounds;

        // 创建虚拟桌面画布
        let canvas_width = bounds.width;
        let canvas_height = bounds.height;
        let mut canvas = vec![0u8; (canvas_width * canvas_height * 4) as usize];

        // 捕获每个显示器并合成到虚拟桌面
        let monitors = xcap::Monitor::all().context("列出显示器失败")?;
        for monitor in monitors {
            let monitor_id = monitor.id().unwrap_or(0);
            let display_info = virtual_desktop
                .displays
                .iter()
                .find(|d| d.id == monitor_id)
                .context("未找到对应的显示器信息")?;

            // 捕获当前显示器
            let img = monitor.capture_image().context("xcap 显示器图像捕获失败")?;
            let (mon_width, mon_height) = (img.width(), img.height());
            let rgba_data = img.into_raw();

            // 计算在虚拟桌面画布中的位置
            let canvas_x = (display_info.x - bounds.min_x) as u32;
            let canvas_y = (display_info.y - bounds.min_y) as u32;

            #[cfg(debug_assertions)]
            tracing::debug!(
                "显示器合成: 显示器{}({},{}) -> canvas({},{}) 尺寸{}x{}",
                monitor_id,
                display_info.x,
                display_info.y,
                canvas_x,
                canvas_y,
                mon_width,
                mon_height
            );

            // 将显示器图像复制到虚拟桌面画布
            for row in 0..mon_height {
                if canvas_y + row >= canvas_height {
                    break;
                }
                let src_row_start = (row * mon_width * 4) as usize;
                let src_row_end = src_row_start + (mon_width * 4) as usize;
                let dst_row_start = (((canvas_y + row) * canvas_width + canvas_x) * 4) as usize;
                let dst_row_end = dst_row_start + (mon_width * 4) as usize;

                if dst_row_end <= canvas.len() && src_row_end <= rgba_data.len() {
                    canvas[dst_row_start..dst_row_end]
                        .copy_from_slice(&rgba_data[src_row_start..src_row_end]);
                }
            }
        }

        metrics::counter("capture_all_ok").inc();
        drop(timer);
        Ok(Self::build_screenshot(canvas_width, canvas_height, canvas))
    }

    /// 自研框选 UI 接口：通过 ui_overlay 的 RegionSelector 获取矩形，
    /// 基于虚拟桌面截图（支持多显示器）在内存裁剪出区域，避免多次系统调用。
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

        // 获取虚拟桌面信息
        let virtual_desktop = VirtualDesktop::detect().context("检测虚拟桌面失败")?;

        // 捕获完整虚拟桌面作为交互背景
        let virtual_screenshot = Self::capture_all().context("获取虚拟桌面截图失败")?;
        let virtual_frame = &virtual_screenshot.raw.primary;

        // 添加调试信息
        #[cfg(debug_assertions)]
        {
            tracing::debug!(
                "虚拟桌面调试: 尺寸 {}x{}",
                virtual_frame.width,
                virtual_frame.height
            );
            tracing::debug!(
                "虚拟桌面调试: 显示器数量 {}",
                virtual_desktop.displays.len()
            );
            for (i, display_info) in virtual_desktop.displays.iter().enumerate() {
                tracing::debug!(
                    "显示器 {}: 位置({}, {}), 尺寸{}x{}, 主屏={}",
                    i,
                    display_info.x,
                    display_info.y,
                    display_info.width,
                    display_info.height,
                    display_info.is_primary
                );
            }
        }

        // 将RGBA转换为RGB用于ui_overlay
        let rgb_data: Vec<u8> = virtual_frame
            .bytes
            .chunks_exact(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]]) // 跳过alpha通道
            .collect();

        // 构建虚拟桌面信息用于区域选择
        let virtual_bounds = (
            virtual_desktop.total_bounds.min_x,
            virtual_desktop.total_bounds.min_y,
            virtual_desktop.total_bounds.width,
            virtual_desktop.total_bounds.height,
        );
        let display_offset = (0, 0); // 现在使用虚拟桌面坐标，不需要偏移

        // 使用支持虚拟桌面的选择方法
        let rect_opt = selector
            .select_with_virtual_background(
                &rgb_data,
                virtual_frame.width,
                virtual_frame.height,
                virtual_bounds,
                display_offset,
            )
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

        // 坐标已经是虚拟桌面坐标，直接使用
        let scale = if rect.scale.is_finite() && rect.scale > 0.0 {
            rect.scale
        } else {
            1.0
        } as f32;

        let x_virtual = (rect.x * scale).floor() as i32; // 不使用 max(0.0)，保留负坐标
        let y_virtual = (rect.y * scale).floor() as i32; // 不使用 max(0.0)，保留负坐标
        let w = (rect.w * scale).round().max(0.0) as u32;
        let h = (rect.h * scale).round().max(0.0) as u32;

        // 在虚拟桌面坐标系中进行裁剪
        let bounds = &virtual_desktop.total_bounds;
        let canvas_x = (x_virtual - bounds.min_x).max(0) as u32;
        let canvas_y = (y_virtual - bounds.min_y).max(0) as u32;
        let canvas_x2 = (canvas_x + w).min(virtual_frame.width);
        let canvas_y2 = (canvas_y + h).min(virtual_frame.height);
        let cw = canvas_x2.saturating_sub(canvas_x);
        let ch = canvas_y2.saturating_sub(canvas_y);

        // 添加详细调试信息
        #[cfg(debug_assertions)]
        {
            tracing::debug!("详细裁剪调试:");
            tracing::debug!(
                "  输入Region: ({}, {}, {}, {})",
                rect.x,
                rect.y,
                rect.w,
                rect.h
            );
            tracing::debug!("  虚拟坐标: ({}, {}, {}, {})", x_virtual, y_virtual, w, h);
            tracing::debug!(
                "  虚拟边界: min({}, {}), max({}, {})",
                bounds.min_x,
                bounds.min_y,
                bounds.max_x,
                bounds.max_y
            );
            tracing::debug!(
                "  Canvas计算: ({} - {}) = {}, ({} - {}) = {}",
                x_virtual,
                bounds.min_x,
                canvas_x,
                y_virtual,
                bounds.min_y,
                canvas_y
            );
            tracing::debug!(
                "  最终Canvas区域: ({}, {}, {}, {})",
                canvas_x,
                canvas_y,
                cw,
                ch
            );
            tracing::debug!(
                "  虚拟桌面尺寸: {}x{}",
                virtual_frame.width,
                virtual_frame.height
            );
            tracing::debug!(
                "裁剪调试: canvas坐标({}, {}, {}, {})",
                canvas_x,
                canvas_y,
                cw,
                ch
            );
        }

        if cw == 0 || ch == 0 {
            anyhow::bail!("empty crop region");
        }

        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for row in 0..ch {
            let src_row_start = (((canvas_y + row) * virtual_frame.width + canvas_x) * 4) as usize;
            let src_row_end = src_row_start + (cw * 4) as usize;
            let dst_row_start = (row * cw * 4) as usize;
            let dst_row_end = dst_row_start + (cw * 4) as usize;
            if src_row_end <= virtual_frame.bytes.len() && dst_row_end <= bytes.len() {
                bytes[dst_row_start..dst_row_end]
                    .copy_from_slice(&virtual_frame.bytes[src_row_start..src_row_end]);
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
