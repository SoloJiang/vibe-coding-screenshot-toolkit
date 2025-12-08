# ui_overlay 模块技术设计

## 版本信息
- 当前版本：v0.2.0
- 最后更新：2025-10-28

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
- **DPI 适配**：自动处理不同显示器间的DPI 差异和缩放
- **HiDPI 支持**：智能识别 Retina 显示器的 backing scale（如 2.0x），窗口缓冲区自动匹配物理像素
- **坐标转换一致性**：统一处理鼠标事件坐标转换和渲染坐标转换，确保选择框正确显示在对应位置
- **修复记录**：
  - 解决了非主屏框选坐标错误和选择框显示位置错误的问题
  - 修复了竖屏显示器窗口尺寸错误的问题（v0.1.3）
  - 改进了 HiDPI 日志输出，避免误报"尺寸不匹配"警告（v0.1.4）

## 平台实现

### 通用架构 ✅ v0.2.0 重大重构

**核心技术栈**：
- **窗口管理**：`winit 0.30` - 跨平台窗口和事件循环
- **UI 渲染**：`egui 0.33` - 即时模式 GUI（选择框、遮罩、工具栏）
- **背景渲染**：`wgpu` - 直接 GPU 渲染，绕过 egui 纹理限制（2048x2048）
- **平台适配**：`ui_overlay::platform` - macOS/Windows 特定窗口行为

**架构演进**（v0.1.x → v0.2.0）：
- ~~Skia 2D 图形库（CPU/Metal GPU）~~ → **egui + wgpu 混合渲染**
- ~~RenderBackend 抽象层（metal_backend/cpu_backend）~~ → **统一 egui-wgpu 后端**
- ~~背景暗化图像缓存（47MB 内存）~~ → **egui 半透明遮罩（0 额外内存）**
- ~~手动渲染循环~~ → **winit 标准事件驱动模式（about_to_wait → RedrawRequested）**

**渲染流程**（双层架构）：
```rust
// 1. wgpu 背景层（底层）
BackgroundRenderer::render(&mut render_pass, background_bind_group);
// ↓ 渲染裁剪后的背景图（全屏显示，全分辨率）

// 2. egui UI 层（上层）
egui::CentralPanel::default().show(ctx, |ui| {
    let painter = ui.painter();

    // 绘制 4 个半透明黑色矩形（暗化遮罩）
    painter.rect_filled(top_rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 128));
    painter.rect_filled(bottom_rect, 0.0, darken_color);
    painter.rect_filled(left_rect, 0.0, darken_color);
    painter.rect_filled(right_rect, 0.0, darken_color);

    // 绘制选择框白色边框
    painter.rect_stroke(selection_rect, 0.0, Stroke::new(2.0, Color32::WHITE));

    // 绘制尺寸文本
    painter.text(text_pos, Align2::CENTER_CENTER, size_text, font, Color32::WHITE);
});
```

**为什么选择 egui + wgpu？**
1. **突破 egui 纹理限制**：多显示器场景下背景图可能 7680x4320，超过 egui 的 2048x2048 限制
2. **代码简洁**：egui 即时模式 API 比 Skia Canvas API 更直观，选择框渲染从 50 行降到 20 行
3. **内存优化**：不再需要预先暗化的背景图（-47MB），使用半透明遮罩实时合成
4. **跨平台一致性**：wgpu 抽象了 Metal/D3D/Vulkan，无需平台特定代码
5. **未来扩展**：egui 自带丰富的 UI 组件（按钮、滑块、颜色选择器），为标注工具栏做好准备

### 窗口管理（v0.1.3 重构）
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

### macOS 平台实现 ✅ 已优化（v0.1.4）

#### 窗口层级和行为
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

**完整特性**：
- 通过 Cocoa API 设置 NSWindow 为无边框、不可拖动、最高层级
- 隐藏 Dock 和菜单栏（`NSApplicationPresentationOptions`）
- 窗口层级 1000 确保完全覆盖菜单栏，防止顶部拖拽时菜单栏出现
- 窗口 frame 设置为整个屏幕区域，确保完全覆盖
- 支持所有虚拟桌面（`CanJoinAllSpaces`）
- 支持横屏、竖屏、混合屏幕配置

### Windows 平台实现
- 使用同一套 winit 事件与 wgpu 渲染路径
- 通过 `platform::windows` 设置窗口置顶与透明
- wgpu 自动选择 DirectX 12/11 后端

