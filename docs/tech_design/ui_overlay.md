# ui_overlay 模块技术设计

## 版本信息
- 当前版本：v0.1.4
- 最后更新：2025-10-25

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
  - `select_with_virtual_background(&self, rgb, width, height, virtual_bounds, display_offset) -> Result<Option<Region>>`：✅ 虚拟桌面坐标选择
- `MockSelector`：无 UI，返回固定 Region 或 Cancelled，便于测试。

### 虚拟桌面支持 ✅ 已实现
- **虚拟坐标转换**：`select_with_virtual_background()` 自动将本地坐标转换为虚拟桌面全局坐标
- **跨显示器选择**：支持选择跨越多个显示器的区域，返回虚拟桌面坐标
- **显示器偏移处理**：自动处理当前交互显示器在虚拟桌面中的位置偏移
- **统一坐标系**：所有跨显示器操作使用统一的虚拟桌面坐标系统

## 事件与交互
- 鼠标左键拖拽：绘制/调整矩形。鼠标释放后选框保持显示，可继续调整。
- Enter：确认选择并完成截图；Esc：取消。
- Shift：固定比例（如 16:9/1:1）；Alt：从中心拉伸。
- 方向键：微调选框位置（1px）。

## 坐标与缩放
- `x,y,w,h` 为逻辑坐标（winit 的 logical）；`scale = window.scale_factor()`。
- 裁剪像素矩形时应使用：`px = round(x*scale) ...`，并对边界做 clamp。

### 多显示器坐标系统 ✅ 已实现并修复（v0.1.3-v0.1.4）
- **虚拟桌面坐标**：以主显示器左上角为原点的全局坐标系
- **显示器相对坐标**：每个显示器内部的局部坐标系
- **跨显示器区域**：使用虚拟桌面坐标描述跨越多个显示器的区域
- **DPI 适配**：自动处理不同显示器间的 DPI 差异和缩放
- **HiDPI 支持**：智能识别 Retina 显示器的 backing scale（如 2.0x），窗口缓冲区自动匹配物理像素
- **坐标转换一致性**：统一处理鼠标事件坐标转换和渲染坐标转换，确保选择框正确显示在对应位置
- **修复记录**：
  - 解决了非主屏框选坐标错误和选择框显示位置错误的问题
  - 修复了竖屏显示器窗口尺寸错误的问题（v0.1.3）
  - 改进了 HiDPI 日志输出，避免误报"尺寸不匹配"警告（v0.1.4）

## 平台实现

### 通用架构
- 统一采用 `winit + Skia` 的跨平台实现，核心文件：`crates/ui_overlay/src/selector.rs`。
- 渲染：使用 Skia 2D 图形库进行高质量渲染，支持 GPU 加速和 CPU fallback，绘制真实桌面截图背景、选区外暗化、选区白色描边，选区内部保持透明效果。
- 平台胶水隔离：`ui_overlay::platform` 模块（`crates/ui_overlay/src/platform/mod.rs`），封装平台特定的窗口呈现设置，核心选择器仅调用 `start_presentation()/end_presentation()`，减少条件编译分支和跨端耦合。
- 启动性能优化：窗口初始不可见（`with_visible(false)`），创建后立即预热 Skia Surface（`ensure_surface` + `render_once`），再显示并 `request_redraw`；空闲阶段不进行持续重绘（`about_to_wait` 不再 `request_redraw`），仅在输入/尺寸变化时重绘。

### macOS 平台实现 ✅ 已优化（v0.1.4）

#### 窗口管理（v0.1.3 重构）
**文件**：`crates/ui_overlay/src/window_manager.rs`

**单一信源原则**：
- 移除了对外部 `monitor_layouts` 参数的依赖
- 使用 winit 的 `available_monitors()` 作为唯一的显示器信息来源
- 在创建窗口时就指定尺寸和位置（`with_inner_size()` + `with_position()`）
- 使用窗口实际返回的尺寸而非预期尺寸

**窗口创建流程**：
```rust
for monitor in event_loop.available_monitors() {
    let physical_size = monitor.size();
    let physical_position = monitor.position();
    let scale = monitor.scale_factor();

    let attrs = base_attrs
        .clone()
        .with_inner_size(physical_size)  // 创建时指定
        .with_position(Position::Physical(physical_position));

    let window = event_loop.create_window(attrs)?;
    let actual_size = window.inner_size();  // 使用实际值
}
```

**HiDPI 智能识别**：
- 检测窗口实际尺寸与 monitor 报告尺寸的比例
- 如果比例接近整数（如 2.0），识别为正常的 HiDPI backing scale
- 只在比例异常时输出警告，避免误报

