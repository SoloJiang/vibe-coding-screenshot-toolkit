# platform_win 模块技术设计

## 职责
Windows 捕获/窗口/剪贴板/快捷键 (WGC + Win32)。

## MVP 子集
允许使用占位 Stub：暂仅返回 Unsupported（主流程先以 mac 验收）。
或可选最简 GDI 全屏截屏（如后续需要快速验证多平台）。

## 延后
WGC/D3D11 管线、窗口/区域捕获、剪贴板、快捷键、性能优化。

## 捕获策略
WGC -> ID3D11Texture2D -> staging -> Map -> BGRA；区域裁剪。

## 组件
WinScreenCapturer, DisplayInfoProviderWin, ClipboardWin, HotkeyRegistrarWin。

## 风险
| 风险 | 缓解 |
|------|------|
| 高 DPI 坐标错位 | 逻辑点统一+单测 |
| 老系统不支持 WGC | 后期 GDI 回退 |
