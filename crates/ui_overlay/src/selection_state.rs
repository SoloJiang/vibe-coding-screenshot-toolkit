/// 选择状态管理模块
///
/// 管理选择框的状态、修饰键、重绘控制等
use crate::Region;

/// 选择器应用的状态
///
/// 包含选择框的位置、大小、修饰键状态以及性能优化相关的缓存
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
    /// Shift 键是否按下（用于正方形选择）
    pub shift_down: bool,
    /// Alt 键是否按下（用于中心扩展选择）
    pub alt_down: bool,
    /// 是否有重绘请求待处理
    pub redraw_pending: bool,
    /// 虚拟桌面边界信息 (min_x, min_y, width, height)
    pub virtual_bounds: Option<(i32, i32, u32, u32)>,
    /// 上次重绘时间（用于 60 FPS 限流）
    pub last_redraw_time: std::time::Instant,
    /// 缓存的选择矩形（性能优化：避免重复计算）
    cached_rect: Option<(f64, f64, f64, f64)>,
    /// 缓存是否有效
    cache_valid: bool,
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
            cached_rect: None,
            cache_valid: false,
        }
    }

    /// 计算当前选择矩形（带缓存优化）
    ///
    /// 基于当前 start/curr 位置与修饰键状态计算矩形。
    ///
    /// 返回值：(x0, y0, x1, y1) 矩形坐标
    ///
    /// 支持的模式：
    /// - 普通模式：从起点到当前点的矩形
    /// - Shift：正方形选择
    /// - Alt：以起点为中心扩展
    /// - Shift + Alt：以起点为中心的正方形
    pub fn calculate_selection_rect(&mut self) -> (f64, f64, f64, f64) {
        if self.cache_valid {
            return self.cached_rect.unwrap();
        }

        let (sx, sy) = self.start;
        let (cx, cy) = self.curr;
        let dx = cx - sx;
        let dy = cy - sy;

        let result = if self.alt_down && self.shift_down {
            // 以起点为中心的正方形
            let side = dx.abs().max(dy.abs());
            (sx - side, sy - side, sx + side, sy + side)
        } else if self.alt_down {
            // 以起点为中心的矩形
            (sx - dx.abs(), sy - dy.abs(), sx + dx.abs(), sy + dy.abs())
        } else if self.shift_down {
            // 正方形（左上/右下随拖拽方向变化）
            let side = dx.abs().max(dy.abs());
            let sxn = if dx >= 0.0 { sx } else { sx - side };
            let syn = if dy >= 0.0 { sy } else { sy - side };
            let exn = if dx >= 0.0 { sx + side } else { sx };
            let eyn = if dy >= 0.0 { sy + side } else { sy };
            (sxn, syn, exn, eyn)
        } else {
            (sx.min(cx), sy.min(cy), sx.max(cx), sy.max(cy))
        };

        self.cached_rect = Some(result);
        self.cache_valid = true;
        result
    }

    /// 使缓存失效
    ///
    /// 在状态变化（移动、修饰键变化等）时调用
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
    }

    /// 检查选择区域是否有效（非零尺寸）
    pub fn has_valid_selection(&mut self) -> bool {
        let (sx, sy, ex, ey) = self.calculate_selection_rect();
        sx != ex && sy != ey
    }

    /// 将当前选择转换为 Region 对象
    pub fn to_region(&mut self, scale: f32) -> Option<Region> {
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

    /// 重绘频率控制
    ///
    /// 限制到 60 FPS (16ms 间隔) 以优化性能
    pub fn should_throttle_redraw(&self) -> bool {
        if self.redraw_pending {
            return true;
        }

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_redraw_time).as_millis();

        elapsed < 16
    }

    /// 标记重绘请求已发送
    pub fn mark_redraw_requested(&mut self) {
        self.redraw_pending = true;
        self.last_redraw_time = std::time::Instant::now();
    }

    /// 清除重绘标记
    pub fn clear_redraw_pending(&mut self) {
        self.redraw_pending = false;
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
        state.invalidate_cache(); // 确保缓存失效
        assert!(!state.has_valid_selection());

        state.curr = (200.0, 200.0);
        state.invalidate_cache(); // 确保缓存失效
        assert!(state.has_valid_selection());
    }

    #[test]
    fn test_throttle_redraw() {
        let mut state = SelectionState::new(None);
        state.redraw_pending = true;
        assert!(state.should_throttle_redraw());

        state.redraw_pending = false;
        // 等待超过16ms，应该不节流
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!state.should_throttle_redraw());
    }
}
