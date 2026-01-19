// GPU渲染模块 - 基于wgpu的硬件加速渲染
// 替代原有的CPU软件渲染

pub mod texture_atlas;
pub mod sprite_batch;
pub mod palette;
pub mod tilemap;

use std::sync::Arc;
use wgpu::util::DeviceExt;

// 游戏渲染常量
pub const GAME_WIDTH: u32 = 320;
pub const GAME_HEIGHT: u32 = 182;
pub const ATLAS_SIZE: u32 = 1024;
pub const MAX_SPRITES_PER_BATCH: usize = 1024;

// 精灵实例数据 - 传递给GPU的每个精灵信息
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    // 屏幕位置 (像素)
    pub position: [f32; 2],
    // 精灵尺寸 (像素)
    pub size: [f32; 2],
    // 纹理UV坐标 (左上角)
    pub uv_offset: [f32; 2],
    // 纹理UV尺寸
    pub uv_size: [f32; 2],
    // 翻转标志: x=水平翻转, y=垂直翻转
    pub flip: [f32; 2],
    // 调色板偏移 (用于recolor)
    pub palette_offset: f32,
    // 调色板索引 (选择哪个预烘焙调色板)
    pub palette_index: f32,
}

impl SpriteInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, uv_x: f32, uv_y: f32, uv_w: f32, uv_h: f32) -> Self {
        Self {
            position: [x, y],
            size: [w, h],
            uv_offset: [uv_x, uv_y],
            uv_size: [uv_w, uv_h],
            flip: [0.0, 0.0],
            palette_offset: 0.0,
            palette_index: 0.0,
        }
    }

    pub fn with_flip(mut self, flip_x: bool, flip_y: bool) -> Self {
        self.flip = [if flip_x { 1.0 } else { 0.0 }, if flip_y { 1.0 } else { 0.0 }];
        self
    }

    pub fn with_palette(mut self, offset: f32, index: f32) -> Self {
        self.palette_offset = offset;
        self.palette_index = index;
        self
    }
}

// 填充矩形实例 - 用于天空/背景填充
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FillRect {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color_index: f32,
    pub palette_index: f32,
    pub _padding: [f32; 2],
}

impl FillRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color_index: u8, palette_index: u32) -> Self {
        Self {
            position: [x, y],
            size: [w, h],
            color_index: color_index as f32,
            palette_index: palette_index as f32,
            _padding: [0.0, 0.0],
        }
    }
}

// 渲染命令枚举 - 用于收集帧内所有渲染操作
#[derive(Clone, Debug)]
pub enum RenderCommand {
    // 绘制精灵实例 (底层)
    DrawSprite(SpriteInstance),
    // 绘制精灵命令 (高层，包含UV信息)
    Sprite(sprite_batch::SpriteCommand),
    // 绘制翻转精灵 (上下颠倒)
    DrawSpriteFlipY(SpriteInstance),
    // 绘制部分精灵 (用于升起动画)
    DrawSpritePart {
        sprite: SpriteInstance,
        visible_height: f32,
    },
    // 填充矩形
    FillRect(FillRect),
}

// 相机/视口uniform
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // 视口偏移 (世界坐标)
    pub view_offset: [f32; 2],
    // 屏幕尺寸 (像素)
    pub screen_size: [f32; 2],
}

impl CameraUniform {
    pub fn new(x_view: i32, y_view: i32) -> Self {
        Self {
            view_offset: [x_view as f32, y_view as f32],
            screen_size: [GAME_WIDTH as f32, GAME_HEIGHT as f32],
        }
    }
}

// GPU渲染器 - 核心结构体
pub struct GpuRenderer {
    // wgpu核心对象
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    
    // 渲染目标纹理 (游戏画面)
    pub render_texture: wgpu::Texture,
    pub render_texture_view: wgpu::TextureView,
    
    // 精灵图集纹理
    pub atlas_texture: wgpu::Texture,
    pub atlas_texture_view: wgpu::TextureView,
    
    // 调色板纹理数组
    pub palette_texture: wgpu::Texture,
    pub palette_texture_view: wgpu::TextureView,
    
    // 精灵渲染管线
    pub sprite_pipeline: wgpu::RenderPipeline,
    pub sprite_bind_group: wgpu::BindGroup,
    
