// Rust translation of backgr.pas - 严格对应 Pascal BACKGR.PAS
// 包含 Pascal 中的静态数据和绘制逻辑

use crate::buffers::{H, MAX_WORLD_SIZE, NH, NV, W, WorldOptions};
use crate::gpu::sprite_batch::FillCommand;
use crate::palettes::Palettes;
use crate::render_state::RenderState;

// Include generated assets produced by build.rs
include!(concat!(env!("OUT_DIR"), "/generated_assets.rs"));

// Generated static slices provided by build.rs:
// BOGEN_BK, BOGEN7_BK, BOGEN26_BK, MOUNT_BK
// They are &[u8] slices defined in generated_assets.rs
// e.g. pub static BOGEN_BK: &[u8] = &[..];

fn bogen() -> &'static [u8] {
    BOGEN_BK
}

fn bogen7() -> &'static [u8] {
    BOGEN7_BK
}

fn bogen26() -> &'static [u8] {
    BOGEN26_BK
}

fn mount() -> &'static [u8] {
    MOUNT_BK
}

/// 获取 Intro 使用的云层高度表（BOGEN26）
pub fn backgr_map_bogen26() -> &'static [u8] {
    bogen26()
}

/// 获取 Intro 使用的山峰高度表（MOUNT）
pub fn backgr_map_mount() -> &'static [u8] {
    mount()
}

/// 获取 MPAL256 调色板（编译期生成的静态常量）
pub fn get_generated_asset_mpal256() -> &'static crate::palettes::PalType {
    &MPAL256_PALETTE
}

pub const LEFT: i32 = 0;
pub const RIGHT: i32 = 1;
pub const SHIFT: i32 = 16;
pub const SPEED: i32 = 3;
pub const BRICK_SPEED: i32 = 2;

pub const HEIGHT: i32 = 26;
pub const CLOUD_SPEED: i32 = 4;
pub const MAX_CLOUDS: i32 = 7;
pub const MIN_CLOUD_SIZE: i32 = 30;
pub const MAX_CLOUD_SIZE: i32 = 70;
pub const CLOUD_HEIGHT: i32 = 19;
pub const MAX: i32 = (MAX_WORLD_SIZE / SPEED) * W;

/// Rust 结构体，封装 BackGr.pas 的全局状态和数据
pub struct BackGr {
    /// 当前背景类型
    pub background: u8, // BackGround: Byte
    /// 云数量
    pub clouds: u8, // Clouds: Byte
    /// 背景地图
    pub backgr_map: Vec<u8>, // BackGrMap: array [0..Max] of Byte
    /// 颜色映射
    pub color_map: Vec<u16>, // ColorMap: array [0..NV*H-1] of Word
    /// 云坐标
    pub cloud_map: Vec<[i32; 2]>, // CloudMap: array [1..2*MaxClouds, 0..1] of Integer
}

impl BackGr {
    /// 构造默认状态
    pub fn new(_max_world_size: usize, _w: usize, nv: usize, h: usize) -> Self {
        BackGr {
            background: 0,
            clouds: 0,
            backgr_map: vec![0u8; MAX as usize + 1],
            color_map: vec![0u16; nv * h],
            cloud_map: vec![[0; 2]; 2 * MAX_CLOUDS as usize + 1],
        }
    }

    pub fn init_clouds(&mut self) {
        // 严格对齐 Pascal 的静态初始化
        let m = MAX_CLOUDS as usize;
        let c = &mut self.cloud_map;
        if c.len() < 2 * m + 2 {
            return;
        }
        c[1] = [50, 58];
        c[m + 1] = [92, 0];
        c[2] = [180, 20];
        c[m + 2] = [228, 0];
        c[3] = [430, 40];
        c[m + 3] = [484, 0];
        c[4] = [570, 15];
        c[m + 4] = [600, 0];
        c[5] = [840, 30];
        c[m + 5] = [900, 0];
        c[6] = [980, 60];
        c[m + 6] = [1040, 0];
        c[7] = [1200, 20];
        c[m + 7] = [1240, 0];
    }

    // 云朵（Oldsrc PutClouds/TraceCloud）在纯 GPU 管线下通过批量命令绘制实现，
    // 不再依赖 CPU framebuffer 的 GetPixel/PutPixel 读写与覆盖语义。

