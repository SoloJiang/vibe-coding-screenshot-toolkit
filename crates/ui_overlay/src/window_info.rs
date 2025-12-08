/// 窗口信息模块
///
/// 提供单个窗口的信息管理和 egui 渲染功能
use egui_wgpu::wgpu;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// WGPU 渲染状态
pub struct WgpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub renderer: egui_wgpu::Renderer,
    /// 设备支持的最大纹理尺寸（用于调试和降级）
    #[allow(dead_code)]
    pub max_texture_size: u32,
    pub background_renderer: crate::background_renderer::BackgroundRenderer,
    pub background_texture: Option<wgpu::Texture>,
    pub background_bind_group: Option<wgpu::BindGroup>,
}

/// 窗口信息结构体
pub struct WindowInfo {
    pub window: Arc<Window>,
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub wgpu_state: Option<WgpuState>,
    pub size_px: PhysicalSize<u32>,
    pub scale: f64,
    pub virtual_x: i32,
    pub virtual_y: i32,
    /// 创建时的正确物理尺寸（用于检测和恢复 winit 的错误报告）
    pub initial_size_px: PhysicalSize<u32>,
    /// 创建时的正确 scale（用于检测和恢复 winit 的错误报告）
    pub initial_scale: f64,
}

impl WindowInfo {
    pub fn new(
        window: Window,
        size_px: PhysicalSize<u32>,
        scale: f64,
        virtual_x: i32,
        virtual_y: i32,
    ) -> Self {
        let window = Arc::new(window);

        // 创建 egui context
        let egui_ctx = egui::Context::default();

        // 创建 egui_winit state
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(scale as f32),
            None, // 使用默认像素每点
            None,
        );