    // 填充矩形渲染管线
    pub fill_pipeline: wgpu::RenderPipeline,
    pub fill_bind_group: wgpu::BindGroup,
    
    // 最终缩放输出管线
    pub scale_pipeline: wgpu::RenderPipeline,
    pub scale_bind_group: wgpu::BindGroup,
    
    // uniform缓冲区
    pub camera_buffer: wgpu::Buffer,
    pub scale_buffer: wgpu::Buffer,
    
    // 当前帧的精灵批次
    pub sprite_instances: Vec<SpriteInstance>,
    pub fill_rects: Vec<FillRect>,
    
    // 当前调色板索引
    pub current_palette: u32,
    
    // 相机状态
    pub camera: CameraUniform,
}

// 精灵着色器源码
const SPRITE_SHADER: &str = r#"
struct CameraUniform {
    view_offset: vec2<f32>,
    screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var palette_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;

struct SpriteInstance {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_offset: vec2<f32>,
    @location(3) uv_size: vec2<f32>,
    @location(4) flip: vec2<f32>,
    @location(5) palette_offset: f32,
    @location(6) palette_index: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) palette_offset: f32,
    @location(2) palette_index: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: SpriteInstance) -> VertexOutput {
    // 四边形顶点 (0,1,2,3,4,5 = 两个三角形)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    let pos = positions[vertex_index];
    
    // 计算屏幕位置
    let screen_pos = instance.position + pos * instance.size;
    let ndc = (screen_pos / camera.screen_size) * 2.0 - 1.0;
    
    // 计算UV (考虑翻转)
    var uv = pos;
    if (instance.flip.x > 0.5) { uv.x = 1.0 - uv.x; }
    if (instance.flip.y > 0.5) { uv.y = 1.0 - uv.y; }
    uv = instance.uv_offset + uv * instance.uv_size;
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    output.uv = uv;
    output.palette_offset = instance.palette_offset;
    output.palette_index = instance.palette_index;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 从图集采样调色板索引
    let index_color = textureSample(atlas_texture, tex_sampler, in.uv);
    let palette_idx = u32(index_color.r * 255.0 + in.palette_offset) % 256u;
    
    // 透明像素 (索引0)
    if (palette_idx == 0u) { discard; }
    
    // 从调色板纹理查找颜色
    let palette_row = u32(in.palette_index);
    let color = textureLoad(palette_texture, vec2<i32>(i32(palette_idx), i32(palette_row)), 0);
    return color;
}
"#;

// 填充矩形着色器
const FILL_SHADER: &str = r#"
struct CameraUniform {
    view_offset: vec2<f32>,
    screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var palette_texture: texture_2d<f32>;

struct FillInstance {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color_index: f32,
    @location(3) palette_index: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color_index: f32,
    @location(1) palette_index: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: FillInstance) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    let pos = positions[vertex_index];
    let screen_pos = instance.position + pos * instance.size;
    let ndc = (screen_pos / camera.screen_size) * 2.0 - 1.0;
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    output.color_index = instance.color_index;
    output.palette_index = instance.palette_index;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let palette_idx = u32(in.color_index) % 256u;
    let palette_row = u32(in.palette_index);
    let color = textureLoad(palette_texture, vec2<i32>(i32(palette_idx), i32(palette_row)), 0);
    return color;
}
"#;

// 缩放输出着色器
const SCALE_SHADER: &str = r#"
struct ScaleUniform {
    scale: vec2<f32>,
    offset: vec2<f32>,
}

@group(0) @binding(0) var render_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> scale_params: ScaleUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0)
    );
    let pos = positions[vertex_index];
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(pos * scale_params.scale + scale_params.offset, 0.0, 1.0);
    output.uv = (pos + 1.0) * 0.5;
    output.uv.y = 1.0 - output.uv.y;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(render_texture, tex_sampler, in.uv);
}
"#;

impl GpuRenderer {
    // 创建GPU渲染器
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // 创建渲染目标纹理
        // 添加COPY_DST以支持从CPU上传framebuffer
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_texture"),
            size: wgpu::Extent3d {
                width: GAME_WIDTH,
                height: GAME_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                 | wgpu::TextureUsages::TEXTURE_BINDING
                 | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let render_texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建精灵图集纹理 (R8格式存储调色板索引)
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_texture_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建调色板纹理 (256x64, 每行一个调色板状态)
        let palette_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("palette_texture"),
            size: wgpu::Extent3d {
                width: 256,
                height: 64,  // 支持64种调色板状态
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let palette_texture_view = palette_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建相机uniform缓冲区
        let camera = CameraUniform::new(0, 0);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 创建采样器
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 创建精灵着色器模块
        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });

