// Glitter System - 闪光效果系统 - GPU版本
use crate::buffers::{Buffers, H, NH, NV, W};
use crate::gpu::{FillRect, RenderCommand};
use crate::vga256::{MAX_PAGE, VGA, VIR_SCREEN_WIDTH};

pub const MAX_GLITTER: usize = 75;

// GPU版 - 简化的闪光结构体
#[derive(Debug, Clone, Copy)]
pub struct Glitter {
    pub attr: u8,  // 颜色属性
    pub pos: u16,  // 屏幕位置
}

// Pascal: var Count: String [MaxGlitter];
// Rust: 用 Vec<u8> 表示 Count，NumGlitter 用 usize
pub struct GlitterSystem {
    pub count: Vec<u8>,             // 长度 MAX_GLITTER+1, count[0] = num_glitter
    pub glitter_list: Vec<Glitter>, // 长度 MAX_GLITTER+1
}

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

    /// GPU版 - 显示所有闪光
    pub fn show_glitter(&mut self, vga: &mut VGA) {
        let num_glitter = self.count[0];
        if num_glitter > 0 {
            for i in 1..=MAX_GLITTER {
                if self.count[i] > (MAX_PAGE as u8 + 1) {
                    let glitter = &self.glitter_list[i];
                    let x = (glitter.pos % VIR_SCREEN_WIDTH as u16) as i32;
                    let y = (glitter.pos / VIR_SCREEN_WIDTH as u16) as i32;
                    // GPU模式：直接绘制1x1像素
                    vga.fill_gpu(x, y, 1, 1, glitter.attr);
                }
            }
        }
    }

    // GPU模式下不需要隐藏操作，每帧重绘
    // 闪光计数器由update_glitter_gpu管理

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

    /// GPU渲染: 收集所有活跃闪光像素
    /// 闪光是单像素效果，使用1x1的填充矩形来渲染
    pub fn collect_glitter_gpu(
        &self,
        commands: &mut Vec<RenderCommand>,
        x_view: i32,
        y_view: i32,
        palette_index: u32,
    ) {
        let num_glitter = self.count[0];
        if num_glitter == 0 {
            return;
        }

        for i in 1..=MAX_GLITTER {
            // 只渲染活跃的闪光 (count > MAX_PAGE + 1 表示可见)
            if self.count[i] > (MAX_PAGE as u8 + 1) {
                let glitter = &self.glitter_list[i];
                let world_x = (glitter.pos % VIR_SCREEN_WIDTH as u16) as i32;
                let world_y = (glitter.pos / VIR_SCREEN_WIDTH as u16) as i32;
                
                // 转换为屏幕坐标
                let sx = (world_x - x_view) as f32;
                let sy = (world_y - y_view) as f32;
                
                // 检查是否在可视区域内
                if sx >= 0.0 && sx < crate::vga256::SCREEN_WIDTH as f32
                    && sy >= 0.0 && sy < crate::vga256::SCREEN_HEIGHT as f32
                {
                    // 使用1x1像素的填充矩形
                    let fill = FillRect::new(sx, sy, 1.0, 1.0, glitter.attr, palette_index);
                    commands.push(RenderCommand::FillRect(fill));
                }
            }
        }
    }

    /// GPU渲染: 更新闪光计数器（不需要VGA）
    /// 在每帧结束时调用以减少闪光持续时间
    pub fn update_glitter_gpu(&mut self) {
        let num_glitter = self.count[0];
        if num_glitter == 0 {
            return;
        }

        for i in (1..=MAX_GLITTER).rev() {
            if self.count[i] > 0 {
                self.count[i] = self.count[i].saturating_sub(1);
                if self.count[i] == 0 {
                    self.count[0] = self.count[0].saturating_sub(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffers::Buffers;
    use crate::vga256::{SCREEN_HEIGHT, SCREEN_WIDTH, VGA};

    #[test]
    fn test_glitter_system() {
        // 初始化VGA和Buffers
        let mut vga = VGA::new_offscreen(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
        let mut buffers = Buffers::new();
        // 初始化GlitterSystem
        let mut glitter = GlitterSystem {
            count: vec![0u8; MAX_GLITTER + 2],
            glitter_list: vec![Glitter { attr: 0, pos: 0 }; MAX_GLITTER + 2],
        };
        // 在屏幕中央生成一组glitter
        let x = (SCREEN_WIDTH / 2) as i32;
        let y = (SCREEN_HEIGHT / 2) as i32;
        glitter.start_glitter(x, y, 20, 20, &mut buffers);
        // 渲染一次glitter
        glitter.show_glitter(&mut vga);
        // GPU模式下不需要测试framebuffer输出
        assert!(glitter.count[0] > 0, "Glitter should be created");
    }
}
