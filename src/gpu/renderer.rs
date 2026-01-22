// ============================================================================
// GPU渲染器 (GpuRenderer)
// ============================================================================
//
// wgpu教学: 这是整个GPU渲染系统的核心
//
// GpuRenderer负责:
// 1. 管理wgpu资源（Device、Queue、纹理、缓冲区、管线）
// 2. 收集渲染命令（精灵、填充矩形）
// 3. 执行渲染流程（渲染到纹理 -> 缩放输出 -> 叠加UI）
//
// wgpu资源层级:
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ Instance                                                                │
// │   GPU实例，wgpu的入口点                                                 │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ Adapter                                                                 │
// │   物理GPU的抽象，用于查询GPU能力                                        │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ Device                                                                  │
// │   逻辑设备，用于创建所有GPU资源                                         │
// │   - 创建Buffer、Texture、Pipeline等                                     │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ Queue                                                                   │
// │   命令队列，用于提交GPU工作                                             │
// │   - write_buffer(): 上传数据                                            │
// │   - submit(): 提交渲染命令                                              │
// └─────────────────────────────────────────────────────────────────────────┘
//
// 本项目渲染流程:
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ Pass 1: 游戏渲染 (render_frame)                                         │
// │   目标: render_texture (320x182)                                        │
// │   内容: fills -> sprites -> ui_fills                                    │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ Pass 2: 缩放输出 (render_to_surface)                                    │
// │   目标: window surface (任意尺寸)                                       │
// │   内容: 等比例缩放游戏画面                                              │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ Pass 3: UI叠加 (render_to_surface)                                      │
// │   目标: window surface                                                  │
// │   内容: 触摸面板、FPS显示等                                             │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ============================================================================

use std::sync::Arc;
use wgpu::util::DeviceExt;

use crate::gpu::buffer_pool::{
    INITIAL_FILL_CAPACITY, INITIAL_SPRITE_CAPACITY, INITIAL_UI_FILL_CAPACITY,
};
use crate::gpu::pipeline;
use crate::gpu::types::{
    ATLAS_SIZE, CameraUniform, FillRect, GAME_HEIGHT, GAME_WIDTH, MAX_SPRITES_PER_BATCH,
    SpriteInstance,
};

// ============================================================================
// GpuRenderer 结构体
// ============================================================================

pub struct GpuRenderer {
    // ========================================================================
    // wgpu核心对象
    // ========================================================================
    /// GPU逻辑设备 - 用于创建所有GPU资源
    pub device: Arc<wgpu::Device>,

    /// 命令队列 - 用于上传数据和提交渲染命令
    pub queue: Arc<wgpu::Queue>,

    /// 纹理采样器 - 控制纹理采样方式（Nearest过滤，像素风格）
    pub sampler: wgpu::Sampler,

    // ========================================================================
    // 渲染目标纹理
    // ========================================================================
    /// 游戏画面渲染目标 (320x182)
    pub render_texture: wgpu::Texture,
    pub render_texture_view: wgpu::TextureView,

    // ========================================================================
    // 游戏资源纹理
    // ========================================================================
    /// 精灵图集纹理 (R8格式，存储调色板索引)
    pub atlas_texture: wgpu::Texture,
    pub atlas_texture_view: wgpu::TextureView,

    /// 调色板纹理 (256x64 RGBA8)
    pub palette_texture: wgpu::Texture,
    pub palette_texture_view: wgpu::TextureView,

    // ========================================================================
    // 渲染管线
    // ========================================================================
    /// 精灵渲染管线
    pub sprite_pipeline: wgpu::RenderPipeline,
    pub sprite_bind_group: wgpu::BindGroup,

    /// 填充矩形渲染管线
    pub fill_pipeline: wgpu::RenderPipeline,
    pub fill_bind_group: wgpu::BindGroup,

    /// 缩放输出管线
    pub scale_pipeline: wgpu::RenderPipeline,
    pub scale_bind_group: wgpu::BindGroup,

