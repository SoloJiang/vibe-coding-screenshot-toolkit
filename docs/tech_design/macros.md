# macros 模块技术设计

## 职责
派生宏：错误码映射、DTO 辅助、AnnotationKind 扩展潜力。

## MVP 状态
宏全部延后；MVP 不新增派生宏，仅手写代码保持清晰。

## 设计思路
ErrorCode derive, NapiDto derive。

## 风险
| 风险 | 缓解 |
|------|------|
| 编译时间增加 | 控制宏数量 |
