//! 坐标转换工具模块
//!
//! 提供虚拟桌面坐标与窗口本地坐标之间的转换功能

/// 将虚拟桌面坐标转换为窗口本地坐标
///
/// # 参数
/// * `virtual_x` - 窗口在虚拟桌面中的X位置
/// * `virtual_y` - 窗口在虚拟桌面中的Y位置
/// * `virtual_bounds` - 虚拟桌面边界 (min_x, min_y, width, height)
/// * `vx0`, `vy0`, `vx1`, `vy1` - 虚拟桌面坐标系中的矩形区域
///
/// # 返回值
/// 窗口本地坐标系中的矩形区域 (x0, y0, x1, y1)
pub fn virtual_to_window_coords(
    virtual_x: i32,
    virtual_y: i32,
    virtual_bounds: Option<(i32, i32, u32, u32)>,
    vx0: i32,
    vy0: i32,
    vx1: i32,
    vy1: i32,
) -> (usize, usize, usize, usize) {
    if virtual_bounds.is_some() {
        // 虚拟桌面模式：将虚拟坐标转换为窗口本地坐标
        let local_x0 = vx0 - virtual_x;
        let local_y0 = vy0 - virtual_y;
        let local_x1 = vx1 - virtual_x;
        let local_y1 = vy1 - virtual_y;

        // 检查选择区域是否与窗口有交集
        // 如果完全在窗口外，返回无效坐标
        if local_x1 <= 0 || local_y1 <= 0 {
            return (0, 0, 0, 0); // 选择区域在窗口左上方，无交集
        }

        // 将坐标限制在窗口范围内，但保持原始范围关系
        let final_x0 = local_x0.max(0) as usize;
        let final_y0 = local_y0.max(0) as usize;
        let final_x1 = local_x1.max(0) as usize;
        let final_y1 = local_y1.max(0) as usize;

        println!(
            "🐛 坐标转换: 虚拟({},{},{},{}) 窗口位置({},{}) -> 本地({},{},{},{}) -> 最终({},{},{},{})",
            vx0, vy0, vx1, vy1, virtual_x, virtual_y,
            local_x0, local_y0, local_x1, local_y1,
            final_x0, final_y0, final_x1, final_y1
        );

        (final_x0, final_y0, final_x1, final_y1)
    } else {
        (vx0 as usize, vy0 as usize, vx1 as usize, vy1 as usize)
    }
}

/// 用于检查区域交集的参数结构体
pub struct IntersectCheck {
    pub window_x: f64,
    pub window_y: f64,
    pub window_w: u32,
    pub window_h: u32,
    pub selection_x0: f64,
    pub selection_y0: f64,
    pub selection_x1: f64,
    pub selection_y1: f64,
}

/// 检查选择区域是否与窗口有交集
///
/// # 参数
/// * `params` - 包含窗口和选择区域坐标的结构体
///
/// # 返回值
/// 如果有交集返回 true，否则返回 false
pub fn check_selection_intersects_window(params: &IntersectCheck) -> bool {
    let window_x_end = params.window_x + params.window_w as f64;
    let window_y_end = params.window_y + params.window_h as f64;

    // 检查选择区域是否与窗口区域有交集
    !(params.selection_x1 <= params.window_x
        || params.selection_x0 >= window_x_end
        || params.selection_y1 <= params.window_y
        || params.selection_y0 >= window_y_end)
}
