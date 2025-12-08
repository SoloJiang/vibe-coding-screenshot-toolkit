/// 工具栏窗口模块
///
/// 使用独立的透明窗口渲染工具栏 UI（egui + wgpu）
use super::{Toolbar, ToolbarRenderer};
use crate::toolbar::ui::ToolbarAction;
use std::sync::Arc;
use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowLevel};

/// 工具栏窗口
///
/// 独立的透明窗口，用于显示标注工具栏
pub struct ToolbarWindow {
    /// winit 窗口
    window: Arc<Window>,
    /// wgpu 渲染状态
    render_state: Option<ToolbarRenderState>,
    /// egui 上下文
    egui_ctx: egui::Context,
    /// egui-winit 状态
    egui_state: egui_winit::State,
    /// 工具栏数据
    toolbar: Toolbar,
    /// 工具栏渲染器
    toolbar_renderer: ToolbarRenderer,
    /// 是否已初始化
    initialized: bool,
}

/// wgpu 渲染状态
struct ToolbarRenderState {
    device: egui_wgpu::wgpu::Device,
    queue: egui_wgpu::wgpu::Queue,
    surface: egui_wgpu::wgpu::Surface<'static>,
    surface_config: egui_wgpu::wgpu::SurfaceConfiguration,
    renderer: egui_wgpu::Renderer,
}

impl ToolbarWindow {
    /// 创建新的工具栏窗口
    pub fn new(
        event_loop: &ActiveEventLoop,
        egui_ctx: egui::Context,
        toolbar: Toolbar,
    ) -> Result<Self, String> {
        // 创建透明窗口
        let window_attrs = WindowAttributes::default()
            .with_title("")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            .with_visible(false); // 初始不可见

        let window = event_loop
            .create_window(window_attrs)
            .map_err(|e| format!("Failed to create toolbar window: {}", e))?;

        let window = Arc::new(window);

        // 创建 egui-winit 状态
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        // 在内部创建 ToolbarRenderer
        let toolbar_renderer = ToolbarRenderer::new();

        Ok(Self {
            window,
            render_state: None,
            egui_ctx,
            egui_state,
            toolbar,
            toolbar_renderer,
            initialized: false,
        })
    }

    /// 初始化 wgpu 渲染
    pub fn initialize_renderer(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        let window = self.window.clone();

        // 使用 pollster 同步执行异步初始化
        let render_state = pollster::block_on(async { Self::create_render_state(window).await })?;

        self.render_state = Some(render_state);
        self.initialized = true;

        Ok(())
    }

    /// 创建 wgpu 渲染状态
    async fn create_render_state(window: Arc<Window>) -> Result<ToolbarRenderState, String> {
        use egui_wgpu::wgpu;

        // 创建 wgpu 实例
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // 创建 surface
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("Failed to create surface: {}", e))?;

        // 请求适配器
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find adapter: {:?}", e))?;

