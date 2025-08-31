# renderer 模块技术设计

## 职责
Annotation 矢量 -> RGBA 合成：形状、文本栅格、混合、马赛克、排序、导出。

## MVP 范围
仅要求 Rect / Arrow / Text(占位) 三类标注与 PNG 编码满足导出需求。已存在的 Highlight / Mosaic / Freehand 实现暂不纳入 MVP 验收（可保留代码，测试延后）。

退出 MVP 后再引入：真正文本栅格、DirtyRect、SIMD、Glyph cache 指标。

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
