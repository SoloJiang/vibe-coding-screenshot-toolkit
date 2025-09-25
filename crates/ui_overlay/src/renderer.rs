use crate::coordinate_utils;
/// 渲染器模块
///
/// 提供选择器界面的渲染功能，包括背景渲染、选择框渲染等
use winit::dpi::PhysicalSize;

/// 包含渲染操作所需的核心上下文信息
pub struct RenderContext<'a> {
    pub frame: &'a mut [u8],
    pub size_px: PhysicalSize<u32>,
    pub virtual_x: i32,
    pub virtual_y: i32,
    pub virtual_bounds: Option<(i32, i32, u32, u32)>,
}

/// 包含背景图像信息的上下文
pub struct Background<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
}

/// 定义选择区域的矩形 (x0, y0, x1, y1)
pub type SelectionRect = (i32, i32, i32, i32);

/// 渲染器结构体，负责处理所有渲染相关操作
pub struct SelectionRenderer;

impl SelectionRenderer {
    /// 渲染虚拟桌面的背景到当前窗口
    pub fn render_virtual_background(ctx: &mut RenderContext, bg: &Background) {
        if let Some((virt_min_x, virt_min_y, _, _)) = ctx.virtual_bounds {
            let window_x_in_virt = ctx.virtual_x - virt_min_x;
            let window_y_in_virt = ctx.virtual_y - virt_min_y;

            for y in 0..ctx.size_px.height as usize {
                let bg_y = window_y_in_virt as usize + y;
                if bg_y >= bg.height as usize {
                    continue;
                }

                let bg_row_start = bg_y * bg.width as usize * 4;
                let frame_row_start = y * ctx.size_px.width as usize * 4;

                let window_x_offset = window_x_in_virt.max(0) as usize;
                let available_bg_pixels = bg.width as usize - window_x_offset;
                let needed_pixels = ctx.size_px.width as usize;
                let copy_pixels = available_bg_pixels.min(needed_pixels);

                if copy_pixels > 0 {
                    let bg_start_idx = bg_row_start + window_x_offset * 4;
                    let frame_start_idx = frame_row_start;
                    let copy_bytes = copy_pixels * 4;

                    if bg_start_idx + copy_bytes <= bg.data.len()
                        && frame_start_idx + copy_bytes <= ctx.frame.len()
                    {
                        ctx.frame[frame_start_idx..frame_start_idx + copy_bytes]
                            .copy_from_slice(&bg.data[bg_start_idx..bg_start_idx + copy_bytes]);
                    }
                }
            }
        } else {
            Self::render_scaled_background(ctx, bg);
        }
    }

    fn render_scaled_background(ctx: &mut RenderContext, bg: &Background) {
        let frame_width = ctx.size_px.width as usize;
        let frame_height = ctx.size_px.height as usize;

        let x_scale = bg.width as f32 / frame_width as f32;
        let y_scale = bg.height as f32 / frame_height as f32;

        for y in 0..frame_height {
            let bg_y = ((y as f32 * y_scale) as usize).min(bg.height as usize - 1);
            let bg_row_start = bg_y * bg.width as usize * 4;
            let frame_row_start = y * frame_width * 4;

            for x in 0..frame_width {
                let bg_x = ((x as f32 * x_scale) as usize).min(bg.width as usize - 1);
                let bg_idx = bg_row_start + bg_x * 4;
                let frame_idx = frame_row_start + x * 4;

                if bg_idx + 3 < bg.data.len() && frame_idx + 3 < ctx.frame.len() {
                    ctx.frame[frame_idx..frame_idx + 4]
                        .copy_from_slice(&bg.data[bg_idx..bg_idx + 4]);
                }
            }
        }
    }

