// ============================================================================
// GPU渲染模块 - 基于wgpu的硬件加速渲染
// ============================================================================
//
// wgpu教学: 本模块是整个游戏的GPU渲染核心
//
// 模块架构:
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ 核心模块                                                                 │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ types.rs          数据类型定义（SpriteInstance, FillRect等）            │
// │ buffer_pool.rs    缓冲区池管理，预分配GPU缓冲区                         │
// │ pipeline.rs       渲染管线创建，着色器编译                              │
// │ renderer.rs       GpuRenderer核心渲染器                                 │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ 辅助模块                                                                 │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ sprite_batch.rs   精灵批处理，收集渲染命令                              │
// │ texture_atlas.rs  纹理图集，将多个精灵打包到单个纹理                    │
// │ palette.rs        调色板管理，支持256色索引调色板                       │
// │ tilemap.rs        地图块渲染                                             │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ 着色器                                                                   │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ shaders/          WGSL着色器文件                                         │
// │   sprite.wgsl     精灵渲染（实例化）                                     │
// │   fill.wgsl       填充矩形渲染                                           │
// │   scale.wgsl      缩放输出到窗口                                         │
// │   overlay.wgsl    UI叠加层                                               │
// └─────────────────────────────────────────────────────────────────────────┘
//
// 渲染流程:
// 1. 游戏逻辑收集渲染命令 (SpriteInstance, FillRect)
// 2. GpuRenderer.render_frame() 渲染到内部纹理 (320x182)
// 3. GpuRenderer.render_to_surface() 缩放输出到窗口
//
// 使用示例:
// ```rust
// // 创建渲染器
// let mut gpu = GpuRenderer::new(device, queue, surface_format);
//
// // 每帧渲染
// gpu.begin_frame();
// gpu.draw_fill(FillRect::new(...));  // 背景
// gpu.draw_sprite(SpriteInstance::new(...));  // 精灵
// gpu.render_frame();  // 渲染到内部纹理
// gpu.render_to_surface(&surface_view);  // 输出到窗口
// ```
//
// ============================================================================

// ============================================================================
// 子模块声明
// ============================================================================

/// 数据类型定义模块
pub mod types;

/// 缓冲区池管理模块
pub mod buffer_pool;

/// 渲染管线模块
pub mod pipeline;

/// 核心渲染器模块
pub mod renderer;

/// 纹理图集模块
pub mod texture_atlas;

/// 精灵批处理模块
pub mod sprite_batch;

/// 调色板管理模块
pub mod palette;

/// 地图块渲染模块
pub mod tilemap;

// ============================================================================
// 公共类型重导出
//
// wgpu教学: 使用 pub use 简化外部导入
// 外部代码可以直接使用 crate::gpu::GpuRenderer 而不是 crate::gpu::renderer::GpuRenderer
// ============================================================================

// 从 types.rs 重导出常量和数据类型
pub use types::{
    ATLAS_SIZE,
    CameraUniform,
    FillRect,
    GAME_HEIGHT,
    // 常量
    GAME_WIDTH,
    MAX_SPRITES_PER_BATCH,
    RenderCommand,
    // 数据类型
    SpriteInstance,
};

// 从 renderer.rs 重导出核心渲染器
pub use renderer::GpuRenderer;

// 从 buffer_pool.rs 重导出缓冲区池相关
pub use buffer_pool::{
    BufferPoolManager, FillBufferPool, INITIAL_FILL_CAPACITY, INITIAL_SPRITE_CAPACITY,
    INITIAL_UI_FILL_CAPACITY, SpriteBufferPool,
};

// 从 pipeline.rs 重导出管线创建函数
pub use pipeline::{
    FILL_SHADER,
    OVERLAY_SHADER,
    SCALE_SHADER,
    // 着色器源码
    SPRITE_SHADER,
    create_fill_bind_group_layout,
    create_fill_instance_layout,
    create_fill_pipeline,
    create_overlay_bind_group_layout,
    create_overlay_pipeline,
    create_scale_bind_group_layout,
    create_scale_pipeline,
    // 绑定组布局
    create_sprite_bind_group_layout,
    // 顶点布局
    create_sprite_instance_layout,
    // 管线创建
    create_sprite_pipeline,
};
