use crate::buffers::WorldOptions;
use crate::mpal256;
use crate::render_state::RenderState;

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
    /// 源调色板：用于fade_up/fade_down的目标/源，对应Pascal中不被FadeDown修改的Palette变量
    pub source_palette: PalType,
    pub fading_up: bool,
    pub fading_down: bool,
    pub fading_pos: u8,
    pub fading_done: bool,
    pub fading_target: u8, // 渐变目标值（用于非阻塞渐变）
    pub fading_step: u8,   // 每帧步进值（用于加速渐变）
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
        let default_pal = *mpal256::mpal256_palette();
        Palettes {
            palette: default_pal,
            source_palette: default_pal,
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
    /// 注意2 当无特效时调用 render_state.read_palette 相当于把 p 写入 RenderState
    /// 注意3 当有特效时走 refresh_palette 对齐 Pascal RefreshPalette 路径
    pub fn read_palette(&mut self, render_state: &mut RenderState, p: &PalType) {
        if self.palette_effect == PE_NO_EFFECT {
            render_state.read_palette(p);
        } else {
            // 如果有特效，则刷新调色板
            self.refresh_palette(p, render_state);
        }
    }

    /// 用新调色板数据替换当前调色板
    pub fn new_palette(&mut self, p: &PalType) {
        self.palette = *p;
        self.source_palette = *p; // 同时更新源调色板
        self.fading_up = false;
        self.fading_down = false;
    }

    /// 清空调色板并读取
    pub fn clear_palette(&mut self, render_state: &mut RenderState) {
        let mut pal: PalType = [[0; 3]; 256]; // FillChar (Pal, SizeOf(Pal), #0)
        self.read_palette(render_state, &mut pal); // ReadPalette(Pal)
        self.fading_up = false; // FadingUp := FALSE
        self.fading_down = false; // FadingDown := FALSE
    }

    /// 修改单色
    pub fn change_palette(&mut self, color: usize, r: u8, g: u8, b: u8) {
        // 内部使用 6-bit VGA 值 (0-63)
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
    /// 淡入从黑屏渐变到当前palette的颜色
    pub fn start_fade_up_steps(&mut self, n: u8) {
        // 保存当前调色板作为渐变目标
        self.source_palette = self.palette;
        // 立即将palette设置为全黑，防止渐显开始前闪烁
        self.palette = [[0; 3]; 256];
        self.fading_up = true;
        self.fading_down = false;
        self.fading_pos = 63; // 从最暗开始
        self.fading_target = 0;
        self.fading_step = if n > 0 { 64 / n } else { 1 };
        self.fading_done = false;
    }

    /// 开始淡出（非阻塞）
    pub fn start_fade_down(&mut self) {
        self.start_fade_down_steps(8);
    }

    /// 开始淡出（非阻塞，指定步数）
    /// 淡出从当前palette颜色渐变到黑屏
    pub fn start_fade_down_steps(&mut self, n: u8) {
        // 保存当前调色板作为渐变源
        self.source_palette = self.palette;
        self.fading_down = true;
        self.fading_up = false;
        self.fading_pos = 0; // 从当前亮度开始
        self.fading_target = 63;
        self.fading_step = if n > 0 { 64 / n } else { 1 };
        self.fading_done = false;
    }

    /// 渐变帧推进（非阻塞，每帧调用）
    /// 返回计算好的临时调色板，调用者需要应用到VGA渲染
    /// 注意：使用source_palette作为源，这样palette可以被正常的blink等操作更新
    pub fn fade_step(&mut self) -> Option<PalType> {
        if !self.fading_up && !self.fading_down {
            return None;
        }

        let mut temp_pal: PalType = [[0; 3]; 256];

        // 先用当前fading_pos计算调色板（与Pascal循环体内的顺序一致）
        // 当fading_pos >= 63时，保持全黑调色板
        if self.fading_pos < 63 {
            for i in 0..256 {
                for j in 0..3 {
                    // 使用source_palette作为源，与Pascal一致
                    if self.source_palette[i][j] as i16 - self.fading_pos as i16 > 0 {
                        temp_pal[i][j] = self.source_palette[i][j] - self.fading_pos;
                    } else {
                        temp_pal[i][j] = 0;
                    }
                }
            }
        }
        // 当fading_pos >= 63时，temp_pal保持全0（全黑）

        // 然后更新fading_pos（准备下一帧）
        if self.fading_up {
            if self.fading_pos == 0 {
                // 已经到达最亮，渐变完成
                self.fading_up = false;
                self.fading_done = true;
            } else {
                self.fading_pos = self.fading_pos.saturating_sub(self.fading_step);
            }
        } else if self.fading_down {
            if self.fading_pos >= 63 {
                // 已经到达最暗，渐变完成
                self.fading_down = false;
                self.fading_done = true;
            } else {
                self.fading_pos = self.fading_pos.saturating_add(self.fading_step);
            }
        }

        Some(temp_pal)
    }

    /// 兼容旧接口（非阻塞渐变帧推进）
    pub fn fade(&mut self, render_state: &mut RenderState) {
        if let Some(temp_pal) = self.fade_step() {
            self.read_palette(render_state, &temp_pal);
        }
    }

    /// 检查渐变是否正在进行
    pub fn is_fading(&self) -> bool {
        self.fading_up || self.fading_down
    }

    /// 执行淡入效果，N为步数
    /// 使用source_palette作为目标值（对应Pascal中不被修改的Palette全局变量）
    pub fn fade_up(&mut self, n: u8, render_state: &mut RenderState) {
        if self.palette_effect == PE_EGA_MODE {
            return;
        }
        let mut temp_pal: PalType = [[0; 3]; 256];

        for k in (0..n).rev() {
            for i in 0..256 {
                for j in 0..3 {
                    // 使用source_palette作为源，与Pascal一致
                    if self.source_palette[i][j] as i16 - k as i16 > 0 {
                        temp_pal[i][j] = self.source_palette[i][j] - k;
                    } else {
                        temp_pal[i][j] = 0;
                    }
                }
            }
            self.read_palette(render_state, &temp_pal);
        }
        // 淡入完成后，palette恢复为source_palette
        self.palette = self.source_palette;
    }

    /// 执行淡出效果，N为步数
    /// 使用source_palette作为源值（对应Pascal中不被修改的Palette全局变量）
    pub fn fade_down(&mut self, n: u8, render_state: &mut RenderState) {
        if self.palette_effect == PE_EGA_MODE {
            return;
        }
        // 保存当前调色板到source_palette，用于fade_up恢复
        self.source_palette = self.palette;

        let mut temp_pal: PalType = [[0; 3]; 256];
        for k in 0..n {
            for i in 0..256 {
                for j in 0..3 {
                    // 使用source_palette作为源，与Pascal一致
                    if self.source_palette[i][j] as i16 - k as i16 > 0 {
                        temp_pal[i][j] = self.source_palette[i][j] - k;
                    } else {
                        temp_pal[i][j] = 0;
                    }
                }
            }
            self.read_palette(render_state, &temp_pal);
        }
        // 淡出完成后，palette保持在变暗状态（k=n-1时的值）
        self.palette = temp_pal;
    }

    pub fn init_grass(&mut self, render_state: &mut RenderState, options: &WorldOptions) {
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

        self.out_palette(6, 60, 40, 35, render_state); // Champ

        // 同步更新source_palette，确保渐显时使用正确的颜色
        self.source_palette = self.palette;
    }

    /// 复制调色板颜色 C1 到 C2，并输出到 RenderState
    pub fn copy_palette(&mut self, c1: usize, c2: usize, render_state: &mut RenderState) {
        let r = self.palette[c1][0];
        let g = self.palette[c1][1];
        let b = self.palette[c1][2];
        self.out_palette(c2, r, g, b, render_state);
    }

    /// 动态闪烁/流水/草地/金币等调色板动画（BlinkPalette）
    pub fn blink_palette(&mut self, render_state: &mut RenderState, options: &WorldOptions) {
        use crate::utils::random_u8;
        if self.fading_up || self.fading_down {
            return;
        }
        // 星星闪烁
        self.out_palette(
            1,
            60 + random_u8(4),
            55 + random_u8(8),
            30 + random_u8(25),
            render_state,
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
                        render_state,
                    ),
                    1 => self.out_palette(
                        7 + idx,
                        (45 + 3 * k).min(255) as u8,
                        (52 + 2 * k).min(255) as u8,
                        (51 + 2 * k).min(255) as u8,
                        render_state,
                    ),
                    2 => self.out_palette(
                        7 + idx,
                        (44 + 3 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        render_state,
                    ),
                    3 => self.out_palette(
                        7 + idx,
                        (34 + 3 * k).min(255) as u8,
                        (40 + 2 * k).min(255) as u8,
                        (40 + 2 * k).min(255) as u8,
                        render_state,
                    ),
                    4 => self.out_palette(
                        7 + idx,
                        (38 + 3 * k).min(255) as u8,
                        (47 + 2 * k).min(255) as u8,
                        (47 + 2 * k).min(255) as u8,
                        render_state,
                    ),
                    5 => self.out_palette(
                        7 + idx,
                        (53 + 2 * k).min(255) as u8,
                        (53 + 2 * k).min(255) as u8,
                        (44 + 3 * k).min(255) as u8,
                        render_state,
                    ),
                    6 | 7 | 8 => self.out_palette(
                        7 + idx,
                        (42 + 4 * k).min(255) as u8,
                        (5 + k * k).min(255) as u8,
                        (2 * k).min(255) as u8,
                        render_state,
                    ),
                    10 => self.out_palette(
                        7 + idx,
                        (40 + 4 * k).min(255) as u8,
                        (45 + 3 * k).min(255) as u8,
                        63,
                        render_state,
                    ),
                    _ => self.out_palette(
                        7 + idx,
                        (50 + 2 * k).min(255) as u8,
                        (50 + 2 * k).min(255) as u8,
                        (50 + 2 * k).min(255) as u8,
                        render_state,
                    ),
                }
            }
        }

        // 闪烁动画
        self.blink_counter += 1;
        if self.blink_counter > BLINK_SPEED {
            self.blink_counter = -BLINK_SPEED;
            self.out_palette(159, 52, 43, 21, render_state);
        } else if self.blink_counter == 0 {
            self.out_palette(159, 55, 46, 24, render_state);
        }

        // 草地动画
        self.grass_counter += 1;
        if self.grass_counter > GRASS_SPEED {
            self.grass_counter = -GRASS_SPEED;
            self.copy_palette(2, 153, render_state);
            self.copy_palette(3, 154, render_state);
            self.copy_palette(2, 155, render_state);
            self.copy_palette(3, 156, render_state);
            let sky_idx = 0xF0 - if options.sky_type == 10 { 1 } else { 0 };
            self.copy_palette(sky_idx, 157, render_state);
            self.copy_palette(sky_idx, 158, render_state);
        } else if self.grass_counter == 0 {
            let sky_idx = 0xF0 - if options.sky_type == 10 { 1 } else { 0 };
            self.copy_palette(sky_idx, 153, render_state);
            self.copy_palette(sky_idx, 154, render_state);
            self.copy_palette(3, 155, render_state);
            self.copy_palette(2, 156, render_state);
            self.copy_palette(2, 157, render_state);
            self.copy_palette(3, 158, render_state);
        }

        // 金币动画
        self.coin_counter += 1;
        if self.coin_counter > 3 * COIN_SPEED {
            self.coin_counter = 0;
            self.out_palette(12, 62, 56, 20, render_state);
            self.out_palette(13, 60, 56, 22, render_state);
            self.out_palette(14, 63, 63, 36, render_state);
        } else if self.coin_counter == COIN_SPEED {
            self.out_palette(14, 62, 56, 20, render_state);
            self.out_palette(12, 60, 56, 22, render_state);
            self.out_palette(13, 63, 63, 36, render_state);
        } else if self.coin_counter == 2 * COIN_SPEED {
            self.out_palette(13, 62, 56, 20, render_state);
            self.out_palette(14, 60, 56, 22, render_state);
            self.out_palette(12, 63, 63, 36, render_state);
        }
    }

    /// 刷新调色板：将 P 的内容输出到 RenderState，期间不修改内存调色板
    pub fn refresh_palette(&mut self, p: &PalType, render_state: &mut RenderState) {
        self.modify_palette = false;
        for i in 0..256 {
            self.out_palette(i, p[i][0], p[i][1], p[i][2], render_state);
        }
        self.modify_palette = true;
    }

    /// 设置并输出单个调色板颜色
    /// 内部使用 6-bit VGA 值 (0-63)
    pub fn out_palette(
        &mut self,
        color: usize,
        mut r: u8,
        mut g: u8,
        mut b: u8,
        render_state: &mut RenderState,
    ) {
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
            render_state.set_palette(color as u8, r, g, b);
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

    /// 将当前调色板转换为GPU格式 (256x RGBA)
    pub fn to_gpu_palette(&self) -> [[u8; 4]; 256] {
        let mut result = [[0u8; 4]; 256];
        for i in 0..256 {
            // VGA调色板是6bit (0-63)，需要转换为8bit (0-255)
            let [r, g, b] = self.palette[i];
            result[i] = [
                ((r as u16 * 255) / 63).min(255) as u8,
                ((g as u16 * 255) / 63).min(255) as u8,
                ((b as u16 * 255) / 63).min(255) as u8,
                255, // 索引0透明由shader控制，palette这里保持不透明以支持PutImage语义
            ];
        }
        result
    }

    /// 获取当前fade级别对应的GPU调色板索引
    pub fn get_fade_palette_index(&self) -> u32 {
        if self.fading_up || self.fading_down {
            // fading_pos: 0-63, 映射到调色板索引 1-16
            let level = self.fading_pos.min(63);
            let index = (level as u32 * 16) / 64;
            1 + index.min(15)
        } else {
            0 // 正常调色板
        }
    }
}

