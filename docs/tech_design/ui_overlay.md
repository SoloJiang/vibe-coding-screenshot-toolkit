# ui_overlay 模块技术设计

## 概述
提供跨平台、自研的截图框选与轻量交互标注基础层，替换平台命令行 (`screencapture -i/-R`)。使用 Iced 替换原有的 FLTK 方案，提供更现代化、高性能的 GUI 解决方案。

## 功能范围
- 区域选择（矩形） -> 返回逻辑坐标 (x,y,w,h) + scale
- 取消/确认交互（Esc / Enter / Space）
- 多显示器支持：坐标统一为虚拟桌面坐标系
- 输入事件：鼠标按下拖拽、拖拽边/角 resize、方向键微调
- 视觉：半透明遮罩、选区边框/八个控制点、尺寸提示

后续扩展（不在第一阶段交付）：
- 实时放大镜 / 网格线
- 内置标注（矩形/箭头/文本快速添加）
- 连续多次截图、窗口模式、延时模式
- 多形状 (自由曲线、圆形、多边形) 选择

## 技术选型：Iced

### 技术优势对比
**原方案 (FLTK)：**
- 传统 GUI 工具包
- C++ 绑定，可能存在内存安全问题
- 较为陈旧的架构设计

**新方案 (Iced)：**
- 现代化的 Rust GUI 框架
- 声明式 UI 设计，类似 React
- 强类型安全保证
- 优秀的跨平台支持
- 基于 GPU 渲染，性能优异
- 活跃的社区和持续更新

## 技术架构

### 核心组件
```
ui_overlay (Iced 版本)
├── selection/
│   ├── overlay_app.rs          // Iced 主应用程序
│   ├── selection_canvas.rs     // Canvas 组件用于绘制
│   ├── interaction_handler.rs  // 鼠标/键盘事件处理
│   └── state_machine.rs        // 选择状态管理
├── widgets/
│   ├── selection_overlay.rs    // 自定义选区覆盖组件
│   ├── size_indicator.rs       // 尺寸提示组件
│   └── crosshair.rs           // 十字准星组件
├── backend/
│   ├── iced_renderer.rs       // Iced 绘制后端
│   └── screen_capture.rs      // 背景截图处理
└── integration/
    ├── region_selector.rs     // RegionSelector trait 实现
    └── service_bridge.rs      // 与 services 模块的桥接
```

### 关键数据结构

```rust
// 选区状态
#[derive(Debug, Clone)]
pub struct SelectionState {
    pub rect: Rect,
    pub mode: SelectionMode,
    pub anchor_point: Option<Point>,
    pub is_dragging: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMode {
    Creating,           // 正在创建选区
    Selected,          // 已选中
    Resizing(Corner),  // 正在调整大小
    Moving,            // 正在移动
}

#[derive(Debug, Clone)]
pub enum Corner {
    TopLeft, TopRight, BottomLeft, BottomRight,
    Top, Bottom, Left, Right,
}

// FLTK 窗口包装
pub struct OverlayWindow {
    window: fltk::window::Window,
    selection_frame: SelectionFrame,
    size_indicator: SizeIndicator,
    background_image: Option<fltk::image::RgbImage>,
    state: SelectionState,
}
```

### Iced 实现细节

#### 应用程序架构
```rust
use iced::{Application, Command, Element, Settings, Theme};

pub struct OverlayApp {
    selection_state: SelectionState,
    background_image: Option<Vec<u8>>,
    screen_bounds: Rect,
}

#[derive(Debug, Clone)]
pub enum Message {
    MousePressed(Point),
    MouseDragged(Point),
    MouseReleased(Point),
    KeyPressed(Key),
    SelectionComplete(Option<Rect>),
}

impl Application for OverlayApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = (Rect, Vec<u8>); // screen_bounds, background_image

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                selection_state: SelectionState::default(),
                background_image: Some(flags.1),
                screen_bounds: flags.0,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Screenshot Overlay")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::MousePressed(point) => {
                self.selection_state.start_creating(point);
            }
            Message::MouseDragged(point) => {
                self.selection_state.update_creating(point);
            }
            Message::MouseReleased(point) => {
                self.selection_state.finish_creating();
            }
            Message::KeyPressed(key) => {
                match key {
                    Key::Named(Named::Escape) => {
                        return iced::window::close();
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(rect) = self.selection_state.get_selection() {
                            return iced::window::close();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // 使用 Canvas 组件绘制选择界面
        Canvas::new(&self.selection_state)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
```
                },
                fltk::enums::Event::Release => {
                    let (x, y) = fltk::app::event_coords();
                    self.handle_mouse_up(x as f32, y as f32);
                    true
                },
                fltk::enums::Event::KeyDown => {
                    match fltk::app::event_key() {
                        fltk::enums::Key::Escape => {
                            self.cancel_selection();
                            true
                        },
                        fltk::enums::Key::Enter | fltk::enums::Key::KPEnter => {
                            self.confirm_selection();
                            true
                        },
                        _ => false,
                    }
                },
                _ => false,
            }
        });
        Ok(())
    }
}
```

#### 自定义绘制
```rust
// 自定义选区组件
#[derive(Clone)]
pub struct SelectionFrame {
    widget: fltk::frame::Frame,
}

