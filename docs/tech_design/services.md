# services 模块技术设计

## 职责
为交互式截图提供核心服务支持：导出处理、剪贴板集成。

当前聚焦：ExportService（PNG 导出和剪贴板）。

## 服务
核心服务：ExportService。

## 流程示例
### 交互式截图流程
1. platform_mac 通过 ui_overlay 获取用户选择区域
2. platform_mac 执行截图和裁剪，生成 Screenshot
3. ExportService 渲染 Screenshot 为 PNG
4. ExportService 保存文件和/或写入剪贴板

## 并发
当前：同步处理，简单直接的导出流程。

## 错误
统一 core::Error。

## 风险
| 风险 | 缓解 |
|------|------|
| 导出阻塞 | 当前同步处理，未来可考虑 spawn_blocking |
