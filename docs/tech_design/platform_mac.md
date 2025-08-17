# platform_mac 模块技术设计
(迁移自 TechDesign_platform_mac.md)

## 职责
macOS 捕获/显示器/剪贴板/快捷键/权限/光标。

## 捕获策略
优先 ScreenCaptureKit；否则 CGDisplayStream/CGWindowListCreateImage；统一 BGRA+scale。

## 组件
MacScreenCapturer, DisplayInfoProvider, ClipboardMac, HotkeyRegistrarMac, PermissionsMac。

## 错误映射
权限不足 -> E_NO_PERMISSION；捕获失败 -> E_CAPTURE_FAIL。

## 风险
| 风险 | 缓解 |
|------|------|
| 权限阻塞 | request_permissions 引导 |
| API 差异 | 运行时检测回退 |