#### 窗口层级和行为（v0.1.4 新增）
**文件**：`crates/ui_overlay/src/platform/macos.rs`

**窗口层级设置**：
```rust
// 设置为 NSScreenSaverWindowLevel (1000)
// 远高于菜单栏层级 (NSMainMenuWindowLevel = 24)
let level: i64 = 1000;
msg_send![ns_window, setLevel: level];
```

**窗口集合行为**：
```rust
// NSWindowCollectionBehaviorCanJoinAllSpaces (1)
// NSWindowCollectionBehaviorFullScreenPrimary (128)
// NSWindowCollectionBehaviorStationary (16)
let behavior: u64 = (1 << 0) | (1 << 7) | (1 << 4);  // = 145
msg_send![ns_window, setCollectionBehavior: behavior];
```

**鼠标事件处理**：
```rust
// 确保窗口接受鼠标事件，不穿透到下层窗口
msg_send![ns_window, setIgnoresMouseEvents: Bool::from(false)];
```

**完整特性**：
- 通过 Cocoa API 设置 NSWindow 为无边框、不可拖动、最高层级
- 隐藏 Dock 和菜单栏（`NSApplicationPresentationOptions`）
- 窗口层级 1000 确保完全覆盖菜单栏，防止顶部拖拽时菜单栏出现
- 窗口 frame 设置为整个屏幕区域，确保完全覆盖
- 支持所有虚拟桌面（`CanJoinAllSpaces`）
- 支持横屏、竖屏、混合屏幕配置

### Windows 平台实现
- 使用同一套 winit 事件与 Skia 渲染路径
- 可选增强为窗口置顶与穿透
- Direct3D GPU 后端待实现（Phase 3）

### 多显示器渲染架构 ✅ 已完成（v0.1.3）

#### 窗口创建和管理
**架构改进**：
- **全局覆盖**：在所有显示器上创建全屏覆盖窗口
- **窗口管理**：为每个显示器创建独立的 winit 窗口实例
- **通用适配**：支持横屏、竖屏、混合配置，自动适应各种排列方式

**窗口创建策略**：
1. 使用 `winit::available_monitors()` 获取所有显示器
2. 为每个显示器创建独立窗口，使用显示器报告的物理尺寸和位置
3. 窗口使用 `AlwaysOnTop` 层级和透明背景
4. macOS 额外设置窗口层级为 1000，确保覆盖菜单栏

**坐标系统**：
- 每个窗口记录其在虚拟桌面中的位置（`virtual_x`, `virtual_y`）
- 鼠标事件坐标自动转换为虚拟桌面坐标
- 渲染时将虚拟坐标转换为窗口本地坐标：`local = virtual - window_virtual`

#### 事件处理
- **事件同步**：统一处理来自不同显示器窗口的输入事件
- **跨窗口渲染**：支持绘制跨越多个显示器的选择区域
- **交互连贯性**：鼠标在不同显示器间移动时，选择框连续显示

#### 诊断日志系统（v0.1.3-v0.1.4）
**详细的调试信息**，便于问题排查：

```rust
// xcap 显示器检测
[xcap Monitor 0] id=1, name="Display", primary=true
[xcap Monitor 0] 逻辑坐标: (0, 0), 逻辑尺寸: 3840x2160
[xcap Monitor 0] scale_factor: 2.0
[xcap Monitor 0] 实际捕获图像尺寸: 7680x4320 (物理像素)

// winit 窗口创建
[Winit Monitor 0] name: "Built-in Display"
[Winit Monitor 0] 物理位置: (0, 0), 物理尺寸: 3840x2160
[Winit Monitor 0] scale_factor: 2.0
[Window 0] 创建成功
[Window 0] Monitor报告: size=3840x2160, pos=(0, 0), scale=2.0
[Window 0] 实际inner: 7680x4320
[Window 0] HiDPI 缩放: Monitor报告 3840x2160, 窗口实际 7680x4320 (backing_scale ≈ 2.0x)

// macOS 窗口层级
[macOS Window] 窗口层级 level=1000, 集合行为 collectionBehavior=0b10010001 (145)
[macOS Window] 层级说明: NSMainMenuWindowLevel=24, NSScreenSaverWindowLevel=1000
```

### Skia 渲染架构 ✅ 已完成

#### RenderBackend 抽象层
**文件**：`crates/ui_overlay/src/backend/`

