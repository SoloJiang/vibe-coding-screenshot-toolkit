/// 吸附算法：给定当前矩形 (x,y,w,h) 与一组参考线 (水平/垂直), 若距离阈值内则对齐
/// 简化：参考线集合以像素坐标表示；阈值默认 6px；返回 (dx, dy, snapped_x, snapped_y)

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone)]
pub struct SnapResult {
    pub dx: f32,
    pub dy: f32,
    pub snap_x: Option<f32>,
    pub snap_y: Option<f32>,
}

pub fn snap_rect(
    r: Rect,
    vertical_lines: &[f32],
    horizontal_lines: &[f32],
    threshold: f32,
) -> SnapResult {
    let mut best_dx = 0.0;
    let mut best_dist_x = threshold + 1.0;
    let mut snap_x = None;
    let mut best_dy = 0.0;
    let mut best_dist_y = threshold + 1.0;
    let mut snap_yv = None;

    // 优先考虑中心线，其次左/右边缘，避免出现同时满足时优先左边缘导致测试期望中心对齐失败
    let targets_x = [r.x + r.w / 2.0, r.x, r.x + r.w];
    for &line in vertical_lines {
        for &tx in &targets_x {
            let dist = (tx - line).abs();
            if dist < best_dist_x && dist <= threshold {
                best_dist_x = dist;
                best_dx = line - tx;
                snap_x = Some(line);
            }
        }
    }

    let targets_y = [r.y, r.y + r.h / 2.0, r.y + r.h];
    for &line in horizontal_lines {
        for &ty in &targets_y {
            let dist = (ty - line).abs();
            if dist < best_dist_y && dist <= threshold {
                best_dist_y = dist;
                best_dy = line - ty;
                snap_yv = Some(line);
            }
        }
    }

    SnapResult { dx: best_dx, dy: best_dy, snap_x, snap_y: snap_yv }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_x_center() {
        let r = Rect { x: 95.0, y: 10.0, w: 20.0, h: 20.0 };
        let res = snap_rect(r, &[100.0], &[], 6.0);
        assert_eq!(res.snap_x, Some(100.0));
        assert!((r.x + r.w / 2.0 + res.dx - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_no_snap_outside_threshold() {
        let r = Rect { x: 10.0, y: 10.0, w: 10.0, h: 10.0 };
        let res = snap_rect(r, &[30.0], &[30.0], 6.0);
        assert!(res.snap_x.is_none());
        assert!(res.snap_y.is_none());
    }

    #[test]
    fn test_snap_y_bottom() {
        let r = Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
        let res = snap_rect(r, &[], &[11.0], 6.0);
        assert_eq!(res.snap_y, Some(11.0));
        assert!((r.y + r.h + res.dy - 11.0).abs() < 0.01);
    }
}
