/// 选择器渲染逻辑模块
///
/// 提供选择框的渲染管理功能
use crate::event_handler::EventHandler;

/// 背景处理工具
pub struct BackgroundProcessor;

impl BackgroundProcessor {
    /// 将背景与叠加颜色混合（暗化效果）
    ///
    /// 性能优化：使用 rayon 并行处理像素
    /// - 小图像（< 256KB）使用单线程避免线程开销
    /// - 大图像使用多线程并行处理
    pub fn tint_background(bg: &[u8], overlay_color: [u8; 4]) -> Vec<u8> {
        use rayon::prelude::*;

        let overlay_alpha = overlay_color[3] as u16;
        let inv_alpha = 255u16.saturating_sub(overlay_alpha);
        let tint_r = overlay_color[0] as u16;
        let tint_g = overlay_color[1] as u16;
        let tint_b = overlay_color[2] as u16;

        let mut tinted = vec![0u8; bg.len()];

        // 对于小图像，使用单线程避免并行开销
        const PARALLEL_THRESHOLD: usize = 256 * 1024; // 256KB

        if bg.len() < PARALLEL_THRESHOLD {
            // 单线程处理小图像
            for (src, dst) in bg.chunks_exact(4).zip(tinted.chunks_exact_mut(4)) {
                dst[0] = (((src[0] as u16) * inv_alpha + tint_r * overlay_alpha) / 255) as u8;
                dst[1] = (((src[1] as u16) * inv_alpha + tint_g * overlay_alpha) / 255) as u8;
                dst[2] = (((src[2] as u16) * inv_alpha + tint_b * overlay_alpha) / 255) as u8;
                dst[3] = 255;
            }
        } else {
            // 并行处理大图像（按像素行分块）
            tinted
                .par_chunks_mut(4)
                .zip(bg.par_chunks(4))
                .for_each(|(dst, src)| {
                    dst[0] = (((src[0] as u16) * inv_alpha + tint_r * overlay_alpha) / 255) as u8;
                    dst[1] = (((src[1] as u16) * inv_alpha + tint_g * overlay_alpha) / 255) as u8;
                    dst[2] = (((src[2] as u16) * inv_alpha + tint_b * overlay_alpha) / 255) as u8;
                    dst[3] = 255;
                });
        }

        tinted
    }
}

/// 窗口渲染器
///
/// 负责将选择状态渲染到窗口
pub struct WindowRenderer;

impl WindowRenderer {
    /// 检查窗口是否需要渲染选择框
    pub fn should_render_selection(
        selection_exists: bool,
        state: &mut crate::event_handler::SelectionState,
        window_virtual_x: i32,
        window_virtual_y: i32,
        window_width: u32,
        window_height: u32,
    ) -> bool {
        if !selection_exists {
            return false;
        }

        if state.virtual_bounds.is_some() {
            EventHandler::selection_intersects_window(
                state,
                window_virtual_x,
                window_virtual_y,
                window_width,
                window_height,
            )
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tint_background() {
        let bg = vec![255, 128, 64, 255, 200, 100, 50, 255];
        let overlay_color = [0, 0, 0, 128]; // 半透明黑色
        let tinted = BackgroundProcessor::tint_background(&bg, overlay_color);

        assert_eq!(tinted.len(), bg.len());
        // 验证颜色被暗化
        assert!(tinted[0] < bg[0]);
        assert!(tinted[4] < bg[4]);
    }
}