impl SelectionFrame {
    pub fn new() -> Self {
        let mut frame = fltk::frame::Frame::default();
        frame.set_frame(fltk::enums::FrameType::NoBox);
        frame.draw(Self::draw_selection);

        Self { widget: frame }
    }

    fn draw_selection() {
        // 使用 FLTK 的绘制 API
        fltk::draw::set_draw_color(fltk::enums::Color::Red);
        fltk::draw::set_line_style(fltk::draw::LineStyle::Solid, 2);

        // 绘制选区边框
        let (x, y, w, h) = self.get_selection_bounds();
        fltk::draw::draw_rect_fill(x, y, w, h, fltk::enums::Color::from_rgba_tuple((255, 0, 0, 128)));
        fltk::draw::draw_rect(x, y, w, h);

        // 绘制控制点
        self.draw_resize_handles(x, y, w, h);
    }

    fn draw_resize_handles(&self, x: i32, y: i32, w: i32, h: i32) {
        let handle_size = 8;
        let positions = [
            (x - handle_size/2, y - handle_size/2),                    // 左上
            (x + w/2 - handle_size/2, y - handle_size/2),              // 上中
            (x + w - handle_size/2, y - handle_size/2),                // 右上
            (x + w - handle_size/2, y + h/2 - handle_size/2),          // 右中
            (x + w - handle_size/2, y + h - handle_size/2),            // 右下
            (x + w/2 - handle_size/2, y + h - handle_size/2),          // 下中
            (x - handle_size/2, y + h - handle_size/2),                // 左下
            (x - handle_size/2, y + h/2 - handle_size/2),              // 左中
        ];

        fltk::draw::set_draw_color(fltk::enums::Color::White);
        for (hx, hy) in positions {
            fltk::draw::draw_rect_fill(hx, hy, handle_size, handle_size, fltk::enums::Color::Blue);
            fltk::draw::set_draw_color(fltk::enums::Color::Black);
            fltk::draw::draw_rect(hx, hy, handle_size, handle_size);
        }
    }
}
```

## 状态机
Idle -> (MouseDown) -> Creating -> (MouseUp) -> Selected
Selected + DragCorner -> Resizing
Selected + DragBody -> Moving
Esc -> Canceled
Enter/Space -> Confirmed

## 性能优化

### 绘制优化
```rust
impl OverlayWindow {
    // 脏矩形更新
    pub fn invalidate_region(&mut self, rect: Rect) {
        self.window.damage_region(
            rect.x as i32,
            rect.y as i32,
            rect.w as i32,
            rect.h as i32
        );
    }

    // 背景缓存
    fn cache_background(&mut self, image_data: &[u8]) {
        self.background_image = Some(
            fltk::image::RgbImage::new(image_data, self.window.w(), self.window.h(), fltk::enums::ColorDepth::Rgb8)
                .expect("Failed to create background image")
        );
    }
}
```

### 性能目标
- 目标：1080p 下 < 1ms per frame CPU；4K 双屏下保持 60 FPS。
- 策略：
  - 脏矩形重绘：只重绘旧选区与新选区包围盒。
  - 复用缓存纹理：背景快照一次性绘制。
  - 减少分配：RectState/事件缓冲池。

## 与现有模块的集成

### RegionSelector trait 实现
```rust
pub struct FltkRegionSelector {
    config: SelectionConfig,
}

impl RegionSelector for FltkRegionSelector {
    fn select(&self, background: FrameSet) -> Result<Option<Rect>, OverlayError> {
        // 合并多显示器背景
        let merged_background = self.merge_displays(&background)?;

        // 创建 FLTK 覆盖窗口
        let mut overlay = OverlayWindow::new(
            merged_background.bounds(),
            &merged_background.pixels()
        )?;

        // 设置事件处理
        overlay.setup_event_handlers()?;

        // 运行事件循环直到用户确认或取消
        let result = self.run_selection_loop(&mut overlay)?;

        Ok(result)
    }
}

impl FltkRegionSelector {
    fn run_selection_loop(&self, overlay: &mut OverlayWindow) -> Result<Option<Rect>, OverlayError> {
        let (sender, receiver) = std::sync::mpsc::channel();

        // 设置回调
        overlay.set_result_callback(sender);

        // 运行 FLTK 事件循环
        while fltk::app::wait() {
            if let Ok(result) = receiver.try_recv() {
                return Ok(result);
            }
        }

        Ok(None)
    }
}
```

## 配置和扩展

### 选择配置
```rust
#[derive(Debug, Clone)]
pub struct SelectionConfig {
    pub min_size: Size,
    pub snap_threshold: f32,
    pub show_size_indicator: bool,
    pub show_crosshair: bool,
    pub selection_color: Color,
    pub handle_color: Color,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            min_size: Size::new(4.0, 4.0),
            snap_threshold: 5.0,
            show_size_indicator: true,
            show_crosshair: true,
            selection_color: Color::from_rgba(255, 0, 0, 128),
            handle_color: Color::from_rgb(0, 120, 255),
        }
    }
}
```

### 主题支持
```rust
pub struct FltkTheme {
    pub background_opacity: f32,
    pub selection_stroke_width: f32,
    pub handle_size: f32,
    pub colors: FltkColorScheme,
}

