# ui_overlay TODO

## MVP
- [x] 定义基础 API：Region / RegionSelector / 错误类型
- [x] 集成 platform_mac：`MacCapturer::capture_region_interactive_custom` 使用 `RegionSelector`
- [x] CLI 接线：`capture-interactive` 调用 `create_gui_region_selector`
- [ ] 集成 services：RegionSelectService 使用 ui_overlay（后续 PR）
- [ ] 为 renderer/export 提供区域像素裁剪适配（由 services 完成）

## 细化与增强
- [x] 半透明蒙层 + 描边绘制（pixels）
- [x] 移除 mac_selector，统一 `selector`（winit + pixels）实现
- [ ] 键盘快捷：Shift 固定比例、Alt 从中心、方向键 n 像素移动
- [ ] 多显示器支持：选择跨屏坐标与 scale 处理
- [ ] 吸附：对齐屏幕边/网格/窗口（可选）
- [ ] 可配置主题/颜色/线宽
- [ ] 点击穿透/置顶策略（Win 专项）

## 文档/测试
- [x] 技术设计文档 `docs/tech_design/ui_overlay.md`
- [x] README 更新（根与 crate）
- [x] 示例删除与 Changelog 记录（内部记录）
