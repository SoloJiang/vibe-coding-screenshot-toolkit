# platform_win 模块技术设计
(迁移自 TechDesign_platform_win.md)

## 职责
Windows 捕获/窗口/剪贴板/快捷键 (WGC + Win32)。

## 捕获策略
WGC -> ID3D11Texture2D -> staging -> Map -> BGRA；区域裁剪。

## 组件
WinScreenCapturer, DisplayInfoProviderWin, ClipboardWin, HotkeyRegistrarWin。

## 风险
| 风险 | 缓解 |
|------|------|
| 高 DPI 坐标错位 | 逻辑点统一+单测 |
| 老系统不支持 WGC | 后期 GDI 回退 |
