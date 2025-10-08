/// 窗口信息模块
///
/// 提供单个窗口的信息管理和渲染功能
use crate::backend::{create_backend, BackendType, RenderBackend};
use softbuffer::{Context as SoftbufferContext, Surface as SoftbufferSurface};
use std::rc::Rc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// 窗口信息结构体
pub struct WindowInfo {
    pub window: Rc<Window>,
    pub render_backend: Option<Box<dyn RenderBackend>>,
    pub softbuffer_context: Option<SoftbufferContext<Rc<Window>>>,
    pub softbuffer_surface: Option<SoftbufferSurface<Rc<Window>, Rc<Window>>>,
    pub size_px: PhysicalSize<u32>,
    pub scale: f64,
    pub virtual_x: i32,
    pub virtual_y: i32,
}

impl WindowInfo {
    pub fn new(
        window: Window,
        size_px: PhysicalSize<u32>,
        scale: f64,
        virtual_x: i32,
        virtual_y: i32,
    ) -> Self {
        Self {
            window: Rc::new(window),
            render_backend: None,
            softbuffer_context: None,
            softbuffer_surface: None,
            size_px,
            scale,
            virtual_x,
            virtual_y,
        }
    }

    pub fn update_size(&mut self, new_size: PhysicalSize<u32>) {
        self.size_px = new_size;
    }

    pub fn update_scale(&mut self, new_scale: f64) {
        self.scale = new_scale;
    }

    pub fn init_softbuffer(&mut self) -> Result<(), String> {
        if self.softbuffer_context.is_some() {
            return Ok(());
        }

        let context = SoftbufferContext::new(self.window.clone())
            .map_err(|e| format!("Failed to create softbuffer context: {e}"))?;

        let surface = SoftbufferSurface::new(&context, self.window.clone())
            .map_err(|e| format!("Failed to create softbuffer surface: {e}"))?;

        self.softbuffer_context = Some(context);
        self.softbuffer_surface = Some(surface);

        Ok(())
    }

    pub fn present_pixels(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        use std::num::NonZeroU32;

        self.init_softbuffer()?;

        let surface = self.softbuffer_surface.as_mut().unwrap();

        if width == 0 || height == 0 {
            return Ok(());
        }

        surface
            .resize(
                NonZeroU32::new(width).ok_or_else(|| "Invalid surface width".to_string())?,
                NonZeroU32::new(height).ok_or_else(|| "Invalid surface height".to_string())?,
            )
            .map_err(|e| format!("Failed to resize softbuffer surface: {e}"))?;

        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| format!("Failed to lock softbuffer buffer: {e}"))?;

        // 转换 RGBA -> ARGB
        for (slot, chunk) in buffer.iter_mut().zip(pixels.chunks_exact(4)) {
            let r = chunk[0] as u32;
            let g = chunk[1] as u32;
            let b = chunk[2] as u32;
            let a = chunk[3] as u32;
            *slot = (a << 24) | (r << 16) | (g << 8) | b;
        }

        buffer
            .present()
            .map_err(|e| format!("Failed to present softbuffer frame: {e}"))?;

        Ok(())
    }

    /// 初始化渲染后端
    pub fn init_backend(&mut self) -> Result<(), String> {
        if self.render_backend.is_some() {
            return Ok(());
        }

        // 使用工厂函数创建最佳可用的 backend
        let backend = create_backend(
            Some(&self.window),
            self.size_px.width as i32,
            self.size_px.height as i32,
        );

        // 如果是 CPU backend，初始化 softbuffer
        if backend.backend_type() == BackendType::CpuRaster {
            self.init_softbuffer()?;
        }

        self.render_backend = Some(backend);
        Ok(())
    }

    /// 渲染一帧
    pub fn render<F>(&mut self, draw_fn: F) -> Result<(), String>
    where
        F: FnOnce(&skia_safe::Canvas),
    {
        let backend = self
            .render_backend
            .as_mut()
            .ok_or("Render backend not initialized")?;

        let backend_type = backend.backend_type();

        // 1. 准备 surface
        backend
            .prepare_surface(self.size_px.width as i32, self.size_px.height as i32)
            .map_err(|e| e.to_string())?;

        // 2. 获取 canvas 并绘制
        if let Some(canvas) = backend.canvas() {
            draw_fn(canvas);
        }

        // 3. Flush 并获取像素数据
        let pixels = backend.flush_and_read_pixels().map_err(|e| e.to_string())?;

        // 4. 根据后端类型决定如何呈现
        match backend_type {
            BackendType::CpuRaster => {
                // CPU backend: 通过 softbuffer 呈现
                if !pixels.is_empty() {
                    self.present_pixels(&pixels, self.size_px.width, self.size_px.height)?;
                }
            }
            BackendType::MetalGpu | BackendType::Direct3dGpu => {
                // GPU backend: 已经直接渲染到窗口，无需额外操作
                // flush_and_read_pixels 已经完成了渲染和呈现
            }
        }

        Ok(())
    }
}