// ============================================================================
// GPU 预烘焙调色板
// ============================================================================

/// 预烘焙调色板数据 - 用于GPU渲染
pub struct PrebakedPalettes {
    /// 所有预烘焙的调色板帧 (64帧)
    pub frames: Vec<[[u8; 4]; 256]>,
}

impl PrebakedPalettes {
    /// 创建预烘焙调色板
    pub fn new(base_palette: &Palettes) -> Self {
        let mut frames = Vec::with_capacity(64);

        // 索引0: 正常调色板
        frames.push(base_palette.to_gpu_palette());

        // 索引1-16: 淡入/淡出帧
        for level in 1..=16 {
            let fade_pos = ((16 - level) * 63) / 16;
            frames.push(Self::generate_fade_frame(base_palette, fade_pos as u8));
        }

        // 索引17-32: 闪烁帧 (金币动画)
        for phase in 0..16 {
            frames.push(Self::generate_blink_frame(base_palette, phase));
        }

        // 索引33-48: 瀑布动画帧
        for phase in 0..16 {
            frames.push(Self::generate_waterfall_frame(base_palette, phase));
        }

        // 填充到64帧
        while frames.len() < 64 {
            frames.push(base_palette.to_gpu_palette());
        }

        Self { frames }
    }

    /// 生成淡入/淡出帧
    fn generate_fade_frame(base: &Palettes, fade_pos: u8) -> [[u8; 4]; 256] {
        let mut result = [[0u8; 4]; 256];
        for i in 0..256 {
            let [r, g, b] = base.palette[i];
            // 应用淡入淡出
            let r = if r as i16 - fade_pos as i16 > 0 {
                r - fade_pos
            } else {
                0
            };
            let g = if g as i16 - fade_pos as i16 > 0 {
                g - fade_pos
            } else {
                0
            };
            let b = if b as i16 - fade_pos as i16 > 0 {
                b - fade_pos
            } else {
                0
            };
            // 转换为8bit
            result[i] = [
                ((r as u16 * 255) / 63).min(255) as u8,
                ((g as u16 * 255) / 63).min(255) as u8,
                ((b as u16 * 255) / 63).min(255) as u8,
                if i == 0 { 0 } else { 255 },
            ];
        }
        result
    }

