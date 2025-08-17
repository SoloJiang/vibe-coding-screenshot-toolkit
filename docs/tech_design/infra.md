# infra 模块技术设计
(迁移自 TechDesign_infra.md)

## 职责
提供通用基础设施：事件总线、配置、日志、ID、Clock、LRU 缓存、命名模板、路径定位、Panic hook、SIMD 检测（后期）。

## 依赖
serde, tracing, uuid, parking_lot (可选), regex.

## 组件
- EventBus<E>: 基于 tokio::broadcast 封装 subscribe()/publish()
- ConfigStore: load()/save(patch) 原子写 (temp + rename)
- IdGenerator: uuid v7
- Clock Trait: now()/unix_ms()
- LruCache<K,V>
- NamingTemplate: 与 core 模板兼容
- PathResolver
- Panic hook (debug)
- CpuFeatures: 运行期指令集探测

## 配置 Schema
与总技术设计一致；validate(Config)->Result<()>。

## 并发
RwLock, broadcast, Mutex LruCache。

## 错误
返回 core::Error。

## 性能
模板/正则缓存；LruCache O(1)。

## 测试
Config 原子写、EventBus 压力、Lru 淘汰、CpuFeatures。

## 风险
| 风险 | 缓解 |
|------|------|
| 事件丢失 | 容量配置+日志 |
| 配置损坏 | 原子写+备份 |

## 扩展
Metrics 导出、多进程共享。
