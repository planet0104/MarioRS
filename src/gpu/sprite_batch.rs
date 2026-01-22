// 精灵批处理 - 收集渲染指令用于GPU批量渲染

use super::texture_atlas::SpriteUV;
use super::{ATLAS_SIZE, FillRect, SpriteInstance};

// 精灵绘制命令
#[derive(Clone, Debug)]
pub struct SpriteCommand {
    // 屏幕位置 (像素)
    pub x: f32,
    pub y: f32,
    // 精灵UV信息
    pub uv: SpriteUV,
    // 翻转
    pub flip_x: bool,
    pub flip_y: bool,
    // 不透明绘制标志: true表示索引0也参与绘制(对齐PutImage语义)
    pub opaque: bool,
    // 旋转: 0=0度, 1=90度, 2=180度, 3=270度
    pub rotation: u8,
    // 调色板偏移
    pub palette_offset: i32,
    // 调色板索引
    pub palette_index: u32,
}

impl SpriteCommand {
    pub fn new(x: i32, y: i32, uv: SpriteUV) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
            uv,
            flip_x: false,
            flip_y: false,
            opaque: false,
            rotation: 0,
            palette_offset: 0,
            palette_index: 0,
        }
    }

    pub fn with_flip(mut self, flip_x: bool, flip_y: bool) -> Self {
        self.flip_x = flip_x;
        self.flip_y = flip_y;
        self
    }

    /// 设置上下颠倒（等同于flip_y=true）
    pub fn with_upside_down(mut self) -> Self {
        self.flip_y = true;
        self
    }

    pub fn with_palette(mut self, offset: i32, index: u32) -> Self {
        self.palette_offset = offset;
        self.palette_index = index;
        self
    }

    pub fn with_opaque(mut self, opaque: bool) -> Self {
        self.opaque = opaque;
        self
    }

    pub fn with_rotation(mut self, rotation: u8) -> Self {
        self.rotation = rotation % 4;
        self
    }

    // 转换为GPU实例数据
    pub fn to_instance(&self) -> SpriteInstance {
        let (uv_x, uv_y, uv_w, uv_h) = self.uv.normalized(ATLAS_SIZE);
        SpriteInstance::new(
            self.x,
            self.y,
            self.uv.width as f32,
            self.uv.height as f32,
            uv_x,
            uv_y,
            uv_w,
            uv_h,
        )
        .with_flip(self.flip_x, self.flip_y)
        .with_palette(self.palette_offset as f32, self.palette_index as f32)
        .with_opaque(self.opaque)
        .with_rotation(self.rotation)
    }

    /// 转换为部分可见的GPU实例（用于升起动画）
    /// visible_height: 可见的高度（像素）
    pub fn to_partial_instance(&self, visible_height: f32) -> SpriteInstance {
        let (uv_x, uv_y, uv_w, uv_h) = self.uv.normalized(ATLAS_SIZE);
        let full_height = self.uv.height as f32;

        // 只显示底部的visible_height像素
        // 调整Y位置和UV
        let clip_ratio = visible_height / full_height;
        let clipped_uv_h = uv_h * clip_ratio;
        let clipped_uv_y = uv_y + uv_h - clipped_uv_h; // 从底部开始
        let clipped_y = self.y + full_height - visible_height;

        SpriteInstance::new(
            self.x,
            clipped_y,
            self.uv.width as f32,
            visible_height,
            uv_x,
            clipped_uv_y,
            uv_w,
            clipped_uv_h,
        )
        .with_flip(self.flip_x, self.flip_y)
        .with_palette(self.palette_offset as f32, self.palette_index as f32)
        .with_opaque(self.opaque)
    }
}

// 填充命令
#[derive(Clone, Debug)]
pub struct FillCommand {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color_index: u8,
    pub palette_index: u32,
}

impl FillCommand {
    pub fn new(x: i32, y: i32, width: i32, height: i32, color_index: u8) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: height as f32,
            color_index,
            palette_index: 0,
        }
    }

    pub fn with_palette(mut self, index: u32) -> Self {
        self.palette_index = index;
        self
    }

    pub fn to_fill_rect(&self) -> FillRect {
        FillRect::new(
            self.x,
            self.y,
            self.width,
            self.height,
            self.color_index,
            self.palette_index,
        )
    }
}

// 渲染批次收集器
pub struct SpriteBatch {
    sprites: Vec<SpriteCommand>,
    fills: Vec<FillCommand>,
    /// UI层的fills，在所有sprites之后渲染（用于状态栏等）
    ui_fills: Vec<FillCommand>,
    instances: Vec<SpriteInstance>,
    current_palette: u32,
}

impl SpriteBatch {
    pub fn new() -> Self {
        Self {
            sprites: Vec::with_capacity(1024),
            fills: Vec::with_capacity(128),
            ui_fills: Vec::with_capacity(64),
            instances: Vec::with_capacity(256),
            current_palette: 0,
        }
    }

    // 清除批次
    pub fn clear(&mut self) {
        self.sprites.clear();
        self.fills.clear();
        self.ui_fills.clear();
        self.instances.clear();
    }

    // 设置当前调色板
    pub fn set_palette(&mut self, index: u32) {
        self.current_palette = index;
    }

