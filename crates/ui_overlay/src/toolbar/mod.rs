/// 工具栏模块
///
/// 提供标注工具栏的 UI 和交互逻辑
mod icons;
pub mod ui;
pub mod window;

pub use icons::IconCache;
pub use ui::{ToolbarAction, ToolbarRenderer};
pub use window::ToolbarWindow;

/// 标注工具类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTool {
    /// 矩形标注
    Rectangle,
    /// 箭头标注
    Arrow,
    /// 自由画笔
    Freehand,
}

/// 工具栏状态
#[derive(Debug, Clone)]
pub struct Toolbar {
    /// 当前选中的工具
    pub selected_tool: Option<AnnotationTool>,
    /// 当前选中的颜色
    pub selected_color: egui::Color32,
    /// 笔画宽度
    pub stroke_width: f32,
    /// 工具栏是否可见
    pub visible: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            selected_tool: None,
            selected_color: egui::Color32::from_rgb(255, 0, 0), // 默认红色
            stroke_width: 2.0,
            visible: false,
        }
    }
}

impl Toolbar {
    /// 创建新的工具栏
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示工具栏
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏工具栏
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 选择工具
    pub fn select_tool(&mut self, tool: AnnotationTool) {
        self.selected_tool = Some(tool);
    }

    /// 取消选择工具
    pub fn deselect_tool(&mut self) {
        self.selected_tool = None;
    }
}