        // 精灵管线绑定组布局
        let sprite_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // 创建精灵绑定组
        let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_bind_group"),
            layout: &sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&palette_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // 精灵实例缓冲区布局
        let sprite_instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 24, shader_location: 3, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 32, shader_location: 4, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 40, shader_location: 5, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 44, shader_location: 6, format: wgpu::VertexFormat::Float32 },
            ],
        };

        // 创建精灵渲染管线
        let sprite_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&sprite_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
                entry_point: Some("vs_main"),
                buffers: &[sprite_instance_layout.clone()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sprite_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

        // 创建填充矩形着色器
        let fill_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fill_shader"),
            source: wgpu::ShaderSource::Wgsl(FILL_SHADER.into()),
        });

        // 填充管线绑定组布局
        let fill_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fill_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let fill_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fill_bind_group"),
            layout: &fill_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&palette_texture_view),
                },
            ],
        });

        let fill_instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FillRect>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 20, shader_location: 3, format: wgpu::VertexFormat::Float32 },
            ],
        };

        let fill_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fill_pipeline_layout"),
            bind_group_layouts: &[&fill_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fill_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fill_pipeline"),
            layout: Some(&fill_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fill_shader,
                entry_point: Some("vs_main"),
                buffers: &[fill_instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fill_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
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

        // 创建缩放着色器
        let scale_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scale_shader"),
            source: wgpu::ShaderSource::Wgsl(SCALE_SHADER.into()),
        });

        // 缩放uniform缓冲区
        let scale_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scale_buffer"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scale_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scale_bind_group_layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let scale_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scale_bind_group"),
            layout: &scale_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scale_buffer.as_entire_binding(),
                },
            ],
        });

        let scale_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scale_pipeline_layout"),
            bind_group_layouts: &[&scale_bind_group_layout],
            push_constant_ranges: &[],
        });

        let scale_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scale_pipeline"),
            layout: Some(&scale_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &scale_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &scale_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: None,
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

        Self {
            device,
            queue,
            render_texture,
            render_texture_view,
            atlas_texture,
            atlas_texture_view,
            palette_texture,
            palette_texture_view,
            sprite_pipeline,
            sprite_bind_group,
            fill_pipeline,
            fill_bind_group,
            scale_pipeline,
            scale_bind_group,
            camera_buffer,
            scale_buffer,
            sprite_instances: Vec::with_capacity(MAX_SPRITES_PER_BATCH),
            fill_rects: Vec::with_capacity(128),
            current_palette: 0,
            camera,
        }
    }

    // 上传精灵图集到GPU
    pub fn upload_atlas(&self, data: &[u8], width: u32, height: u32) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    // 上传调色板到GPU (row是调色板索引)
    pub fn upload_palette(&self, row: u32, colors: &[[u8; 4]; 256]) {
        let data: Vec<u8> = colors.iter().flat_map(|c| c.iter().copied()).collect();
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.palette_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: row, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(256 * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 256,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    // 更新相机位置
    pub fn update_camera(&mut self, x_view: i32, y_view: i32) {
        self.camera = CameraUniform::new(x_view, y_view);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
    }
    
    /// 更新缩放参数（用于保持宽高比的等比例缩放）
    /// 
    /// 参数：
    /// - window_width: 窗口宽度
    /// - window_height: 窗口高度
    /// 
    /// 计算逻辑：
    /// - 保持游戏画面宽高比 (320:182)
    /// - 计算最大缩放倍数
    /// - 居中显示（letterbox/pillarbox）
    pub fn update_scale(&self, window_width: u32, window_height: u32) {
        if window_width == 0 || window_height == 0 {
            return;
        }
        
        // 计算缩放比例（保持宽高比）
        let scale_x = window_width as f32 / GAME_WIDTH as f32;
        let scale_y = window_height as f32 / GAME_HEIGHT as f32;
        let scale = scale_x.min(scale_y);
        
        // 计算实际显示尺寸
        let display_width = GAME_WIDTH as f32 * scale;
        let display_height = GAME_HEIGHT as f32 * scale;
        
        // 计算偏移（居中）
        let offset_x = (window_width as f32 - display_width) / 2.0;
        let offset_y = (window_height as f32 - display_height) / 2.0;
        
        // 转换为NDC坐标系（-1到1）
        // scale: 显示区域占窗口的比例
        let ndc_scale_x = display_width / window_width as f32;
        let ndc_scale_y = display_height / window_height as f32;
        
        // offset: 从中心的偏移量（NDC坐标系中居中时offset为0）
        // 如果display占满窗口的一个维度，另一个维度居中，offset为0
        let ndc_offset_x = (offset_x * 2.0 / window_width as f32) - (1.0 - ndc_scale_x);
        let ndc_offset_y = -((offset_y * 2.0 / window_height as f32) - (1.0 - ndc_scale_y));
        
        // 更新uniform
        let scale_data = [ndc_scale_x, ndc_scale_y, ndc_offset_x, ndc_offset_y];
        self.queue.write_buffer(&self.scale_buffer, 0, bytemuck::cast_slice(&scale_data));
    }

    // 设置当前调色板索引
    pub fn set_palette(&mut self, index: u32) {
        self.current_palette = index;
    }

    // 清除当前帧批次
    pub fn begin_frame(&mut self) {
        self.sprite_instances.clear();
        self.fill_rects.clear();
    }

    // 添加精灵到批次
    pub fn draw_sprite(&mut self, instance: SpriteInstance) {
        self.sprite_instances.push(instance);
    }

    // 添加填充矩形到批次
    pub fn draw_fill(&mut self, rect: FillRect) {
        self.fill_rects.push(rect);
    }

    // 渲染当前帧到内部纹理
    pub fn render_frame(&mut self) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame_encoder"),
        });

        // 渲染到游戏画面纹理
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("game_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 先渲染填充矩形 (背景层)
            if !self.fill_rects.is_empty() {
                let fill_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("fill_buffer"),
                    contents: bytemuck::cast_slice(&self.fill_rects),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                render_pass.set_pipeline(&self.fill_pipeline);
                render_pass.set_bind_group(0, &self.fill_bind_group, &[]);
                render_pass.set_vertex_buffer(0, fill_buffer.slice(..));
                render_pass.draw(0..6, 0..self.fill_rects.len() as u32);
            }

            // 再渲染精灵 (实体层)
            if !self.sprite_instances.is_empty() {
                let sprite_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite_buffer"),
                    contents: bytemuck::cast_slice(&self.sprite_instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                render_pass.set_pipeline(&self.sprite_pipeline);
                render_pass.set_bind_group(0, &self.sprite_bind_group, &[]);
                render_pass.set_vertex_buffer(0, sprite_buffer.slice(..));
                render_pass.draw(0..6, 0..self.sprite_instances.len() as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    // 渲染到窗口surface (缩放输出)
    pub fn render_to_surface(&self, surface_view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scale_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scale_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.scale_pipeline);
            render_pass.set_bind_group(0, &self.scale_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
    
    /// 上传CPU渲染的RGBA framebuffer到GPU纹理
    /// 这是一个临时方案，用于在完整GPU渲染管线实现之前显示游戏画面
    pub fn upload_framebuffer(&self, rgba_data: &[u8]) {
        // 验证数据大小
        let expected_size = (GAME_WIDTH * GAME_HEIGHT * 4) as usize;
        if rgba_data.len() != expected_size {
            eprintln!("[GPU] framebuffer大小不匹配: {} vs {}", rgba_data.len(), expected_size);
            return;
        }
        
        // 上传到渲染纹理
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(GAME_WIDTH * 4),
                rows_per_image: Some(GAME_HEIGHT),
            },
            wgpu::Extent3d {
                width: GAME_WIDTH,
                height: GAME_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
    }
    
    /// 将CPU渲染的framebuffer显示到窗口surface
    /// 这是一个完整的渲染流程：上传framebuffer + 缩放输出
    pub fn present_framebuffer(&self, rgba_data: &[u8], surface_view: &wgpu::TextureView) {
        // 上传framebuffer到GPU纹理
        self.upload_framebuffer(rgba_data);
        
        // 缩放输出到窗口surface
        self.render_to_surface(surface_view);
    }
}
