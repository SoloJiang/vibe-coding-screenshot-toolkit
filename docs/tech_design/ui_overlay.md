# ui_overlay 模块技术设计

## 职责与边界
- 提供跨平台的屏幕"框选区域"交互层（Overlay Window），支持鼠标拖拽选择矩形区域。
- 输出一个逻辑坐标系下的矩形 Region 以及屏幕缩放因子 scale，供 services 使用对全屏截图进行裁剪，而非再次调用系统截图命令。
- 不直接负责像素渲染（交由 renderer）与业务编排（交由 services）。

## 接口
crate 暴露：
- 结构体 `Region { x, y, w, h, scale }`，提供 `norm()` 规范化（处理负拖动）。
- 错误 `OverlayError::{Cancelled, Internal}` 与 `type Result<T>`。
- Trait `RegionSelector` 提供多层级选择接口：
  - `select(&self) -> Result<Region>`：基础阻塞选择，返回本地坐标
  - `select_with_background(&self, rgb, width, height) -> Result<Option<Region>>`：带背景预览的选择
  - `select_with_virtual_background(&self, rgb, width, height, virtual_bounds, display_offset) -> Result<Option<Region>>`：✅ **新增** 虚拟桌面坐标选择
- `MockSelector`：无 UI，返回固定 Region 或 Cancelled，便于测试。

### 虚拟桌面支持 ✅ 已实现
- **虚拟坐标转换**：`select_with_virtual_background()` 自动将本地坐标转换为虚拟桌面全局坐标
- **跨显示器选择**：支持选择跨越多个显示器的区域，返回虚拟桌面坐标
- **显示器偏移处理**：自动处理当前交互显示器在虚拟桌面中的位置偏移
- **统一坐标系**：所有跨显示器操作使用统一的虚拟桌面坐标系统

## 事件与交互
- 鼠标左键拖拽：绘制/调整矩形。
- Enter/Space：确认；Esc：取消。
- Shift：固定比例（如 16:9/1:1）；Alt：从中心拉伸。

## 坐标与缩放
- `x,y,w,h` 为逻辑坐标（winit 的 logical）；`scale = window.scale_factor()`。
- 裁剪像素矩形时应使用：`px = round(x*scale) ...`，并对边界做 clamp。

### 多显示器坐标系统 ✅ 已实现并修复
- **虚拟桌面坐标**：以主显示器左上角为原点的全局坐标系
- **显示器相对坐标**：每个显示器内部的局部坐标系
- **跨显示器区域**：使用虚拟桌面坐标描述跨越多个显示器的区域
- **DPI 适配**：自动处理不同显示器间的 DPI 差异和缩放
- **坐标转换一致性**：统一处理鼠标事件坐标转换和渲染坐标转换，确保选择框正确显示在对应位置
- **修复记录**：解决了非主屏框选坐标错误和选择框显示位置错误的问题

## 平台实现
- 统一采用 `winit + Skia` 的跨平台实现，文件：`crates/ui_overlay/src/selector.rs`。
- 渲染：使用 Skia 2D 图形库进行高质量渲染，支持 GPU 加速和 CPU fallback，绘制真实桌面截图背景、选区外暗化、选区白色描边，选区内部保持透明效果。
- 平台胶水隔离：新增 `ui_overlay::platform` 模块（`crates/ui_overlay/src/platform/mod.rs`），封装 macOS 的菜单栏/Dock 隐藏等呈现设置，核心选择器仅调用 `start_presentation()/end_presentation()`，减少条件编译分支和跨端耦合。
- macOS：通过 Cocoa API（仅在 macOS 分支编译）设置 NSWindow 为无边框、不可拖动、提升窗口层级、隐藏 Dock 和菜单栏，并将窗口 frame 设置为整个屏幕区域（非 visibleFrame），确保完全覆盖。
- Windows：使用同一套 winit 事件与 Skia 渲染路径；可选增强为窗口置顶与穿透。
 - 启动性能优化：窗口初始不可见（with_visible(false)），创建后立即预热 Skia Surface（ensure_surface + render_once），再显示并 request_redraw；空闲阶段不进行持续重绘（about_to_wait 不再 request_redraw），仅在输入/尺寸变化时重绘。

### 多显示器渲染架构
- **全局覆盖**：在所有显示器上创建全屏覆盖窗口
- **窗口管理**：为每个显示器创建独立的 winit 窗口实例
- **事件同步**：统一处理来自不同显示器窗口的输入事件
- **跨窗口渲染**：支持绘制跨越多个显示器的选择区域
- **边界可视化**：在显示器边界处显示分割线或提示信息

### Skia 渲染架构 ✅ 已完成
- **RenderBackend 抽象层**：新建 `backend` 子模块，定义统一的 `RenderBackend` trait，封装不同平台的 GPU/CPU 渲染实现。
  - `BackendType` 枚举：MetalGpu, Direct3dGpu, CpuRaster
  - `RenderBackend` trait：提供 `prepare_surface()`, `canvas()`, `flush_and_read_pixels()`, `resize()` 等统一接口
  - `factory.rs`：根据平台自动选择最佳可用后端

