// Tilemap GPU渲染器 - 高效渲染游戏地形层
//
// 使用单次draw call渲染整个可见区域的地形tile
// 支持滚动视口和tile动画

use super::{GAME_WIDTH, GAME_HEIGHT};
use crate::buffers::{NH, NV, W, H};

/// Tile数据 - 存储可见区域的tile信息
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileData {
    /// Tile在图集中的UV坐标 (归一化)
    pub uv_offset: [f32; 2],
    /// Tile尺寸 (归一化)
    pub uv_size: [f32; 2],
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            uv_offset: [0.0, 0.0],
            uv_size: [0.0, 0.0],
        }
    }
}

/// Tilemap uniform数据
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TilemapUniform {
    /// 视口偏移 (世界像素坐标)
    pub view_offset: [f32; 2],
    /// 屏幕尺寸 (像素)
    pub screen_size: [f32; 2],
    /// Tile尺寸 (像素)
    pub tile_size: [f32; 2],
    /// Tilemap尺寸 (tile数)
    pub map_size: [f32; 2],
}

/// 可见区域的tile数据缓冲
pub struct TilemapData {
    /// Tile数据 [y][x] = TileData
    tiles: Vec<Vec<TileData>>,
    /// 可见宽度 (tile数)
    visible_width: usize,
    /// 可见高度 (tile数)
    visible_height: usize,
    /// 是否有数据变化
    dirty: bool,
}

impl TilemapData {
    pub fn new() -> Self {
        // 可见区域：NH+2 x NV tiles (额外2列用于滚动过渡)
        let visible_width = (NH as usize) + 2;
        let visible_height = NV as usize;
        
        let tiles = vec![vec![TileData::default(); visible_width]; visible_height];
        
        Self {
            tiles,
            visible_width,
            visible_height,
            dirty: true,
        }
    }
    
    /// 设置tile数据
    pub fn set_tile(&mut self, x: usize, y: usize, uv_offset: [f32; 2], uv_size: [f32; 2]) {
        if x < self.visible_width && y < self.visible_height {
            self.tiles[y][x] = TileData { uv_offset, uv_size };
            self.dirty = true;
        }
    }
    
    /// 清空所有tile
    pub fn clear(&mut self) {
        for row in &mut self.tiles {
            for tile in row {
                *tile = TileData::default();
            }
        }
        self.dirty = true;
    }
    
    /// 获取tile数据的扁平化数组
    pub fn as_flat_data(&self) -> Vec<TileData> {
        let mut data = Vec::with_capacity(self.visible_width * self.visible_height);
        for row in &self.tiles {
            data.extend_from_slice(row);
        }
        data
    }
    
    /// 标记为干净（数据已上传）
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// 检查是否需要更新
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    pub fn visible_width(&self) -> usize {
        self.visible_width
    }
    
    pub fn visible_height(&self) -> usize {
        self.visible_height
    }
}

impl Default for TilemapData {
    fn default() -> Self {
        Self::new()
    }
}

/// Tilemap着色器源码
pub const TILEMAP_SHADER: &str = r#"
struct TilemapUniform {
    view_offset: vec2<f32>,
    screen_size: vec2<f32>,
    tile_size: vec2<f32>,
    map_size: vec2<f32>,
}

struct TileData {
    uv_offset: vec2<f32>,
    uv_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> tilemap: TilemapUniform;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var palette_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<storage, read> tile_data: array<TileData>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // 全屏四边形
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    let pos = positions[vertex_index];
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    output.clip_position.y = -output.clip_position.y;
    
    // 计算世界坐标
    output.world_pos = tilemap.view_offset + pos * tilemap.screen_size;
    
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 计算当前像素所在的tile坐标
    let tile_x = i32(floor(in.world_pos.x / tilemap.tile_size.x));
    let tile_y = i32(floor(in.world_pos.y / tilemap.tile_size.y));
    
    // 计算在可见tilemap中的索引
    let view_tile_x = tile_x - i32(floor(tilemap.view_offset.x / tilemap.tile_size.x));
    let view_tile_y = tile_y;
    
