# core 模块技术设计

## 范围与职责
提供截图/标注编辑核心领域模型与纯逻辑：Screenshot/Frame/Annotation/UndoStack/History 内存结构、错误类型、几何与对齐算法、命名模板解析、公共 trait（Renderer/ExportEncoder 接口定义）等。

当前使用子集：AnnotationKind(Rect/Arrow/Text)，Undo 合并策略，HistoryItem + 裁剪，命名模板。

## 依赖与被依赖
- 依赖：标准库、uuid、serde（模型序列化）、thiserror
- 被依赖：infra, renderer, services, api_napi, privacy, ocr_adapter

## 关键数据结构
- Screenshot { id, raw:Arc<FrameSet>, scale, created_at }
- Frame { width,height,pixel_format,bytes:Arc<[u8]> }
- FrameSet { primary, all, layout }
- Annotation { meta:AnnotationMeta, kind:AnnotationKind }
- UndoStack { ops:Vec<UndoOp>, cursor }
- HistoryItem { id, path, thumb, created_at, title }
- Error / ErrorKind

## 主要 Trait 契约
```rust
pub trait Renderer { fn render(&self, shot:&Screenshot, ann:&[Annotation]) -> Result<Image>; }
pub trait ExportEncoder { fn encode_png(&self, img:&Image, opts:&PngOptions)->Result<Vec<u8>>; fn encode_jpeg(&self,img:&Image,opts:&JpegOptions)->Result<Vec<u8>>; }
```
合约：纯函数式，无全局可变状态；错误统一 core::Error。

## 算法要点
- Undo merge：基于 Op 提供 can_merge(prev,next) -> bool；拖拽/连续属性修改合并。
- 对齐吸附：输入移动矩形与参考线集合，阈值 5px，输出偏移。
- 命名模板解析：正则 `{identifier(:format)?}`；支持 date/seq/screen。

## 错误模型
ErrorKind: Permission, Capture, IO, Upload, Ocr, Hook, Config, Validation, Privacy, Unsupported。

## 性能 & 内存
- Annotation 轻量（文本 content Arc<str>）
- Frame/FrameSet 使用 Arc 共享，避免拷贝
- UndoStack 限制深度 100，超过丢弃底部

## 并发与线程安全
核心结构使用 Arc + 内部不做锁（编辑流程由上层串行保证）。

## 测试
- Undo 合并策略
- 命名模板解析
- History 裁剪
- Annotation 序列化往返

## 风险与缓解
| 风险 | 缓解 |
|------|------|
| Undo 合并错误导致状态错乱 | 严格快照前后对比单测 |
| 模板解析性能不足 | 预编译正则 + LRU 模板缓存 |

注：吸附/对齐、更多 AnnotationKind、动态扩展注册与性能基准等不在当前范围。