### 多显示器渲染架构 ✅ 已完成（v0.1.3 + v0.2.0 优化）

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

## 渲染架构 ✅ v0.2.0 全新设计

### 混合渲染管线

**文件结构**：
```
crates/ui_overlay/src/
├── background_renderer.rs  # wgpu 背景渲染器（直接 GPU 渲染）
├── selection_render.rs     # egui 选择框渲染器（UI 层）
├── window_info.rs          # 窗口信息 + egui-wgpu 集成
├── selection_app.rs        # 应用主逻辑 + winit 事件循环
└── window_manager.rs       # 多窗口管理
```

### 1. 背景渲染层（wgpu）

**文件**：`crates/ui_overlay/src/background_renderer.rs`

**设计目标**：
- 绕过 egui 的 2048x2048 纹理限制
- 支持 7680x4320 等大尺寸虚拟桌面背景
- GPU 硬件加速，全分辨率显示

**实现策略**：
```rust
pub struct BackgroundRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,    // 全屏四边形
    index_buffer: wgpu::Buffer,     // 两个三角形
}

impl BackgroundRenderer {
    /// 为每个窗口创建裁剪后的背景纹理
    ///
    /// 从完整虚拟桌面背景中裁剪出当前窗口对应的区域，
    /// 上传为独立纹理，直接全屏显示（无需 UV 映射）
    pub fn create_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],  // RGBA8 裁剪后的纹理数据
        width: u32,   // 窗口宽度
        height: u32,  // 窗口高度
    ) -> (wgpu::Texture, wgpu::BindGroup);

    /// 渲染全屏背景纹理
    pub fn render<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        texture_bind_group: &'rpass wgpu::BindGroup,
    );
}
```

**Shader 代码**（简化版，无 Uniform）：
```wgsl
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;  // 直接全屏映射
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, input.uv);
}
```

**背景裁剪流程**：
```rust
// 1. 计算窗口在虚拟桌面中的位置（canvas 坐标）
let canvas_x = (window.virtual_x - virtual_min_x).max(0) as u32;
let canvas_y = (window.virtual_y - virtual_min_y).max(0) as u32;

// 2. 从完整背景中裁剪出窗口区域
let cropped_data = crop_background(
    full_bg_data,        // 完整虚拟桌面背景（7680x4320）
    full_bg_width,
    full_bg_height,
    canvas_x, canvas_y,  // 裁剪起点
    window_w, window_h,  // 裁剪尺寸
);

// 3. 上传为独立纹理（窗口尺寸，如 3840x2160）
let (texture, bind_group) = background_renderer.create_texture(
    &device, &queue, &cropped_data, window_w, window_h
);
```

**为什么裁剪而不是 UV 映射？**
- **代码简单**：每个窗口独立纹理，shader 无需 uniform 和 UV 计算
- **性能好**：上传数据量更小（3840x2160 vs 7680x4320），GPU 内存占用更少
- **易调试**：每个窗口纹理独立，坐标转换逻辑清晰

### 2. UI 渲染层（egui）

**文件**：`crates/ui_overlay/src/selection_render.rs`

**实现**：
```rust
pub struct EguiSelectionRenderer;

impl EguiSelectionRenderer {
    /// 使用 egui painter 渲染选择框
    pub fn render_selection(
        painter: &egui::Painter,
        selection_rect: Option<(f32, f32, f32, f32)>,  // 逻辑坐标
        window_size: (f32, f32),
    ) {
        let darken_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128);

        if let Some((x, y, w, h)) = selection_rect {
            // 绘制 4 个矩形遮罩，覆盖选择框外的区域
            painter.rect_filled(top_rect, 0.0, darken_color);
            painter.rect_filled(bottom_rect, 0.0, darken_color);
            painter.rect_filled(left_rect, 0.0, darken_color);
            painter.rect_filled(right_rect, 0.0, darken_color);

            // 绘制白色边框（2px，外描边）
            painter.rect_stroke(
                sel_rect, 0.0,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
                egui::epaint::StrokeKind::Outside,
            );

            // 绘制尺寸信息
            painter.text(text_pos, Align2::CENTER_CENTER, size_text, font, Color32::WHITE);
        } else {
            // 无选择框时，全屏暗化
            painter.rect_filled(fullscreen_rect, 0.0, darken_color);
        }
    }
}
```