    /// Rust严格模拟Pascal InitBackGr 逻辑
    pub fn init_backgr(&mut self, new_backgr: u8, b_clouds: u8) {
        self.background = new_backgr;
        let map_len = self.backgr_map.len();
        match self.background {
            1 | 2 => {
                // Pascal注释块：随机生成地形，已省略
                // move(@BOGEN^, BackGrMap, SizeOf(BackGrMap));
                let data = bogen();
                if data.len() >= map_len {
                    self.backgr_map.copy_from_slice(&data[..map_len]);
                } else {
                    for (i, v) in data.iter().enumerate() {
                        if i < map_len {
                            self.backgr_map[i] = *v;
                        }
                    }
                }
            }
            3 => {
                // Pascal注释块：随机生成山地，已省略
                // move(@MOUNT^, BackGrMap, SizeOf(BackGrMap));
                let data = mount();
                if data.len() >= map_len {
                    self.backgr_map.copy_from_slice(&data[..map_len]);
                } else {
                    for (i, v) in data.iter().enumerate() {
                        if i < map_len {
                            self.backgr_map[i] = *v;
                        }
                    }
                }
            }
            9 => {
                // move(@BOGEN7^, BackGrMap, SizeOf(BackGrMap));
                let data = bogen7();
                if data.len() >= map_len {
                    self.backgr_map.copy_from_slice(&data[..map_len]);
                } else {
                    for (i, v) in data.iter().enumerate() {
                        if i < map_len {
                            self.backgr_map[i] = *v;
                        }
                    }
                }
            }
            10 => {
                // move(@BOGEN26^, BackGrMap, SizeOf(BackGrMap));
                let data = bogen26();
                if data.len() >= map_len {
                    self.backgr_map.copy_from_slice(&data[..map_len]);
                } else {
                    for (i, v) in data.iter().enumerate() {
                        if i < map_len {
                            self.backgr_map[i] = *v;
                        }
                    }
                }
            }
            _ => {}
        }
        // BackGrMap 是循环表，尾部若有 0 会在滚屏时被采样到，导致出现竖线
        // 当前 backgr_data BOGEN BOGEN7 BOGEN26 已是目标方向的数据
        // 这里避免再次执行 Height-Map+1 反转，否则会出现地形方向错误
        self.clouds = b_clouds;
        if self.clouds != 0 {
            self.init_clouds();
        }
    }

    /// Rust严格模拟Pascal BrickPalette 逻辑
    /// i: 当前帧或砖块索引
    pub fn brick_palette(&self, i: i32, palette: &mut Palettes, render_state: &mut RenderState) {
        let i = i % 20;
        for j in 0..20 {
            if i == j {
                palette.copy_palette(0xFE, 0xE0 + j as usize, render_state);
            } else if ((i + 2) % 20) == j {
                palette.copy_palette(0xFF, 0xE0 + j as usize, render_state);
            } else {
                palette.copy_palette(0xFD, 0xE0 + j as usize, render_state);
            }
        }
    }

    /// Rust严格模拟Pascal LargeBrickPalette 逻辑
    /// i: 当前帧或砖块索引
    pub fn large_brick_palette(
        &self,
        i: i32,
        palette: &mut Palettes,
        render_state: &mut RenderState,
    ) {
        let i = i % 32;
        for j in 0..32 {
            if i == j || ((i + 1) % 32) == j {
                palette.copy_palette(0xD6, 0xE0 + j as usize, render_state);
            } else if ((i + 3) % 32) == j || ((i + 4) % 32) == j {
                palette.copy_palette(0xD4, 0xE0 + j as usize, render_state);
            } else {
                palette.copy_palette(0xD1, 0xE0 + j as usize, render_state);
            }
        }
    }

