# core 模块 todo

## 当前状态
- ✅ 数据结构 Screenshot / Frame / FrameSet
- ✅ 命名模板解析 + 单测
- ✅ Error / ErrorKind + thiserror 实现

## 交互式截图专用优化 (优先级 M1)
- [ ] 命名模板增强：支持更多时间格式变量
- [ ] Screenshot 元数据：记录截图来源（交互式、显示器信息等）
- [ ] 错误处理完善：针对交互式截图的特定错误类型
- [ ] 性能优化：大尺寸截图的内存管理

## 多显示器数据模型 (优先级 M2)
- [ ] DisplayInfo 结构体：显示器 ID、名称、边界、DPI、主显示器标识
- [ ] VirtualDesktop 结构体：虚拟桌面边界和显示器列表
- [ ] CrossDisplayRegion 结构体：跨显示器区域描述
- [ ] FrameSet 多显示器扩展：显示器布局信息、按 ID 获取 Frame

## 扩展功能 (优先级 M3+)
- [ ] 序列化优化：针对大型截图的高效序列化
- [ ] 压缩支持：Frame 数据的可选压缩存储
- [ ] 元数据标准化：符合常见图片元数据标准

## 移除的功能（已完成清理）
- ✅ 移除 Annotation 相关数据结构
- ✅ 移除 UndoStack 相关逻辑
- ✅ 移除 HistoryItem 相关结构
- ✅ 简化 ErrorKind，仅保留交互式截图相关的错误类型

## 持续维护
- [ ] 文档注释完善
- [ ] 单元测试覆盖率提升
- [ ] 性能基准测试
