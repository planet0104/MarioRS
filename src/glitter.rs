// Glitter System - 闪光效果系统
// 严格按照Pascal GLITTER.PAS翻译
use crate::buffers::{H, NH, NV, W};
use crate::vga256::{MAX_PAGE, VGA, VIR_SCREEN_WIDTH};

pub const MAX_GLITTER: usize = 75;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Glitter {
    pub attr: u8,
    pub pos: u16,
    pub back_gr: [u8; MAX_PAGE as usize + 1],
    pub dummy1: u8,
    pub dummy2: u8,
    pub dummy3: u8,
}

// Pascal: var Count: String [MaxGlitter];
// Rust: 用 Vec<u8> 表示 Count，NumGlitter 用 usize
pub struct GlitterSystem {
    pub count: Vec<u8>,             // 长度 MAX_GLITTER+1, count[0] = num_glitter
    pub glitter_list: Vec<Glitter>, // 长度 MAX_GLITTER+1
}

use crate::buffers::Buffers;

impl GlitterSystem {
    /// 清空所有闪光
    pub fn clear_glitter(&mut self, _vga: &mut VGA, _buffers: &mut Buffers) {
        for v in self.count.iter_mut() {
            *v = 0;
        }
    }

    /// 新建一个闪光
    pub fn new_glitter(
        &mut self,
        x: i32,
        y: i32,
        new_attr: u8,
        duration: u8,
        buffers: &mut Buffers,
    ) {
        // Pascal:
        // if (X < XView) or (X >= XView + NH * W) then Exit;
        // i := 1;
        // while (Count [i] > #0) and (i < MaxGlitter) do Inc (i);
        // if (i < MaxGlitter) then
        //   if (Y < 0) or (Y > NV * H) then Exit;
        //   Count [i] := Chr (Duration);
        //   Inc (NumGlitter);
        //   with GlitterList [i] do
        //     Pos := Y * VIR_SCREEN_WIDTH + X;
        //     FillChar (BackGr, SizeOf (BackGr), #0);
        //     Attr := NewAttr;

        // Rust 变量说明：
        // - self.count[0] = num_glitter
        // - self.count[1..=MAX_GLITTER] = Count[1..MaxGlitter]
        // - self.glitter_list[1..=MAX_GLITTER] = GlitterList[1..MaxGlitter]

        let x_view = buffers.x_view;
        let y_view = buffers.y_view;
        let screen_x = x - x_view;
        let screen_y = y - y_view;

        if screen_x < 0 || screen_x >= (NH * W) as i32 {
            return;
        }
        let mut i = 1;
        while i <= MAX_GLITTER && self.count[i] > 0 {
            i += 1;
        }
        if i <= MAX_GLITTER {
            if screen_y < 0 || screen_y >= (NV * H) as i32 {
                return;
            }
            self.count[i] = duration;
            self.count[0] = self.count[0].saturating_add(1); // NumGlitter++
            let pos = (screen_y as u16)
                .wrapping_mul(VIR_SCREEN_WIDTH as u16)
                .wrapping_add(screen_x as u16);
            let glitter = &mut self.glitter_list[i];
            glitter.pos = pos;
            for b in glitter.back_gr.iter_mut() {
                *b = 0;
            }
            glitter.attr = new_attr;
        }
    }

    /// 新建一个星形闪光
    pub fn new_star(&mut self, x: i32, y: i32, new_attr: u8, duration: u8, buffers: &mut Buffers) {
        self.new_glitter(x, y, new_attr, duration + 4, buffers);
        self.new_glitter(x + 1, y, new_attr, duration, buffers);
        self.new_glitter(x, y + 1, new_attr, duration, buffers);
        self.new_glitter(x - 1, y, new_attr, duration, buffers);
        self.new_glitter(x, y - 1, new_attr, duration, buffers);
    }

    /// 显示所有闪光
    pub fn show_glitter(&mut self, vga: &mut VGA) {
        // Pascal:
        // PageOffset := GetPageOffset;
        // Page := CurrentPage;
        // if NumGlitter > 0 then
        //   for i := 1 to MaxGlitter do
        //     if Count [i] > Chr (MAX_PAGE + 1) then
        //       { 记录背景像素并绘制闪光像素 }
        //     else if Count [i] > #0 then
        //       { BackGr [CurrentPage] := 0 }

        let num_glitter = self.count[0];
        if num_glitter > 0 {
            let working_page = vga.current_page() as usize;
            for i in 1..=MAX_GLITTER {
                if self.count[i] > (MAX_PAGE as u8 + 1) {
                    let glitter = &mut self.glitter_list[i];
                    let x = (glitter.pos % VIR_SCREEN_WIDTH as u16) as i32;
                    let y = (glitter.pos / VIR_SCREEN_WIDTH as u16) as i32;
                    glitter.back_gr[working_page] = vga.get_pixel(x, y);
                    vga.put_pixel(x, y, glitter.attr);
                } else if self.count[i] > 0 {
                    let current_page = vga.current_page() as usize;
                    self.glitter_list[i].back_gr[current_page] = 0;
                }
            }
        }
    }