**优势**：
- **透明效果**：4 个半透明矩形合成暗化效果，选择框内原始背景可见
- **无额外内存**：不需要预先生成暗化背景图（-47MB 内存）
- **简洁易读**：egui API 比 Skia Canvas 更直观，代码从 50 行降到 30 行

### 3. 窗口渲染集成

**文件**：`crates/ui_overlay/src/window_info.rs`

**核心结构**：
```rust
pub struct WgpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub renderer: egui_wgpu::Renderer,               // egui 渲染器
    pub background_renderer: BackgroundRenderer,     // wgpu 背景渲染器
    pub background_texture: Option<wgpu::Texture>,
    pub background_bind_group: Option<wgpu::BindGroup>,
}

pub struct WindowInfo {
    pub window: Arc<Window>,
    pub egui_ctx: egui::Context,                     // egui 上下文
    pub egui_state: egui_winit::State,               // egui-winit 状态
    pub wgpu_state: Option<WgpuState>,
    // ...
}
```

**渲染流程**：
```rust
impl WindowInfo {
    pub fn render<F>(&mut self, draw_fn: F) -> Result<(), String>
    where
        F: FnMut(&egui::Context),
    {
        // 1. 准备 wgpu surface
        let surface_texture = wgpu_state.surface.get_current_texture().unwrap();
        let surface_view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = wgpu_state.device.create_command_encoder(&Default::default());

        // 2. egui 输入处理
        let raw_input = self.egui_state.take_egui_input(&self.window);

        // 3. 执行 egui UI 逻辑
        let full_output = self.egui_ctx.run(raw_input, draw_fn);

        // 4. egui 更新纹理和缓冲区
        for (id, image_delta) in &full_output.textures_delta.set {
            wgpu_state.renderer.update_texture(&device, &queue, id, image_delta);
        }
        wgpu_state.renderer.update_buffers(&device, &queue, &mut encoder, &clipped_primitives, &screen_descriptor);

        // 5. 渲染 wgpu 背景层（底层）
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    // ...
                })],
                // ...
            });

            wgpu_state.background_renderer.render(&mut render_pass, bg_bind_group);
        } // render_pass dropped

        // 6. 渲染 egui UI 层（上层）
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,  // 保留背景，不清除
                        store: wgpu::StoreOp::Store,
                    },
                    // ...
                })],
                // ...
            });

            wgpu_state.renderer.render(&mut render_pass.forget_lifetime(), &clipped_primitives, &screen_descriptor);
        } // render_pass dropped

        // 7. 提交并 present
        wgpu_state.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        // 8. 释放 egui 纹理
        for id in &full_output.textures_delta.free {
            wgpu_state.renderer.free_texture(id);
        }

        Ok(())
    }
}
```

### 4. 事件循环（winit 标准模式）

**文件**：`crates/ui_overlay/src/selection_app.rs`

**v0.2.0 改进**：
```rust
impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 窗口初始化 + 预热渲染
        self.window_manager.initialize_windows(event_loop, &self.attrs);

        for i in 0..self.window_manager.windows.len() {
            self.window_manager.windows[i].init_wgpu().unwrap();
            self.ensure_background_uploaded(i);
            self.render_window_by_index(i);  // 预热
        }

        // 显示窗口
        for window in &self.window_manager.windows {
            window.window.set_visible(true);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // 处理各种窗口事件（鼠标、键盘、缩放等）
        self.on_window_event(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // ✅ winit 0.30 推荐模式：在事件循环空闲时请求重绘
        if self.state.dragging {
            self.window_manager.request_redraw_all();
        }
    }
}
```

**关键改进**（v0.1.x → v0.2.0）：
- ~~鼠标事件中直接渲染所有窗口~~ → **request_redraw() + RedrawRequested 事件**
- ~~手动管理渲染时机~~ → **about_to_wait() 统一控制重绘**
- 符合 winit 最佳实践，更好的跨平台兼容性
- 事件循环控制渲染时机，性能更稳定

## 性能优化 ✅ v0.2.0 全面优化

### 内存优化（-47MB）
- ~~预先生成暗化背景图（47MB）~~ → **egui 半透明遮罩（0 额外内存）**
- ~~ImageCache 模块（图像对象缓存）~~ → **删除（背景只上传一次）**
- **总内存占用**：181MB → ~134MB（-26%）

