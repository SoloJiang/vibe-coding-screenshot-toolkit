/// 选择状态管理模块
///
/// 管理选择框的状态、修饰键、重绘控制等
use crate::Region;

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
            return false;
        }

        if self.redraw_pending {
            return true;
        }

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_redraw_time).as_millis();

        // 动态阈值：限制到 60fps (16ms) 以优化性能
        // 拖动时也保持 60fps，避免过度重绘导致卡顿
        let threshold = if self.dragging { 16 } else { 32 };
        elapsed < threshold
    }

    /// 标记重绘请求已发送
    pub fn mark_redraw_requested(&mut self) {
        self.redraw_pending = true;
        self.last_redraw_time = std::time::Instant::now();

        if self.redraw_budget > 0 && !self.force_redraw {
            self.redraw_budget -= 1;
        }

        self.force_redraw = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_selection_rect_normal() {
        let mut state = SelectionState::new(None);
        state.start = (100.0, 100.0);
        state.curr = (200.0, 200.0);

        let (x0, y0, x1, y1) = state.calculate_selection_rect();
        assert_eq!((x0, y0, x1, y1), (100.0, 100.0, 200.0, 200.0));
    }

    #[test]
    fn test_calculate_selection_rect_with_shift() {
        let mut state = SelectionState::new(None);
        state.start = (100.0, 100.0);
        state.curr = (200.0, 150.0);
        state.shift_down = true;

        let (x0, y0, x1, y1) = state.calculate_selection_rect();
        // 应该形成正方形
        assert_eq!((x1 - x0), (y1 - y0));
    }

    #[test]
    fn test_has_valid_selection() {
        let mut state = SelectionState::new(None);
        state.start = (100.0, 100.0);
        state.curr = (100.0, 100.0);
        assert!(!state.has_valid_selection());

        state.curr = (200.0, 200.0);
        assert!(state.has_valid_selection());
    }

    #[test]
    fn test_throttle_redraw() {
        let mut state = SelectionState::new(None);
        state.redraw_pending = true;
        assert!(state.should_throttle_redraw());

        state.redraw_pending = false;
        state.force_redraw = true;
        assert!(!state.should_throttle_redraw());
    }
}
