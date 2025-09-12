# renderer 模块技术设计

## 职责
Annotation 矢量 -> RGBA 合成：形状、文本栅格、混合、马赛克、排序、导出。

## MVP 范围
当前要求：Rect / Arrow / Text(占位) 三类标注与 PNG 编码满足导出需求。模型中存在 Highlight / Mosaic / Freehand 实现，但不纳入当期验收。

## 渲染管线
排序 -> 遍历匹配 Kind -> 绘制 (Rect/Highlight/Arrow/Mosaic/Text 占位/Freehand) -> 输出 Image -> 编码 PNG/JPEG。

## 组件
Painter, TextRasterizer(占位), MosaicFilter, BlendOps。

## 性能
常规分辨率下同步渲染可接受；更深入的优化（DirtyRect、SIMD、Glyph cache）不在当前范围。

## 测试
单元：各形状像素。

## 风险
| 风险 | 缓解 |
|------|------|
| 文本锯齿 | 后期 swash |
| 标注多导致 O(N^2) | 预分配/局部重绘 |

## 备注
可能的扩展方向（GPU/wgpu、局部重绘、fontdue）留待后续讨论。
