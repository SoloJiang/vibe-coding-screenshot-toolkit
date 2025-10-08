use skia_safe::image::CachingHint;
use skia_safe::{Color, Surface};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::metal::MetalRenderContext;
use super::opengl::OpenGLRenderContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Metal,
    GL,
    Cpu,
}

/// GPU 渲染上下文的具体实现
enum GpuContext {
    Metal(MetalRenderContext),
    OpenGL(OpenGLRenderContext),
    Cpu { surface: Surface },
}

pub struct SkiaRenderContext {
    pub size_px: PhysicalSize<u32>,
    pub virtual_x: i32,
    pub virtual_y: i32,
    pub virtual_bounds: Option<(i32, i32, u32, u32)>,
    gpu_context: GpuContext,
}

impl SkiaRenderContext {
    pub fn new(
        window: &Window,
        size_px: PhysicalSize<u32>,
        virtual_x: i32,
        virtual_y: i32,
        virtual_bounds: Option<(i32, i32, u32, u32)>,
    ) -> Result<Self, String> {
        let backend = Self::select_backend(window);
        let gpu_context = Self::create_gpu_context(window, size_px, virtual_x, virtual_y, virtual_bounds, &backend)?;

        Ok(Self {
            size_px,
            virtual_x,
            virtual_y,
            virtual_bounds,
            gpu_context,
        })
    }

    fn select_backend(_window: &Window) -> Backend {
        // 智能选择最佳可用的后端
        #[cfg(target_os = "macos")]
        {
            // macOS 优先尝试 Metal
            return Backend::Metal;
        }

        #[cfg(not(target_os = "macos"))]
        {
            // 其他平台尝试 OpenGL
            return Backend::GL;
        }
    }

    fn create_gpu_context(
        window: &Window,
        size_px: PhysicalSize<u32>,
        virtual_x: i32,
        virtual_y: i32,
        virtual_bounds: Option<(i32, i32, u32, u32)>,
        backend: &Backend,
    ) -> Result<GpuContext, String> {
        match backend {
            Backend::Metal => {
                // 尝试创建 Metal 上下文，失败时降级到 CPU
                match MetalRenderContext::new(window, size_px, virtual_x, virtual_y, virtual_bounds) {
                    Ok(metal_ctx) => {
                        println!("✅ Successfully initialized Metal GPU backend");
                        Ok(GpuContext::Metal(metal_ctx))
                    }
                    Err(e) => {
                        println!("⚠️  Metal backend failed: {}, falling back to CPU", e);
                        Self::create_cpu_context(size_px)
                    }
                }
            }
            Backend::GL => {
                // 尝试创建 OpenGL 上下文，失败时降级到 CPU
                match OpenGLRenderContext::new(window, size_px, virtual_x, virtual_y, virtual_bounds) {
                    Ok(gl_ctx) => {
                        println!("✅ Successfully initialized OpenGL GPU backend");
                        Ok(GpuContext::OpenGL(gl_ctx))
                    }
                    Err(e) => {
                        println!("⚠️  OpenGL backend failed: {}, falling back to CPU", e);
                        Self::create_cpu_context(size_px)
                    }
                }
            }
            Backend::Cpu => Self::create_cpu_context(size_px),
        }
    }

    fn create_cpu_context(size_px: PhysicalSize<u32>) -> Result<GpuContext, String> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((size_px.width as i32, size_px.height as i32))
            .ok_or_else(|| "Failed to create raster surface".to_string())?;
        surface.canvas().clear(Color::TRANSPARENT);

        println!("📊 Using CPU backend for rendering");
        Ok(GpuContext::Cpu { surface })
    }

    pub fn resize(&mut self, window: &Window, new_size: PhysicalSize<u32>) -> Result<(), String> {
        self.size_px = new_size;

        match &mut self.gpu_context {
            GpuContext::Metal(ctx) => ctx.resize(window, new_size),
            GpuContext::OpenGL(ctx) => ctx.resize(window, new_size),
            GpuContext::Cpu { surface } => {
                *surface = skia_safe::surfaces::raster_n32_premul((new_size.width as i32, new_size.height as i32))
                    .ok_or_else(|| "Failed to recreate CPU surface".to_string())?;
                surface.canvas().clear(Color::TRANSPARENT);
                Ok(())
            }
        }
    }

    pub fn canvas(&mut self) -> &skia_safe::Canvas {
        match &mut self.gpu_context {
            GpuContext::Metal(ctx) => ctx.canvas(),
            GpuContext::OpenGL(ctx) => ctx.canvas(),
            GpuContext::Cpu { surface } => surface.canvas(),
        }
    }

    pub fn flush(&mut self) -> Result<(), String> {
        match &mut self.gpu_context {
            GpuContext::Metal(ctx) => ctx.flush(),
            GpuContext::OpenGL(ctx) => ctx.flush(),
            GpuContext::Cpu { .. } => {
                // CPU 渲染不需要 flush
                Ok(())
            }
        }
    }

    /// 获取 Skia Surface 的可变引用，用于外部呈现
    pub fn surface_mut(&mut self) -> &mut Surface {
        match &mut self.gpu_context {
            GpuContext::Metal(ctx) => ctx.surface_mut(),
            GpuContext::OpenGL(ctx) => ctx.surface_mut(),
            GpuContext::Cpu { surface } => surface,
        }
    }

    pub fn snapshot_pixels(&mut self) -> Option<Vec<u8>> {
        match &mut self.gpu_context {
            GpuContext::Metal(ctx) => ctx.snapshot_pixels(),
            GpuContext::OpenGL(ctx) => ctx.snapshot_pixels(),
            GpuContext::Cpu { surface } => {
                let image = surface.image_snapshot();
                let info = image.image_info();
                let mut pixels = vec![0u8; (info.width() * info.height() * 4) as usize];
                let row_bytes = info.min_row_bytes();
                if image.read_pixels(
                    &info,
                    pixels.as_mut_slice(),
                    row_bytes,
                    (0, 0),
                    CachingHint::Allow,
                ) {
                    Some(pixels)
                } else {
                    None
                }
            }
        }
    }

    /// 获取当前使用的后端类型（用于调试和监控）
    pub fn current_backend(&self) -> Backend {
        match &self.gpu_context {
            GpuContext::Metal(_) => Backend::Metal,
            GpuContext::OpenGL(_) => Backend::GL,
            GpuContext::Cpu { .. } => Backend::Cpu,
        }
    }

    /// 检查是否正在使用 GPU 加速
    pub fn is_gpu_accelerated(&self) -> bool {
        matches!(self.gpu_context, GpuContext::Metal(_) | GpuContext::OpenGL(_))
    }
}
