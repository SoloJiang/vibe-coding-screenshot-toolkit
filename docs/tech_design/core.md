# core 模块技术设计

## 范围与职责
提供截图核心领域模型与纯逻辑：Screenshot/Frame 内存结构、Annotation 标注模型、错误类型、命名模板解析、撤销/重做栈等。

核心原则：不依赖任何平台或 UI 框架，保持纯数据和算法。

## 依赖与被依赖
- 依赖：标准库、uuid、serde、chrono、thiserror
- 被依赖：infra, renderer, services, platform_mac, ui_overlay

## 关键数据结构

### 截图数据
- `Screenshot { id, raw:Arc<FrameSet>, scale, created_at }`
- `Frame { width, height, pixel_format, bytes:Arc<[u8]> }`
- `FrameSet { primary, all }` - 支持多显示器
- `PixelFormat`: Bgra8 / Rgba8

### 标注数据
```rust
pub struct Annotation {
    pub meta: AnnotationMeta,  // 通用属性（位置、颜色、透明度等）
    pub kind: AnnotationKind,  // 具体类型
}

pub enum AnnotationKind {
    Rect { corner_radius: u8 },
    Arrow { head_size: u8, line_style: LineStyle },
    Text { content: String, font_family: String, font_size: u32 },
    Highlight { mode: BlendMode },
    Mosaic { level: u8 },
    Freehand { points: Vec<(f32, f32)>, smoothing: f32 },
}
```

标注元信息：
- 位置和尺寸：`x, y, w, h`
- 样式：`stroke_color, fill_color, stroke_width, opacity`
- 层级：`z` - 用于渲染排序
- 状态：`locked` - 是否锁定编辑
- 标识：`id` - UUID v7
- 时间：`created_at` - 创建时间

### 撤销/重做
```rust
pub struct UndoStack {
    ops: Vec<UndoOp>,
    cap: usize,
    redo: Vec<UndoOp>,
}

pub struct UndoOp {
    pub apply: Box<dyn Fn(&mut UndoContext)>,
    pub revert: Box<dyn Fn(&mut UndoContext)>,
    pub merge_key: Option<String>,  // 用于拖拽合并
}

pub struct UndoContext {
    pub annotations: Vec<Annotation>,
}
```

特性：
- 支持操作合并（拖拽时多次位置更新合并为一次）
- 有限容量（默认 100 操作）
- 新操作清空 redo 栈

### 历史记录
```rust
pub struct HistoryItem {
    pub id: Uuid,
    pub path: String,
    pub thumb: Option<Vec<u8>>,  // 缩略图
    pub created_at: DateTime<Utc>,
    pub title: Option<String>,
    pub version: u8,
}
```

### 错误模型
```rust
pub struct Error {
    kind: ErrorKind,
    message: String,
}

pub enum ErrorKind {
    Permission,    // 权限不足
    Capture,       // 截图失败
    IO,           // 文件操作失败
    Clipboard,    // 剪贴板操作失败
    Validation,   // 数据验证失败
    Unsupported,  // 不支持的操作
    Unknown,      // 未知错误
}
```

## 主要算法

### 命名模板解析
支持的变量：
- `{date:format}` - 时间格式化（如 `{date:yyyyMMdd-HHmmss}`）
- `{seq}` - 当日序列号
- `{screen}` - 屏幕索引

实现：
- 正则表达式匹配变量
- 时间格式化使用 chrono
- 序列号通过文件系统扫描计算

### Region 规范化
```rust
impl Region {
    pub fn norm(&self) -> Self {
        // 处理负宽高（向左/向上拖拽）
        let (mut x, mut y, mut w, mut h) = (self.x, self.y, self.w, self.h);
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        Self { x, y, w, h, scale: self.scale }
    }
}
```

## 性能 & 内存
- Frame/FrameSet 使用 `Arc` 共享像素数据，避免大块内存拷贝
- Annotation 列表采用 `Vec`，按 z 值排序后遍历
- UndoStack 有容量限制，防止内存无限增长
- 命名模板正则预编译（待实现优化）

## 并发与线程安全
- 核心数据结构不可变或使用 `Arc` 共享
- 编辑流程由上层（services/ui_overlay）串行保证
- Screenshot 可在线程间安全传递（通过 Arc）

## 测试
- 命名模板解析：时间格式、序列号生成
- Screenshot 序列化：serde 往返测试
- UndoStack：单操作、合并操作、redo 流程
- Region 规范化：负宽高处理
- 历史记录裁剪：容量限制

## 风险与缓解
| 风险 | 缓解 |
|------|------|
| Undo 合并错误导致状态错乱 | 严格快照前后对比单测 |
| 模板解析性能不足 | 预编译正则 + LRU 模板缓存 |
| Annotation Vec 过大导致排序慢 | 限制标注数量 + 空间索引（未来） |
| Frame bytes Arc 引用计数开销 | 可接受，避免拷贝的收益更大 |

## 扩展点
- 更多 AnnotationKind（椭圆、多边形等）
- Annotation 属性动画（淡入淡出等）
- 空间索引（四叉树，用于快速碰撞检测）
- 持久化格式版本迁移

## 使用示例

### 创建截图
```rust
let frame = Frame {
    width: 1920,
    height: 1080,
    pixel_format: PixelFormat::Rgba8,
    bytes: Arc::from(pixels),
};
let screenshot = Screenshot {
    id: Uuid::now_v7(),
    raw: Arc::new(FrameSet {
        primary: frame,
        all: vec![],
    }),
    scale: 2.0,
    created_at: Utc::now(),
};
```

### 创建标注
```rust
let annotation = Annotation {
    meta: AnnotationMeta {
        id: Uuid::now_v7(),
        x: 100.0,
        y: 100.0,
        w: 200.0,
        h: 150.0,
        rotation: 0,
        opacity: 1.0,
        stroke_color: Some("#FF0000".into()),
        fill_color: None,
        stroke_width: Some(2.0),
        z: 0,
        locked: false,
        created_at: Utc::now(),
    },
    kind: AnnotationKind::Rect { corner_radius: 0 },
};
```

### 撤销操作
```rust
let mut undo = UndoStack::new(100);
let mut ctx = UndoContext {
    annotations: vec![],
};

// 添加标注
undo.push(UndoOp {
    apply: Box::new(|c| c.annotations.push(annotation.clone())),
    revert: Box::new(|c| { c.annotations.pop(); }),
    merge_key: None,
});

// 撤销
undo.undo(&mut ctx);

// 重做
undo.redo(&mut ctx);
```