    /// 生成金币闪烁帧
    fn generate_blink_frame(base: &Palettes, phase: u32) -> [[u8; 4]; 256] {
        let mut result = base.to_gpu_palette();

        // 金币动画: 颜色12,13,14循环
        let coin_colors = [
            [62, 56, 20], // 状态0
            [60, 56, 22], // 状态1
            [63, 63, 36], // 状态2
        ];

        let offset = (phase / 5) % 3;
        for i in 0..3 {
            let src = (i + offset as usize) % 3;
            let [r, g, b] = coin_colors[src];
            result[12 + i] = [
                ((r as u16 * 255) / 63).min(255) as u8,
                ((g as u16 * 255) / 63).min(255) as u8,
                ((b as u16 * 255) / 63).min(255) as u8,
                255,
            ];
        }

        result
    }

    /// 生成瀑布动画帧
    fn generate_waterfall_frame(base: &Palettes, phase: u32) -> [[u8; 4]; 256] {
        let mut result = base.to_gpu_palette();

        // 瀑布动画: 颜色7-11变化
        for idx in 0..5 {
            let j = ((phase + idx as u32) % 5) as i32;
            let k = 5 - j;
            // 使用默认sky_type=0的颜色
            let r = (40 + 3 * k).min(63) as u8;
            let g = (50 + 2 * k).min(63) as u8;
            let b = (53 + 2 * k).min(63) as u8;
            result[7 + idx] = [
                ((r as u16 * 255) / 63).min(255) as u8,
                ((g as u16 * 255) / 63).min(255) as u8,
                ((b as u16 * 255) / 63).min(255) as u8,
                255,
            ];
        }

        result
    }

    /// 获取帧数据
    pub fn get_frame(&self, index: u32) -> &[[u8; 4]; 256] {
        &self.frames[(index as usize).min(self.frames.len() - 1)]
    }

    /// 获取所有帧数据 (用于GPU上传)
    pub fn all_frames(&self) -> &[[[u8; 4]; 256]] {
        &self.frames
    }
}
