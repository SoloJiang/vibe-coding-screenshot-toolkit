# core 模块 todo

## MVP
- 已完：数据结构 Screenshot / Frame / FrameSet
- 已完：AnnotationMeta / AnnotationKind (Rect / Arrow / Text 用于 MVP)
- 已完：UndoStack + 合并策略 + Redo 支持 + 单测
- 已完：命名模板解析 + 单测
- 已完：HistoryItem 内存结构 + 裁剪策略 + 单测
- 已完：Annotation/History serde（Frame 不序列化）
- 已完：Annotation 更新/删除/重排：更新 + 合并（通过 merge_key）已在 services 层提供；删除/重排留待 UI 集成

> 说明：以下原里程碑保持原顺序，仅在 m1 中勾选已完成；未移动条目，仅通过 MVP 块声明使用子集。

## m1
- [x] 定义数据结构 Screenshot / Frame / FrameSet
- [x] 定义 AnnotationMeta / AnnotationKind / UndoOp / UndoStack
- [x] Error / ErrorKind + thiserror 实现
- [x] 命名模板解析器 (date/seq/screen) + 单测
 - [x] UndoStack merge 策略 + 单测 (拖拽/属性修改)
 - [x] 吸附算法实现 + 单测
- [x] HistoryItem 内存结构 + 容量裁剪策略 (实现：push_history_trim 已含按时间裁剪 + 单测 history_tests::trim_history_capacity)
- [x] Renderer/ExportEncoder Trait 协议定义

## m2
- [x] Annotation 序列化 (serde) 基本实现（model.rs 内 #[derive(Serialize,Deserialize)]；版本字段仍待后续扩展策略设计）
- [x] HistoryItem 序列化/反序列化（已含 version 字段；load_from_disk 对 version==0 兼容升级为 1）
 - [x] 命名序列持久化支持：`set_sequence_for` / `current_sequence` API（CLI 已使用）

## m3
- [ ] 扩展 ErrorKind: Ocr/Privacy 适配映射

## m4
- [ ] 性能基准：Undo 合并，命名模板解析

## m5
- [ ] 评估为 AnnotationKind 增加 dynamic 注册机制 (宏)

## 持续
- [ ] 文档注释添加示例 (pub API)
- [ ] Clippy 提示清理
 - [x] 命名序列跨日自动测试（已添加 tests/naming_seq_cross_day.rs）
