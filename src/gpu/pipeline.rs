// ============================================================================
// 渲染管线模块 (Render Pipeline)
// ============================================================================
//
// wgpu教学: 渲染管线 (RenderPipeline)
//
// 什么是渲染管线?
// 渲染管线定义了GPU如何处理顶点数据并生成最终图像。它包含:
// 1. 着色器程序（顶点着色器 + 片段着色器）
// 2. 顶点缓冲区布局（数据如何传递给着色器）
// 3. 资源绑定布局（纹理、uniform等如何绑定）
// 4. 渲染状态（混合模式、图元类型等）
//
// 管线创建原则:
// - 管线一旦创建就不可修改（immutable）
// - 切换管线有性能开销，应该减少切换次数
// - 相同渲染需求的对象应该共用管线
//
// 本模块包含:
// - 着色器模块创建
// - 绑定组布局创建
// - 管线创建
// - 顶点布局定义
//
// ============================================================================

use crate::gpu::types::{FillRect, SpriteInstance};

// ============================================================================
// 着色器源码
//
// wgpu教学: 使用include_str!()在编译时嵌入着色器文件
// 优点:
// 1. 着色器文件独立，便于语法高亮和IDE支持
// 2. 便于版本控制和代码审查
// 3. 可以添加详细的教学注释
//
// 着色器文件位置: src/gpu/shaders/
// ============================================================================

/// 精灵渲染着色器 - 实例化渲染批量绘制2D精灵
pub const SPRITE_SHADER: &str = include_str!("shaders/sprite.wgsl");

/// 填充矩形着色器 - 绘制纯色矩形（天空、背景、UI）
pub const FILL_SHADER: &str = include_str!("shaders/fill.wgsl");

/// 缩放输出着色器 - 将游戏画面等比例缩放到窗口
pub const SCALE_SHADER: &str = include_str!("shaders/scale.wgsl");

/// 叠加层着色器 - 在游戏画面上叠加UI元素
pub const OVERLAY_SHADER: &str = include_str!("shaders/overlay.wgsl");

// ============================================================================
// 顶点布局 (Vertex Layout)
//
// wgpu教学: VertexBufferLayout
// 定义顶点数据如何从CPU传递到GPU着色器
//
// 关键概念:
// - array_stride: 每个实例/顶点的字节大小
// - step_mode: Vertex(每顶点) 或 Instance(每实例)
// - attributes: 描述每个字段的偏移和格式
// ============================================================================

/// 创建精灵实例的顶点缓冲区布局
///
/// wgpu教学: 实例化渲染的顶点布局
/// step_mode = Instance 表示每个实例（而非每个顶点）使用一份数据
/// 6个顶点共享同一份实例数据
pub fn create_sprite_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            // position: [f32; 2] at offset 0
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            // size: [f32; 2] at offset 8
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            // uv_offset: [f32; 2] at offset 16
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
            // uv_size: [f32; 2] at offset 24
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x2,
            },
            // flip: [f32; 2] at offset 32
            wgpu::VertexAttribute {
                offset: 32,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32x2,
            },
            // palette_offset: f32 at offset 40
            wgpu::VertexAttribute {
                offset: 40,
                shader_location: 5,
                format: wgpu::VertexFormat::Float32,
            },
            // palette_index: f32 at offset 44
            wgpu::VertexAttribute {
                offset: 44,
                shader_location: 6,
                format: wgpu::VertexFormat::Float32,
            },
            // opaque: f32 at offset 48
            wgpu::VertexAttribute {
                offset: 48,
                shader_location: 7,
                format: wgpu::VertexFormat::Float32,
            },
            // rotation: f32 at offset 52
            wgpu::VertexAttribute {
                offset: 52,
                shader_location: 8,
                format: wgpu::VertexFormat::Float32,
            },
        ],
    }
}

