// Blocks.PAS interface 对应的 Rust 模块
// 方块碰撞和动画管理模块 - GPU版本

use crate::{
    gpu::{RenderCommand, SpriteInstance},
    sprites::{SpriteAtlas, SpriteId},
};

pub const BUMP_HEIGHT: i32 = 4;
pub const MOVE_DELAY: i32 = 0;

pub struct Blocks {
    /// 是否正在碰撞动画
    pub bumping: bool,
    /// 当前碰撞方块坐标
    pub bump_x: i32,
    pub bump_y: i32,
    /// Y方向偏移
    pub dy: i32,
    /// 延迟计数器
    pub delay_counter: i32,
    /// 当前碰撞方块的精灵ID
    pub bump_sprite_id: SpriteId,
}

impl Blocks {
    pub fn new() -> Self {
        Self {
            bumping: false,
            bump_x: 0,
            bump_y: 0,
            dy: 0,
            delay_counter: 0,
            bump_sprite_id: SpriteId::QUEST_000,
        }
    }

    /// 初始化方块动画状态
    pub fn init_blocks(&mut self) {
        self.bumping = false;
    }

    /// 触发方块碰撞动画（GPU版本）
    pub fn bump_block(&mut self, x: i32, y: i32, sprite_id: SpriteId) {
        if self.bumping {
            return;
        }
        self.bump_x = x;
        self.bump_y = y;
        self.dy = -BUMP_HEIGHT;
        self.bump_sprite_id = sprite_id;
        self.bumping = true;
        self.delay_counter = 0;
    }

    /// 推进动画帧
    pub fn move_blocks(&mut self) {
        if self.bumping {
            self.delay_counter += 1;
            if self.delay_counter > MOVE_DELAY && self.dy < BUMP_HEIGHT {
                self.dy += 1;
                self.delay_counter = 0;
            }
        }
    }

    /// GPU渲染: 收集碰撞动画方块的精灵命令
    pub fn collect_bump_sprites_gpu(
        &self,
        commands: &mut Vec<RenderCommand>,
        x_view: i32,
        y_view: i32,
        atlas: &SpriteAtlas,
    ) {
        if !self.bumping || self.dy >= BUMP_HEIGHT {
            return;
        }
        
        // 计算碰撞方块的当前位置
        let block_y = self.bump_y - BUMP_HEIGHT + self.dy.abs();
        
        // 转换为屏幕坐标
        let sx = (self.bump_x - x_view) as f32;
        let sy = (block_y - y_view) as f32;
        
        let uv = atlas.get(self.bump_sprite_id);
        let (u, v, u_size, v_size) = uv.normalized(atlas.size());
        let inst = SpriteInstance::new(
            sx, sy,
            uv.width as f32, uv.height as f32,
            u, v, u_size, v_size
        );
        commands.push(RenderCommand::DrawSprite(inst));
    }
}
