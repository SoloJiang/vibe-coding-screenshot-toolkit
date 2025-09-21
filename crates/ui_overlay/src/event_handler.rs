use crate::{coordinate_utils, Region};
/// 事件处理模块
///
/// 提供选择器应用的键盘和鼠标事件处理功能
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, NamedKey};

/// 选择器应用的状态
pub struct SelectionState {
    /// 是否正在拖拽
    pub dragging: bool,
    /// 拖拽起始点坐标
    pub start: (f64, f64),
    /// 当前鼠标位置
    pub curr: (f64, f64),
    /// 上次光标位置（用于防抖）
    pub last_cursor_pos: (f64, f64),
    /// 选择结果
    pub result: Option<Region>,
    /// Shift 键是否按下
    pub shift_down: bool,
    /// Alt 键是否按下
    pub alt_down: bool,
    /// 是否有重绘请求待处理
    pub redraw_pending: bool,
    /// 虚拟桌面边界信息
    pub virtual_bounds: Option<(i32, i32, u32, u32)>,
    /// 上次重绘时间（用于限制重绘频率）
    pub last_redraw_time: std::time::Instant,
    /// 重绘预算（防止过度重绘）
    pub redraw_budget: u32,
    /// 强制重绘标志
    pub force_redraw: bool,
}

impl SelectionState {
    /// 创建新的选择状态
    pub fn new(virtual_bounds: Option<(i32, i32, u32, u32)>) -> Self {
        Self {
            dragging: false,
            start: (0.0, 0.0),
            curr: (0.0, 0.0),
            last_cursor_pos: (0.0, 0.0),
            result: None,
            shift_down: false,
            alt_down: false,
            redraw_pending: false,
            virtual_bounds,
            last_redraw_time: std::time::Instant::now(),
            redraw_budget: 4, // 初始预算
            force_redraw: false,
        }
    }

    /// 计算当前选择矩形
    ///
    /// 基于当前 start/curr 位置与修饰键状态计算矩形
    /// 返回与 start/curr 相同坐标空间的矩形坐标
    ///
    /// # 返回值
    /// (x0, y0, x1, y1) 矩形的左上角和右下角坐标
    pub fn calculate_selection_rect(&self) -> (f64, f64, f64, f64) {
        let (sx, sy) = self.start;
        let (cx, cy) = self.curr;
        let dx = cx - sx;
        let dy = cy - sy;

        if self.alt_down && self.shift_down {
            // 以起点为中心的正方形
            let side = dx.abs().max(dy.abs());
            return (sx - side, sy - side, sx + side, sy + side);
        }
        if self.alt_down {
            // 以起点为中心的矩形
            return (sx - dx.abs(), sy - dy.abs(), sx + dx.abs(), sy + dy.abs());
        }
        if self.shift_down {
            // 正方形（左上/右下随拖拽方向变化）
            let side = dx.abs().max(dy.abs());
            let sxn = if dx >= 0.0 { sx } else { sx - side };
            let syn = if dy >= 0.0 { sy } else { sy - side };
            let exn = if dx >= 0.0 { sx + side } else { sx };
            let eyn = if dy >= 0.0 { sy + side } else { sy };
            return (sxn, syn, exn, eyn);
        }
        (sx.min(cx), sy.min(cy), sx.max(cx), sy.max(cy))
    }

    /// 检查选择区域是否有效（非零尺寸）
    pub fn has_valid_selection(&self) -> bool {
        let (sx, sy, ex, ey) = self.calculate_selection_rect();
        sx != ex && sy != ey
    }

    /// 将当前选择转换为 Region 对象
    ///
    /// # 参数
    /// * `scale` - 缩放因子
    ///
    /// # 返回值
    /// Region 对象，如果选择无效则返回 None
    pub fn to_region(&self, scale: f32) -> Option<Region> {
        if !self.has_valid_selection() {
            return None;
        }

        let (sx, sy, ex, ey) = self.calculate_selection_rect();
        Some(Region {
            x: sx.round() as f32,
            y: sy.round() as f32,
            w: (ex - sx).abs().round() as f32,
            h: (ey - sy).abs().round() as f32,
            scale,
        })
    }

    /// 智能重绘频率控制
    pub fn should_throttle_redraw(&self) -> bool {
        if self.force_redraw || self.redraw_budget == 0 {
            return false; // 强制重绘或无预算时不限制
        }

        if self.redraw_pending {
            return true; // 避免重复请求重绘
        }

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_redraw_time).as_millis();

        // 动态阈值：拖拽时允许更高频率，静态时降低频率
        let threshold = if self.dragging { 8 } else { 16 }; // 125fps vs 62.5fps
        elapsed < threshold
    }

    /// 标记重绘请求已发送
    pub fn mark_redraw_requested(&mut self) {
        self.redraw_pending = true;
        self.last_redraw_time = std::time::Instant::now();

        // 消耗重绘预算
        if self.redraw_budget > 0 && !self.force_redraw {
            self.redraw_budget -= 1;
        }

        self.force_redraw = false; // 重置强制重绘标志
    }

    /// 清除重绘标记
    pub fn clear_redraw_pending(&mut self) {
        self.redraw_pending = false;

        // 定期恢复重绘预算
        let now = std::time::Instant::now();
        if now.duration_since(self.last_redraw_time).as_millis() > 250 {
            self.redraw_budget = self.redraw_budget.saturating_add(2).min(4);
        }
    }

    /// 强制下次重绘（用于重要状态变化）
    pub fn force_next_redraw(&mut self) {
        self.force_redraw = true;
    }
}

/// 事件处理器
pub struct EventHandler;

