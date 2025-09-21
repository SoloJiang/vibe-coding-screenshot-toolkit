/// 窗口管理模块
///
/// 提供窗口信息管理和多窗口操作功能
use pixels::Pixels;
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// 窗口信息结构体
///
/// 包含每个窗口的渲染器、尺寸、缩放比例和虚拟位置信息
pub struct WindowInfo {
    /// 窗口实例
    pub window: Box<Window>,
    /// 像素渲染器，用于窗口内容渲染
    pub pixels: Option<Pixels<'static>>,
    /// 窗口的物理像素尺寸
    pub size_px: PhysicalSize<u32>,
    /// 窗口的DPI缩放比例
    pub scale: f64,
    /// 窗口在虚拟桌面中的X坐标
    pub virtual_x: i32,
    /// 窗口在虚拟桌面中的Y坐标
    pub virtual_y: i32,
}

impl WindowInfo {
    /// 创建新的窗口信息实例
    ///
    /// # 参数
    /// * `window` - winit 窗口实例
    /// * `size_px` - 窗口的物理像素尺寸
    /// * `scale` - DPI缩放比例
    /// * `virtual_x` - 虚拟桌面X坐标
    /// * `virtual_y` - 虚拟桌面Y坐标
    pub fn new(
        window: Window,
        size_px: PhysicalSize<u32>,
        scale: f64,
        virtual_x: i32,
        virtual_y: i32,
    ) -> Self {
        Self {
            window: Box::new(window),
            pixels: None,
            size_px,
            scale,
            virtual_x,
            virtual_y,
        }
    }

    /// 更新窗口尺寸
    ///
    /// # 参数
    /// * `new_size` - 新的物理像素尺寸
    pub fn update_size(&mut self, new_size: PhysicalSize<u32>) {
        self.size_px = new_size;
        // 重新创建 Pixels 实例需要在外部处理，因为需要窗口引用
        self.pixels = None;
    }

    /// 更新缩放比例
    ///
    /// # 参数
    /// * `new_scale` - 新的DPI缩放比例
    pub fn update_scale(&mut self, new_scale: f64) {
        self.scale = new_scale;
    }
}

/// 窗口管理器
///
/// 提供多窗口的统一管理功能
pub struct WindowManager {
    /// 所有窗口的信息列表
    pub windows: Vec<WindowInfo>,
}

impl WindowManager {
    /// 创建新的窗口管理器
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    /// 根据窗口ID查找窗口索引
    ///
    /// # 参数
    /// * `window_id` - winit 窗口ID
    ///
    /// # 返回值
    /// 窗口在列表中的索引，如果未找到返回 None
    pub fn find_window_index(&self, window_id: winit::window::WindowId) -> Option<usize> {
        self.windows.iter().position(|w| w.window.id() == window_id)
    }

    /// 检查是否有窗口
    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    /// 为所有显示器创建并初始化窗口
    ///
    /// # 参数
    /// * `event_loop` - winit 事件循环
    /// * `attrs` - 窗口属性
    pub fn initialize_windows(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        attrs: &winit::window::WindowAttributes,
    ) {
        if self.has_windows() {
            return; // 已经初始化过了
        }

        // 检测所有显示器并为每个创建窗口
        let monitors: Vec<_> = event_loop.available_monitors().collect();

        for monitor in monitors {
            let physical_size = monitor.size();
            let scale = monitor.scale_factor();
            let position = monitor.position();
            let attrs = attrs.clone();

            match event_loop.create_window(attrs) {
                Ok(window) => {
                    // 设置窗口位置和大小到显示器
                    let _ = window.request_inner_size(physical_size);
                    window.set_outer_position(winit::dpi::Position::Physical(
                        winit::dpi::PhysicalPosition::new(position.x, position.y),
                    ));

                    let info =
                        WindowInfo::new(window, physical_size, scale, position.x, position.y);
                    self.windows.push(info);
                }
                Err(e) => {
                    eprintln!("无法为显示器创建窗口：{}", e);
                }
            }
        }
    }

    /// 为所有窗口请求重绘
    pub fn request_redraw_all(&self) {
        for window_info in &self.windows {
            window_info.window.request_redraw();
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