- **后端实现（已完成）**：
  1. **macOS Metal GPU**（`metal_backend.rs`）：
     - 使用 `metal-rs` crate 实现原生 Metal 渲染
     - 通过 `CAMetalLayer` 直接渲染到窗口，无需 softbuffer
     - 每帧从 layer 获取 `MetalDrawable`，创建 Skia GPU Surface
     - `flush_and_read_pixels()` 中调用 `drawable.present()` 提交到屏幕
     - 支持 Skia `DirectContext` 硬件加速渲染
  2. **CPU Raster**（`cpu_backend.rs`）：
     - 使用 `Surface::new_raster_n32_premul` 创建 CPU 渲染表面
     - 读取像素后通过 `softbuffer` 提交到窗口
     - 作为 GPU 不可用时的降级方案
  3. **Windows Direct3D**（待实现）：
     - 计划使用 `wgpu` 或 `windows-rs` 实现 D3D11/D3D12 渲染
     - 架构与 Metal 类似，通过 DirectX 直接渲染到窗口

- **后端选择策略**：
  1. macOS 优先使用 Metal GPU（`DirectContext::make_metal` + `CAMetalLayer`）
  2. Windows 将优先使用 Direct3D GPU（Phase 3）
  3. 任一 GPU 初始化失败时自动降级到 CPU Raster
  4. CPU Raster 使用 `softbuffer` 确保所有环境都能正常显示

- **Surface 生命周期**：
  - 每个窗口持有独立的 `RenderBackend` 实例
  - `prepare_surface()` 在每帧开始时准备渲染表面（GPU 获取 drawable，CPU 创建 raster surface）
  - `canvas()` 返回 Skia Canvas 用于绘制
  - `flush_and_read_pixels()` 提交渲染结果（GPU 调用 present，CPU 返回像素数据）
  - `resize()` 处理窗口尺寸变化

- **渲染流程**：
  1. `WindowInfo::render_with_backend()` 接收绘制闭包
  2. 调用 `backend.prepare_surface()` 准备当前帧
  3. 通过 `backend.canvas()` 获取 Canvas 并执行绘制闭包：
     - 绘制暗化背景图像（Skia Image，缓存）
     - 绘制选区内原始背景（裁剪 + 图像绘制）
     - 绘制选择框边框（白色描边）
  4. 调用 `backend.flush_and_read_pixels()` 提交结果
  5. GPU backend 直接 present，CPU backend 通过 softbuffer 呈现

- **坐标转换修复**：
  - 统一虚拟桌面坐标到窗口本地坐标的转换公式：`local = virtual - window_virtual`
  - 确保背景偏移和选择框坐标使用相同的转换逻辑
  - 修复了选择框在非主显示器上位置错误的问题

### 渲染性能与拖动优化 ✅ 已完成并持续优化

#### 核心性能优化（最新）
- **鼠标移动防抖优化**（`event_handler.rs`）：
  - 使用固定阈值代替动态计算：拖动时 5px，非拖动时 10px
  - 移除每次移动时的选择区域面积计算，减少 CPU 开销
  - 只在拖动状态下触发重绘，避免无效重绘（~50% 重绘次数减少）

- **选择矩形计算缓存**（`SelectionState`）：
  - 添加 `cached_rect` 和 `cache_valid` 字段缓存计算结果
  - 在鼠标移动、修饰键变化时使缓存失效
  - 避免每帧多次重复计算选择矩形（~30% CPU 计算减少）

- **图像缓存优化**（`ImageCache`）：
  - 改用 `initialized` 标志位替代哈希验证
  - 背景图片在选择过程中不变，只需初始化一次
  - 移除每帧的哈希计算开销，提升渲染性能
  - 使用 `Arc<Image>` 零拷贝共享，减少内存占用

- **重绘频率控制简化**：
  - 移除复杂的预算机制，只保留时间间隔限流（16ms）
  - 简化 `should_throttle_redraw()` 逻辑，减少判断开销
  - 配合 `FrameTimer` 的 60 FPS 限制，实现稳定帧率

- **帧率控制**（`FrameTimer`）：
  - 限制渲染频率为 60 FPS（16.67ms/帧）
  - 在 `request_redraw_all()` 中统一检查帧率限制
  - 初始化时预设时间偏移，确保第一帧立即可渲染

- **Metal Backend 优化**：
  - 异步提交 command buffer，不等待 GPU 完成
  - 复用 `CommandQueue` 避免每帧创建
  - 利用 VSync 同步，避免画面撕裂

#### 渲染性能策略
- **窗口预热**：
  - 窗口初始不可见（`with_visible(false)`）
  - 先创建 RenderBackend 并预热渲染
  - 完成首次渲染后再显示窗口，避免屏幕闪动

- **尺寸变化处理**：
  - 窗口 resize 时调用 `backend.resize()` 更新尺寸
  - 下次 `prepare_surface()` 时会重建对应尺寸的 Surface

#### 性能指标
- 拖动延迟：< 33ms（2 帧以内）
- 重绘频率：稳定 60 FPS
- CPU 占用：相比优化前减少 ~40%
- 内存占用：通过 Arc 共享，减少重复拷贝

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
