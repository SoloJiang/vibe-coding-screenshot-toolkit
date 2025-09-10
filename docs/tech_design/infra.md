# infra 模块技术设计

## 职责
提供通用基础设施：事件总线、配置、日志、ID、Clock、LRU 缓存、命名模板、路径定位、Panic hook、SIMD 检测（后期）。

## 依赖
serde, tracing, uuid, parking_lot (可选), regex.

## 组件
当前：PathResolver（含 history/output 目录）、NamingTemplate、LruCache、IdGenerator、Panic hook、Metrics。

## 配置 Schema
与总技术设计一致；validate(Config)->Result<()>。

## 并发
RwLock, broadcast, Mutex LruCache。

## 错误
返回 core::Error。

## 性能
模板/正则缓存；LruCache O(1)。

## 测试
- Lru 淘汰
- 命名模板解析

## 风险
| 风险 | 缓解 |
|------|------|
| 事件丢失 | 容量配置+日志 |
| 配置损坏 | 原子写+备份 |

注：配置系统、事件总线、CPU 特性探测、跨进程指标共享等不在当前范围。
