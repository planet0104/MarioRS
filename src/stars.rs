// STARS.PAS interface 对应的 Rust 模块
// 自动转换自 Pascal 结构体和全局变量

use crate::{
    buffers::{Buffers, W, WorldOptions},
    vga256::VGA,
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
        for i in 0..buffers.star_backgr.len() {
            buffers.star_backgr[i] = [0u8; 320];
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

    pub fn show_stars(&mut self, vga: &mut VGA, buffers: &mut Buffers) {
        use crate::utils::random_i32;
        // 当前页号
        let current_page = vga.current_page();
        // 当前视口X
        let x_view = buffers.x_view;
        // 记录本页的X
        self.last_x[current_page as usize] = x_view;
        // 星星偏移
        let x_offset = (8 * x_view) / STAR_SPEED as i32;
        // star_backgr: [页][像素]，每页320
        let star_backgr = &mut buffers.star_backgr[current_page as usize];
        // 随机闪烁计数器
        self.blink_counter = random_i32(320);
        let mut bx = self.blink_counter;
        // 清空本页星星背景
        for px in star_backgr.iter_mut() {
            *px = 0;
        }
        // 主循环，模拟Pascal汇编逻辑
        for i in 0..320 {
            let star_pos = self.star_map[i];
            if star_pos == 0 {
                continue;
            }
            let ax = star_pos as i32 + x_offset;
            if ax < 0 || ax >= 320 {
                continue;
            }
            // 取屏幕像素值（framebuffer）
            let y = (star_pos as i32 / 320) as i32;
            let dl = vga.get_pixel(ax, y);
            let mut draw = true;
            if dl == 0 || dl == 0xF0 {
                draw = true;
            } else if dl >= 0xA0 {
                draw = false;
            }
            if draw {
                // 画星星
                let mut al = self.c1;
                bx -= 1;
                if bx == 0 {
                    al = self.c2;
                    bx = self.blink_counter;
                }
                star_backgr[ax as usize] = al;
                vga.put_pixel(ax, y, al);
            }
        }
    }

    /// 恢复星星背景到屏幕（Pascal HideStars 过程的Rust实现）
    pub fn hide_stars(&mut self, vga: &mut VGA, buffers: &mut Buffers) {
        // 当前页号
        let current_page = vga.current_page();
        // 上次本页的X
        let x = (8 * self.last_x[current_page as usize]) / STAR_SPEED as i32;
        let star_backgr = &mut buffers.star_backgr[current_page as usize];
        let width = 320;
        // 恢复星星背景到vga.framebuffer
        for i in 0..width {
            let star_pos = self.star_map[i];
            if star_pos == 0 {
                continue;
            }
            let ax = star_pos as i32 + x;
            if ax < 0 || ax >= 320 {
                continue;
            }
            let bx = ax as usize;
            let al = star_backgr[i];
            if al == 0 {
                continue;
            }
            // 这里假设星星在第 al 行（或可根据实际需求调整），否则默认 y=0
            vga.put_pixel(bx as i32, (star_pos as i32 / 320) as i32, al);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffers::{Buffers, WorldOptions};
    use crate::mpal256;
    use crate::palettes::Palettes;
    use crate::vga256::VGA;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn test_stars_to_png() {
        // 初始化参数
        let width = 320;
        let height = 200;
        let mut palette = Palettes::new();
        palette.new_palette(mpal256::mpal256_palette());

        let mut vga = VGA::new_offscreen(width, height);
        vga.palette = palette.clone();

        let mut buffers = Buffers::new();
        let options = WorldOptions {
            horizon: 200,
            stars: 100,
            ..Default::default()
        };
        let mut stars = Stars::new();
        // 初始化星星
        stars.init_stars(&mut buffers, &options);
        // 绘制星星到vga
        stars.show_stars(&mut vga, &mut buffers);

        // 转换为 RGBA 图像
        // 只根据framebuffer生成黑白图像
        let width = vga.width as u32;
        let height = vga.height as u32;
        let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        for (i, &val) in vga.framebuffer.iter().enumerate() {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            let color = if val == 0 {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            };
            img.put_pixel(x, y, Rgba(color));
        }
        img.save("./output/test_stars_output.png").unwrap();
    }
}
