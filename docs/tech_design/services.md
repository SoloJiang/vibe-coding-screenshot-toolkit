# services 模块技术设计

## 职责
提供业务服务层，协调 core、renderer、platform 等模块，实现完整的业务流程。

核心服务：
- **CaptureService**: 截图捕获编排（已实现占位，实际捕获由 platform 模块提供）
- **ExportService**: 导出处理（PNG/JPEG 渲染和保存）
- **AnnotationService**: 标注管理（CRUD + 撤销/重做）
- **HistoryService**: 历史记录管理
- **OcrService**: OCR 服务占位（未实现）
- **PrivacyService**: 隐私扫描服务占位（基础正则实现）

## 服务详解

### ExportService
负责将 Screenshot + Annotations 渲染并导出为文件或剪贴板。

**核心流程**：
```
Screenshot + Annotations
    ↓
SimpleRenderer (CPU 渲染)
    ↓
Image (RGBA 像素)
    ↓
PngEncoder / JpegEncoder
    ↓
文件 / 剪贴板
```

**主要方法**：
- `render_png_bytes()`: 渲染为 PNG 字节
- `export_png_to_file()`: 保存为文件
- `export_png_to_clipboard()`: 写入剪贴板（带重试）
- `render_jpeg_bytes()`: 渲染为 JPEG 字节
- `export_jpeg_to_file()`: 保存为 JPEG 文件

**特性**：
- 剪贴板写入失败时自动重试一次
- 支持生成缩略图（最大边 240px）
- 集成 HistoryService 自动记录历史
- 内置 metrics 指标采集

### AnnotationService
管理标注列表和撤销/重做栈。

**核心能力**：
- `add()`: 添加标注并记录到 undo 栈
- `update()`: 修改标注并记录到 undo 栈（支持合并）
- `move_up/down()`: 调整标注层级
- `undo()`: 撤销上一次操作
- `redo()`: 重做已撤销的操作
- `list()`: 获取当前所有标注

**撤销/重做机制**：
- 基于 `core::UndoStack`
- 支持操作合并（通过 `merge_key`）
- 每次修改都创建完整快照（简单但可靠）

**使用场景**：
```rust
let mut service = AnnotationService::new();

// 添加矩形
service.add(rect_annotation);

// 修改位置（拖拽中，使用 merge_key）
service.update(id, Some("drag"), |ann| {
    ann.meta.x = new_x;
    ann.meta.y = new_y;
    true  // 返回 true 表示有修改
});

// 撤销
service.undo();

// 重做
service.redo();
```

### HistoryService
管理历史记录（最近截图列表）。

**存储格式**：
- `history.jsonl`: 每行一个 JSON 对象
- 支持缩略图内嵌（base64 或二进制）
- 自动裁剪超出容量的旧记录

**主要方法**：
- `load_from_disk()`: 从磁盘加载历史
- `append()`: 添加新记录并持久化
- `list()`: 获取历史列表

### CaptureService（占位）
截图捕获服务的抽象层，实际实现在 platform 模块。

**Trait 定义**：
```rust
pub trait Capturer: Send + Sync {
    fn capture_full(&self) -> anyhow::Result<Screenshot>;
}
```

**职责**：
- 提供统一的截图接口
- 集成 metrics 指标
- 错误处理和重试逻辑

**实现**：
- `MacCapturer` (platform_mac)
- `WinCapturer` (platform_win，占位)

### Clipboard Trait
剪贴板抽象，由 platform 模块实现。

```rust
pub trait Clipboard: Send + Sync {
    fn write_image(&self, bytes: &[u8]) -> CoreResult<()>;
}
```

实现：
- macOS: `MacClipboard` (platform_mac)
- Windows: `WinClipboard` (platform_win，占位)
- Stub: `StubClipboard`（无操作，用于测试）

### OcrService（占位）
OCR 文本识别服务框架。

**当前状态**：
- 多线程任务队列已实现
- 实际 OCR 逻辑未实现（返回 Unsupported 错误）

**架构**：
- `mpsc` 通道 + 工作线程池
- 异步请求-响应模式
- 预留 tesseract 集成接口

### PrivacyService（基础实现）
隐私信息检测和脱敏。

**检测规则**：
- Email: 正则匹配
- 电话: 国际格式 + 中国手机号
- URL: http/https 链接
- IP: IPv4 地址

**主要方法**：
- `scan()`: 返回命中区间 `(start, end)`
- `scan_detailed()`: 返回 `(start, end, kind)`
- `mask()`: 将命中内容替换为 `*`

## 流程示例

### 交互式截图完整流程
```
1. CLI 命令 `capture-interactive`
    ↓
2. platform_mac::MacCapturer 创建 RegionSelector
    ↓
3. ui_overlay::WinitRegionSelector 显示选择界面
    ↓
4. 用户选择区域，返回 Region
    ↓
5. MacCapturer 捕获虚拟桌面截图
    ↓
6. MacCapturer 裁剪为选中区域
    ↓
7. ExportService::export_png_to_file() 保存文件
    ↓
8. ExportService::export_png_to_clipboard() 复制到剪贴板
    ↓
9. HistoryService::append() 记录历史
```

### 标注编辑流程（未来）
```
1. 用户完成区域选择
    ↓
2. 进入编辑模式（工具栏显示）
    ↓
3. 用户绘制标注
    ↓
4. AnnotationService::add() 添加到列表
    ↓
5. ui_overlay 实时渲染预览
    ↓
6. 用户点击"完成"
    ↓
7. ExportService::render_png_bytes() 渲染最终图像
    ↓
8. 保存文件 + 剪贴板
```

## 辅助功能

### 文件命名
```rust
pub fn gen_file_name(template: &str, screen_index: usize) -> String
```

支持模板变量：
- `{date:format}`: 时间格式化
- `{seq}`: 当日序列号
- `{screen}`: 屏幕索引

委托给 `core::naming` 模块实现。

## 并发模型
- 当前：同步处理，简单直接
- ExportService: 导出操作在主线程同步执行
- OcrService: 异步线程池（占位）
- 未来：可考虑 `spawn_blocking` 避免阻塞 UI

## 错误处理
- 统一使用 `anyhow::Result`
- 关键错误（权限、捕获失败）向上层传递
- 剪贴板写入失败自动重试一次
- 指标记录成功/失败次数

## 性能指标
通过 `infra::metrics` 记录：
- `capture_full_ok` / `capture_full_err`
- `render_png_ok` / `render_png_err`
- `export_png_file_ok` / `export_png_file_err`
- `clipboard_write_*` 系列

## 测试策略
- 单元测试：AnnotationService 的撤销/重做逻辑
- 集成测试：ExportService 的完整渲染流程
- Mock: StubClipboard, MockCapturer

## 风险与缓解
| 风险 | 缓解 |
|------|------|
| 导出阻塞 UI | 当前同步处理，未来可考虑异步 |
| 剪贴板不稳定 | 已实现重试机制 |
| 历史记录文件损坏 | 忽略错误行，继续加载 |
| 标注过多导致内存问题 | 限制标注数量（未来） |
