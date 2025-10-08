/// CPU 软件渲染后端
///
/// 使用 Skia 的 CPU raster surface 进行软件渲染
/// 优点：兼容性最好，所有平台都支持
/// 缺点：性能较低，需要 CPU 读取像素并通过 softbuffer 呈现
use anyhow::{anyhow, Result};
use skia_safe::{AlphaType, Canvas, Color, ColorType, ImageInfo, Surface};

use super::{BackendType, RenderBackend};

/// CPU 软件渲染后端实现
pub struct CpuRasterBackend {
    /// Skia CPU 渲染 surface
    surface: Surface,
    /// Surface 宽度
    width: i32,
    /// Surface 高度
    height: i32,
}

impl CpuRasterBackend {
    /// 创建新的 CPU 渲染后端
    pub fn new(width: i32, height: i32) -> Result<Self> {
        let surface = Self::create_surface(width, height)?;

        println!("🖥️  使用 CPU 软件渲染后端 ({}x{})", width, height);

        Ok(Self {
            surface,
            width,
            height,
        })
    }

    /// 创建 Skia CPU raster surface
    fn create_surface(width: i32, height: i32) -> Result<Surface> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))
            .ok_or_else(|| anyhow!("Failed to create CPU raster surface"))?;

        // 清空 surface
        surface.canvas().clear(Color::TRANSPARENT);

        Ok(surface)
    }

    /// 读取 surface 像素数据
    fn read_pixels(&mut self) -> Result<Vec<u8>> {
        let width = self.width;
        let height = self.height;

        // 创建像素缓冲区
        let pixel_count = (width * height) as usize;
        let mut pixels = vec![0u8; pixel_count * 4];

        // 定义目标图像格式（RGBA8888, Unpremul）
        let image_info = ImageInfo::new(
            (width, height),
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );

        // 从 surface 读取像素
        let row_bytes = (width * 4) as usize;
        if self
            .surface
            .read_pixels(&image_info, pixels.as_mut_slice(), row_bytes, (0, 0))
        {
            Ok(pixels)
        } else {
            Err(anyhow!("Failed to read pixels from CPU surface"))
        }
    }
}

impl RenderBackend for CpuRasterBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::CpuRaster
    }

    fn prepare_surface(&mut self, width: i32, height: i32) -> Result<()> {
        // 如果尺寸未变化，复用现有 surface
        if width == self.width && height == self.height {
            return Ok(());
        }

        // 尺寸变化，重新创建 surface
        self.surface = Self::create_surface(width, height)?;
        self.width = width;
        self.height = height;

        Ok(())
    }

    fn canvas(&mut self) -> Option<&Canvas> {
        // 获取 surface 的 canvas（内部可变性）
        Some(self.surface.canvas())
    }

    fn flush_and_read_pixels(&mut self) -> Result<Vec<u8>> {
        // CPU 渲染需要读取像素数据
        self.read_pixels()
    }

    fn resize(&mut self, width: i32, height: i32) {
        // 标记需要重建 surface
        self.width = width;
        self.height = height;
        // 实际重建在下次 prepare_surface 时进行
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_backend_creation() {
        let backend = CpuRasterBackend::new(800, 600);
        assert!(backend.is_ok());

        let backend = backend.unwrap();
        assert_eq!(backend.backend_type(), BackendType::CpuRaster);
        assert_eq!(backend.width, 800);
        assert_eq!(backend.height, 600);
    }

    #[test]
    fn test_cpu_backend_resize() {
        let mut backend = CpuRasterBackend::new(800, 600).unwrap();

        backend.resize(1024, 768);
        backend.prepare_surface(1024, 768).unwrap();

        assert_eq!(backend.width, 1024);
        assert_eq!(backend.height, 768);
    }

    #[test]
    fn test_cpu_backend_render() {
        let mut backend = CpuRasterBackend::new(100, 100).unwrap();

        // 获取 canvas 并绘制
        if let Some(canvas) = backend.canvas() {
            canvas.clear(Color::from_argb(255, 255, 0, 0)); // 红色背景
        }

        // 读取像素
        let pixels = backend.flush_and_read_pixels();
        assert!(pixels.is_ok());

        let pixels = pixels.unwrap();
        assert_eq!(pixels.len(), 100 * 100 * 4);

        // 检查第一个像素是否为红色（RGBA）
        assert_eq!(pixels[0], 255); // R
        assert_eq!(pixels[1], 0); // G
        assert_eq!(pixels[2], 0); // B
        assert_eq!(pixels[3], 255); // A
    }
}