        Self {
            window,
            egui_ctx,
            egui_state,
            wgpu_state: None,
            size_px,
            scale,
            virtual_x,
            virtual_y,
            initial_size_px: size_px, // 保存创建时的正确值
            initial_scale: scale,     // 保存创建时的正确值
        }
    }

    pub fn update_size(&mut self, new_size: PhysicalSize<u32>) {
        if self.size_px != new_size {
            tracing::warn!(
                "⚠️  WindowInfo size 变化: {}x{} -> {}x{} (可能导致背景不匹配)",
                self.size_px.width,
                self.size_px.height,
                new_size.width,
                new_size.height
            );
        }
        self.size_px = new_size;

        // 更新 wgpu surface 配置
        if let Some(wgpu_state) = &mut self.wgpu_state {
            wgpu_state.surface_config.width = new_size.width.max(1);
            wgpu_state.surface_config.height = new_size.height.max(1);
            wgpu_state
                .surface
                .configure(&wgpu_state.device, &wgpu_state.surface_config);
        }
    }

    pub fn update_scale(&mut self, new_scale: f64) {
        self.scale = new_scale;
    }

    /// 检查是否已上传背景纹理
    pub fn has_background_texture(&self) -> bool {
        self.wgpu_state
            .as_ref()
            .and_then(|s| s.background_bind_group.as_ref())
            .is_some()
    }

    /// 上传背景纹理
    ///
    /// 从完整虚拟桌面背景中裁剪出当前窗口对应的区域并上传
    ///
    /// # Arguments
    /// * `full_bg_data` - 完整虚拟桌面背景图数据（RGBA8）
    /// * `full_bg_width` - 完整背景图宽度
    /// * `full_bg_height` - 完整背景图高度
    /// * `virtual_min_x` - 虚拟桌面的最小X坐标（用于坐标转换）
    /// * `virtual_min_y` - 虚拟桌面的最小Y坐标（用于坐标转换）
    pub fn upload_background_texture(
        &mut self,
        full_bg_data: &[u8],
        full_bg_width: u32,
        full_bg_height: u32,
        virtual_min_x: i32,
        virtual_min_y: i32,
    ) {
        let wgpu_state = match &mut self.wgpu_state {
            Some(state) => state,
            None => {
                tracing::error!("无法上传背景：WGPU 未初始化");
                return;
            }
        };

        // 重要：self.virtual_x/y 来自 winit 的 monitor.position()
        // 根据实际测试，monitor.position() 返回的是物理坐标（即使类型名为 PhysicalPosition）
        // 完整背景画布也使用物理坐标系统，因此直接使用，不需要转换
        //
        // 证据：
        // - 窗口1: monitor.scale=1, position=(-1080, -351), window.scale=2
        // - 如果 position 是逻辑坐标，用 monitor.scale=1 转换：(-1080, -351)*1=(-1080, -351)
        // - 如果 position 已是物理坐标：(-1080, -351)
        // - 背景画布使用物理坐标构建，所以应该直接使用 position
        let phys_x = self.virtual_x;
        let phys_y = self.virtual_y;

        tracing::debug!(
            "📍 窗口坐标: virtual=({}, {}), 直接作为物理坐标使用, scale={}",
            phys_x,
            phys_y,
            self.scale
        );

        // 计算窗口在完整背景图中的位置（canvas 坐标）
        let canvas_x = (phys_x - virtual_min_x).max(0) as u32;
        let canvas_y = (phys_y - virtual_min_y).max(0) as u32;
        let window_w = self.size_px.width;
        let window_h = self.size_px.height;

        tracing::info!(
            "🎨 为窗口裁剪背景: 完整背景={}x{} (物理画布)",
            full_bg_width,
            full_bg_height
        );
        tracing::info!(
            "   └─ 窗口物理坐标 (直接使用 monitor.position): ({}, {}), scale={}",
            phys_x,
            phys_y,
            self.scale
        );
        tracing::info!("   └─ 窗口物理尺寸={}x{}", window_w, window_h);
        tracing::info!(
            "   └─ 物理最小值=({}, {}) -> canvas坐标=({}, {})",
            virtual_min_x,
            virtual_min_y,
            canvas_x,
            canvas_y
        );

        // 裁剪出窗口对应的背景区域
        let cropped_data = Self::crop_background(
            full_bg_data,
            full_bg_width,
            full_bg_height,
            canvas_x,
            canvas_y,
            window_w,
            window_h,
        );

        tracing::info!(
            "   └─ 裁剪后纹理: {}x{} ({:.2}MB)",
            window_w,
            window_h,
            cropped_data.len() as f64 / 1024.0 / 1024.0
        );

        // 创建纹理并直接全屏显示
        let (texture, bind_group) = wgpu_state.background_renderer.create_texture(
            &wgpu_state.device,
            &wgpu_state.queue,
            &cropped_data,
            window_w,
            window_h,
        );

        wgpu_state.background_texture = Some(texture);
        wgpu_state.background_bind_group = Some(bind_group);

        tracing::info!("✅ 背景纹理上传成功");
    }

    /// 从完整背景图中裁剪出指定区域
    fn crop_background(
        src_data: &[u8],
        src_width: u32,
        src_height: u32,
        crop_x: u32,
        crop_y: u32,
        crop_width: u32,
        crop_height: u32,
    ) -> Vec<u8> {
        let mut cropped = vec![0u8; (crop_width * crop_height * 4) as usize];

        for row in 0..crop_height {
            let src_y = crop_y + row;
            if src_y >= src_height {
                break;
            }

            let src_row_start = ((src_y * src_width + crop_x) * 4) as usize;
            let src_row_end = src_row_start + (crop_width * 4) as usize;
            let dst_row_start = (row * crop_width * 4) as usize;
            let dst_row_end = dst_row_start + (crop_width * 4) as usize;

            if src_row_end <= src_data.len() && dst_row_end <= cropped.len() {
                cropped[dst_row_start..dst_row_end]
                    .copy_from_slice(&src_data[src_row_start..src_row_end]);
            }
        }

        cropped
    }

    /// 初始化 WGPU 渲染器
    pub fn init_wgpu(&mut self) -> Result<(), String> {
        if self.wgpu_state.is_some() {
            return Ok(());
        }

        // 创建 wgpu 实例
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // 创建 surface - 使用 Arc<Window> 获得 'static 生命周期
        let surface = instance
            .create_surface(self.window.clone())
            .map_err(|e| format!("Failed to create surface: {}", e))?;

        // 请求 adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("Failed to find suitable adapter: {}", e))?;

        // 请求 device 和 queue
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("egui_device"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            experimental_features: Default::default(),
            trace: Default::default(),
        }))
        .map_err(|e| format!("Failed to create device: {}", e))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // 配置 surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.size_px.width.max(1),
            height: self.size_px.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // 创建 egui_wgpu 渲染器
        let renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        // 获取 GPU 的最大纹理尺寸限制
        let max_texture_size = device.limits().max_texture_dimension_2d;

        // 创建背景渲染器
        let background_renderer =
            crate::background_renderer::BackgroundRenderer::new(&device, surface_format);

        self.wgpu_state = Some(WgpuState {
            surface,
            device,
            queue,
            surface_config,
            renderer,
            max_texture_size,
            background_renderer,
            background_texture: None,
            background_bind_group: None,
        });

        tracing::info!(
            "🚀 WGPU 渲染器初始化成功 ({}x{}, format: {:?}, max_texture: {})",
            self.size_px.width,
            self.size_px.height,
            surface_format,
            max_texture_size
        );

        Ok(())
    }

    /// 渲染一帧
    ///
    /// # Arguments
    /// * `draw_fn` - 绘制函数，接收 egui Context 用于 UI 绘制
    pub fn render<F>(&mut self, draw_fn: F) -> Result<(), String>
    where
        F: FnMut(&egui::Context),
    {
        let wgpu_state = self.wgpu_state.as_mut().ok_or("WGPU not initialized")?;

        // 1. 收集 egui input
        let raw_input = self.egui_state.take_egui_input(&self.window);

        // 2. 运行 egui 逻辑
        let full_output = self.egui_ctx.run(raw_input, draw_fn);

        // 3. 处理 platform output
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // 4. 获取下一帧的 surface texture
        let surface_texture = wgpu_state
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {}", e))?;

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 5. 创建 command encoder
        let mut encoder =
            wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_encoder"),
                });

        // 6. 准备 egui 渲染数据
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size_px.width, self.size_px.height],
            pixels_per_point: self.scale as f32,
        };

        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // 7. 更新 egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            wgpu_state.renderer.update_texture(
                &wgpu_state.device,
                &wgpu_state.queue,
                *id,
                image_delta,
            );
        }

        // 8. 更新 buffers
        wgpu_state.renderer.update_buffers(
            &wgpu_state.device,
            &wgpu_state.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        // 9. 渲染背景（如果有）
        if let Some(ref bg_bind_group) = &wgpu_state.background_bind_group {
            tracing::trace!("渲染 wgpu 背景层");
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("background_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            wgpu_state
                .background_renderer
                .render(&mut render_pass, bg_bind_group);
        } else {
            tracing::trace!("背景 bind_group 不存在，跳过背景渲染");
        } // render_pass 在这里被 drop

        // 10. 渲染 egui（UI 层，覆盖在背景之上）
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // 保留背景，不清除
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 使用 forget_lifetime() 来满足 egui_wgpu::Renderer::render() 的生命周期要求
            wgpu_state.renderer.render(
                &mut render_pass.forget_lifetime(),
                &clipped_primitives,
                &screen_descriptor,
            );
        } // render_pass 在这里被 drop

        // 11. 提交 command buffer
        wgpu_state.queue.submit(std::iter::once(encoder.finish()));

        // 12. Present
        surface_texture.present();

        // 13. 释放不再使用的 textures
        for id in &full_output.textures_delta.free {
            wgpu_state.renderer.free_texture(id);
        }

        Ok(())
    }
}
