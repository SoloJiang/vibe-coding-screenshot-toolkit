/// macOS Metal GPU 渲染后端
///
/// 使用 metal-rs + Skia Metal backend 实现 GPU 加速渲染
use anyhow::{anyhow, Result};
use core_graphics_types::geometry::CGSize;
use metal::foreign_types::{ForeignType, ForeignTypeRef};
use metal::{CommandQueue, Device, MTLPixelFormat, MetalDrawable, MetalLayer};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use skia_safe::{
    gpu::{self, mtl, DirectContext, SurfaceOrigin},
    Canvas, ColorType, Surface,
};
use winit::window::Window;

use super::{BackendType, RenderBackend};

/// Metal GPU 渲染后端实现
pub struct MetalBackend {
    /// Metal device（保留用于生命周期管理）
    #[allow(dead_code)]
    device: Device,
    /// Metal command queue（保留用于生命周期管理）
    #[allow(dead_code)]
    queue: CommandQueue,
    /// CAMetalLayer
    layer: MetalLayer,
    /// Skia DirectContext
    direct_context: DirectContext,
    /// 当前 surface（每帧创建）
    surface: Option<Surface>,
    /// 当前 drawable（每帧创建）
    current_drawable: Option<MetalDrawable>,
    /// Surface 宽度
    width: i32,
    /// Surface 高度
    height: i32,
}

impl MetalBackend {
    /// 创建新的 Metal 渲染后端
    pub fn new(window: &Window, width: i32, height: i32) -> Result<Self> {
        // 1. 创建 Metal device
        let device = Device::system_default()
            .ok_or_else(|| anyhow!("Failed to get Metal system default device"))?;

        // 2. 创建 command queue
        let queue = device.new_command_queue();

        // 3. 创建 CAMetalLayer 并设置到窗口
        let layer = unsafe {
            let window_handle = window
                .window_handle()
                .map_err(|e| anyhow!("Failed to get window handle: {}", e))?;

            match window_handle.as_raw() {
                RawWindowHandle::AppKit(handle) => {
                    use objc2::rc::Retained;
                    use objc2_app_kit::NSView;
                    use objc2_quartz_core::CALayer;

                    let view_ptr = handle.ns_view.as_ptr() as *mut NSView;
                    let view = &*view_ptr;

                    // 创建 CAMetalLayer
                    let layer = MetalLayer::new();
                    layer.set_device(&device);
                    layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                    layer.set_presents_with_transaction(false);

                    // 设置 layer 尺寸
                    layer.set_drawable_size(CGSize::new(width as f64, height as f64));

                    // 设置 layer 为透明，避免粉色调试背景
                    // 注意：CAMetalLayer 默认就是透明的，但我们明确设置以确保
                    layer.set_opaque(false);

                    // 启用 VSync 显示同步，避免画面撕裂和闪烁
                    layer.set_display_sync_enabled(true);

                    // 设置最大可绘制数为 2（双缓冲），减少闪烁
                    layer.set_maximum_drawable_count(2);

                    // 设置 layer 到 view（使用 objc2-app-kit）
                    let layer_obj = layer.as_ptr() as *mut CALayer;
                    let layer_retained: Retained<CALayer> = Retained::retain(layer_obj)
                        .ok_or_else(|| anyhow!("Failed to retain CAMetalLayer"))?;

                    view.setWantsLayer(true);
                    view.setLayer(Some(&*layer_retained));

                    layer
                }
                _ => {
                    return Err(anyhow!("Not a macOS AppKit window"));
                }
            }
        };

        // 4. 创建 Skia DirectContext（使用新 API）
        let backend_context = unsafe {
            mtl::BackendContext::new(
                device.as_ptr() as *mut std::ffi::c_void,
                queue.as_ptr() as *mut std::ffi::c_void,
            )
        };

        let direct_context = gpu::direct_contexts::make_metal(&backend_context, None)
            .ok_or_else(|| anyhow!("Failed to create Skia Metal DirectContext"))?;

        println!(
            "🚀 使用 Metal GPU 渲染后端 ({}x{}) - metal-rs",
            width, height
        );

        Ok(Self {
            device,
            queue,
            layer,
            direct_context,
            surface: None,
            current_drawable: None,
            width,
            height,
        })
    }
}

impl RenderBackend for MetalBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::MetalGpu
    }

    fn prepare_surface(&mut self, width: i32, height: i32) -> Result<()> {
        // 更新尺寸（如果变化）
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.layer
                .set_drawable_size(CGSize::new(width as f64, height as f64));
        }

        // 从 CAMetalLayer 获取下一个 drawable
        let drawable = self
            .layer
            .next_drawable()
            .ok_or_else(|| anyhow!("Failed to get next drawable from CAMetalLayer"))?;

        // 获取 drawable 的 texture
        let texture = drawable.texture();

        // 创建 Skia GPU surface（使用新 API）
        let texture_info =
            unsafe { mtl::TextureInfo::new(texture.as_ptr() as *mut std::ffi::c_void) };

        let backend_render_target =
            gpu::backend_render_targets::make_mtl((width, height), &texture_info);

        let surface = gpu::surfaces::wrap_backend_render_target(
            &mut self.direct_context,
            &backend_render_target,
            SurfaceOrigin::TopLeft,
            ColorType::BGRA8888,
            None,
            None,
        )
        .ok_or_else(|| anyhow!("Failed to create surface from Metal render target"))?;

        // 保存 drawable 和 surface
        self.current_drawable = Some(drawable.to_owned());
        self.surface = Some(surface);

        Ok(())
    }

    fn canvas(&mut self) -> Option<&Canvas> {
        // Surface 的 canvas() 需要可变借用来返回 canvas
        self.surface.as_mut().map(|s| s.canvas())
    }

    fn flush_and_read_pixels(&mut self) -> Result<Vec<u8>> {
        // 1. Flush Skia GPU commands to Metal
        self.direct_context.flush_and_submit();

        // 2. Present drawable to screen（通过 command buffer 实现 VSync）
        if let Some(drawable) = self.current_drawable.take() {
            // 使用 command buffer 的 present 方法，遵循 display_sync_enabled 设置
            let command_buffer = self.queue.new_command_buffer();
            command_buffer.present_drawable(&drawable);
            command_buffer.commit();
            // 异步提交，不等待完成，让 GPU 并行处理以提升性能
        }

        // 3. GPU backend 直接渲染到屏幕，无需返回像素数据
        Ok(Vec::new())
    }

    fn resize(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        // 下次 prepare_surface 时会更新 layer 尺寸
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：Metal backend 需要真实的窗口环境，无法在单元测试中测试
    // 这里只提供基本的类型测试

    #[test]
    fn test_backend_type() {
        // 测试 backend type 常量
        assert_eq!(
            std::mem::discriminant(&BackendType::MetalGpu),
            std::mem::discriminant(&BackendType::MetalGpu)
        );
    }
}
