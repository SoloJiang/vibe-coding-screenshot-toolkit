//! 坐标转换工具模块
//!
//! 提供虚拟桌面坐标与窗口本地坐标之间的转换功能

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
