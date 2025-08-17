# core 模块 todo

(迁移自 TODO_core.md)

## m1
- [x] 定义数据结构 Screenshot / Frame / FrameSet
- [x] 定义 AnnotationMeta / AnnotationKind / UndoOp / UndoStack
- [x] Error / ErrorKind + thiserror 实现
- [x] 命名模板解析器 (date/seq/screen) + 单测
 - [x] UndoStack merge 策略 + 单测 (拖拽/属性修改)
 - [x] 吸附算法实现 + 单测
- [ ] HistoryItem 内存结构 + 容量裁剪策略
- [x] Renderer/ExportEncoder Trait 协议定义

## m2
- [ ] Annotation 序列化 (serde) 与版本字段预留
- [ ] HistoryItem 序列化/反序列化

## m3
- [ ] 扩展 ErrorKind: Ocr/Privacy 适配映射

## m4
- [ ] 性能基准：Undo 合并，命名模板解析

## m5
- [ ] 评估为 AnnotationKind 增加 dynamic 注册机制 (宏)

## 持续
- [ ] 文档注释添加示例 (pub API)
- [ ] Clippy 提示清理