    // 边界检查
    if (view_tile_x < 0 || view_tile_x >= i32(tilemap.map_size.x) ||
        view_tile_y < 0 || view_tile_y >= i32(tilemap.map_size.y)) {
        discard;
    }
    
    // 获取tile数据
    let tile_index = u32(view_tile_y) * u32(tilemap.map_size.x) + u32(view_tile_x);
    let tile = tile_data[tile_index];
    
    // 空tile跳过
    if (tile.uv_size.x == 0.0 && tile.uv_size.y == 0.0) {
        discard;
    }
    
    // 计算tile内的局部坐标
    let local_x = fract(in.world_pos.x / tilemap.tile_size.x);
    let local_y = fract(in.world_pos.y / tilemap.tile_size.y);
    
    // 计算UV坐标
    let uv = tile.uv_offset + vec2<f32>(local_x, local_y) * tile.uv_size;
    
    // 采样图集获取调色板索引
    let index_color = textureSample(atlas_texture, tex_sampler, uv);
    let palette_idx = u32(index_color.r * 255.0) % 256u;
    
    // 透明像素
    if (palette_idx == 0u) {
        discard;
    }
    
    // 从调色板查找颜色
    let color = textureLoad(palette_texture, vec2<i32>(i32(palette_idx), 0), 0);
    return color;
}
"#;

/// Tilemap GPU渲染器
pub struct TilemapRenderer {
    /// 渲染管线
    pipeline: wgpu::RenderPipeline,
    /// Uniform缓冲区
    uniform_buffer: wgpu::Buffer,
    /// Tile数据存储缓冲区
    tile_data_buffer: wgpu::Buffer,
    /// 绑定组
    bind_group: wgpu::BindGroup,
    /// 绑定组布局
    bind_group_layout: wgpu::BindGroupLayout,
    /// 当前uniform数据
    uniform: TilemapUniform,
    /// 缓冲区容量
    buffer_capacity: usize,
}

impl TilemapRenderer {
    pub fn new(
        device: &wgpu::Device,
        atlas_view: &wgpu::TextureView,
        palette_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        // 创建着色器模块
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tilemap Shader"),
            source: wgpu::ShaderSource::Wgsl(TILEMAP_SHADER.into()),
        });
        
        // 绑定组布局
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tilemap Bind Group Layout"),
            entries: &[
                // Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Atlas texture
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
                // Palette texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Tile data storage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // 管线布局
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tilemap Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // 渲染管线
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tilemap Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        
        // 初始uniform
        let uniform = TilemapUniform {
            view_offset: [0.0, 0.0],
            screen_size: [GAME_WIDTH as f32, GAME_HEIGHT as f32],
            tile_size: [W as f32, H as f32],
            map_size: [(NH as usize + 2) as f32, NV as f32],
        };
        
        // Uniform缓冲区
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tilemap Uniform Buffer"),
            size: std::mem::size_of::<TilemapUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Tile数据存储缓冲区
        let buffer_capacity = (NH as usize + 2) * NV as usize;
        let tile_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tilemap Data Buffer"),
            size: (buffer_capacity * std::mem::size_of::<TileData>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // 绑定组
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tilemap Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(palette_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: tile_data_buffer.as_entire_binding(),
                },
            ],
        });
        
        Self {
            pipeline,
            uniform_buffer,
            tile_data_buffer,
            bind_group,
            bind_group_layout,
            uniform,
            buffer_capacity,
        }
    }
    
    /// 更新视口偏移
    pub fn update_view(&mut self, queue: &wgpu::Queue, x_view: i32, y_view: i32) {
        self.uniform.view_offset = [x_view as f32, y_view as f32];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniform));
    }
    
    /// 上传tile数据
    pub fn upload_tile_data(&self, queue: &wgpu::Queue, data: &TilemapData) {
        let flat_data = data.as_flat_data();
        queue.write_buffer(&self.tile_data_buffer, 0, bytemuck::cast_slice(&flat_data));
    }
    
    /// 渲染tilemap
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