    /// Rust严格模拟Pascal PillarPalette 逻辑
    pub fn pillar_palette(
        &self,
        i: i32,
        palette: &mut Palettes,
        render_state: &mut RenderState,
        options: &WorldOptions,
    ) {
        const SHADOW_POS: i32 = 28;
        const SHADOW_END: i32 = 36;
        let i = i % 60;
        // 第一段 Base1
        let mut base = options.backgr_color1;
        let [mut c1, mut c2, mut c3] = palette.get_rgb(base);
        c1 /= 4;
        c2 /= 4;
        c3 /= 4;
        let mut j = 0;
        let mut k = 1;
        while k < 15 {
            for l in j..=k {
                let idx1 = 0xC0 + ((l + i) % 60) as usize;
                let idx2 = 0xC0 + ((SHADOW_POS + i - l) % 60) as usize;
                palette.out_palette(idx1, c1 + k as u8, c2 + k as u8, c3 + k as u8, render_state);
                palette.out_palette(idx2, c1 + k as u8, c2 + k as u8, c3 + k as u8, render_state);
            }
            j = k;
            k += 1;
        }
        for j in SHADOW_POS..=SHADOW_END {
            if c1 > 0 {
                c1 -= 1;
            }
            if c2 > 0 {
                c2 -= 1;
            }
            if c3 > 0 {
                c3 -= 1;
            }
            let idx = 0xC0 + ((j + i) % 60) as usize;
            palette.out_palette(idx, c1, c2, c3, render_state);
        }
        // 第二段 Base2
        base = options.backgr_color2;
        let [mut c1, mut c2, mut c3] = palette.get_rgb(base);
        c1 /= 4;
        c2 /= 4;
        c3 /= 4;
        for j in (SHADOW_END + 1)..60 {
            let idx = 0xC0 + ((i + j) % 60) as usize;
            palette.out_palette(idx, c1, c2, c3, render_state);
        }
    }

    /// Rust严格模拟Pascal WindowPalette 逻辑
    /// i: 当前帧或窗口索引
    pub fn window_palette(&self, i: i32, palette: &mut Palettes, render_state: &mut RenderState) {
        let i = i % 32;
        for j in 0..6 {
            let idx = 0xE0 + ((i + j) % 32) as usize;
            palette.copy_palette(1, idx, render_state);
        }
        for j in 6..32 {
            let idx = 0xE0 + ((i + j) % 32) as usize;
            palette.copy_palette(16, idx, render_state);
        }
    }

    /// Rust严格模拟Pascal/汇编 DrawPalBackGr 逻辑
    /// 调整背景砖块柱子窗口等的动态调色板
    ///
    /// palette: 调色板对象
    /// vga: RenderState 对象
    /// options: 世界参数，仅柱子调色板需要
    ///
    /// 注意 地下场景 sky_type=8 时不更新 0xE0..0xFF 区域的动态调色板 防止闪烁
    /// 若每帧修改这些索引会导致砖墙闪烁
    /// 因此 sky_type==8 时直接返回 不再对 0xE0.. 区域做动态修改
    pub fn draw_pal_backgr(
        &self,
        palette: &mut Palettes,
        render_state: &mut RenderState,
        options: Option<&WorldOptions>,
    ) {
        if let Some(opts) = options {
            // 地下天空类型=8) 禁用动砖柱子调色板，防止调色板闪烁成背景闪屏
            if opts.sky_type == 8 {
                return;
            }
        }

        let i = ((render_state.x_view as f32) / BRICK_SPEED as f32).round() as i32;
        match self.background {
            4 => self.brick_palette(i, palette, render_state),
            5 => self.large_brick_palette(i, palette, render_state),
            6 => {
                if let Some(opts) = options {
                    self.pillar_palette(i, palette, render_state, opts);
                }
            }
            7 => self.window_palette(i, palette, render_state),
            _ => {}
        }
    }

    // ========== GPU渲染支持方法 ==========

    /// GPU模式：收集 PutBackGr 的背景形状（对齐 Oldsrc put_backgr 的填充效果）
    /// 说明：
    /// - 原版会用 get_pixel_world 做“天空掩码”避免覆盖前景
    /// - GPU 版无读回，这里只负责生成背景形状，渲染顺序保证它在前景之前
    pub fn collect_put_backgr_fills(
        &self,
        x_view: i32,
        options: &WorldOptions,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();
        if !(matches!(options.backgr_type, 1..=3 | 9..=11)) {
            return fills;
        }

        let horizon = options.horizon as i32;
        let y_base = horizon - HEIGHT;
        let y_end = horizon - 1;
        let screen_w = crate::render_state::SCREEN_WIDTH as i32;

        // BackGrMap 是循环表，尾部 0 会在滚屏时被采样到
        let mut effective_len = self.backgr_map.len() as i32;
        while effective_len > 0 && self.backgr_map[(effective_len - 1) as usize] == 0 {
            effective_len -= 1;
        }
        if effective_len <= 0 {
            effective_len = self.backgr_map.len().max(1) as i32;
        }

        let x_start = x_view / SPEED;
        for sx in 0..screen_w {
            let idx = (x_start + sx).rem_euclid(effective_len) as usize;
            let h = self.backgr_map.get(idx).copied().unwrap_or(0) as i32;
            let top = (y_base + (HEIGHT - h)).clamp(y_base, y_end);
            if top <= y_end {
                fills.push(FillCommand::new(sx, top, 1, y_end - top + 1, 0xF0));
            }
        }
        fills
    }

