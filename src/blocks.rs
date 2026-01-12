// Blocks.PAS interface 对应的 Rust 模块
// 方块碰撞和动画管理模块

use crate::{
    backgr::BackGr,
    buffers::{H, ImageBuffer, W, WorldOptions},
    sprites,
    vga256::VGA,
};

pub const BUMP_HEIGHT: i32 = 4;
pub const MOVE_DELAY: i32 = 0;

pub struct Blocks {
    /// 背景缓冲区 [1..W*(H+BumpHeight)]
    pub backgr_buffer: Vec<u8>,
    /// 方块图像缓冲区
    pub block_buffer: ImageBuffer,
    /// 是否正在碰撞动画
    pub bumping: bool,
    /// 当前碰撞方块坐标
    pub bump_x: i32,
    pub bump_y: i32,
    /// 上一次碰撞方块坐标
    pub old_bump_x: i32,
    pub old_bump_y: i32,
    /// Y方向偏移
    pub dy: i32,
    /// Y位置
    pub y_pos: i32,
    /// 延迟计数器
    pub delay_counter: i32,
    // pub bump_fill_attr: u8, // 可选
}

impl Blocks {
    pub fn new() -> Self {
        Self {
            backgr_buffer: vec![0u8; (W * (H + BUMP_HEIGHT)) as usize],
            block_buffer: [[0u8; W as usize]; H as usize],
            bumping: false,
            bump_x: 0,
            bump_y: 0,
            old_bump_x: 0,
            old_bump_y: 0,
            dy: 0,
            y_pos: 0,
            delay_counter: 0,
        }
    }

    /// 初始化方块动画状态
    pub fn init_blocks(&mut self) {
        self.bumping = false;
    }

    /// 保存碰撞前的背景
    pub fn save_bump_backgr(&mut self, vga: &VGA) {
        // 保存背景到 backgr_buffer
        let y = self.bump_y - BUMP_HEIGHT;
        vga.get_image_world(
            self.bump_x,
            y,
            W as i32,
            H as i32 + BUMP_HEIGHT,
            &mut self.backgr_buffer,
        );
        self.old_bump_x = self.bump_x;
        self.old_bump_y = self.bump_y;
    }

    /// 触发方块碰撞动画
    pub fn bump_block(&mut self, x: i32, y: i32, vga: &VGA) {
        if self.bumping {
            return;
        }
        self.bump_x = x;
        self.bump_y = y;
        self.dy = -BUMP_HEIGHT;
        // 获取当前方块图像
        vga.get_image_imagebuffer_world(x, y, &mut self.block_buffer);
        self.save_bump_backgr(vga);
        self.bumping = true;
        self.delay_counter = 0;
    }

    /// 擦除动画中的方块
    pub fn erase_blocks(&mut self, vga: &mut VGA) {
        if self.bumping {
            let y = self.old_bump_y - BUMP_HEIGHT;
            vga.put_image_world(
                self.old_bump_x,
                y,
                W as i32,
                H as i32 + BUMP_HEIGHT,
                &self.backgr_buffer,
            );
        }
    }

    /// 绘制动画中的方块
    pub fn draw_blocks(
        &mut self,
        vga: &mut VGA,
        backgr: &mut BackGr,
        options: &WorldOptions,
        sprites: &sprites::SpriteDataManager,
    ) {
        if self.bumping {
            if self.dy < BUMP_HEIGHT {
                self.save_bump_backgr(vga);
                let y = self.bump_y - BUMP_HEIGHT + self.dy.abs();
                vga.put_image_imagebuffer_world(self.bump_x, y, &self.block_buffer);
                // 填充下方背景
                backgr.draw_backgr_block(
                    self.bump_x,
                    y + H as i32,
                    W as i32,
                    BUMP_HEIGHT - self.dy.abs(),
                    vga,
                    options,
                    sprites,
                );
            } else if self.delay_counter >= 4 {
                self.bumping = false;
            }
        }
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
}