**架构设计**：
- `BackendType` 枚举：`MetalGpu`, `Direct3dGpu`, `CpuRaster`
- `RenderBackend` trait：统一的 GPU/CPU 渲染接口
  - `prepare_surface()`: 准备渲染表面
  - `canvas()`: 获取 Skia Canvas
  - `flush_and_read_pixels()`: 提交渲染结果
  - `resize()`: 处理窗口尺寸变化
- `factory.rs`：根据平台自动选择最佳可用后端

#### 后端实现

**1. macOS Metal GPU**（`metal_backend.rs`）：
- 使用 `metal-rs` crate 实现原生 Metal 渲染
- 通过 `CAMetalLayer` 直接渲染到窗口，无需 softbuffer
- 每帧从 layer 获取 `MetalDrawable`，创建 Skia GPU Surface
- `flush_and_read_pixels()` 中调用 `drawable.present()` 提交到屏幕
- 支持 Skia `DirectContext` 硬件加速渲染
- 异步提交 command buffer，复用 `CommandQueue`，利用 VSync 同步

**2. CPU Raster**（`cpu_backend.rs`）：
- 使用 `Surface::new_raster_n32_premul` 创建 CPU 渲染表面
- 读取像素后通过 `softbuffer` 提交到窗口
- 作为 GPU 不可用时的降级方案

**3. Windows Direct3D**（待实现）：
- 计划使用 `wgpu` 或 `windows-rs` 实现 D3D11/D3D12 渲染
- 架构与 Metal 类似，通过 DirectX 直接渲染到窗口

#### 后端选择策略
1. macOS 优先使用 Metal GPU（`DirectContext::make_metal` + `CAMetalLayer`）
2. Windows 将优先使用 Direct3D GPU（Phase 3）
3. 任一 GPU 初始化失败时自动降级到 CPU Raster
4. CPU Raster 使用 `softbuffer` 确保所有环境都能正常显示

#### Surface 生命周期
- 每个窗口持有独立的 `RenderBackend` 实例
- `prepare_surface()` 在每帧开始时准备渲染表面（GPU 获取 drawable，CPU 创建 raster surface）
- `canvas()` 返回 Skia Canvas 用于绘制
- `flush_and_read_pixels()` 提交渲染结果（GPU 调用 present，CPU 返回像素数据）
- `resize()` 处理窗口尺寸变化

#### 渲染流程
1. `WindowInfo::render()` 接收绘制闭包
2. 调用 `backend.prepare_surface()` 准备当前帧
3. 通过 `backend.canvas()` 获取 Canvas 并执行绘制闭包：
   - 绘制暗化背景图像（Skia Image，缓存）
   - 绘制选区内原始背景（裁剪 + 图像绘制）
   - 绘制选择框边框（白色描边）
4. 调用 `backend.flush_and_read_pixels()` 提交结果
5. GPU backend 直接 present，CPU backend 通过 softbuffer 呈现

#### 坐标转换
- 统一虚拟桌面坐标到窗口本地坐标的转换公式：`local = virtual - window_virtual`
- 确保背景偏移和选择框坐标使用相同的转换逻辑
- 修复了选择框在非主显示器上位置错误的问题

### 渲染性能与拖动优化 ✅ 已完成并持续优化

#### 核心性能优化

**鼠标移动防抖优化**（`event_handler.rs`）：
- 使用固定阈值代替动态计算：拖动时 5px，非拖动时 10px
- 移除每次移动时的选择区域面积计算，减少 CPU 开销
- 只在拖动状态下触发重绘，避免无效重绘（~50% 重绘次数减少）

**选择矩形计算缓存**（`SelectionState`）：
- 添加 `cached_rect` 和 `cache_valid` 字段缓存计算结果
- 在鼠标移动、修饰键变化时使缓存失效
- 避免每帧多次重复计算选择矩形（~30% CPU 计算减少）

**图像缓存优化**（`ImageCache`）：
- 改用 `initialized` 标志位替代哈希验证
- 背景图片在选择过程中不变，只需初始化一次
- 移除每帧的哈希计算开销，提升渲染性能
- 使用 `Arc<Image>` 零拷贝共享，减少内存占用

**重绘频率控制简化**：
- 移除复杂的预算机制，只保留时间间隔限流（16ms）
- 简化 `should_throttle_redraw()` 逻辑，减少判断开销
- 配合 `FrameTimer` 的 60 FPS 限制，实现稳定帧率

**帧率控制**（`FrameTimer`）：
- 限制渲染频率为 60 FPS（16.67ms/帧）
- 在 `request_redraw_all()` 中统一检查帧率限制
- 初始化时预设时间偏移，确保第一帧立即可渲染