pub struct FltkColorScheme {
    pub selection_border: Color,
    pub selection_fill: Color,
    pub handle_border: Color,
    pub handle_fill: Color,
    pub size_indicator_bg: Color,
    pub size_indicator_text: Color,
}
```

## 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum FltkOverlayError {
    #[error("FLTK initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Image creation failed: {0}")]
    ImageCreationFailed(String),

    #[error("Event handling failed: {0}")]
    EventHandlingFailed(String),

    #[error("User canceled selection")]
    Canceled,

    #[error("Invalid selection bounds")]
    InvalidBounds,
}
```

## 多显示器支持

```rust
impl FltkRegionSelector {
    fn handle_multiple_displays(&self, displays: &[DisplayInfo]) -> Result<OverlayWindow> {
        // 计算虚拟桌面边界
        let virtual_bounds = self.calculate_virtual_bounds(displays);

        // 为每个显示器创建子窗口
        let mut windows = Vec::new();
        for display in displays {
            let window = self.create_display_window(display)?;
            windows.push(window);
        }

        // 创建主协调窗口
        let main_window = self.create_main_overlay_window(virtual_bounds, windows)?;

        Ok(main_window)
    }
}
```

## 阶段规划

| 阶段 | 内容 | 完成判据 |
|------|------|----------|
| m1 | FLTK-rs 基础集成 + 矩形选择 | CLI `capture-interactive` 可返回坐标 |
| m2 | 交互增强：控制点 + 键盘微调 + 尺寸提示 | 所有交互测试用例通过 |
| m3 | 多显示器 + DPI Scale 支持 | 不同 DPI 下返回正确逻辑坐标 |
| m4 | 视觉优化：遮罩 + 主题 + 动画 | 视觉效果符合设计要求 |
| m5 | 性能优化：脏矩形 + 缓存 | FPS >= 60 在 4K 双屏 |
| m6 | 高级功能：内置标注 & 扩展点 | 标注事件回调接入 services |

## 与其他模块交互
- **platform_***：提供一次全屏捕获 FrameSet；ui_overlay 返回 Rect -> services 使用 renderer 在内存裁剪，不再二次平台调用。
- **services::CaptureService**：新增 capture_interactive() 分支 -> 全屏捕获 + 调用 RegionSelector -> 裁剪 + 后续流程。
- **infra::metrics**：计数 (interactive_ok / interactive_cancel / interactive_error) + 时长直方图。

## 测试策略

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_state_machine() {
        let mut state = SelectionState::default();

        // 测试状态转换
        state.start_creating(Point::new(10.0, 10.0));
        assert_eq!(state.mode, SelectionMode::Creating);

        state.finish_creating(Point::new(100.0, 100.0));
        assert_eq!(state.mode, SelectionMode::Selected);
    }

    #[test]
    fn test_bounds_calculation() {
        let selector = FltkRegionSelector::default();
        let rect = selector.calculate_selection_bounds(
            Point::new(10.0, 10.0),
            Point::new(100.0, 100.0)
        );

        assert_eq!(rect.w, 90.0);
        assert_eq!(rect.h, 90.0);
    }
}
```

## 部署配置

### 依赖配置
```toml
[dependencies]
fltk = { version = "^1.5", features = ["fltk-bundled"], optional = true }
tao = { version = "0.34.1", optional = true }
screenshot_core = { path = "../core" }
anyhow = { workspace = true }
thiserror = { workspace = true }

[features]
default = ["fltk-ui"]
fltk-ui = ["fltk"]
tao-ui = ["tao"]  # 保留旧实现作为备选
```

## 迁移路径

### 阶段 1：基础实现
- [ ] 实现 FltkRegionSelector
- [ ] 基础矩形选择
- [ ] Esc/Enter 键处理

### 阶段 2：交互增强
- [ ] 调整大小控制点
- [ ] 拖拽移动
- [ ] 尺寸指示器

### 阶段 3：多显示器
- [ ] 虚拟桌面支持
- [ ] 跨屏拖拽
- [ ] DPI 缩放支持

### 阶段 4：性能优化
- [ ] 脏矩形更新
- [ ] 背景缓存
- [ ] 60fps 目标达成

## 安全 & 权限
- macOS 屏幕录制权限不足时，全屏捕获失败在进入 UI 前即返回。
- 不额外请求权限；使用已有捕获路径。

## 后续扩展点
- 插件：注册新的 Overlay Tool（标注/遮罩）
- 脚本 API：暴露 JS / NAPI 绑定用于自动化测试与插件化配置。

## 当前状态
- 已完成 FLTK-rs 技术设计方案
- 已更新依赖配置，支持 fltk-ui 和 tao-ui 特性标志
- 待开始：FltkRegionSelector 的具体实现

这个设计方案提供了从 tao 到 FLTK-rs 的完整迁移路径，保持了与现有架构的兼容性，同时提供了更好的性能和更简洁的实现。
