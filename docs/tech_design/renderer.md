# renderer 模块技术设计

## 职责
纯 CPU 像素级别的标注渲染器：将 Annotation 矢量数据渲染到 Frame 像素上，输出 RGBA 图像。

**核心特点**：
- 不依赖 GPU 或 UI 框架
- 纯像素操作，可在服务端/无 GUI 环境运行
- 支持完整的标注类型渲染
- 与 ui_overlay 的 Skia GPU 渲染是**互补关系**

## 定位说明

### 当前架构中的角色
```
┌─────────────────┐       ┌──────────────────┐
│  ui_overlay     │       │   renderer       │
│  (Skia GPU)     │       │   (CPU 像素)      │
├─────────────────┤       ├──────────────────┤
│ • 选区背景      │       │ • 标注合成        │
│ • 选择框边框    │       │ • 离线渲染        │
│ • 实时交互      │       │ • 无 GUI 环境     │
│ • 60fps 流畅度  │       │ • 最终图像导出    │
└─────────────────┘       └──────────────────┘
        ↓                         ↓
   交互阶段                   导出阶段
```

### 与 ui_overlay 的关系
- **ui_overlay/skia**: 用于交互式 UI 渲染（区域选择、未来的标注预览）
- **renderer**: 用于最终导出时的标注合成（Screenshot + Annotations → PNG）

### 未来演进
- v0.1: 保持当前 CPU 渲染（已完成）
- v0.2: 考虑增加 Skia 标注渲染器（GPU 加速），作为 renderer 的补充
- 策略: 保留 CPU renderer 作为 fallback，增加 Skia renderer 作为性能优化

## 渲染管线

### 核心流程
```
Frame (底图)
    ↓
复制像素到 Image (RGBA)
    ↓
按 z 值排序 Annotations
    ↓
遍历渲染每个标注 (match AnnotationKind)
    ↓
输出 Image
    ↓
ExportEncoder::encode_png()
    ↓
PNG bytes
```

### 支持的标注类型

| 类型 | 实现方法 | 特性 |
|------|---------|------|
| **Rect** | blend_fill_rect + stroke_rect | 填充、描边、透明度 |
| **Arrow** | draw_thick_line + draw_arrow_head | 实线/虚线、箭头头部 |
| **Highlight** | highlight_rect with blend mode | Multiply/Screen 混合 |
| **Mosaic** | apply_mosaic block average | 块平均模糊 |
| **Freehand** | Chaikin smoothing + draw_thick_line | 路径平滑、压力感应（未来） |
| **Text** | 占位实现（字符块） | 真正字形渲染需要 fontdue（未来） |

## 核心组件

### Image 结构
```rust
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA, len = width * height * 4
}
```

方法：
- `fill_rgba()`: 填充纯色
- `fill_rect()`: 填充矩形区域
- 像素访问通过 `idx(x, y)` 计算偏移

### SimpleRenderer
```rust
impl Renderer for SimpleRenderer {
    fn render(&self, frame: &Frame, annotations: &[Annotation]) -> Image
}
```

流程：
1. 转换 Frame (BGRA/RGBA) 到 Image
2. 保留原始像素副本（用于 Mosaic 采样）
3. 按 z 值排序标注
4. 逐个渲染标注
5. 返回合成后的 Image

### 混合模式
```rust
enum Blend {
    Multiply,  // 乘法混合（变暗）
    Screen,    // 屏幕混合（变亮）
}
```

实现：
- Multiply: `(src * dst) / 255`
- Screen: `255 - (255 - src) * (255 - dst) / 255`
- Alpha compositing: Porter-Duff over 算法

### ExportEncoder Trait
```rust
pub trait ExportEncoder {
    fn encode_png(&self, img: &Image) -> anyhow::Result<Vec<u8>>;
    fn encode_jpeg(&self, img: &Image, quality: u8) -> anyhow::Result<Vec<u8>>;
}
```

实现：
- **PngEncoder**: 使用 `png` crate，支持快速压缩和 Paeth 过滤
- JPEG: 使用 `image` crate 的 JpegEncoder

## 算法细节

