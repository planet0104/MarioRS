use crate::buffers::WorldOptions;
use crate::mpal256;
use crate::vga256::VGA;

pub const BLINK_COUNTER_DEFAULT: i32 = 0;
pub const GRASS_COUNTER_DEFAULT: i32 = 0;
pub const COIN_COUNTER_DEFAULT: i32 = 0;
pub const WATERFALL_COUNTER_DEFAULT: i32 = 0;
pub const PALETTE_EFFECT_DEFAULT: i32 = PE_NO_EFFECT;

/// 调色板类型：256色，每色3通道
pub type PalType = [[u8; 3]; 256];

pub const STEPS: i32 = 32;
pub const BLINK_SPEED: i32 = 25;
pub const GRASS_SPEED: i32 = 40;
pub const COIN_SPEED: i32 = 25;
pub const WATERFALL_SPEED: i32 = 10;

pub const PE_NO_EFFECT: i32 = 0;
pub const PE_BLACK_WHITE: i32 = 1;
pub const PE_EGA_MODE: i32 = 2;

pub const LOCK_PALETTE_DEFAULT: bool = false;
pub const MODIFY_PALETTE_DEFAULT: bool = true;
pub const FADING_DONE_DEFAULT: bool = true;

#[derive(Clone)]
pub struct Palettes {
    pub palette: PalType,
    pub fading_up: bool,
    pub fading_down: bool,
    pub fading_pos: u8,
    pub fading_done: bool,
    pub fading_target: u8,    // 渐变目标值（用于非阻塞渐变）
    pub fading_step: u8,      // 每帧步进值（用于加速渐变）
    pub lock_palette: bool,
    pub modify_palette: bool,
    pub blink_counter: i32,
    pub grass_counter: i32,
    pub coin_counter: i32,
    pub waterfall_counter: i32,
    pub palette_effect: i32,
}

impl Default for Palettes {
    fn default() -> Self {
        Palettes {
            palette: *mpal256::mpal256_palette(),
            fading_up: false,
            fading_down: false,
            fading_pos: 0,
            fading_done: FADING_DONE_DEFAULT,
            fading_target: 0,
            fading_step: 1,
            lock_palette: LOCK_PALETTE_DEFAULT,
            modify_palette: MODIFY_PALETTE_DEFAULT,
            blink_counter: BLINK_COUNTER_DEFAULT,
            grass_counter: GRASS_COUNTER_DEFAULT,
            coin_counter: COIN_COUNTER_DEFAULT,
            waterfall_counter: WATERFALL_COUNTER_DEFAULT,
            palette_effect: PALETTE_EFFECT_DEFAULT,
        }
    }
}

impl Palettes {
    /// 创建调色板实例
    ///
    /// Pascal: {$I MPAL256.}
    /// 初始化必须来自 MPAL256 的 256 色 6bit 调色板，不能用精灵侧 0..159 的 PALETTE 或自行补齐。
    pub fn new() -> Self {
        Self::default()
    }

    /// ReadPalette 对齐 Pascal 语义
    ///
    /// 注意1 Pascal 的 VGA256.ReadPalette 名字是 ReadPalette 但实现为写入调色板
    /// 注意2 当无特效时调用 vga.read_palette 相当于把 p 写入 VGA
    /// 注意3 当有特效时走 refresh_palette 对齐 Pascal RefreshPalette 路径
    pub fn read_palette(&mut self, vga: &mut VGA, p: &PalType) {
        if self.palette_effect == PE_NO_EFFECT {
            vga.read_palette(p);
        } else {
            // 如果有特效，则刷新调色板
            self.refresh_palette(p, vga);
        }
    }

    /// 用新调色板数据替换当前调色板
    pub fn new_palette(&mut self, p: &PalType) {
        self.palette = *p;
        self.fading_up = false;
        self.fading_down = false;
    }

    /// 清空调色板并读取
    pub fn clear_palette(&mut self, vga: &mut VGA) {
        let mut pal: PalType = [[0; 3]; 256]; // FillChar (Pal, SizeOf(Pal), #0)
        self.read_palette(vga, &mut pal); // ReadPalette(Pal)
        self.fading_up = false; // FadingUp := FALSE
        self.fading_down = false; // FadingDown := FALSE
    }