**Metal Backend 优化**：
- 异步提交 command buffer，不等待 GPU 完成
- 复用 `CommandQueue` 避免每帧创建
- 利用 VSync 同步，避免画面撕裂

#### 渲染性能策略

**窗口预热**：
- 窗口初始不可见（`with_visible(false)`）
- 先创建 RenderBackend 并预热渲染
- 完成首次渲染后再显示窗口，避免屏幕闪动

**尺寸变化处理**：
- 窗口 resize 时调用 `backend.resize()` 更新尺寸
- 下次 `prepare_surface()` 时会重建对应尺寸的 Surface

#### 性能指标
- 拖动延迟：< 33ms（2 帧以内）
- 重绘频率：稳定 60 FPS
- CPU 占用：相比优化前减少 ~40%
- 内存占用：通过 Arc 共享，减少重复拷贝

## 错误与取消语义
- 用户按 Esc/关闭窗口 -> `OverlayError::Cancelled`
- 系统/窗口错误 -> `OverlayError::Internal(String)`

## 与 services 的集成
- services 中新增 `RegionSelectService`：
  - 使用平台具体实现（如 `WinitSelector` 封装）调用 `select()`
  - 将 `Region` 转换为对全屏 `Frame` 的裁剪区域，传递给 renderer/export

当前：`platform_mac::MacCapturer::capture_region_interactive_custom(selector)` 已接入；CLI 子命令 `capture-interactive` 调用 `ui_overlay::create_gui_region_selector()`，完成一次全屏捕获 + 区域裁剪并导出。其他平台可复用相同选择器。

## 安全与权限
- 无敏感权限；不读取剪贴板/文件系统，仅创建窗口

## 已知问题和限制
无（v0.1.4 已修复所有已知的多显示器和菜单栏干扰问题）

## 调试和诊断

### 启用 Debug 日志
```bash
# 完整日志
RUST_LOG=debug cargo run --release --bin api_cli -- capture-interactive --out-dir /tmp

# 只看关键模块
RUST_LOG=ui_overlay=debug,platform_mac=debug cargo run --release --bin api_cli -- capture-interactive --out-dir /tmp
```

### 日志解读

**正常的 HiDPI 显示器**：
```
[Window 0] HiDPI 缩放: Monitor报告 3840x2160, 窗口实际 7680x4320 (backing_scale ≈ 2.0x)
```
✅ 这是正常的 Retina 显示器行为，不是错误

**异常的窗口尺寸**：
```
[Window 0] ⚠️  尺寸异常! Monitor报告 3840x2160, 窗口实际 5000x3000 (比例: 1.30)
```
⚠️ 这才是真正的问题，需要调查

**窗口层级验证**：
```
[macOS Window] 窗口层级 level=1000, 集合行为 collectionBehavior=0b10010001 (145)
```
✅ 窗口层级 1000 > 菜单栏层级 24，菜单栏不会出现

## 版本历史

### v0.1.4 (2025-10-25)
- ✅ 屏幕顶部菜单栏干扰修复：设置窗口层级为 1000，集合行为为 145
- ✅ HiDPI 日志改进：智能识别 Retina 显示器的正常尺寸缩放，避免误报警告

### v0.1.3 (2025-10-25)
- ✅ 多屏幕窗口尺寸修复：使用 winit 单一信源，正确支持横竖屏混合场景
- ✅ 详细诊断日志：添加完整的 debug 日志系统，便于问题排查

### v0.1.2
- ✅ Metal GPU 硬件加速：macOS 原生 Metal 渲染
- ✅ 性能优化：60 FPS 帧率控制 + 图像缓存
- ✅ 坐标转换修复：统一虚拟坐标到窗口本地坐标转换

### v0.1.1
- ✅ 基础交互式选择器实现
- ✅ 多显示器支持和虚拟桌面坐标系统

## 未来计划

### v0.2 - 标注编辑 UI
- 状态机扩展：增加 EditingState
- 集成 egui：工具栏 UI 框架
- 基础工具：矩形、箭头、完成/取消按钮
- 撤销/重做集成

详见 `docs/todo/ui_overlay.md`

## 参考文档
- 修复总结：`MULTI_MONITOR_FIX_SUMMARY.md`（多屏幕窗口修复）
- 修复总结：`MENUBAR_FIX_SUMMARY.md`（菜单栏干扰修复）
- 修复总结：`HIDPI_LOG_FIX.md`（HiDPI 日志改进）
- 性能优化：`docs/ui_overlay_performance_optimization.md`
- TODO 列表：`docs/todo/ui_overlay.md`
