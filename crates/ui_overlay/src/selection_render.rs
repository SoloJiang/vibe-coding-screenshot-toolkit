/// 选择器渲染逻辑模块
///
/// 提供选择框的渲染管理功能，使用 egui painter 进行绘制
use crate::event_handler::EventHandler;

/// egui 选择框渲染器
pub struct EguiSelectionRenderer;

impl EguiSelectionRenderer {
    /// 使用 egui painter 渲染选择框
    ///
    /// # Arguments
    /// * `painter` - egui painter
    /// * `selection_rect` - 选择框矩形 (x, y, w, h)，逻辑坐标
    /// * `window_size` - 窗口尺寸 (width, height)，逻辑坐标
    pub fn render_selection(
        painter: &egui::Painter,
        selection_rect: Option<(f32, f32, f32, f32)>,
        window_size: (f32, f32),
    ) {
        let (win_w, win_h) = window_size;

        // 暗化遮罩颜色 (半透明黑色)
        let darken_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128);

        // 如果有选择框，绘制选择框外的暗化遮罩
        if let Some((x, y, w, h)) = selection_rect {
            tracing::trace!(
                "📐 绘制选择框和遮罩: pos=({:.1}, {:.1}), size=({:.1}x{:.1})",
                x,
                y,
                w,
                h
            );

            let sel_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

            // 绘制4个矩形遮罩覆盖选择框外的区域
            // 1. 上方遮罩
            if y > 0.0 {
                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(win_w, y)),
                    0.0,
                    darken_color,
                );
            }

            // 2. 下方遮罩
            let bottom = y + h;
            if bottom < win_h {
                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(0.0, bottom), egui::pos2(win_w, win_h)),
                    0.0,
                    darken_color,
                );
            }

            // 3. 左侧遮罩
            if x > 0.0 {
                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(0.0, y), egui::pos2(x, bottom)),
                    0.0,
                    darken_color,
                );
            }

            // 4. 右侧遮罩
            let right = x + w;
            if right < win_w {
                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(right, y), egui::pos2(win_w, bottom)),
                    0.0,
                    darken_color,
                );
            }

            // 绘制白色边框（2像素宽）
            painter.rect_stroke(
                sel_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );

            // 绘制尺寸信息
            let size_text = format!("{}x{}", w as i32, h as i32);
            let text_pos = egui::pos2(x + w / 2.0, (y - 20.0_f32).max(5.0));
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                size_text,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        } else {
            // 没有选择框时，整个窗口显示暗化遮罩
            tracing::trace!("📐 绘制全屏暗化遮罩");
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(win_w, win_h)),
                0.0,
                darken_color,
            );
        }
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

    #[test]
    fn test_tint_background() {
        // This test was for BackgroundProcessor, which is now removed.
        // So this test case is also removed.
    }
}
