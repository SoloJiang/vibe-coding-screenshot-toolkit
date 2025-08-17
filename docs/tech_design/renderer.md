# renderer 模块技术设计
(迁移自 TechDesign_renderer.md)

## 职责
Annotation 矢量 -> RGBA 合成：形状、文本栅格、混合、马赛克、排序、导出。

## 渲染管线
排序 -> 遍历匹配 Kind -> 绘制 (Rect/Highlight/Arrow/Mosaic/Text 占位/Freehand) -> 输出 Image -> 编码 PNG/JPEG。

## 组件
Painter, TextRasterizer(占位), MosaicFilter, BlendOps。

## 性能
后续 DirtyRect；SIMD blend/mosaic；Glyph cache。

## 测试
单元：各形状像素；计划快照 diff；性能基准。

## 风险
| 风险 | 缓解 |
|------|------|
| 文本锯齿 | 后期 swash |
| 标注多导致 O(N^2) | 预分配/局部重绘 |

## 扩展
wgpu 后端、局部重绘、fontdue 集成。