    /// 叠加层（Android触摸面板/FPS等）
    pub overlay_texture: wgpu::Texture,
    pub overlay_texture_view: wgpu::TextureView,
    pub overlay_bind_group_layout: wgpu::BindGroupLayout,
    pub overlay_bind_group: wgpu::BindGroup,
    pub overlay_pipeline: wgpu::RenderPipeline,
    pub overlay_size: [u32; 2],

    // ========================================================================
    // Uniform缓冲区
    // ========================================================================
    pub camera_buffer: wgpu::Buffer,
    pub scale_buffer: wgpu::Buffer,

    // ========================================================================
    // 预分配顶点缓冲区
    // ========================================================================
    pub sprite_buffer: wgpu::Buffer,
    pub sprite_buffer_capacity: usize,
    pub fill_buffer: wgpu::Buffer,
    pub fill_buffer_capacity: usize,
    pub ui_fill_buffer: wgpu::Buffer,
    pub ui_fill_buffer_capacity: usize,

    // ========================================================================
    // CPU端渲染数据
    // ========================================================================
    pub sprite_instances: Vec<SpriteInstance>,
    pub fill_rects: Vec<FillRect>,
    pub ui_fill_rects: Vec<FillRect>,

    // ========================================================================
    // 渲染状态
    // ========================================================================
    pub current_palette: u32,
    pub camera: CameraUniform,
}

impl GpuRenderer {
    /// 创建GPU渲染器
    ///
    /// wgpu教学: 渲染器初始化流程
    /// 1. 创建采样器
    /// 2. 创建纹理（渲染目标、图集、调色板、叠加层）
    /// 3. 创建Uniform缓冲区
    /// 4. 创建绑定组布局和绑定组
    /// 5. 创建渲染管线
    /// 6. 预分配顶点缓冲区
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // 创建采样器（Nearest过滤，保持像素风格）
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // 创建渲染目标纹理
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let render_texture_view =
            render_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let palette_texture_view =
            palette_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建相机uniform缓冲区
        let camera = CameraUniform::new(0, 0);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 创建精灵管线
        let sprite_bind_group_layout = pipeline::create_sprite_bind_group_layout(&device);
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
        let sprite_pipeline = pipeline::create_sprite_pipeline(&device, &sprite_bind_group_layout);

        // 创建填充管线
        let fill_bind_group_layout = pipeline::create_fill_bind_group_layout(&device);
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
        let fill_pipeline = pipeline::create_fill_pipeline(&device, &fill_bind_group_layout);

        // 创建缩放管线
        let scale_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scale_buffer"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scale_bind_group_layout = pipeline::create_scale_bind_group_layout(&device);
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
        let scale_pipeline =
            pipeline::create_scale_pipeline(&device, &scale_bind_group_layout, surface_format);

        // 创建叠加层管线
        let overlay_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let overlay_texture_view =
            overlay_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let overlay_bind_group_layout = pipeline::create_overlay_bind_group_layout(&device);
        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay_bind_group"),
            layout: &overlay_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&overlay_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let overlay_pipeline =
            pipeline::create_overlay_pipeline(&device, &overlay_bind_group_layout, surface_format);

