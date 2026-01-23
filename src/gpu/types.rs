// ============================================================================
// 渲染类型定义
// ============================================================================
//
// 本模块包含渲染所需的所有数据类型和常量定义
// 这些类型被游戏逻辑使用，无论是GPU还是CPU渲染后端
//
// 设计原则:
// 1. #[repr(C)] - 确保内存布局一致
// 2. bytemuck::Pod/Zeroable（仅wgpu-backend）- 允许安全地转换为字节数组传递给GPU
//
// ============================================================================

use crate::gpu::sprite_batch;

// ============================================================================
// 游戏渲染常量
// ============================================================================

/// 游戏画面宽度（像素）- 复古320x200分辨率
pub const GAME_WIDTH: u32 = 320;

/// 游戏画面高度（像素）- 实际可视区域
pub const GAME_HEIGHT: u32 = 182;

/// 精灵图集尺寸 - 所有精灵打包到一个1024x1024的纹理中
pub const ATLAS_SIZE: u32 = 1024;

/// 每批次最大精灵数量
pub const MAX_SPRITES_PER_BATCH: usize = 1024;

// ============================================================================
// 精灵实例数据 (SpriteInstance)
// ============================================================================
//
// wgpu教学: 实例化渲染 (Instanced Rendering)
//
// 传统渲染: 每个精灵一次DrawCall
//   for sprite in sprites { draw(sprite); }  // 1000个精灵 = 1000次DrawCall
//
// 实例化渲染: 所有精灵一次DrawCall
//   draw_instanced(sprites, count);  // 1000个精灵 = 1次DrawCall
//
// SpriteInstance 是每个精灵的实例数据，包含:
// - 位置、大小: 在屏幕上的位置和尺寸
// - UV: 在纹理图集中的位置
// - 变换: 翻转、旋转
// - 调色板: 颜色变换参数
//
// #[repr(C)]: 确保内存布局与C/GPU兼容
// bytemuck::Pod/Zeroable: 允许安全地转换为字节数组传递给GPU
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "wgpu-backend", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct SpriteInstance {
    /// 屏幕位置（像素）- 精灵左上角在屏幕上的坐标
    pub position: [f32; 2],

    /// 精灵尺寸（像素）- 宽度和高度
    pub size: [f32; 2],

    /// 纹理UV偏移（归一化0-1）- 在图集中的起始位置
    pub uv_offset: [f32; 2],

    /// 纹理UV尺寸（归一化）- 在图集中占用的区域
    pub uv_size: [f32; 2],

    /// 翻转标志 [水平, 垂直] - 1.0表示翻转
    pub flip: [f32; 2],

    /// 调色板偏移 - 用于颜色变换效果（如无敌闪烁）
    pub palette_offset: f32,

    /// 调色板索引 - 选择使用哪个调色板行（支持多调色板）
    pub palette_index: f32,

    /// 不透明标志 - 1.0表示索引0也绘制（不透明模式）
    pub opaque: f32,

    /// 旋转 - 0/1/2/3 对应 0/90/180/270度
    pub rotation: f32,
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
            opaque: 0.0,
            rotation: 0.0,
        }
    }

    pub fn with_flip(mut self, flip_x: bool, flip_y: bool) -> Self {
        self.flip = [
            if flip_x { 1.0 } else { 0.0 },
            if flip_y { 1.0 } else { 0.0 },
        ];
        self
    }

    pub fn with_palette(mut self, offset: f32, index: f32) -> Self {
        self.palette_offset = offset;
        self.palette_index = index;
        self
    }

    pub fn with_opaque(mut self, opaque: bool) -> Self {
        self.opaque = if opaque { 1.0 } else { 0.0 };
        self
    }

    pub fn with_rotation(mut self, rotation: u8) -> Self {
        self.rotation = (rotation % 4) as f32;
        self
    }
}

// ============================================================================
// 填充矩形实例 (FillRect)
// ============================================================================
//
// wgpu教学: 简化的实例数据
//
// FillRect比SpriteInstance简单，只需要:
// - 位置和大小
// - 颜色索引（从调色板查找）
//
// _padding字段: GPU通常要求数据按16字节对齐
// 添加padding确保结构体大小是16的倍数，提高性能
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "wgpu-backend", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct FillRect {
    /// 屏幕位置（像素）
    pub position: [f32; 2],
    /// 矩形尺寸（像素）
    pub size: [f32; 2],
    /// 调色板颜色索引 (0-255)
    pub color_index: f32,
    /// 调色板行索引
    pub palette_index: f32,
    /// 填充以满足16字节对齐
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

// ============================================================================
// 渲染命令枚举 (RenderCommand)
// ============================================================================
//
// wgpu教学: 命令模式 (Command Pattern)
//
// 游戏逻辑不直接调用GPU渲染，而是收集渲染命令:
// 1. 游戏逻辑遍历所有对象，生成RenderCommand
// 2. 渲染器统一处理所有命令，批量提交GPU
//
// 优点:
// - 可以对命令排序（如按层级、按纹理）
// - 可以合并相邻的相同类型命令
// - 游戏逻辑和渲染逻辑解耦
// ============================================================================

#[derive(Clone, Debug)]
pub enum RenderCommand {
    /// 绘制精灵实例（底层，已计算好所有参数）
    DrawSprite(SpriteInstance),
    /// 绘制精灵命令（高层，包含UV查找信息）
    Sprite(sprite_batch::SpriteCommand),
    /// 绘制Y轴翻转的精灵（上下颠倒）
    DrawSpriteFlipY(SpriteInstance),
    /// 绘制部分可见的精灵（用于升起动画）
    DrawSpritePart {
        sprite: SpriteInstance,
        visible_height: f32,
    },
    /// 填充矩形（背景层，在sprites之前渲染）
    FillRect(FillRect),
    /// UI层填充矩形（在sprites之后渲染，用于状态栏）
    UIFillRect(FillRect),
}

// ============================================================================
// 相机Uniform (CameraUniform)
// ============================================================================
//
// wgpu教学: Uniform缓冲区
//
// Uniform是着色器中的全局常量，对所有顶点/片段相同:
// - 与顶点数据不同（每个顶点不同）
// - 与实例数据不同（每个实例不同）
//
// CameraUniform包含:
// - view_offset: 视口在世界中的位置（用于滚动）
// - screen_size: 屏幕尺寸（用于NDC坐标转换）
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "wgpu-backend", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct CameraUniform {
    /// 视口偏移（世界坐标）
    pub view_offset: [f32; 2],
    /// 屏幕尺寸（像素）- 用于坐标转换
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
