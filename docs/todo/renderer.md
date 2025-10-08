# renderer 模块 TODO

## 当前状态（v0.1 已完成）
- ✅ Image 结构定义
- ✅ Rect 绘制（填充 + 描边）
- ✅ Arrow 绘制（实线 + 虚线 + 箭头头）
- ✅ Highlight（Multiply/Screen 混合模式）
- ✅ Mosaic 马赛克滤镜
- ✅ Freehand 手绘 + Chaikin 平滑
- ✅ Text 占位实现（字符块）
- ✅ Z 排序渲染
- ✅ PNG 编码
- ✅ JPEG 编码
- ✅ 完整测试套件

## v0.2 - 渲染质量提升
- [ ] 抗锯齿支持（线条、箭头）
- [ ] 真正的字形渲染（集成 fontdue）
- [ ] 圆角矩形支持（当前 corner_radius 未使用）
- [ ] 渐变填充（线性、径向）
- [ ] 阴影效果（drop shadow）

## v0.3 - 性能优化
- [ ] DirtyRect 局部重绘
- [ ] SIMD 向量化混合操作
- [ ] Glyph cache 字形缓存
- [ ] Mosaic SIMD 优化
- [ ] 多线程渲染（大尺寸截图分块）

## v0.4 - 与 Skia 集成
- [ ] 创建 SkiaAnnotationRenderer（GPU 加速）
- [ ] 统一 Renderer trait（CPU 和 GPU 实现）
- [ ] 性能对比测试（CPU vs GPU）
- [ ] 降级策略（GPU 失败时使用 CPU）
- [ ] 保留当前 SimpleRenderer 作为 fallback

## v1.0 - 高级功能
- [ ] 图层合成优化
- [ ] 更多混合模式（Overlay、Soft Light 等）
- [ ] 滤镜效果（模糊、锐化、色彩调整）
- [ ] 矢量输出（SVG 导出）
- [ ] PDF 导出支持

## 持续维护
- [ ] 快照测试基线（golden file testing）
- [ ] 性能基准测试（不同尺寸、标注数量）
- [ ] 内存泄漏检测
- [ ] 跨平台渲染一致性验证
