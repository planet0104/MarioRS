// ============================================================================
// GPU渲染模块 - 渲染数据类型和可选的wgpu硬件加速渲染
// ============================================================================
//
// 本模块包含两部分:
// 1. 渲染数据类型（始终编译）- 被游戏逻辑使用
// 2. wgpu渲染器（仅在wgpu-backend时编译）- 硬件加速渲染
//
// 模块架构:
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ 数据类型模块（始终编译）                                                │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ types.rs          数据类型定义（SpriteInstance, FillRect等）            │
// │ sprite_batch.rs   精灵批处理，收集渲染命令                              │
// │ texture_atlas.rs  纹理图集，将多个精灵打包到单个纹理                    │
// │ palette.rs        调色板管理，支持256色索引调色板                       │
// │ tilemap.rs        地图块渲染                                             │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ wgpu渲染模块（仅在wgpu-backend时编译）                                  │
// ├─────────────────────────────────────────────────────────────────────────┤
// │ buffer_pool.rs    缓冲区池管理，预分配GPU缓冲区                         │
// │ pipeline.rs       渲染管线创建，着色器编译                              │
// │ renderer.rs       GpuRenderer核心渲染器                                 │
// │ shaders/          WGSL着色器文件                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ============================================================================

// ============================================================================
// 数据类型模块（始终编译）- 被游戏逻辑使用
// ============================================================================

/// 数据类型定义模块
pub mod types;

/// 纹理图集模块
pub mod texture_atlas;

/// 精灵批处理模块
pub mod sprite_batch;

/// 调色板管理模块
pub mod palette;

// ============================================================================
// wgpu渲染模块（仅在wgpu-backend时编译）
// ============================================================================

/// 地图块渲染模块（使用wgpu）
#[cfg(feature = "wgpu-backend")]
pub mod tilemap;

/// 缓冲区池管理模块
#[cfg(feature = "wgpu-backend")]
pub mod buffer_pool;

/// 渲染管线模块
#[cfg(feature = "wgpu-backend")]
pub mod pipeline;

/// 核心渲染器模块
#[cfg(feature = "wgpu-backend")]
pub mod renderer;

// ============================================================================
// 公共类型重导出
// ============================================================================

// 从 types.rs 重导出常量和数据类型（始终可用）
pub use types::{
    ATLAS_SIZE,
    FillRect,
    GAME_HEIGHT,
    GAME_WIDTH,
    MAX_SPRITES_PER_BATCH,
    RenderCommand,
    SpriteInstance,
};

// wgpu-backend 特有的类型（需要bytemuck）
#[cfg(feature = "wgpu-backend")]
pub use types::CameraUniform;

// 从 renderer.rs 重导出核心渲染器（仅wgpu-backend）
#[cfg(feature = "wgpu-backend")]
pub use renderer::GpuRenderer;

// 从 buffer_pool.rs 重导出缓冲区池相关（仅wgpu-backend）
#[cfg(feature = "wgpu-backend")]
pub use buffer_pool::{
    BufferPoolManager, FillBufferPool, INITIAL_FILL_CAPACITY, INITIAL_SPRITE_CAPACITY,
    INITIAL_UI_FILL_CAPACITY, SpriteBufferPool,
};

// 从 pipeline.rs 重导出管线创建函数（仅wgpu-backend）
#[cfg(feature = "wgpu-backend")]
pub use pipeline::{
    FILL_SHADER,
    OVERLAY_SHADER,
    SCALE_SHADER,
    SPRITE_SHADER,
    create_fill_bind_group_layout,
    create_fill_instance_layout,
    create_fill_pipeline,
    create_overlay_bind_group_layout,
    create_overlay_pipeline,
    create_scale_bind_group_layout,
    create_scale_pipeline,
    create_sprite_bind_group_layout,
    create_sprite_instance_layout,
    create_sprite_pipeline,
};