    /// 在选择区域内渲染原始背景 - 优化版本
    pub fn render_selection_background(
        ctx: &mut RenderContext,
        original_bg: &Background,
        selection: SelectionRect,
    ) {
        let (x0, y0, x1, y1) = selection;
        // 将虚拟桌面坐标转换为窗口本地坐标
        let (local_x0, local_y0, local_x1, local_y1) = coordinate_utils::virtual_to_window_coords(
            ctx.virtual_x,
            ctx.virtual_y,
            ctx.virtual_bounds,
            x0,
            y0,
            x1,
            y1,
        );

        if local_x1 <= local_x0 || local_y1 <= local_y0 {
            return; // 选择框不在此窗口内
        }

        let width = ctx.size_px.width as usize;
        let height = ctx.size_px.height as usize;

        // 在选择区域内恢复原始背景 - 优化版本
        if let Some((virt_min_x, virt_min_y, _, _)) = ctx.virtual_bounds {
            // 虚拟桌面模式：使用行拷贝优化
            let window_x_in_virt = ctx.virtual_x - virt_min_x;
            let window_y_in_virt = ctx.virtual_y - virt_min_y;

            for y in local_y0..local_y1.min(height) {
                let bg_y = window_y_in_virt as usize + y;
                if bg_y >= original_bg.height as usize {
                    continue;
                }

                let bg_row_start = bg_y * original_bg.width as usize * 4;
                let frame_row_start = y * width * 4;

                // 计算行内的拷贝范围
                let start_x = local_x0.max(0);
                let end_x = local_x1.min(width);
                let copy_pixels = end_x - start_x;

                if copy_pixels > 0 {
                    let bg_x_offset = window_x_in_virt as usize + start_x;
                    if bg_x_offset + copy_pixels <= original_bg.width as usize {
                        let bg_start_idx = bg_row_start + bg_x_offset * 4;
                        let frame_start_idx = frame_row_start + start_x * 4;
                        let copy_bytes = copy_pixels * 4;

                        if bg_start_idx + copy_bytes <= original_bg.data.len()
                            && frame_start_idx + copy_bytes <= ctx.frame.len()
                        {
                            ctx.frame[frame_start_idx..frame_start_idx + copy_bytes]
                                .copy_from_slice(
                                    &original_bg.data[bg_start_idx..bg_start_idx + copy_bytes],
                                );
                        }
                    }
                }
            }
        } else {
            // 非虚拟模式：优化的缩放拷贝
            let x_scale = original_bg.width as f32 / width as f32;
            let y_scale = original_bg.height as f32 / height as f32;

            for y in local_y0..local_y1.min(height) {
                let bg_y = ((y as f32 * y_scale) as usize).min(original_bg.height as usize - 1);
                let bg_row_start = bg_y * original_bg.width as usize * 4;
                let frame_row_start = y * width * 4;

                for x in local_x0..local_x1.min(width) {
                    let bg_x = ((x as f32 * x_scale) as usize).min(original_bg.width as usize - 1);
                    let bg_idx = bg_row_start + bg_x * 4;
                    let frame_idx = frame_row_start + x * 4;

                    if bg_idx + 3 < original_bg.data.len() && frame_idx + 3 < ctx.frame.len() {
                        ctx.frame[frame_idx..frame_idx + 4]
                            .copy_from_slice(&original_bg.data[bg_idx..bg_idx + 4]);
                    }
                }
            }
        }
    }

    /// 渲染选择框边框
    pub fn render_selection_border(ctx: &mut RenderContext, selection: SelectionRect) {
        let (x0, y0, x1, y1) = selection;
        let width = ctx.size_px.width as usize;
        let height = ctx.size_px.height as usize;

        // 将虚拟桌面坐标转换为窗口本地坐标
        let (local_x0, local_y0, local_x1, local_y1) = coordinate_utils::virtual_to_window_coords(
            ctx.virtual_x,
            ctx.virtual_y,
            ctx.virtual_bounds,
            x0,
            y0,
            x1,
            y1,
        );

        if local_x1 <= local_x0 || local_y1 <= local_y0 {
            return; // 选择框不在此窗口内
        }

        // 绘制选择框边框
        for y in local_y0..local_y1 {
            if y >= height {
                break;
            }
            for x in local_x0..local_x1 {
                if x >= width {
                    break;
                }
                let is_border =
                    y == local_y0 || y == local_y1 - 1 || x == local_x0 || x == local_x1 - 1;
                if is_border {
                    let idx = (y * width + x) * 4;
                    if idx + 3 < ctx.frame.len() {
                        ctx.frame[idx] = 255; // R
                        ctx.frame[idx + 1] = 255; // G
                        ctx.frame[idx + 2] = 255; // B
                        ctx.frame[idx + 3] = 255; // A
                    }
                }
            }
        }
    }

    /// 渲染纯色背景（通常用于无背景图像时）
    ///
    /// # 参数
    /// * `frame` - 窗口的像素帧缓冲区
    /// * `r`, `g`, `b`, `a` - RGBA颜色值
    pub fn render_solid_background(frame: &mut [u8], r: u8, g: u8, b: u8, a: u8) {
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[r, g, b, a]);
        }
    }
}
