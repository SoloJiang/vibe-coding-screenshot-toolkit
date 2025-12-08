/// 工具栏 UI 渲染模块
///
/// 负责渲染工具栏界面和处理交互
use super::{AnnotationTool, IconCache, Toolbar};

/// 工具栏操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    /// 无操作
    None,
    /// 点击完成按钮
    Finish,
    /// 点击取消按钮
    Cancel,
    /// 选择工具
    SelectTool(AnnotationTool),
}

/// 工具栏渲染器
pub struct ToolbarRenderer {
    #[allow(dead_code)]
    icon_cache: IconCache,
}

impl ToolbarRenderer {
    /// 创建新的工具栏渲染器
    pub fn new() -> Self {
        Self {
            icon_cache: IconCache::new(),
        }
    }

    /// 渲染工具栏
    ///
    /// 返回用户的操作
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        toolbar: &mut Toolbar,
        selection_rect: (f32, f32, f32, f32), // (x, y, w, h)
        window_size: (f32, f32),              // (width, height)
    ) -> ToolbarAction {
        if !toolbar.visible {
            return ToolbarAction::None;
        }

        let mut action = ToolbarAction::None;

        // 计算工具栏位置
        let toolbar_pos = self.calculate_position(selection_rect, window_size);

        // 创建工具栏窗口
        egui::Window::new("toolbar")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .fixed_pos(toolbar_pos)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(40, 40, 40, 230))
                    .corner_radius(8.0)
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // 工具按钮区域
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if self.render_tool_button(
                                ui,
                                "矩形",
                                toolbar.selected_tool == Some(AnnotationTool::Rectangle),
                            ) {
                                toolbar.select_tool(AnnotationTool::Rectangle);
                                action = ToolbarAction::SelectTool(AnnotationTool::Rectangle);
                            }

                            if self.render_tool_button(
                                ui,
                                "箭头",
                                toolbar.selected_tool == Some(AnnotationTool::Arrow),
                            ) {
                                toolbar.select_tool(AnnotationTool::Arrow);
                                action = ToolbarAction::SelectTool(AnnotationTool::Arrow);
                            }

                            if self.render_tool_button(
                                ui,
                                "画笔",
                                toolbar.selected_tool == Some(AnnotationTool::Freehand),
                            ) {
                                toolbar.select_tool(AnnotationTool::Freehand);
                                action = ToolbarAction::SelectTool(AnnotationTool::Freehand);
                            }
                        });
                    });

                    ui.separator();

                    // 颜色选择器
                    self.render_color_picker(ui, toolbar);

                    ui.separator();

                    // 操作按钮
                    if let Some(btn_action) = self.render_action_buttons(ui) {
                        action = btn_action;
                    }
                });
            });

        action
    }

    /// 计算工具栏位置
    ///
    /// 默认位于选择框下方，如果空间不足则显示在上方
    fn calculate_position(
        &self,
        selection_rect: (f32, f32, f32, f32),
        window_size: (f32, f32),
    ) -> egui::Pos2 {
        let (sel_x, sel_y, sel_w, sel_h) = selection_rect;
        let (win_w, win_h) = window_size;

        // 工具栏估计尺寸
        const TOOLBAR_WIDTH: f32 = 400.0;
        const TOOLBAR_HEIGHT: f32 = 50.0;
        const SPACING: f32 = 10.0;

        // 计算水平居中位置
        let mut x = sel_x + (sel_w - TOOLBAR_WIDTH) / 2.0;
        x = x.max(10.0).min(win_w - TOOLBAR_WIDTH - 10.0);

        // 计算垂直位置（优先下方）
        let y_below = sel_y + sel_h + SPACING;
        let y_above = sel_y - TOOLBAR_HEIGHT - SPACING;

        let y = if y_below + TOOLBAR_HEIGHT + 10.0 < win_h {
            y_below
        } else if y_above > 10.0 {
            y_above
        } else {
            // 如果上下都放不下，放在窗口底部
            win_h - TOOLBAR_HEIGHT - 10.0
        };

        egui::pos2(x, y)
    }

    /// 渲染工具按钮
    fn render_tool_button(&mut self, ui: &mut egui::Ui, label: &str, selected: bool) -> bool {
        let button = egui::Button::new(label)
            .min_size(egui::vec2(60.0, 32.0))
            .fill(if selected {
                egui::Color32::from_rgb(70, 130, 180)
            } else {
                egui::Color32::from_rgb(60, 60, 60)
            });

        ui.add(button).clicked()
    }

    /// 渲染颜色选择器
    fn render_color_picker(&mut self, ui: &mut egui::Ui, toolbar: &mut Toolbar) {
        ui.horizontal(|ui| {
            ui.label("颜色:");
            egui::color_picker::color_edit_button_srgba(
                ui,
                &mut toolbar.selected_color,
                egui::color_picker::Alpha::Opaque,
            );
        });
    }

    /// 渲染操作按钮（完成/取消）
    fn render_action_buttons(&mut self, ui: &mut egui::Ui) -> Option<ToolbarAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            let finish_btn = egui::Button::new("✓ 完成")
                .min_size(egui::vec2(70.0, 32.0))
                .fill(egui::Color32::from_rgb(50, 150, 50));

            if ui.add(finish_btn).clicked() {
                action = Some(ToolbarAction::Finish);
            }

            let cancel_btn = egui::Button::new("✕ 取消")
                .min_size(egui::vec2(70.0, 32.0))
                .fill(egui::Color32::from_rgb(150, 50, 50));

            if ui.add(cancel_btn).clicked() {
                action = Some(ToolbarAction::Cancel);
            }
        });

        action
    }
}

impl Default for ToolbarRenderer {
    fn default() -> Self {
        Self::new()
    }
}
