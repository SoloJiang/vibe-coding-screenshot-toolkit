/// 渲染后端抽象层
///
/// 提供统一的渲染接口，支持多种后端实现：
/// - CPU Raster: 软件渲染（兼容性最好）
/// - Metal GPU: macOS 硬件加速
/// - Direct3D GPU: Windows 硬件加速
use anyhow::Result;
use skia_safe::Canvas;

/// 渲染后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// CPU 软件渲染
    CpuRaster,
    /// macOS Metal GPU
    MetalGpu,
    /// Windows Direct3D GPU
    Direct3dGpu,
}

/// 跨平台渲染后端抽象
///
/// 所有渲染后端必须实现此 trait，提供统一的渲染接口
pub trait RenderBackend {
    /// 获取后端类型
    fn backend_type(&self) -> BackendType;

    /// 准备渲染 surface
    ///
    /// 对于 GPU 后端，可能每帧创建新的 surface（从 drawable）
    /// 对于 CPU 后端，surface 可以复用
    fn prepare_surface(&mut self, width: i32, height: i32) -> Result<()>;

    /// 获取 canvas 用于绘制
    ///
    /// 注意：Canvas 使用内部可变性，即使是不可变引用也可以进行绘制
    fn canvas(&mut self) -> Option<&Canvas>;

    /// 提交渲染并获取像素数据（如需要）
    ///
    /// - GPU 后端：直接 flush 到屏幕，返回空 Vec
    /// - CPU 后端：读取像素数据，返回 RGBA 字节数组
    fn flush_and_read_pixels(&mut self) -> Result<Vec<u8>>;

    /// 处理窗口大小变化
    fn resize(&mut self, width: i32, height: i32);
}

// 导出各平台实现
mod cpu_backend;
pub use cpu_backend::CpuRasterBackend;

#[cfg(target_os = "macos")]
mod metal_backend;
#[cfg(target_os = "macos")]
pub use metal_backend::MetalBackend;

// 导出工厂函数
mod factory;
pub use factory::create_backend;