    /// 修改单色
    pub fn change_palette(&mut self, color: usize, r: u8, g: u8, b: u8) {
        self.palette[color][0] = r;
        self.palette[color][1] = g;
        self.palette[color][2] = b;
    }

    /// 开始淡入（非阻塞）
    /// n: 总步数（8表示每帧步进8，共8帧完成；64表示每帧步进1，共64帧完成）
    pub fn start_fade_up(&mut self) {
        self.start_fade_up_steps(8);
    }

    /// 开始淡入（非阻塞，指定步数）
    pub fn start_fade_up_steps(&mut self, n: u8) {
        self.fading_up = true;
        self.fading_down = false;
        self.fading_pos = 63;
        self.fading_target = 0;
        self.fading_step = if n > 0 { 64 / n } else { 1 };
        self.fading_done = false;
    }

    /// 开始淡出（非阻塞）
    pub fn start_fade_down(&mut self) {
        self.start_fade_down_steps(8);
    }

    /// 开始淡出（非阻塞，指定步数）
    pub fn start_fade_down_steps(&mut self, n: u8) {
        self.fading_down = true;
        self.fading_up = false;
        self.fading_pos = 0;
        self.fading_target = 63;
        self.fading_step = if n > 0 { 64 / n } else { 1 };
        self.fading_done = false;
    }

    /// 渐变帧推进（非阻塞，每帧调用）
    pub fn fade(&mut self, vga: &mut VGA) {
        if self.fading_up || self.fading_down {
            let mut temp_pal: PalType = [[0; 3]; 256];
            
            // 当fading_pos >= 63时，保持全黑调色板，防止亮色在淡入初始帧闪烁
            if self.fading_pos < 63 {
                for i in 0..256 {
                    for j in 0..3 {
                        if self.palette[i][j] as i16 - self.fading_pos as i16 > 0 {
                            temp_pal[i][j] = self.palette[i][j] - self.fading_pos;
                        } else {
                            temp_pal[i][j] = 0;
                        }
                    }
                }
            }
            // 当fading_pos >= 63时，temp_pal保持全0（全黑）
            
            self.read_palette(vga, &mut temp_pal);

            if self.fading_up {
                if self.fading_pos <= self.fading_step {
                    self.fading_pos = 0;
                    self.fading_up = false;
                    self.fading_done = true;
                } else {
                    self.fading_pos = self.fading_pos.saturating_sub(self.fading_step);
                }
            }
            if self.fading_down {
                if self.fading_pos >= 63 - self.fading_step {
                    self.fading_pos = 63;
                    self.fading_down = false;
                    self.fading_done = true;
                } else {
                    self.fading_pos = self.fading_pos.saturating_add(self.fading_step);
                }
            }
        }
    }

    /// 检查渐变是否正在进行
    pub fn is_fading(&self) -> bool {
        self.fading_up || self.fading_down
    }

    /// 执行淡入效果，N为步数 - 修复调用方法
    pub fn fade_up(&mut self, n: u8, vga: &mut VGA) {
        if self.palette_effect == PE_EGA_MODE {
            return;
        }
        let mut temp_pal: PalType = [[0; 3]; 256];

        for k in (0..n).rev() {
            for i in 0..256 {
                for j in 0..3 {
                    if self.palette[i][j] as i16 - k as i16 > 0 {
                        temp_pal[i][j] = self.palette[i][j] - k;
                    } else {
                        temp_pal[i][j] = 0;
                    }
                }
            }
            vga.wait_display();
            vga.wait_retrace();
            // 修复：按Pascal逻辑调用read_palette
            self.read_palette(vga, &mut temp_pal);
            // 每次更新调色板后都显示到窗口，产生淡入动画效果
            vga.present();
        }
    }

    /// 执行淡出效果，N为步数 - 修复调用方法
    pub fn fade_down(&mut self, n: u8, vga: &mut VGA) {
        if self.palette_effect == PE_EGA_MODE {
            return;
        }
        let mut temp_pal: PalType = [[0; 3]; 256];
        for k in 0..n {
            for i in 0..256 {
                for j in 0..3 {
                    if self.palette[i][j] as i16 - k as i16 > 0 {
                        temp_pal[i][j] = self.palette[i][j] - k;
                    } else {
                        temp_pal[i][j] = 0;
                    }
                }
            }
            vga.wait_display();
            vga.wait_retrace();
            // 修复：按Pascal逻辑调用read_palette
            self.read_palette(vga, &mut temp_pal);
            // 每次更新调色板后都显示到窗口，产生淡出动画效果
            vga.present();
        }
    }