### 矩形渲染
- **填充**: `blend_fill_rect` - Alpha 混合，支持透明度
- **描边**: 四条边的矩形填充（top/bottom/left/right）

### 箭头渲染
- **线条**: Bresenham 算法 + 粗细扩展（方形笔刷）
- **箭头头**: 等腰三角形填充（基于方向向量计算顶点）
- **虚线**: 按 dash_len 和 gap_len 分段绘制

### 马赛克
- **块平均**: 将区域分成 NxN 块，每块取平均颜色
- **块大小**: 根据 level 计算（level 1 = 6px, level 2 = 12px）

### 手绘平滑
- **Chaikin 算法**: 迭代细分路径
- **平滑度**: 根据 smoothing 参数决定迭代次数（0-3 次）
- **防爆炸**: 限制点数不超过 4096

### 文字渲染（占位）
当前实现：
- 按 font_size 计算固定宽度字符块
- 填充彩色矩形作为占位

未来：
- 集成 `fontdue` 进行真正的字形光栅化
- 支持抗锯齿
- 支持字体选择

## 性能特征

### 时间复杂度
- 图像初始化: O(W × H)
- 标注渲染: O(N × A)
  - N = 标注数量
  - A = 每个标注的像素数
- 排序: O(N log N)

### 空间复杂度
- 主图像: W × H × 4 bytes
- 原始副本（Mosaic 用）: W × H × 4 bytes
- 标注列表: N × sizeof(Annotation)

### 性能基准（参考）
- 1920×1080 截图 + 10 个标注: ~50ms（CPU）
- 4K 截图 + 20 个标注: ~200ms（CPU）

优化空间：
- DirtyRect: 只重绘变化区域
- SIMD: 向量化像素混合
- Glyph Cache: 缓存字形光栅化结果

## 测试

### 单元测试覆盖
- ✅ Image 填充和 PNG 编码
- ✅ 矩形渲染和 Alpha 混合
- ✅ 描边渲染
- ✅ Highlight 混合模式
- ✅ 箭头绘制
- ✅ 马赛克块平均
- ✅ 手绘平滑（Chaikin）
- ✅ 虚线箭头
- ✅ JPEG 编码

### 测试策略
- 像素级验证：检查关键像素的颜色值
- 区域验证：统计非透明像素数量
- 格式验证：PNG/JPEG 文件头校验

## 使用示例

### 渲染带标注的截图
```rust
use renderer::{SimpleRenderer, Renderer, PngEncoder, ExportEncoder};

let renderer = SimpleRenderer;
let frame = screenshot.raw.primary;
let annotations = vec![rect_annotation, arrow_annotation];

// 渲染
let image = renderer.render(&frame, &annotations);

// 编码为 PNG
let encoder = PngEncoder;
let png_bytes = encoder.encode_png(&image)?;

// 保存文件
std::fs::write("output.png", &png_bytes)?;
```

### 创建标注
```rust
let rect = Annotation {
    meta: AnnotationMeta {
        id: Uuid::now_v7(),
        x: 100.0, y: 100.0, w: 200.0, h: 150.0,
        stroke_color: Some("#FF0000".into()),
        fill_color: Some("#0000FF".into()),
        stroke_width: Some(2.0),
        opacity: 0.8,
        z: 0,
        // ...
    },
    kind: AnnotationKind::Rect { corner_radius: 0 },
};
```

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| CPU 渲染性能不足 | 当前可接受，未来可增加 Skia 渲染器 |
| 文字锯齿问题 | 未来集成 fontdue |
| 标注过多导致 O(N×A) 过慢 | 限制标注数量，局部重绘 |
| 马赛克块大小不合理 | 提供 level 参数调节 |

## 扩展方向

### 短期（v0.2）
- 增加 Skia 标注渲染器（GPU 加速）
- 优化箭头渲染质量（抗锯齿）

### 中期（v0.3）
- 集成 fontdue 真正字形渲染
- DirtyRect 局部重绘优化

### 长期（v1.0+）
- SIMD 向量化优化
- GPU 计算着色器（wgpu）
- 图层合成优化
