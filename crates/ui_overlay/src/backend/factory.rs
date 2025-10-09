use super::{CpuRasterBackend, RenderBackend};

#[cfg(target_os = "macos")]
use super::MetalBackend;

#[cfg(test)]
use super::BackendType;

#[cfg(test)]
use anyhow::Result;

/// 创建最佳可用的渲染后端
///
/// 尝试顺序：
/// 1. macOS: Metal GPU > CPU Raster
/// 2. Windows: Direct3D GPU > CPU Raster (Phase 3)
/// 3. 其他平台: CPU Raster
pub fn create_backend(
    #[allow(unused_variables)] window: Option<&winit::window::Window>,
    width: i32,
    height: i32,
) -> Box<dyn RenderBackend> {
    // macOS: 尝试 Metal GPU
    #[cfg(target_os = "macos")]
    {
        if let Some(win) = window {
            if let Ok(metal_backend) = MetalBackend::new(win, width, height) {
                println!("✅ 使用 Metal GPU 渲染后端");
                return Box::new(metal_backend);
            } else {
                println!("⚠️  Metal GPU 初始化失败，降级到 CPU 渲染");
            }
        }
    }

    // Windows: 尝试 Direct3D GPU (Phase 3)
    #[cfg(target_os = "windows")]
    {
        // Phase 3: 实现 D3D backend
        // if let Some(win) = window {
        //     if let Ok(d3d_backend) = D3DBackend::new(win, width, height) {
        //         println!("✅ 使用 Direct3D GPU 渲染后端");
        //         return Box::new(d3d_backend);
        //     }
        // }
        println!("⚠️  Direct3D 后端未实现（Phase 3），使用 CPU 渲染");
    }

    // 降级到 CPU 渲染
    match CpuRasterBackend::new(width, height) {
        Ok(cpu_backend) => {
            println!("✅ 使用 CPU 软件渲染后端");
            Box::new(cpu_backend)
        }
        Err(e) => {
            panic!("Failed to create CPU raster backend: {}", e);
        }
    }
}

/// 创建指定类型的渲染后端（用于测试）
#[cfg(test)]
pub fn create_backend_with_type(
    backend_type: BackendType,
    #[allow(unused_variables)] window: Option<&winit::window::Window>,
    width: i32,
    height: i32,
) -> Result<Box<dyn RenderBackend>> {
    match backend_type {
        BackendType::CpuRaster => {
            let backend = CpuRasterBackend::new(width, height)?;
            Ok(Box::new(backend))
        }

        #[cfg(target_os = "macos")]
        BackendType::MetalGpu => {
            if let Some(win) = window {
                let backend = MetalBackend::new(win, width, height)?;
                Ok(Box::new(backend))
            } else {
                Err(anyhow::anyhow!("Window required for Metal backend"))
            }
        }

        #[cfg(not(target_os = "macos"))]
        BackendType::MetalGpu => Err(anyhow::anyhow!("Metal backend only available on macOS")),

        BackendType::Direct3dGpu => {
            // Phase 3: 实现 D3D backend
            Err(anyhow::anyhow!(
                "Direct3D backend not implemented yet (Phase 3)"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cpu_backend() {
        let backend = create_backend(None, 800, 600);
        assert_eq!(backend.backend_type(), BackendType::CpuRaster);
    }

    #[test]
    fn test_create_backend_with_type() {
        let backend = create_backend_with_type(BackendType::CpuRaster, None, 800, 600);
        assert!(backend.is_ok());
        assert_eq!(backend.unwrap().backend_type(), BackendType::CpuRaster);
    }
}
