# macros 模块技术设计
(迁移自 TechDesign_macros.md)

## 职责
派生宏：错误码映射、DTO 辅助、AnnotationKind 扩展潜力。

## 设计思路
ErrorCode derive, NapiDto derive。

## 风险
| 风险 | 缓解 |
|------|------|
| 编译时间增加 | 控制宏数量 |