        // 预分配顶点缓冲区
        let sprite_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_buffer_pooled"),
            size: (INITIAL_SPRITE_CAPACITY * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let fill_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fill_buffer_pooled"),
            size: (INITIAL_FILL_CAPACITY * std::mem::size_of::<FillRect>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_fill_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_fill_buffer_pooled"),
            size: (INITIAL_UI_FILL_CAPACITY * std::mem::size_of::<FillRect>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            sampler,
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
            overlay_texture,
            overlay_texture_view,
            overlay_bind_group_layout,
            overlay_bind_group,
            overlay_pipeline,
            overlay_size: [1, 1],
            camera_buffer,
            scale_buffer,
            sprite_buffer,
            sprite_buffer_capacity: INITIAL_SPRITE_CAPACITY,
            fill_buffer,
            fill_buffer_capacity: INITIAL_FILL_CAPACITY,
            ui_fill_buffer,
            ui_fill_buffer_capacity: INITIAL_UI_FILL_CAPACITY,
            sprite_instances: Vec::with_capacity(MAX_SPRITES_PER_BATCH),
            fill_rects: Vec::with_capacity(128),
            ui_fill_rects: Vec::with_capacity(64),
            current_palette: 0,
            camera,
        }
    }

    // ========================================================================
    // 资源上传方法
    // ========================================================================

    /// 上传叠加层RGBA数据到GPU
    pub fn upload_overlay_rgba(&mut self, width: u32, height: u32, rgba: &[u8]) {
        if width == 0 || height == 0 {
            return;
        }
        let expected = (width * height * 4) as usize;
        if rgba.len() != expected {
            eprintln!("[GPU] overlay大小不匹配: {} vs {}", rgba.len(), expected);
            return;
        }
        self.ensure_overlay_texture(width, height);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.overlay_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// 清空叠加层
    pub fn clear_overlay(&mut self) {
        self.ensure_overlay_texture(1, 1);
        let transparent = [0u8, 0u8, 0u8, 0u8];
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.overlay_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &transparent,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    fn ensure_overlay_texture(&mut self, width: u32, height: u32) {
        if self.overlay_size[0] == width && self.overlay_size[1] == height {
            return;
        }

        let overlay_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay_texture"),
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
        let overlay_texture_view =
            overlay_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let overlay_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay_bind_group"),
            layout: &self.overlay_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&overlay_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.overlay_texture = overlay_texture;
        self.overlay_texture_view = overlay_texture_view;
        self.overlay_bind_group = overlay_bind_group;
        self.overlay_size = [width, height];
    }

    /// 上传精灵图集到GPU
    pub fn upload_atlas(&self, data: &[u8], width: u32, height: u32) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
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

    /// 上传调色板到GPU
    pub fn upload_palette(&self, row: u32, colors: &[[u8; 4]; 256]) {
        let data: Vec<u8> = colors.iter().flat_map(|c| c.iter().copied()).collect();
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.palette_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: row, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
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

    // ========================================================================
    // 状态更新方法
    // ========================================================================

    /// 更新相机位置
    pub fn update_camera(&mut self, x_view: i32, y_view: i32) {
        self.camera = CameraUniform::new(x_view, y_view);
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
    }

    /// 更新缩放参数
    pub fn update_scale(&self, window_width: u32, window_height: u32) {
        if window_width == 0 || window_height == 0 {
            return;
        }

        let scale_x = window_width as f32 / GAME_WIDTH as f32;
        let scale_y = window_height as f32 / GAME_HEIGHT as f32;
        let scale = scale_x.min(scale_y);

        let display_width = GAME_WIDTH as f32 * scale;
        let display_height = GAME_HEIGHT as f32 * scale;

        let offset_x = (window_width as f32 - display_width) / 2.0;
        let offset_y = (window_height as f32 - display_height) / 2.0;

        let ndc_scale_x = display_width / window_width as f32;
        let ndc_scale_y = display_height / window_height as f32;
        let ndc_offset_x = (offset_x * 2.0 / window_width as f32) - (1.0 - ndc_scale_x);
        let ndc_offset_y = -((offset_y * 2.0 / window_height as f32) - (1.0 - ndc_scale_y));

        let scale_data = [ndc_scale_x, ndc_scale_y, ndc_offset_x, ndc_offset_y];
        self.queue
            .write_buffer(&self.scale_buffer, 0, bytemuck::cast_slice(&scale_data));
    }

    /// 设置当前调色板索引
    pub fn set_palette(&mut self, index: u32) {
        self.current_palette = index;
    }

    // ========================================================================
    // 渲染命令收集
    // ========================================================================

    /// 清除当前帧批次
    pub fn begin_frame(&mut self) {
        self.sprite_instances.clear();
        self.fill_rects.clear();
        self.ui_fill_rects.clear();
    }

    /// 添加精灵到批次
    pub fn draw_sprite(&mut self, instance: SpriteInstance) {
        self.sprite_instances.push(instance);
    }

    /// 添加填充矩形到批次
    pub fn draw_fill(&mut self, rect: FillRect) {
        self.fill_rects.push(rect);
    }

    /// 添加UI层填充矩形到批次
    pub fn draw_ui_fill(&mut self, rect: FillRect) {
        self.ui_fill_rects.push(rect);
    }

    // ========================================================================
    // 缓冲区管理
    // ========================================================================

    fn ensure_sprite_buffer_capacity(&mut self, required: usize) {
        if required > self.sprite_buffer_capacity {
            let new_capacity = required.next_power_of_two().max(64);
            self.sprite_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_buffer_pooled"),
                size: (new_capacity * std::mem::size_of::<SpriteInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.sprite_buffer_capacity = new_capacity;
        }
    }

    fn ensure_fill_buffer_capacity(&mut self, required: usize) {
        if required > self.fill_buffer_capacity {
            let new_capacity = required.next_power_of_two().max(64);
            self.fill_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fill_buffer_pooled"),
                size: (new_capacity * std::mem::size_of::<FillRect>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.fill_buffer_capacity = new_capacity;
        }
    }

    fn ensure_ui_fill_buffer_capacity(&mut self, required: usize) {
        if required > self.ui_fill_buffer_capacity {
            let new_capacity = required.next_power_of_two().max(64);
            self.ui_fill_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui_fill_buffer_pooled"),
                size: (new_capacity * std::mem::size_of::<FillRect>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.ui_fill_buffer_capacity = new_capacity;
        }
    }

    // ========================================================================
    // 渲染执行
    // ========================================================================

    /// 渲染当前帧到内部纹理
    pub fn render_frame(&mut self) {
        // 确保缓冲区容量足够
        self.ensure_sprite_buffer_capacity(self.sprite_instances.len().max(1));
        self.ensure_fill_buffer_capacity(self.fill_rects.len().max(1));
        self.ensure_ui_fill_buffer_capacity(self.ui_fill_rects.len().max(1));

        // 上传数据到GPU
        if !self.sprite_instances.is_empty() {
            self.queue.write_buffer(
                &self.sprite_buffer,
                0,
                bytemuck::cast_slice(&self.sprite_instances),
            );
        }
        if !self.fill_rects.is_empty() {
            self.queue
                .write_buffer(&self.fill_buffer, 0, bytemuck::cast_slice(&self.fill_rects));
        }
        if !self.ui_fill_rects.is_empty() {
            self.queue.write_buffer(
                &self.ui_fill_buffer,
                0,
                bytemuck::cast_slice(&self.ui_fill_rects),
            );
        }

        // 创建命令编码器
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // 开始渲染Pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("game_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // 渲染填充矩形（背景层）
            if !self.fill_rects.is_empty() {
                render_pass.set_pipeline(&self.fill_pipeline);
                render_pass.set_bind_group(0, &self.fill_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.fill_buffer.slice(..));
                render_pass.draw(0..6, 0..self.fill_rects.len() as u32);
            }

            // 渲染精灵（实体层）
            if !self.sprite_instances.is_empty() {
                render_pass.set_pipeline(&self.sprite_pipeline);
                render_pass.set_bind_group(0, &self.sprite_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.sprite_buffer.slice(..));
                render_pass.draw(0..6, 0..self.sprite_instances.len() as u32);
            }

            // 渲染UI层填充矩形
            if !self.ui_fill_rects.is_empty() {
                render_pass.set_pipeline(&self.fill_pipeline);
                render_pass.set_bind_group(0, &self.fill_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.ui_fill_buffer.slice(..));
                render_pass.draw(0..6, 0..self.ui_fill_rects.len() as u32);
            }
        }

        // 提交命令到GPU队列
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// 渲染到窗口surface（缩放输出 + 叠加层）
    pub fn render_to_surface(&self, surface_view: &wgpu::TextureView) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scale_encoder"),
            });

        // 缩放Pass
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
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.scale_pipeline);
            render_pass.set_bind_group(0, &self.scale_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // 叠加层Pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.overlay_pipeline);
            render_pass.set_bind_group(0, &self.overlay_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
