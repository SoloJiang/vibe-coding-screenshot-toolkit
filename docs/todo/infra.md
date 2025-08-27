# infra 模块 todo

## MVP
- 已完：IdGenerator (uuid v7)
- 已完：NamingTemplate
- 已完：PathResolver 基础
- 已完：LruCache
- 待做：History/Output 目录预创建（由 services::HistoryService 初始化目录；计划添加 infra 封装）

> 原里程碑保持：

## m1
 - [x] EventBus 封装 (publish/subscribe) + 单测
 - [x] IdGenerator (uuid v7)
 - [x] NamingTemplate 与 core 模板一致实现
 - [x] PathResolver (macOS + Windows 路径占位实现)
 - [x] Config Schema 定义 + 读取/写入 (原子)
 - [x] LruCache 实现 + 单测

## m2
- [x] Clipboard/History 目录预创建 (ensure_directories)
- [x] Panic hook (debug)

## m3
- [ ] privacy/ocr/upload 配置校验
 - [ ] ui_overlay 窗口/透明度/输入事件抽象 (跨平台接口)

## m4
- [ ] CpuFeatures 探测
- [ ] EventBus 丢弃计数指标

## m5
- [x] Metrics 框架评估 (初版实现 counters / histogram / 导出)

## 持续
- [ ] 文档注释 & 示例
- [ ] Clippy 清零