    // 添加精灵
    pub fn push_sprite(&mut self, cmd: SpriteCommand) {
        let mut cmd = cmd;
        if cmd.palette_index == 0 {
            cmd.palette_index = self.current_palette;
        }
        self.sprites.push(cmd);
    }

    /// 直接添加底层GPU实例（用于已经算好UV的场景）
    pub fn push_instance(&mut self, inst: SpriteInstance) {
        self.instances.push(inst);
    }

    /// 添加部分可见精灵（用于升起动画）
    pub fn push_sprite_partial(&mut self, cmd: SpriteCommand, visible_height: f32) {
        let mut cmd = cmd;
        if cmd.palette_index == 0 {
            cmd.palette_index = self.current_palette;
        }
        // 对齐 Oldsrc DrawPart：显示从顶部开始的 visible_height 像素
        // 这里直接修改 UV 高度来实现部分渲染
        let full_height = cmd.uv.height as f32;
        if visible_height >= full_height {
            self.sprites.push(cmd);
        } else if visible_height > 0.0 {
            // 只显示顶部部分
            let clip_ratio = visible_height / full_height;
            let clipped_height = (cmd.uv.height as f32 * clip_ratio) as u32;
            let mut clipped_cmd = cmd.clone();
            clipped_cmd.uv.height = clipped_height;
            self.sprites.push(clipped_cmd);
        }
    }

    // 添加填充
    pub fn push_fill(&mut self, cmd: FillCommand) {
        let mut cmd = cmd;
        if cmd.palette_index == 0 {
            cmd.palette_index = self.current_palette;
        }
        self.fills.push(cmd);
    }

    /// 添加UI层的填充（在所有sprites之后渲染，用于状态栏等）
    pub fn push_ui_fill(&mut self, cmd: FillCommand) {
        let mut cmd = cmd;
        if cmd.palette_index == 0 {
            cmd.palette_index = self.current_palette;
        }
        self.ui_fills.push(cmd);
    }

    /// 直接添加简单精灵（便捷方法）
    pub fn add_sprite(&mut self, x: i32, y: i32, uv: SpriteUV) {
        self.push_sprite(SpriteCommand::new(x, y, uv));
    }

    /// 添加翻转的精灵
    pub fn add_sprite_flipped(&mut self, x: i32, y: i32, uv: SpriteUV, flip_x: bool, flip_y: bool) {
        self.push_sprite(SpriteCommand::new(x, y, uv).with_flip(flip_x, flip_y));
    }

    /// 添加上下颠倒的精灵
    pub fn add_sprite_upside_down(&mut self, x: i32, y: i32, uv: SpriteUV) {
        self.push_sprite(SpriteCommand::new(x, y, uv).with_upside_down());
    }

    /// 添加带调色板偏移的精灵
    pub fn add_sprite_recolored(&mut self, x: i32, y: i32, uv: SpriteUV, palette_offset: i32) {
        self.push_sprite(SpriteCommand::new(x, y, uv).with_palette(palette_offset, 0));
    }

    /// 添加填充矩形（便捷方法）
    pub fn add_fill(&mut self, x: i32, y: i32, w: i32, h: i32, color_index: u8) {
        self.push_fill(FillCommand::new(x, y, w, h, color_index));
    }

    // 获取精灵实例数据（分配新Vec，保留兼容性）
    pub fn sprite_instances(&self) -> Vec<SpriteInstance> {
        let mut out: Vec<SpriteInstance> = self.sprites.iter().map(|s| s.to_instance()).collect();
        out.extend_from_slice(&self.instances);
        out
    }

    // 获取填充矩形数据（分配新Vec，保留兼容性）
    pub fn fill_rects(&self) -> Vec<FillRect> {
        self.fills.iter().map(|f| f.to_fill_rect()).collect()
    }

    /// 获取UI层填充矩形数据（分配新Vec，保留兼容性）
    pub fn ui_fill_rects(&self) -> Vec<FillRect> {
        self.ui_fills.iter().map(|f| f.to_fill_rect()).collect()
    }

    // ========================================================================
    // 零分配迭代器方法（性能优化）
    // ========================================================================

    /// 迭代精灵命令（零分配）
    #[inline]
    pub fn sprites_iter(&self) -> impl Iterator<Item = &SpriteCommand> {
        self.sprites.iter()
    }

    /// 迭代直接实例（零分配）
    #[inline]
    pub fn instances_iter(&self) -> impl Iterator<Item = &SpriteInstance> {
        self.instances.iter()
    }

    /// 迭代填充命令（零分配）
    #[inline]
    pub fn fills_iter(&self) -> impl Iterator<Item = &FillCommand> {
        self.fills.iter()
    }

    /// 迭代UI层填充命令（零分配）
    #[inline]
    pub fn ui_fills_iter(&self) -> impl Iterator<Item = &FillCommand> {
        self.ui_fills.iter()
    }

    // 精灵数量
    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    // 填充数量
    pub fn fill_count(&self) -> usize {
        self.fills.len()
    }

    /// 直接实例数量
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// UI层填充数量
    pub fn ui_fill_count(&self) -> usize {
        self.ui_fills.len()
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new()
    }
}
