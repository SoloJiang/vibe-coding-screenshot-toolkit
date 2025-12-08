use crate::coordinate_utils;
/// 事件处理模块
///
/// 提供选择器应用的键盘和鼠标事件处理功能
pub use crate::selection_state::SelectionState;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, NamedKey};

/// 事件处理器
pub struct EventHandler;

impl EventHandler {
    /// 处理键盘事件
    pub fn handle_keyboard_event(state: &mut SelectionState, key_event: KeyEvent) -> EventResult {
        let KeyEvent {
            logical_key,
            state: key_state,
            ..
        } = key_event;

        // ESC 键退出
        if key_state == ElementState::Pressed && logical_key.to_text() == Some("\u{1b}") {
            state.result = None;
            return EventResult::Exit;
        }

        match logical_key {
            Key::Named(NamedKey::Shift) => {
                let was_pressed = state.shift_down;
                state.shift_down = key_state == ElementState::Pressed;
                if was_pressed != state.shift_down {
                    state.invalidate_cache(); // 修饰键变化使缓存失效
                }
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::Alt) => {
                let was_pressed = state.alt_down;
                state.alt_down = key_state == ElementState::Pressed;
                if was_pressed != state.alt_down {
                    state.invalidate_cache(); // 修饰键变化使缓存失效
                }
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowLeft) if key_state == ElementState::Pressed => {
                state.curr.0 -= 1.0;
                state.invalidate_cache();
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowRight) if key_state == ElementState::Pressed => {
                state.curr.0 += 1.0;
                state.invalidate_cache();
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowUp) if key_state == ElementState::Pressed => {
                state.curr.1 -= 1.0;
                state.invalidate_cache();
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowDown) if key_state == ElementState::Pressed => {
                state.curr.1 += 1.0;
                state.invalidate_cache();
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::Enter) if key_state == ElementState::Pressed => {
                if state.has_valid_selection() {
                    EventResult::Finish
                } else {
                    EventResult::Continue(false)
                }
            }
            _ => EventResult::Continue(false),
        }
    }

    /// 处理鼠标移动事件（带防抖优化）
    ///
    /// 使用固定阈值减少计算开销和重绘频率：
    /// - 拖动时：5px 阈值，保证流畅度
    /// - 非拖动时：10px 阈值，减少无效重绘
    pub fn handle_cursor_moved(state: &mut SelectionState, new_pos: (f64, f64)) -> bool {
        let distance = ((new_pos.0 - state.last_cursor_pos.0).powi(2)
            + (new_pos.1 - state.last_cursor_pos.1).powi(2))
        .sqrt();

        let threshold = if state.dragging { 5.0 } else { 10.0 };

        if distance > threshold {
            state.curr = new_pos;
            state.last_cursor_pos = new_pos;
            state.invalidate_cache();

            // 只在拖动时触发重绘
            return state.dragging;
        }

        false
    }

    /// 处理鼠标按键事件
    pub fn handle_mouse_input(
        state: &mut SelectionState,
        button: MouseButton,
        button_state: ElementState,
    ) -> EventResult {
        match (button, button_state) {
            (MouseButton::Left, ElementState::Pressed) => {
                state.dragging = true;
                state.start = state.curr;
                state.invalidate_cache(); // 开始拖动时使缓存失效
                tracing::debug!(
                    "🖱️  开始拖拽: 起点=({:.0}, {:.0})",
                    state.start.0,
                    state.start.1
                );
                EventResult::Continue(true)
            }
            (MouseButton::Left, ElementState::Released) => {
                state.dragging = false;
                tracing::debug!(
                    "🖱️  结束拖拽: 终点=({:.0}, {:.0})",
                    state.curr.0,
                    state.curr.1
                );
                // 拖动结束后显示工具栏
                if state.has_valid_selection() {
                    let (x0, y0, x1, y1) = state.calculate_selection_rect();
                    tracing::debug!(
                        "   └─ 有效选择: ({:.0}, {:.0}) -> ({:.0}, {:.0}), 尺寸=({:.0}x{:.0})",
                        x0,
                        y0,
                        x1,
                        y1,
                        (x1 - x0).abs(),
                        (y1 - y0).abs()
                    );
                    state.toolbar_visible = true;
                } else {
                    tracing::debug!("   └─ 选择无效（太小）");
                }
                // 触发重绘以显示最终选框和工具栏
                EventResult::Continue(state.has_valid_selection())
            }
            _ => EventResult::Continue(false),
        }
    }

    /// 检查选择区域是否与窗口有交集
    pub fn selection_intersects_window(
        state: &mut SelectionState,
        window_virtual_x: i32,
        window_virtual_y: i32,
        window_width: u32,
        window_height: u32,
    ) -> bool {
        let (x0, y0, x1, y1) = state.calculate_selection_rect();

        // 检查是否有有效的选择尺寸
        if (x1 - x0).abs() <= 1.0 && (y1 - y0).abs() <= 1.0 {
            return false;
        }

        if state.virtual_bounds.is_some() {
            coordinate_utils::check_selection_intersects_window(&coordinate_utils::IntersectCheck {
                window_x: window_virtual_x as f64,
                window_y: window_virtual_y as f64,
                window_w: window_width,
                window_h: window_height,
                selection_x0: x0,
                selection_y0: y0,
                selection_x1: x1,
                selection_y1: y1,
            })
        } else {
            true
        }
    }
}

/// 事件处理结果
#[derive(Debug, Clone, Copy)]
pub enum EventResult {
    /// 继续运行，布尔值表示是否需要重绘
    Continue(bool),
    /// 退出应用（用户取消）
    Exit,
    /// 完成选择
    Finish,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_mouse_input_press() {
        let mut state = SelectionState::new(None);
        state.curr = (100.0, 100.0);

        let result =
            EventHandler::handle_mouse_input(&mut state, MouseButton::Left, ElementState::Pressed);

        assert!(state.dragging);
        assert_eq!(state.start, (100.0, 100.0));
        assert!(matches!(result, EventResult::Continue(true)));
    }

    #[test]
    fn test_handle_mouse_input_release_with_selection() {
        let mut state = SelectionState::new(None);
        state.dragging = true;
        state.start = (100.0, 100.0);
        state.curr = (200.0, 200.0);

        let result =
            EventHandler::handle_mouse_input(&mut state, MouseButton::Left, ElementState::Released);

        assert!(!state.dragging);
        // 修改断言：鼠标释放不再立即完成，而是继续显示选框等待确认
        assert!(matches!(result, EventResult::Continue(true)));
    }

    #[test]
    fn test_handle_cursor_moved_with_threshold() {
        let mut state = SelectionState::new(None);
        state.curr = (100.0, 100.0);
        state.last_cursor_pos = (100.0, 100.0);
        state.dragging = true;

        // 小移动应该被忽略
        let result = EventHandler::handle_cursor_moved(&mut state, (101.0, 101.0));
        assert!(!result);

        // 大移动应该触发重绘
        let result = EventHandler::handle_cursor_moved(&mut state, (110.0, 110.0));
        assert!(result);
    }
}