    pub fn init_grass(&mut self, vga: &mut VGA, options: &WorldOptions) {
        // 设置草地相关调色板颜色
        self.palette[2][0] = options.c2r;
        self.palette[2][1] = options.c2g;
        self.palette[2][2] = options.c2b;

        self.palette[3][0] = options.c3r;
        self.palette[3][1] = options.c3g;
        self.palette[3][2] = options.c3b;

        self.palette[153] = self.palette[2];
        self.palette[154] = self.palette[3];
        self.palette[155] = self.palette[2];
        self.palette[156] = self.palette[3];

        // 修复：Pascal中是Options.SkyType in [10]，即只有当SkyType=10时才为true
        let sky_index = if options.sky_type == 10 { 1 } else { 0 };
        self.palette[157] = self.palette[0xF0 - sky_index];
        self.palette[158] = self.palette[0xF0 - sky_index];

        self.out_palette(6, 60, 40, 35, vga); // Champ
    }

    /// 复制调色板颜色 C1 到 C2，并输出到 VGA
    pub fn copy_palette(&mut self, c1: usize, c2: usize, vga: &mut VGA) {
        let r = self.palette[c1][0];
        let g = self.palette[c1][1];
        let b = self.palette[c1][2];
        self.out_palette(c2, r, g, b, vga);
    }