/// 创建填充矩形实例的顶点缓冲区布局
pub fn create_fill_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<FillRect>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            // position: [f32; 2]
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            // size: [f32; 2]
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            // color_index: f32
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32,
            },
            // palette_index: f32
            wgpu::VertexAttribute {
                offset: 20,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32,
            },
        ],
    }
}

// ============================================================================
// 绑定组布局 (Bind Group Layout)
//
// wgpu教学: BindGroupLayout
// 定义着色器可以访问哪些GPU资源（纹理、缓冲区、采样器）
//
// 每个绑定槽(binding)对应着色器中的 @group(n) @binding(m)
// ============================================================================

/// 创建精灵管线的绑定组布局
///
/// 绑定内容:
/// - binding 0: Camera uniform（顶点着色器使用）
/// - binding 1: 图集纹理（片段着色器使用）
/// - binding 2: 调色板纹理（片段着色器使用）
/// - binding 3: 采样器（片段着色器使用）
pub fn create_sprite_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sprite_bind_group_layout"),
        entries: &[
            // binding 0: Camera uniform
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
            // binding 1: 图集纹理（可过滤）
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
            // binding 2: 调色板纹理（不可过滤，使用textureLoad）
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
            // binding 3: 采样器
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// 创建填充管线的绑定组布局
///
/// 绑定内容:
/// - binding 0: Camera uniform
/// - binding 1: 调色板纹理
pub fn create_fill_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

/// 创建缩放管线的绑定组布局
pub fn create_scale_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scale_bind_group_layout"),
        entries: &[
            // binding 0: 渲染纹理
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
            // binding 1: 采样器
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // binding 2: 缩放参数uniform
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
    })
}

/// 创建叠加层管线的绑定组布局
pub fn create_overlay_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("overlay_bind_group_layout"),
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
        ],
    })
}

// ============================================================================
// 渲染管线创建 (Pipeline Creation)
//
// wgpu教学: RenderPipeline创建
//
// 管线创建需要指定:
// 1. layout: 管线布局（包含所有绑定组布局）
// 2. vertex: 顶点着色器配置
// 3. fragment: 片段着色器配置（可选）
// 4. primitive: 图元类型（三角形、线等）
// 5. depth_stencil: 深度/模板测试（可选）
// 6. multisample: 多重采样配置
// ============================================================================

/// 创建精灵渲染管线
///
/// wgpu教学: 精灵管线特点
/// - 使用实例化渲染（一次绘制多个精灵）
/// - 启用Alpha混合（支持透明）
/// - 输出格式为内部渲染纹理格式
pub fn create_sprite_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sprite_shader"),
        source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sprite_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[create_sprite_instance_layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
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
        multiview_mask: None,
        cache: None,
    })
}

/// 创建填充矩形渲染管线
///
/// wgpu教学: 填充管线特点
/// - 不使用Alpha混合（填充矩形总是不透明）
/// - 更简单的着色器
pub fn create_fill_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fill_shader"),
        source: wgpu::ShaderSource::Wgsl(FILL_SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fill_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fill_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[create_fill_instance_layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None, // 不使用混合
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
        multiview_mask: None,
        cache: None,
    })
}

/// 创建缩放输出渲染管线
///
/// wgpu教学: 后处理管线特点
/// - 不使用顶点缓冲区（在着色器中生成全屏四边形）
/// - 输出格式为窗口surface格式（参数传入）
pub fn create_scale_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scale_shader"),
        source: wgpu::ShaderSource::Wgsl(SCALE_SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scale_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scale_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // 不使用顶点缓冲区
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
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
        multiview_mask: None,
        cache: None,
    })
}

/// 创建叠加层渲染管线
///
/// wgpu教学: 叠加层管线特点
/// - 启用Alpha混合（与之前的渲染结果混合）
/// - 输出格式为窗口surface格式
pub fn create_overlay_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("overlay_shader"),
        source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("overlay_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("overlay_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING), // 启用Alpha混合
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
        multiview_mask: None,
        cache: None,
    })
}
