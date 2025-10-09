# core 模块 TODO

## 当前状态
- ✅ 数据结构 Screenshot / Frame / FrameSet
- ✅ Annotation 完整模型（7 种标注类型）
- ✅ UndoStack 撤销/重做机制
- ✅ HistoryItem 历史记录结构
- ✅ 命名模板解析 + 单测
- ✅ Error / ErrorKind + thiserror 实现

## v0.2 - 标注编辑增强
- [ ] Annotation 序列化性能优化（大量标注场景）
- [ ] Region 增加更多辅助方法（contains_point, intersects 等）
- [ ] 命名模板 LRU 缓存（避免重复正则匹配）
- [ ] UndoStack 内存使用监控和自动裁剪

## v0.3 - 多显示器数据模型完善
- [ ] DisplayInfo 结构体完善（亮度、方向等元数据）
- [ ] VirtualDesktop 序列化支持（配置保存/恢复）
- [ ] CrossDisplayRegion 结构体（跨显示器区域描述）
- [ ] FrameSet 多显示器扩展（按 ID 索引 Frame）

## v0.4 - 性能优化
- [ ] Annotation 空间索引（四叉树，用于快速碰撞检测）
- [ ] Frame 延迟加载（大尺寸截图按需读取）
- [ ] 命名模板预编译正则缓存
- [ ] UndoOp 快照压缩（仅记录 diff 而非完整状态）

## v1.0 - 扩展功能
- [ ] Annotation 动画属性（淡入淡出、位移等）
- [ ] 更多 AnnotationKind（椭圆、多边形、贝塞尔曲线）
- [ ] 压缩支持：Frame 数据的可选压缩存储
- [ ] 元数据标准化：符合 EXIF/PNG tEXt 等标准

## 持续维护
- [ ] 文档注释完善（所有公开 API）
- [ ] 单元测试覆盖率提升到 85%+
- [ ] 性能基准测试（大数据量场景）
- [ ] 模糊测试（fuzz testing）输入验证