    /// GPU模式：收集 DrawBackGrMap 的背景装饰（对齐 Oldsrc draw_backgr_map）
    pub fn collect_backgr_map_fills(
        &self,
        y1: i32,
        y2: i32,
        shift: i32,
        c: u8,
    ) -> Vec<FillCommand> {
        self.collect_backgr_map_fills_from_map(&self.backgr_map, y1, y2, shift, c)
    }

    /// GPU模式：收集 DrawBackGrMap 的背景装饰（允许指定高度表）
    pub fn collect_backgr_map_fills_from_map(
        &self,
        map: &[u8],
        y1: i32,
        y2: i32,
        shift: i32,
        c: u8,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();
        let screen_w = crate::render_state::SCREEN_WIDTH as i32;

        let mut effective_len = map.len() as i32;
        while effective_len > 0 && map[(effective_len - 1) as usize] == 0 {
            effective_len -= 1;
        }
        if effective_len <= 0 {
            effective_len = map.len().max(1) as i32;
        }

        for sx in 0..screen_w {
            let idx = (sx + shift).rem_euclid(effective_len) as usize;
            let h = map.get(idx).copied().unwrap_or(0) as i32;
            let top = y1 - h;
            if top <= y2 {
                fills.push(FillCommand::new(sx, top, 1, y2 - top + 1, c));
            }
        }

        fills
    }

    /// GPU模式：收集云朵填充命令（简化版本）
    /// 使用椭圆形填充矩形来模拟云朵形状
    pub fn collect_cloud_fills(&self, x_view: i32) -> Vec<FillCommand> {
        let mut fills = Vec::new();

        if self.clouds == 0 {
            return fills;
        }

        let max_clouds = MAX_CLOUDS as usize;

        for i in 1..=max_clouds {
            if i >= self.cloud_map.len() || i + max_clouds >= self.cloud_map.len() {
                continue;
            }

            let cloud_start = &self.cloud_map[i];
            let cloud_end = &self.cloud_map[i + max_clouds];

            let x1 = cloud_start[0] - x_view / CLOUD_SPEED;
            let x2 = cloud_end[0] - x_view / CLOUD_SPEED;
            let y = cloud_start[1];
            let cloud_width = (x2 - x1).max(0);

            // 只渲染在可视范围内的云朵
            if x2 < 0 || x1 > NH * W {
                continue;
            }

            // 使用渐变色来模拟云朵的圆形效果
            // 云朵颜色使用背景色系 (0xE0-0xEF范围)
            let cloud_color = 0xE8u8; // 浅蓝色

            // 简化版：使用单个矩形表示云朵
            if cloud_width > 0 && y < NV * H {
                fills.push(FillCommand::new(
                    x1,
                    y,
                    cloud_width,
                    CLOUD_HEIGHT.min(NV * H - y),
                    cloud_color,
                ));
            }
        }

        fills
    }

    /// GPU模式：收集smooth_fill填充命令（严格对齐 Oldsrc）
    pub fn collect_smooth_fills(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        options: &WorldOptions,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();
        let horizon = options.horizon.saturating_sub(4) as i32;

        let mut cur_y = y;
        // 严格对齐 Oldsrc：根据起始 y 计算初始 dh 和 dl
        let mut dh: i32 = ((cur_y % 6 + 6) % 6) as i32;
        let mut dl: u8 = if cur_y >= horizon {
            0xF0
        } else {
            let q = (cur_y / 6).max(0) as u8;
            let mut v = 0xEFu8.wrapping_sub(q);
            if v < 0xE0 {
                v = 0xE0;
            }
            v
        };

        for _ in 0..h {
            fills.push(FillCommand::new(x, cur_y, w, 1, dl));

            cur_y += 1;
            if cur_y >= horizon {
                dl = 0xF0;
            }
            dh += 1;
            if dh == 6 {
                dh = 0;
                if dl != 0xE0 && dl != 0xF0 {
                    dl = dl.wrapping_sub(1);
                }
            }
        }

        fills
    }

