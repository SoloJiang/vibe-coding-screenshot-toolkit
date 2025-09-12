# services 模块技术设计

## 职责
编排核心业务：捕获、标注、导出、历史；扩展期再加入 OCR、隐私、上传、Hook。

当前聚焦：CaptureService / AnnotationService / ExportService / HistoryService（最小）。
备注：OcrService / PrivacyService / HookService / Upload 管线不在当前范围。

## 服务
MVP: CaptureService, AnnotationService, ExportService, HistoryService。
补充：RegionSelectService（调用 UI overlay 选择矩形，输出 Screenshot 子集 + 事件）。

## 流程示例 (MVP)
capture_full -> platform_capturer.capture -> Screenshot
export_png -> renderer.render -> encoder.png -> save / clipboard -> history.append

## 自定义框选 UI
独立 crate `ui_overlay` 提供：
- RegionSelector trait: select() -> Result<Region>；可选 select_with_background(...)，用户取消 -> Err(Cancelled)，或由上层转换为 None
- Mac/Win 使用 winit/tao 创建无边框全屏透明窗口，绘制半透明蒙层 + 实时矩形。
- 交互：按下拖拽=调整；Esc 取消；Enter/Space 确认；Shift/Alt 约束比例。
- 输出：返回逻辑显示坐标 + scale，用于对全屏截图裁剪，而非再次系统命令截图。

目标：移除对 `screencapture -R/-i` 的依赖，统一跨平台行为；允许在 UI 层叠加标注/预览。

## 并发
当前：同步或简单阻塞渲染（1080p 性能可接受）。

## 错误
统一 core::Error。

## 事件 (MVP 预留)
可选插桩：CaptureDone, ExportDone（内部调试用）；其余延后。

## 风险
| 风险 | 缓解 |
|------|------|
| 导出阻塞 runtime | spawn_blocking |
| Hook 阻塞 | 超时 + 互斥 |