        // 请求设备和队列
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Toolbar Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        // 配置 surface
        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);

        // 选择支持的 alpha mode（优先 PostMultiplied，回退到 Opaque）
        let alpha_mode = if capabilities
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            wgpu::CompositeAlphaMode::PostMultiplied
        } else if capabilities
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            capabilities.alpha_modes[0]
        };

        tracing::info!(
            "Toolbar window alpha mode: {:?}, available: {:?}",
            alpha_mode,
            capabilities.alpha_modes
        );

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // 创建 egui-wgpu 渲染器
        let renderer = egui_wgpu::Renderer::new(
            &device,
            surface_config.format,
            egui_wgpu::RendererOptions::default(),
        );

        Ok(ToolbarRenderState {
            device,
            queue,
            surface,
            surface_config,
            renderer,
        })
    }

    /// 显示窗口
    pub fn show(&self) {
        tracing::debug!("Setting toolbar window visible");
        self.window.set_visible(true);
        self.window.request_redraw();
    }

    /// 隐藏窗口
    pub fn hide(&self) {
        tracing::debug!("Hiding toolbar window");
        self.window.set_visible(false);
    }

    /// 更新窗口位置
    ///
    /// # 参数
    /// - `selection_rect`: 选择框在虚拟桌面坐标系中的位置 (x, y, width, height)
    /// - `monitor_info`: 选择框所在的显示器信息 (virtual_x, virtual_y, width, height)
    ///
    /// 在多显示器环境下，虚拟坐标可能为负数，需要转换为屏幕坐标
    pub fn update_position(
        &self,
        selection_rect: (f32, f32, f32, f32),
        monitor_info: Option<(i32, i32, u32, u32)>,
    ) {
        let (sel_x, sel_y, sel_w, sel_h) = selection_rect;

        // 工具栏尺寸
        const TOOLBAR_WIDTH: f32 = 400.0;
        const TOOLBAR_HEIGHT: f32 = 50.0;
        const SPACING: f32 = 10.0;

        // 在虚拟坐标系中计算工具栏位置（居中对齐，位于选择框下方）
        let virtual_x = sel_x + (sel_w - TOOLBAR_WIDTH) / 2.0;
        let virtual_y = sel_y + sel_h + SPACING;

        // 如果有显示器信息，将虚拟坐标转换为屏幕坐标
        let (screen_x, screen_y) = if let Some((monitor_vx, monitor_vy, monitor_w, monitor_h)) =
            monitor_info
        {
            // 虚拟坐标 -> 显示器本地坐标
            let local_x = virtual_x - monitor_vx as f32;
            let local_y = virtual_y - monitor_vy as f32;

            // 确保工具栏在显示器范围内
            let clamped_x = local_x
                .max(10.0)
                .min(monitor_w as f32 - TOOLBAR_WIDTH - 10.0);
            let clamped_y = local_y
                .max(10.0)
                .min(monitor_h as f32 - TOOLBAR_HEIGHT - 10.0);

            // 本地坐标 -> 屏幕坐标（winit 的 set_outer_position 需要屏幕绝对坐标）
            let screen_x = monitor_vx as f32 + clamped_x;
            let screen_y = monitor_vy as f32 + clamped_y;

            tracing::debug!(
                "Toolbar position: virtual=({:.1}, {:.1}), local=({:.1}, {:.1}), screen=({:.1}, {:.1}), monitor=({}, {}) {}x{}",
                virtual_x, virtual_y, local_x, local_y, screen_x, screen_y,
                monitor_vx, monitor_vy, monitor_w, monitor_h
            );

            (screen_x, screen_y)
        } else {
            // 没有显示器信息，使用简单钳制策略
            let clamped_x = virtual_x.clamp(10.0, 10000.0);
            let clamped_y = virtual_y.clamp(10.0, 10000.0);

            tracing::warn!(
                "Toolbar position without monitor info: virtual=({:.1}, {:.1}), clamped=({:.1}, {:.1})",
                virtual_x, virtual_y, clamped_x, clamped_y
            );

            (clamped_x, clamped_y)
        };

        // 设置窗口大小（使用逻辑像素以支持 HiDPI）
        let _ = self.window.request_inner_size(winit::dpi::LogicalSize::new(
            TOOLBAR_WIDTH as f64,
            TOOLBAR_HEIGHT as f64,
        ));

        // 设置窗口位置（屏幕绝对坐标）
        self.window
            .set_outer_position(PhysicalPosition::new(screen_x as i32, screen_y as i32));
    }

    /// 渲染工具栏并返回操作
    pub fn render(&mut self) -> ToolbarAction {
        use egui_wgpu::wgpu;

        if !self.initialized || self.render_state.is_none() {
            tracing::warn!("Toolbar render called but not initialized");
            return ToolbarAction::None;
        }

        let render_state = self.render_state.as_mut().unwrap();

        // 获取 surface texture
        let output = match render_state.surface.get_current_texture() {
            Ok(output) => output,
            Err(e) => {
                tracing::error!("Failed to get surface texture: {:?}", e);
                return ToolbarAction::None;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 准备 egui 输入
        let raw_input = self.egui_state.take_egui_input(&self.window);

        // 运行 egui UI
        let mut action = ToolbarAction::None;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            let size = self.window.inner_size();
            let window_size = (size.width as f32, size.height as f32);

            // 工具栏占据整个窗口，所以选择框相对坐标为整个窗口
            let selection_rect = (0.0, 0.0, window_size.0, window_size.1);

            tracing::debug!(
                "Toolbar render: visible={}, size={:?}",
                self.toolbar.visible,
                window_size
            );

            action =
                self.toolbar_renderer
                    .render(ctx, &mut self.toolbar, selection_rect, window_size);
        });

        // 处理平台输出
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // 渲染
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                render_state.surface_config.width,
                render_state.surface_config.height,
            ],
            pixels_per_point: full_output.pixels_per_point,
        };

        // 更新纹理
        for (id, image_delta) in &full_output.textures_delta.set {
            render_state.renderer.update_texture(
                &render_state.device,
                &render_state.queue,
                *id,
                image_delta,
            );
        }

        // 更新缓冲区
        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Toolbar Update Encoder"),
                });

        render_state.renderer.update_buffers(
            &render_state.device,
            &render_state.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        render_state.queue.submit(Some(encoder.finish()));

        // 创建渲染 encoder
        let mut render_encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Toolbar Render Encoder"),
                });

        {
            let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Toolbar Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_state.renderer.render(
                &mut render_pass.forget_lifetime(),
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        render_state.queue.submit(Some(render_encoder.finish()));
        output.present();

        // 释放纹理
        for id in &full_output.textures_delta.free {
            render_state.renderer.free_texture(id);
        }

        action
    }

    /// 处理窗口事件
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.egui_state
            .on_window_event(&self.window, event)
            .consumed
    }

    /// 获取窗口 ID
    pub fn window_id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    /// 获取底层窗口引用（用于请求重绘等操作）
    pub fn window(&self) -> &Window {
        &self.window
    }
}