impl EventHandler {
    /// 处理键盘事件
    ///
    /// # 参数
    /// * `state` - 选择状态
    /// * `key_event` - 键盘事件
    ///
    /// # 返回值
    /// 事件处理结果：Continue(需要重绘), Exit(退出), Finish(完成选择)
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
                // 状态变化时强制重绘
                if was_pressed != state.shift_down {
                    state.force_next_redraw();
                }
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::Alt) => {
                let was_pressed = state.alt_down;
                state.alt_down = key_state == ElementState::Pressed;
                // 状态变化时强制重绘
                if was_pressed != state.alt_down {
                    state.force_next_redraw();
                }
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowLeft) if key_state == ElementState::Pressed => {
                state.curr.0 -= 1.0;
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowRight) if key_state == ElementState::Pressed => {
                state.curr.0 += 1.0;
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowUp) if key_state == ElementState::Pressed => {
                state.curr.1 -= 1.0;
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::ArrowDown) if key_state == ElementState::Pressed => {
                state.curr.1 += 1.0;
                EventResult::Continue(true)
            }
            Key::Named(NamedKey::Enter) if key_state == ElementState::Pressed => {
                if state.has_valid_selection() {
                    // 这里需要外部传入 scale 信息来创建 region
                    EventResult::Finish
                } else {
                    EventResult::Continue(false)
                }
            }
            _ => EventResult::Continue(false),
        }
    }

    /// 智能鼠标移动事件处理 - 上下文感知优化
    ///
    /// # 参数
    /// * `state` - 选择状态
    /// * `new_pos` - 新的鼠标位置
    ///
    /// # 返回值
    /// 是否需要重绘
    pub fn handle_cursor_moved(state: &mut SelectionState, new_pos: (f64, f64)) -> bool {
        let distance = ((new_pos.0 - state.last_cursor_pos.0).powi(2)
            + (new_pos.1 - state.last_cursor_pos.1).powi(2))
        .sqrt();

        // 动态阈值：根据当前选择区域大小调整移动敏感度
        let current_area = {
            let (x0, y0, x1, y1) = state.calculate_selection_rect();
            (x1 - x0).abs() * (y1 - y0).abs()
        };

        // 选择区域越大，移动阈值越大，减少不必要的重绘
        let threshold = if current_area > 50000.0 {
            4.0 // 大区域：4像素阈值
        } else if current_area > 10000.0 {
            2.5 // 中等区域：2.5像素阈值
        } else if state.dragging {
            1.5 // 拖拽中：1.5像素阈值
        } else {
            3.0 // 非拖拽：3像素阈值
        };

        if distance > threshold || !state.dragging {
            state.curr = new_pos;
            state.last_cursor_pos = new_pos;

            // 只有在拖拽或选择状态改变时才重绘
            if state.dragging || state.alt_down {
                return true;
            }
        }

        false
    }

    /// 处理鼠标按键事件
    ///
    /// # 参数
    /// * `state` - 选择状态
    /// * `button` - 鼠标按键
    /// * `button_state` - 按键状态
    ///
    /// # 返回值
    /// 事件处理结果
    pub fn handle_mouse_input(
        state: &mut SelectionState,
        button: MouseButton,
        button_state: ElementState,
    ) -> EventResult {
        match (button, button_state) {
            (MouseButton::Left, ElementState::Pressed) => {
                state.dragging = true;
                state.start = state.curr;
                EventResult::Continue(true)
            }
            (MouseButton::Left, ElementState::Released) => {
                state.dragging = false;
                if state.has_valid_selection() {
                    EventResult::Finish
                } else {
                    EventResult::Continue(false)
                }
            }
            _ => EventResult::Continue(false),
        }
    }

    /// 转换鼠标位置坐标
    ///
    /// # 参数
    /// * `position` - winit 鼠标位置
    /// * `virtual_x`, `virtual_y` - 窗口的虚拟桌面位置
    /// * `virtual_bounds` - 虚拟桌面边界
    /// * `scale` - 窗口缩放比例
    ///
    /// # 返回值
    /// 转换后的坐标
    pub fn convert_cursor_position(
        position: winit::dpi::PhysicalPosition<f64>,
        virtual_x: i32,
        virtual_y: i32,
        virtual_bounds: Option<(i32, i32, u32, u32)>,
        scale: f64,
    ) -> (f64, f64) {
        if virtual_bounds.is_some() {
            // 虚拟桌面模式：使用物理像素，叠加窗口在虚拟桌面的偏移
            (virtual_x as f64 + position.x, virtual_y as f64 + position.y)
        } else {
            // 非虚拟模式：转换为逻辑坐标，便于与 UI 一致
            let logical_pos = position.to_logical::<f64>(scale);
            (logical_pos.x, logical_pos.y)
        }
    }

    /// 检查选择区域是否与窗口有交集（用于渲染优化）
    ///
    /// # 参数
    /// * `state` - 选择状态
    /// * `window_virtual_x`, `window_virtual_y` - 窗口的虚拟位置
    /// * `window_width`, `window_height` - 窗口尺寸
    ///
    /// # 返回值
    /// 如果选择区域与窗口有交集返回 true
    pub fn selection_intersects_window(
        state: &SelectionState,
        window_virtual_x: i32,
        window_virtual_y: i32,
        window_width: u32,
        window_height: u32,
    ) -> bool {
        if !state.dragging && !state.alt_down {
            return false;
        }

        let (x0, y0, x1, y1) = state.calculate_selection_rect();

        if (x1 - x0).abs() <= 1.0 && (y1 - y0).abs() <= 1.0 {
            return false; // 选择区域太小
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
            true // 非虚拟模式默认需要渲染
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
