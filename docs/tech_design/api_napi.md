# api_napi 模块技术设计

## 职责
napi-rs 暴露 Rust 服务：异步 API、事件、错误映射、Handle 管理。

## MVP 状态
NAPI 暂不进入 MVP；只保留 version() 占位，后续在扩展阶段接入。

## 架构
Runtime Once 初始化；HandleRegistry；DTO #[napi(object)]；事件订阅转发。

## 错误映射
core::Error.kind -> JS Error { code, message }。

## 风险
| 风险 | 缓解 |
|------|------|
| 多次 runtime 初始化 | OnceCell |
| 事件背压 | 队列限制+丢弃日志 |
