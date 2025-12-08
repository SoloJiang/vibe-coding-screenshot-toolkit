/// 背景渲染器
///
/// 使用 wgpu 直接渲染背景图，绕过 egui 的纹理大小限制（2048x2048）
///
/// ## 设计说明
///
/// ### 为什么使用 wgpu 直接渲染？
/// - egui 对单个纹理有 2048x2048 的尺寸限制
/// - 多显示器场景下，完整虚拟桌面背景可能远超此限制（如 7680x4320）
/// - wgpu 可以直接渲染更大的纹理（设备限制通常是 8192x8192 或更大）
///
/// ### 为什么裁剪背景而不是 UV 映射？
/// - **代码简单**：每个窗口直接上传裁剪后的区域，shader 不需要复杂的 UV 计算
/// - **性能好**：上传的数据量更小，GPU 内存占用更少
/// - **易维护**：逻辑清晰，调试方便
///
/// ### 渲染流程
/// 1. **wgpu 背景层**：全屏显示裁剪后的背景图（本模块）
/// 2. **egui UI 层**：绘制半透明遮罩和选择框（selection_render.rs）
///
use egui_wgpu::wgpu;

pub struct BackgroundRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl BackgroundRenderer {
    /// 创建背景渲染器
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // 创建纹理 bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("background_bind_group_layout"),
            entries: &[
                // 纹理
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 采样器
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // 创建 pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("background_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 着色器代码
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("background_shader"),
            source: wgpu::ShaderSource::Wgsl(BACKGROUND_SHADER.into()),
        });

        // 创建渲染管线
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("background_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 4 * 4, // 4 floats: pos(x,y) + uv(x,y)
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 创建采样器
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("background_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 创建全屏四边形的顶点缓冲区
        // 顶点格式：[x, y, u, v]
        #[rustfmt::skip]
        let vertices: &[f32] = &[
            // 左上
            -1.0,  1.0,  0.0, 0.0,
            // 右上
             1.0,  1.0,  1.0, 0.0,
            // 左下
            -1.0, -1.0,  0.0, 1.0,
            // 右下
             1.0, -1.0,  1.0, 1.0,
        ];

        let vertex_data = bytemuck::cast_slice(vertices);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("background_vertex_buffer"),
            size: vertex_data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        vertex_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(vertex_data);
        vertex_buffer.unmap();

        // 索引缓冲区（两个三角形）
        let indices: &[u16] = &[0, 2, 1, 1, 2, 3];
        let index_data = bytemuck::cast_slice(indices);
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("background_index_buffer"),
            size: index_data.len() as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });
        index_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(index_data);
        index_buffer.unmap();

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            vertex_buffer,
            index_buffer,
        }
    }

    /// 创建全屏背景纹理
    ///
    /// 每个窗口上传裁剪后的背景区域，直接全屏显示
    ///
    /// # Arguments
    /// * `data` - RGBA8 纹理数据（已裁剪到窗口大小）
    /// * `width` - 纹理宽度（应与窗口宽度一致）
    /// * `height` - 纹理高度（应与窗口高度一致）
    ///
    /// # Returns
    /// `(Texture, BindGroup)` - 纹理对象和用于渲染的 bind group
    pub fn create_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        // 创建纹理
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("background_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // 上传纹理数据
        queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // 创建纹理视图
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建纹理 bind group
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("background_texture_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        (texture, texture_bind_group)
    }

    /// 渲染全屏背景纹理
    ///
    /// # Arguments
    /// * `render_pass` - wgpu 渲染通道
    /// * `texture_bind_group` - 由 `create_texture()` 返回的 bind group
    pub fn render<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        texture_bind_group: &'rpass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

// Shader 代码（简化版：全屏显示纹理，无 UV 映射）
const BACKGROUND_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;  // 直接使用输入 UV，全屏显示纹理
    return output;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, input.uv);
}
"#;
