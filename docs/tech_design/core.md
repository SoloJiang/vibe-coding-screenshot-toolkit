# core 模块技术设计

## 范围与职责
提供截图核心领域模型与纯逻辑：Screenshot/Frame 内存结构、错误类型、命名模板解析、公共 trait（Renderer/ExportEncoder 接口定义）等。

当前使用子集：Screenshot/Frame 数据结构，命名模板，ExportEncoder 接口。

## 依赖与被依赖
- 依赖：标准库、uuid、serde（模型序列化）、thiserror
- 被依赖：infra, renderer, services, platform_mac

## 关键数据结构
- Screenshot { id, raw:Arc<FrameSet>, scale, created_at }
- Frame { width,height,pixel_format,bytes:Arc<[u8]> }
- FrameSet { primary, all }
- Error / ErrorKind

### 多显示器数据结构（规划）
- DisplayInfo { id, name, bounds, scale_factor, is_primary }
- VirtualDesktop { bounds, displays: Vec<DisplayInfo> }
- CrossDisplayRegion { bounds, affected_displays: Vec<DisplayId> }

## 主要 Trait 契约
```rust
pub trait ExportEncoder {
    fn encode_png(&self, img:&Image, opts:&PngOptions)->Result<Vec<u8>>;
}
```
合约：纯函数式，无全局可变状态；错误统一 core::Error。

## 算法要点
- 命名模板解析：正则 `{identifier(:format)?}`；支持 date/seq。

## 错误模型
ErrorKind: Permission, Capture, IO, Clipboard, Validation, Unsupported。

## 性能 & 内存
- Frame/FrameSet 使用 Arc 共享，避免拷贝

## 并发与线程安全
核心结构使用 Arc + 内部不做锁（编辑流程由上层串行保证）。

## 测试
- 命名模板解析
- Screenshot 序列化往返
| 风险 | 缓解 |
|------|------|
| Undo 合并错误导致状态错乱 | 严格快照前后对比单测 |
| 模板解析性能不足 | 预编译正则 + LRU 模板缓存 |

注：吸附/对齐、更多 AnnotationKind、动态扩展注册与性能基准等不在当前范围。