    /// GPU模式：收集背景渲染数据
    pub fn collect_background_data(
        &self,
        _x_view: i32,
        options: &WorldOptions,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();

        // 根据背景类型生成填充命令
        match options.backgr_type {
            0 => {
                // 单色背景
                fills.push(FillCommand::new(0, 0, NH * W, NV * H, 0xE0));
            }
            1..=3 => {
                // 山峰/渐变背景 - 简化为渐变填充
                for row in 0..(NV * H) {
                    let color = 0xE0 + (row / 12).min(15) as u8;
                    fills.push(FillCommand::new(0, row, NH * W, 1, color));
                }
            }
            4..=7 => {
                // 地下室背景
                fills.push(FillCommand::new(0, 0, NH * W, NV * H, 0x18));
            }
            _ => {}
        }

        fills
    }

    // ========== GPU直接渲染方法 ==========

    /// GPU版smooth_fill - 直接向render_state.sprite_batch添加填充命令
    /// 严格对齐 Oldsrc BACKGR.PAS SmoothFill 的像素效果
    pub fn smooth_fill_gpu(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        options: &WorldOptions,
        render_state: &mut RenderState,
    ) {
        // 算法说明：按 6 行为一个周期生成渐变色
        // cur_y >= horizon 时使用 0xF0，否则从 0xEF 开始随行数递减并下限到 0xE0
        let horizon = options.horizon.saturating_sub(4) as i32;
        let mut cur_y = y as i32;

        // 严格对齐 Oldsrc：根据起始 y 计算初始 dh
        let mut dh: i32 = ((cur_y % 6 + 6) % 6) as i32;

        // 严格对齐 Oldsrc：根据起始 y 计算初始 dl
        let mut dl: u8 = if cur_y >= horizon {
            0xF0
        } else {
            let q = (cur_y / 6).max(0) as u8;
            let mut v = 0xEFu8.wrapping_sub(q);
            if v < 0xE0 {
                v = 0xE0;
            }
            v
        };

        for _ in 0..h {
            render_state.fill_world_gpu(x as i32, cur_y, w as i32, 1, dl);

            cur_y += 1;
            if cur_y >= horizon {
                dl = 0xF0;
            }
            dh += 1;
            if dh == 6 {
                dh = 0;
                if dl != 0xE0 && dl != 0xF0 {
                    dl = dl.wrapping_sub(1);
                }
            }
        }
    }

    /// GPU版draw_bricks - 平铺砖块纹理（使用填充颜色模拟）
    pub fn draw_bricks_gpu(&self, x: i32, y: i32, w: i32, h: i32, render_state: &mut RenderState) {
        // 简化版本：使用填充色模拟砖块纹理
        let brick_color = 0x48u8; // 砖块基础色
        render_state.fill_world_gpu(x, y, w, h, brick_color);
    }

    /// GPU版large_bricks - 大砖块填充
    pub fn large_bricks_gpu(&self, x: i32, y: i32, w: i32, h: i32, render_state: &mut RenderState) {
        // 渐变砖块效果
        for dy in 0..h {
            let color = 0xE0 + ((y + dy) & 0x1F) as u8;
            render_state.fill_world_gpu(x, y + dy, w, 1, color);
        }
    }

    /// GPU版pillar - 柱子装饰（使用精灵）
    pub fn pillar_gpu(&self, x: i32, y: i32, _w: i32, _h: i32, render_state: &mut RenderState) {
        // 柱子使用填充色模拟
        let pillar_color = 0x30u8;
        render_state.fill_world_gpu(x, y, W, H, pillar_color);
    }

    /// GPU版windows - 窗户填充
    pub fn windows_gpu(&self, x: i32, y: i32, w: i32, h: i32, render_state: &mut RenderState) {
        // 窗户背景
        render_state.fill_world_gpu(x, y, w, h, 0x18);
    }
}