    /// 隐藏所有闪光
    pub fn hide_glitter(&mut self, vga: &mut VGA) {
        // Pascal:
        // PageOffset := GetPageOffset;
        // if NumGlitter = 0 then Exit;
        // Page := CurrentPage;
        // for i := MaxGlitter downto 1 do
        //   if Count [i] > #0 then
        //     { 恢复背景像素，减少计数，必要时减少 NumGlitter }

        let num_glitter = self.count[0];
        if num_glitter == 0 {
            return;
        }
        let working_page = vga.current_page() as usize;
        for i in (1..=MAX_GLITTER).rev() {
            if self.count[i] > 0 {
                let glitter = &mut self.glitter_list[i];
                let x = (glitter.pos % VIR_SCREEN_WIDTH as u16) as i32;
                let y = (glitter.pos / VIR_SCREEN_WIDTH as u16) as i32;
                let back = glitter.back_gr[working_page];
                if back != 0 {
                    vga.put_pixel(x, y, back);
                }
                self.count[i] = self.count[i].saturating_sub(1);
                if self.count[i] == 0 {
                    self.count[0] = self.count[0].saturating_sub(1);
                }
                glitter.back_gr[working_page] = 0;
            }
        }
    }

    /// 金币特效
    pub fn coin_glitter(&mut self, x: i32, y: i32, buffers: &mut Buffers) {
        self.new_star(x + 5, y + 2, 0x1F, 20, buffers);
        self.new_star(x + W as i32 - 6, y + 6, 0x1F, 18, buffers);
        self.new_star(x + 10, y + H as i32 - 3, 0x1F, 16, buffers);
        self.new_glitter(x + W as i32 - 9, y + 2, 0x1F, 15, buffers);
        self.new_glitter(x + 6, y + 7, 0x1F, 17, buffers);
        self.new_glitter(x + 3, y + 9, 0x1F, 15, buffers);
    }

    /// 区域内随机生成闪光
    pub fn start_glitter(&mut self, x: i32, y: i32, w: i32, h: i32, buffers: &mut Buffers) {
        use crate::utils::{random_i32, random_u8};
        self.new_star(
            x + random_i32(w),
            y + random_i32(h),
            0x1F,
            10 + random_u8(10),
            buffers,
        );
        for _ in 0..4 {
            self.new_glitter(
                x + random_i32(w),
                y + random_i32(h),
                0x1F,
                5 + random_u8(10),
                buffers,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffers::Buffers;
    use crate::vga256::{SCREEN_HEIGHT, SCREEN_WIDTH, VGA};
    use image::{ImageBuffer, Rgba};

    #[test]
    fn test_glitter_to_png() {
        // 初始化VGA和Buffers
        let mut vga = VGA::new_offscreen(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
        let mut buffers = Buffers::new();
        // 初始化GlitterSystem
        let mut glitter = GlitterSystem {
            count: vec![0u8; MAX_GLITTER + 2],
            glitter_list: vec![
                Glitter {
                    attr: 0,
                    pos: 0,
                    back_gr: [0; MAX_PAGE as usize + 1],
                    dummy1: 0,
                    dummy2: 0,
                    dummy3: 0,
                };
                MAX_GLITTER + 2
            ],
        };
        // 在屏幕中央生成一组glitter
        let x = (SCREEN_WIDTH / 2) as i32;
        let y = (SCREEN_HEIGHT / 2) as i32;
        glitter.start_glitter(x, y, 20, 20, &mut buffers);
        // 渲染一次glitter
        glitter.show_glitter(&mut vga);
        // 保存framebuffer为PNG
        let mut img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);
        for y in 0..SCREEN_HEIGHT as u32 {
            for x in 0..SCREEN_WIDTH as u32 {
                let idx = (y as usize) * SCREEN_WIDTH as usize + (x as usize);
                let pal_idx = vga.framebuffer[idx];
                let rgb = vga.palette.get_rgb(pal_idx);
                img.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], 255]));
            }
        }
        img.save("./output/test_glitter.png")
            .expect("Failed to save PNG");
    }
}