    /// 动态闪烁/流水/草地/金币等调色板动画（BlinkPalette）
    pub fn blink_palette(&mut self, vga: &mut VGA, options: &WorldOptions) {
        use crate::utils::{random_i32, random_u8};
        if self.fading_up || self.fading_down {
            return;
        }
        // 星星闪烁
        self.out_palette(
            1,
            60 + random_u8(4),
            55 + random_u8(8),
            30 + random_u8(25),
            vga,
        );

        // 瀑布动画
        self.waterfall_counter += 1;
        if self.waterfall_counter >= 5 * WATERFALL_SPEED {
            self.waterfall_counter = 0;
        }
        let i = self.waterfall_counter % WATERFALL_SPEED;
        if i == 0 {
            let mut j = self.waterfall_counter / WATERFALL_SPEED;
            for idx in 0..5 {
                j -= 1; // 修复：Pascal中是先Dec(j)
                if j < 0 {
                    j = 4; // 修复：Pascal中是j := 4而不是5
                }
                let k = 5 - j;
                match options.sky_type {
                    0 => self.out_palette(
                        7 + idx,
                        (40 + 3 * k).min(255) as u8,
                        (50 + 2 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        vga,
                    ),
                    1 => self.out_palette(
                        7 + idx,
                        (45 + 3 * k).min(255) as u8,
                        (52 + 2 * k).min(255) as u8,
                        (51 + 2 * k).min(255) as u8,
                        vga,
                    ),
                    2 => self.out_palette(
                        7 + idx,
                        (44 + 3 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        vga,
                    ),
                    3 => self.out_palette(
                        7 + idx,
                        (34 + 3 * k).min(255) as u8,
                        (40 + 2 * k).min(255) as u8,
                        (40 + 2 * k).min(255) as u8,
                        vga,
                    ),
                    4 => self.out_palette(
                        7 + idx,
                        (38 + 3 * k).min(255) as u8,
                        (47 + 2 * k).min(255) as u8,
                        (47 + 2 * k).min(255) as u8,
                        vga,
                    ),
                    5 => self.out_palette(
                        7 + idx,
                        (53 + 2 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        (44 + 3 * k).min(255) as u8,
                        vga,
                    ),
                    6 | 7 | 8 => self.out_palette(
                        7 + idx,
                        (42 + 4 * k).min(255) as u8,
                        (5 + k * k).min(255) as u8,
                        (2 * k).min(255) as u8,
                        vga,
                    ),
                    10 => self.out_palette(
                        7 + idx,
                        (40 + 4 * k).min(255) as u8,
                        (45 + 3 * k).min(255) as u8,
                        63,
                        vga,
                    ),
                    _ => self.out_palette(
                        7 + idx,
                        (50 + 2 * k).min(255) as u8,
                        (50 + 2 * k).min(255) as u8,
                        (50 + 2 * k).min(255) as u8,
                        vga,
                    ),
                }
            }
        }

        // 闪烁动画
        self.blink_counter += 1;
        if self.blink_counter > BLINK_SPEED {
            self.blink_counter = -BLINK_SPEED;
            self.out_palette(159, 52, 43, 21, vga);
        } else if self.blink_counter == 0 {
            self.out_palette(159, 55, 46, 24, vga);
        }

        // 草地动画
        self.grass_counter += 1;
        if self.grass_counter > GRASS_SPEED {
            self.grass_counter = -GRASS_SPEED;
            self.copy_palette(2, 153, vga);
            self.copy_palette(3, 154, vga);
            self.copy_palette(2, 155, vga);
            self.copy_palette(3, 156, vga);
            let sky_idx = 0xF0 - if options.sky_type == 10 { 1 } else { 0 };
            self.copy_palette(sky_idx, 157, vga);
            self.copy_palette(sky_idx, 158, vga);
        } else if self.grass_counter == 0 {
            let sky_idx = 0xF0 - if options.sky_type == 10 { 1 } else { 0 };
            self.copy_palette(sky_idx, 153, vga);
            self.copy_palette(sky_idx, 154, vga);
            self.copy_palette(3, 155, vga);
            self.copy_palette(2, 156, vga);
            self.copy_palette(2, 157, vga);
            self.copy_palette(3, 158, vga);
        }

        // 金币动画
        self.coin_counter += 1;
        if self.coin_counter > 3 * COIN_SPEED {
            self.coin_counter = 0;
            self.out_palette(12, 62, 56, 20, vga);
            self.out_palette(13, 60, 56, 22, vga);
            self.out_palette(14, 63, 63, 36, vga);
        } else if self.coin_counter == COIN_SPEED {
            self.out_palette(14, 62, 56, 20, vga);
            self.out_palette(12, 60, 56, 22, vga);
            self.out_palette(13, 63, 63, 36, vga);
        } else if self.coin_counter == 2 * COIN_SPEED {
            self.out_palette(13, 62, 56, 20, vga);
            self.out_palette(14, 60, 56, 22, vga);
            self.out_palette(12, 63, 63, 36, vga);
        }
    }

    /// 刷新调色板：将 P 的内容输出到 VGA，期间不修改内存调色板
    pub fn refresh_palette(&mut self, p: &PalType, vga: &mut VGA) {
        self.modify_palette = false;
        for i in 0..256 {
            self.out_palette(i, p[i][0], p[i][1], p[i][2], vga);
        }
        self.modify_palette = true;
    }

    /// 设置并输出单个调色板颜色
    pub fn out_palette(&mut self, color: usize, mut r: u8, mut g: u8, mut b: u8, vga: &mut VGA) {
        // 修改内存调色板
        if self.modify_palette {
            self.palette[color][0] = r;
            self.palette[color][1] = g;
            self.palette[color][2] = b;
        }
        // 特效处理
        if self.palette_effect != PE_NO_EFFECT {
            match self.palette_effect {
                PE_BLACK_WHITE => {
                    let i = (r as u16 + g as u16 + b as u16) / 3;
                    let i = i as u8;
                    r = i;
                    g = i;
                    b = i;
                }
                PE_EGA_MODE => {
                    r = r & 0xF0;
                    g = g & 0xF0;
                    b = b & 0xF0;
                }
                _ => {}
            }
        }
        // 输出到硬件
        if !self.lock_palette {
            vga.set_palette(color as u8, r, g, b);
        }
    }

    /// 锁定调色板，禁止写入硬件
    pub fn lock_pal(&mut self) {
        self.lock_palette = true;
    }

    /// 解锁调色板，允许写入硬件
    pub fn unlock_pal(&mut self) {
        self.lock_palette = false;
    }

    /// 获取调色板中某个颜色的RGB值，返回 [r, g, b]
    pub fn get_rgb(&self, idx: u8) -> [u8; 3] {
        let idx = idx as usize;
        if idx < self.palette.len() {
            self.palette[idx]
        } else {
            [0, 0, 0]
        }
    }
}
