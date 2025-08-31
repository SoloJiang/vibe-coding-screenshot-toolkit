# infra 模块技术设计

## 职责
提供通用基础设施：事件总线、配置、日志、ID、Clock、LRU 缓存、命名模板、路径定位、Panic hook、SIMD 检测（后期）。

## 依赖
serde, tracing, uuid, parking_lot (可选), regex.

## 组件
MVP 必需：PathResolver（含 history/output 目录）、NamingTemplate、LruCache（供 renderer/ocr 未来使用）、IdGenerator。
可选（留存实现）：EventBus（MVP 可最小或空壳）、ConfigStore（若尚无配置读写则延后）、Clock Trait。
延后：Panic hook、CpuFeatures 探测。

## 配置 Schema
与总技术设计一致；validate(Config)->Result<()>。

## 并发
RwLock, broadcast, Mutex LruCache。

## 错误
返回 core::Error。

## 性能
模板/正则缓存；LruCache O(1)。

## 测试
MVP：Lru 淘汰；命名模板（若不复用 core 逻辑则至少覆盖 parse）。
Later：Config 原子写、EventBus 压力、CpuFeatures。

## 风险
| 风险 | 缓解 |
|------|------|
| 事件丢失 | 容量配置+日志 |
| 配置损坏 | 原子写+备份 |

## 扩展
Metrics 导出、多进程共享。
