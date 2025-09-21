# ui_overlay 模块技术设计

## 职责与边界
- 提供跨平台的屏幕“框选区域”交互层（Overlay UI）。
- 输出一个逻辑坐标系下的矩形 Region 以及屏幕缩放因子 scale，供 services 使用对全屏截图进行裁剪，而非再次调用系统截图命令。
- 不直接负责像素渲染（交由 renderer）与业务编排（交由 services）。

## 接口
crate 暴露：
- 结构体 `Region { x, y, w, h, scale }`，提供 `norm()` 规范化（处理负拖动）。
- 错误 `OverlayError::{Cancelled, Internal}` 与 `type Result<T>`。
- Trait `RegionSelector { fn select(&self) -> Result<Region> }`：阻塞直到用户确认或取消。
- 扩展方法 `select_with_background(&self, rgb, width, height) -> Result<Option<Region>>`：默认包装 `select()`；平台可利用背景预览（如 macOS 集成）。
- `MockSelector`：无 UI，返回固定 Region 或 Cancelled，便于测试。

### 多显示器接口扩展
- `MultiDisplayRegionSelector`：专门处理多显示器环境的选择器
- `VirtualDesktopBounds`：虚拟桌面边界信息结构体
- `CrossDisplayRegion`：跨显示器区域的描述，包含影响的显示器列表
- `select_cross_display(&self, displays: &[DisplayInfo]) -> Result<Option<CrossDisplayRegion>>`：多显示器区域选择接口

## 事件与交互
- 鼠标左键拖拽：绘制/调整矩形。
- Enter/Space：确认；Esc：取消。
- Shift：固定比例（如 16:9/1:1）；Alt：从中心拉伸。

## 坐标与缩放
- `x,y,w,h` 为逻辑坐标（winit 的 logical）；`scale = window.scale_factor()`。
- 裁剪像素矩形时应使用：`px = round(x*scale) ...`，并对边界做 clamp。

### 多显示器坐标系统
- **虚拟桌面坐标**：以主显示器左上角为原点的全局坐标系
- **显示器相对坐标**：每个显示器内部的局部坐标系
- **跨显示器区域**：使用虚拟桌面坐标描述跨越多个显示器的区域
- **DPI 适配**：自动处理不同显示器间的 DPI 差异和缩放

## 平台实现
- 统一采用 `winit + pixels` 的跨平台实现，文件：`crates/ui_overlay/src/selector.rs`。
- 渲染：在 `pixels` 帧缓冲中绘制真实桌面截图背景、选区外暗化、选区白色描边，选区内部保持透明效果。
- 平台胶水隔离：新增 `ui_overlay::platform` 模块（`crates/ui_overlay/src/platform/mod.rs`），封装 macOS 的菜单栏/Dock 隐藏等呈现设置，核心选择器仅调用 `start_presentation()/end_presentation()`，减少条件编译分支和跨端耦合。
- macOS：通过 Cocoa API（仅在 macOS 分支编译）设置 NSWindow 为无边框、不可拖动、提升窗口层级、隐藏 Dock 和菜单栏，并将窗口 frame 设置为整个屏幕区域（非 visibleFrame），确保完全覆盖。
- Windows：使用同一套 winit 事件与 pixels 渲染路径；可选增强为窗口置顶与穿透。
 - 启动性能优化：窗口初始不可见（with_visible(false)），创建后立即预热 Pixels（ensure_pixels + render_once），再显示并 request_redraw；空闲阶段不进行持续重绘（about_to_wait 不再 request_redraw），仅在输入/尺寸变化时重绘。

### 多显示器渲染架构
- **全局覆盖**：在所有显示器上创建全屏覆盖窗口
- **窗口管理**：为每个显示器创建独立的 winit 窗口实例
- **事件同步**：统一处理来自不同显示器窗口的输入事件
- **跨窗口渲染**：支持绘制跨越多个显示器的选择区域
- **边界可视化**：在显示器边界处显示分割线或提示信息

### 渲染性能与拖动优化
- 背景暗化策略：
  - 若提供背景 RGB（截图像素），在窗口创建后预先计算一份“变暗背景”（bg_dim），暗化公式与原先一致（a=90/255）。
  - 每帧渲染时直接将 bg_dim 拷贝到帧缓冲，不再进行全屏 per-pixel 混合。
  - 选区内部从原始 bg 恢复（逐行拷贝），然后绘制白色描边。
- 重绘节流：
  - 引入 `redraw_pending` 标志，避免在高频 `CursorMoved`/按键事件中重复调用 `request_redraw()`，在一次有效绘制完成后清零。
- 尺寸变化：
  - `resize_surface` 后仍按需 request_redraw，保持与预热路径一致。

## 错误与取消语义
- 用户按 Esc/关闭窗口 -> `OverlayError::Cancelled`。
- 系统/窗口错误 -> `OverlayError::Internal(String)`。

## 与 services 的集成
- services 中新增 `RegionSelectService`：
  - 使用平台具体实现（如 `WinitSelector` 封装）调用 `select()`。
  - 将 `Region` 转换为对全屏 `Frame` 的裁剪区域，传递给 renderer/export。

当前：`platform_mac::MacCapturer::capture_region_interactive_custom(selector)` 已接入；CLI 子命令 `capture-interactive` 调用 `ui_overlay::create_gui_region_selector()`，完成一次全屏捕获 + 区域裁剪并导出。其他平台可复用相同选择器。


## 安全与权限
- 无敏感权限；不读取剪贴板/文件系统，仅创建窗口。

## 备注
本文档描述当前行为；扩展能力（多显示器、更多快捷键、性能优化等）留待后续讨论。
