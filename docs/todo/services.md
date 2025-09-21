# services 模块 todo

## MVP (进度)

 - [x] 原生区域截图 (mac screencapture -R 集成)

## 多显示器服务增强 (优先级 M2)
- [ ] MultiDisplayService 实现：显示器发现、元数据管理、捕获协调
- [ ] CaptureService 多显示器扩展：批量捕获、错误隔离、并行处理
- [ ] 跨显示器区域截图服务：虚拟桌面坐标处理、区域分解与合成
- [ ] RegionSelectService 多显示器集成：跨显示器选择、坐标转换
- [ ] ExportService 批量导出：多文件命名、批量历史记录
- [ ] HistoryService 多显示器支持：显示器信息记录、筛选功能

## m1
 - [x] CaptureService 骨架
 - [x] AnnotationService CRUD + Undo
 - [x] ExportService 初版 (含 PNG)

## m2
- [x] 平台 capturer 集成 (mac)
- [x] 缩略图写入 (已有 History 持久化框架)
- [x] HistoryService 持久化 (JSONL)
- [x] Export JPEG (renderer 已具备)
- [x] PrivacyService stub
- [x] OcrService stub
 - [x] OcrService 简易线程池队列 (占位)
 - [x] PrivacyService 邮箱/手机号正则扫描基础
 - [x] RegionSelectService 集成（调用 ui_overlay 框选 UI，返回 Rect）

## m4
- [ ] HookService + 超时
- [ ] 全事件发布
- [ ] Export 并发限制

## m5
- [ ] 性能调优 buffer 池

## 持续
- [ ] 集成测试完善
- [ ] 事件负载回归
 - [ ] 剪贴板失败重试策略实现 (当前直接返回)
 - [ ] Export 并发写锁/限速评估
 - [ ] History 查询/过滤 API
	- [ ] 剪贴板失败重试策略监控（已添加一次重试，实现后需指标化）
 - [x] 基础 Metrics 计数/耗时导出 (infra::metrics)
