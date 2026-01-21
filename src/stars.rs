// STARS.PAS interface 对应的 Rust 模块
// 自动转换自 Pascal 结构体和全局变量

use crate::{
    buffers::{Buffers, W, WorldOptions},
    gpu::{FillRect, RenderCommand},
    render_state::RenderState,
};

pub const STAR_SPEED: i32 = 10;
pub const MAX: i32 = (crate::buffers::MAX_WORLD_SIZE / STAR_SPEED) * W;

pub struct Stars {
    /// 星星位置映射，长度320
    pub star_map: [u16; 320],
    /// 上一次每页的X坐标，长度4
    pub last_x: [i32; 4],
    /// 闪烁计数器
    pub blink_counter: i32,
    /// 星星颜色1
    pub c1: u8,
    /// 星星颜色2
    pub c2: u8,
}

impl Stars {
    pub fn new() -> Self {
        Self {
            star_map: [0u16; 320],
            last_x: [0i32; 4],
            blink_counter: 0,
            c1: 0,
            c2: 0,
        }
    }

    /// 清空星星背景和 last_x
    pub fn clear_stars(&mut self, buffers: &mut Buffers) {
        let star_backgr = buffers.star_backgr.as_mut();
        for i in 0..star_backgr.len() {
            star_backgr[i] = [0u8; 320];
        }

        for i in 0..self.last_x.len() {
            self.last_x[i] = 0;
        }
    }

    pub fn init_stars(&mut self, buffers: &mut Buffers, options: &WorldOptions) {
        use crate::utils::random_i32;
        self.clear_stars(buffers);
        // RandSeed := 0;  // Rust的rand库不直接支持全局种子，忽略
        for i in 0..320 {
            self.star_map[i] = ((random_i32(options.horizon as i32) as u16) * 320 + i as u16) as u16;
        }
        if options.stars == 1 || options.stars == 2 {
            for i in 0..320 {
                if random_i32(10) > 2 {
                    self.star_map[i] = 0;
                }
            }
        }
        match options.stars {
            1 => {
                self.c1 = 29;
                self.c2 = 31;
            }
            2 => {
                self.c1 = 90;
                self.c2 = 92;
            }
            _ => {}
        }
    }

    /// GPU版 - 显示星星
    pub fn show_stars(&mut self, render_state: &mut RenderState, buffers: &Buffers) {
        use crate::utils::random_i32;
        
        let x_view = buffers.x_view;
        let x_offset = (8 * x_view) / STAR_SPEED as i32;
        
        // 随机闪烁计数器
        self.blink_counter = random_i32(320);
        let mut bx = self.blink_counter;
        
        // 主循环
        for i in 0..320 {
            let star_pos = self.star_map[i];
            if star_pos == 0 {
                continue;
            }
            let ax = star_pos as i32 + x_offset;
            if ax < 0 || ax >= 320 {
                continue;
            }
            let y = (star_pos as i32 / 320) as i32;
            
            // GPU模式下无法直接读取像素，假设可以绘制
            // 选择颜色（闪烁效果）
            let mut al = self.c1;
            bx -= 1;
            if bx == 0 {
                al = self.c2;
                bx = self.blink_counter;
            }
            
            // 使用GPU填充1x1像素
            render_state.fill_gpu(ax, y, 1, 1, al);
        }
    }

    // GPU模式下不需要隐藏操作，每帧重绘

    /// GPU渲染: 收集星星像素
    /// 星星是单像素效果，使用1x1的填充矩形来渲染
    pub fn collect_stars_gpu(
        &mut self,
        commands: &mut Vec<RenderCommand>,
        buffers: &Buffers,
        palette_index: u32,
    ) {
        use crate::utils::random_i32;
        
        let x_view = buffers.x_view;
        let x_offset = (8 * x_view) / STAR_SPEED as i32;
        
        // 更新闪烁计数器
        self.blink_counter = random_i32(320);
        let mut bx = self.blink_counter;
        
        for i in 0..320 {
            let star_pos = self.star_map[i];
            if star_pos == 0 {
                continue;
            }
            
            let ax = star_pos as i32 + x_offset;
            if ax < 0 || ax >= 320 {
                continue;
            }
            
            let y = (star_pos as i32 / 320) as i32;
            
            // 选择颜色（闪烁效果）
            let mut color = self.c1;
            bx -= 1;
            if bx == 0 {
                color = self.c2;
                bx = self.blink_counter;
            }
            
            // 使用1x1像素的填充矩形
            let fill = FillRect::new(ax as f32, y as f32, 1.0, 1.0, color, palette_index);
            commands.push(RenderCommand::FillRect(fill));
        }
    }
}

// tests removed: pure wgpu mode does not keep CPU framebuffer snapshots.
