# services 模块技术设计
(迁移自 TechDesign_services.md)

## 职责
捕获、标注 CRUD、导出、历史、OCR、隐私、Hook、上传编排。

## 服务
CaptureService, AnnotationService, ExportService, HistoryService, OcrService, PrivacyService, HookService(M4)。

## 流程示例
capture.start -> capturer.capture -> Screenshot -> 事件 capture_done。
export -> render -> encode -> (upload) -> 剪贴板/历史 -> 事件 export。

## 并发
spawn_blocking 渲染；OCR 线程池。

## 错误
统一 core::Error。

## 事件
CaptureDone, ExportDone, OcrDone, PrivacyScanDone, HookResult。

## 风险
| 风险 | 缓解 |
|------|------|
| 导出阻塞 runtime | spawn_blocking |
| Hook 阻塞 | 超时 + 互斥 |
