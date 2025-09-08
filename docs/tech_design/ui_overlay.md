# ui_overlay 模块技术设计

## 职责与边界
- 提供跨平台的屏幕“框选区域”交互层（Overlay UI）。
- 输出一个逻辑坐标系下的矩形 Region 以及屏幕缩放因子 scale，供 services 使用对全屏截图进行裁剪，而非再次调用系统截图命令。
- 不直接负责像素渲染（交由 renderer）与业务编排（交由 services）。

## 最小接口（当前）
crate 暴露：
- 结构体 `Region { x, y, w, h, scale }`，提供 `norm()` 规范化（处理负拖动）。
- 错误 `OverlayError::{Cancelled, Internal}` 与 `type Result<T>`。
- Trait `RegionSelector { fn select(&self) -> Result<Region> }`：阻塞直到用户确认或取消。
- 扩展方法 `select_with_background(&self, rgb, width, height) -> Result<Option<Region>>`：默认包装 `select()`；平台可利用背景预览（如 macOS 集成）。
- `MockSelector`：无 UI，返回固定 Region 或 Cancelled，便于测试。

## 事件与交互（候选快捷键）
- 鼠标左键拖拽：绘制/调整矩形。
- Enter/Space：确认；Esc：取消。
- Shift：固定比例（如 16:9/1:1）；Alt：从中心拉伸（后续迭代）。

## 坐标与缩放
- `x,y,w,h` 为逻辑坐标（winit 的 logical）；`scale = window.scale_factor()`。
- 裁剪像素矩形时应使用：`px = round(x*scale) ...`，并对边界做 clamp。

## 平台实现（当前）
- 统一采用 `winit + pixels` 的跨平台实现，文件：`crates/ui_overlay/src/selector.rs`。
- 渲染：在 `pixels` 帧缓冲中绘制真实桌面截图背景、选区外暗化、选区白色描边，选区内部保持透明效果。
- macOS：通过 Cocoa API（仅在 macOS 分支编译）设置 NSWindow 为无边框、不可拖动、提升窗口层级、隐藏 Dock 和菜单栏，并将窗口 frame 设置为整个屏幕区域（非 visibleFrame），确保完全覆盖。
- Windows：使用同一套 winit 事件与 pixels 渲染路径；窗口置顶与透明穿透为后续可选增强。

## 错误与取消语义
- 用户按 Esc/关闭窗口 -> `OverlayError::Cancelled`。
- 系统/窗口错误 -> `OverlayError::Internal(String)`。

## 与 services 的集成（计划）
- services 中新增 `RegionSelectService`（后续 PR）：
  - 使用平台具体实现（如 `WinitSelector` 封装）调用 `select()`。
  - 将 `Region` 转换为对全屏 `Frame` 的裁剪区域，传递给 renderer/export。

当前：`platform_mac::MacCapturer::capture_region_interactive_custom(selector)` 已接入；CLI 子命令 `capture-interactive` 调用 `ui_overlay::create_gui_region_selector()`，完成一次全屏捕获 + 区域裁剪并导出。其他平台可复用相同选择器。


## 安全与权限
- 无敏感权限；不读取剪贴板/文件系统，仅创建窗口。

## 未来迭代点
- 绘制半透明蒙层与矩形描边（GPU 或 CPU）。
- 键盘快捷键扩展（移动/扩大/吸附屏幕边缘/网格）。
- 多显示器支持（聚合虚拟坐标系）。
- 屏幕快照预览层（仅 UI 显示，不参与导出）。
- 点击穿透与交互层性能优化。