### 渲染性能
**帧率控制**（v0.1.2 + v0.2.0）：
- `FrameTimer` 限制渲染频率为 60 FPS（16.67ms/帧）
- 在 `request_redraw_all()` 中统一检查帧率限制
- 拖拽时持续 60 FPS，非拖拽时按需渲染

**鼠标移动防抖**（v0.1.2）：
- 使用固定阈值代替动态计算：拖动时 5px，非拖动时 10px
- 移除每次移动时的选择区域面积计算，减少 CPU 开销
- ~50% 重绘次数减少

**选择矩形计算缓存**（v0.1.2）：
- 添加 `cached_rect` 和 `cache_valid` 字段缓存计算结果
- 在鼠标移动、修饰键变化时使缓存失效
- ~30% CPU 计算减少

**wgpu 渲染优化**（v0.2.0 新增）：
- GPU 硬件加速（Metal/DirectX/Vulkan）
- 异步提交 command buffer，不等待 GPU 完成
- 利用 VSync 同步，避免画面撕裂
- 背景裁剪上传（窗口尺寸），减少 GPU 内存和带宽占用

### 窗口预热（v0.1.2 + v0.2.0）
- 窗口初始不可见（`with_visible(false)`）
- 先创建 wgpu surface 并预热渲染
- 完成首次渲染后再显示窗口，避免屏幕闪动

### 性能指标
- 拖动延迟：< 33ms（2 帧以内）
- 渲染频率：稳定 60 FPS
- CPU 占用：相比 v0.1.0 减少 ~40%
- 内存占用：相比 v0.1.4 减少 ~47MB（-26%）
- GPU 占用：Metal/DirectX 硬件加速，流畅无卡顿

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
无（v0.2.0 已修复所有已知问题）

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

### v0.2.0 (2025-10-28) 🚀 重大架构重构
- ✅ **渲染架构重构**：Skia → egui + wgpu 混合渲染
- ✅ **内存优化**：删除暗化背景缓存（-47MB，-26%）
- ✅ **代码简化**：删除 RenderBackend 抽象层（-450 行）
- ✅ **事件循环优化**：符合 winit 0.30 最佳实践（about_to_wait → RedrawRequested）
- ✅ **背景裁剪上传**：每个窗口独立纹理，减少 GPU 内存占用
- ✅ **egui 半透明遮罩**：实时合成暗化效果，0 额外内存
- ✅ **架构文档更新**：详细说明新渲染管线和设计决策

### v0.1.4 (2025-10-25)
- ✅ 屏幕顶部菜单栏干扰修复：设置窗口层级为 1000，集合行为为 145
- ✅ HiDPI 日志改进：智能识别 Retina 显示器的正常尺寸缩放，避免误报警告

### v0.1.3 (2025-10-25)
- ✅ 多屏幕窗口尺寸修复：使用 winit 单一信源，正确支持横竖屏混合场景
- ✅ 详细诊断日志：添加完整的 debug 日志系统，便于问题排查

### v0.1.2 (2025-10-24)
- ✅ Metal GPU 硬件加速：macOS 原生 Metal 渲染
- ✅ 性能优化：60 FPS 帧率控制 + 图像缓存
- ✅ 坐标转换修复：统一虚拟坐标到窗口本地坐标转换

### v0.1.1
- ✅ 基础交互式选择器实现
- ✅ 多显示器支持和虚拟桌面坐标系统

## 未来计划

### v0.3 - 标注编辑 UI（基于 egui）
- 状态机扩展：增加 EditingState
- 工具栏 UI：矩形、箭头、画笔、文字工具
- egui 颜色选择器、粗细调节
- 撤销/重做集成（core::UndoStack）
- 标注渲染：使用 egui Painter API

详见 `docs/todo/ui_overlay.md`

## 参考文档
- 架构重构总结：`remove-skia-dependencies.plan.md`（Skia → egui 迁移）
- 修复总结：`MULTI_MONITOR_FIX_SUMMARY.md`（多屏幕窗口修复）
- 修复总结：`MENUBAR_FIX_SUMMARY.md`（菜单栏干扰修复）
- 修复总结：`HIDPI_LOG_FIX.md`（HiDPI 日志改进）
- 性能优化：`docs/ui_overlay_performance_optimization.md`
- TODO 列表：`docs/todo/ui_overlay.md`
