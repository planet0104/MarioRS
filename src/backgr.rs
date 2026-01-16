// Rust translation of backgr.pas - 严格对应 Pascal BACKGR.PAS
// 包含 Pascal 中的静态数据和绘制逻辑

use crate::buffers::{Buffers, H, MAX_WORLD_SIZE, NH, NV, W, WorldOptions};
use crate::palettes::Palettes;
use crate::sprites::SpriteDataManager;
use crate::vga256::{self, VGA};
use crate::gpu::sprite_batch::{FillCommand, SpriteCommand};
use crate::gpu::texture_atlas::SpriteUV;

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

    /// 严格模拟 Pascal 或汇编 PutClouds 逻辑
    /// 内部实现：使用指定的 x_view 绘制云，不修改 buffers
    fn put_clouds_internal(
        &mut self,
        temp_x_view: i32,
        offset: i32,
        n: i32,
        vga: &mut VGA,
        _buffers: &Buffers,
    ) {
        if self.clouds == 0 {
            return;
        }
        let max_clouds = MAX_CLOUDS as usize;
        let nh = NH;
        let w = W;
        let x_view = temp_x_view; // 使用传入的临时 x_view
        let mut i = 1;
        while i <= max_clouds {
            let attr = self.clouds;
            let ovr = 0xE0;
            let x1 = x_view - offset + self.cloud_map[i][0];
            let x2 = x_view - offset + self.cloud_map[i + max_clouds][0];
            let xsize = x2 - x1 + 1;
            let y = self.cloud_map[i][1];

            if n > 0 {
                let mut size = 0;
                if x2 + 10 >= x_view + nh * w {
                    size = 10;
                }
                if (x2 + 10 > x_view) && (x2 < x_view + nh * w + 10) {
                    self.trace_cloud(x2 - n - size, y, n + size, RIGHT as u8, attr, ovr, vga);
                }
                if (x1 + 10 > x_view) && (x1 < x_view + nh * w) {
                    self.trace_cloud(x1 - n, y, n, LEFT as u8, ovr, attr, vga);
                    if !(x2 < x_view + nh * w) {
                        self.trace_cloud(x1, y, xsize, LEFT as u8, attr, ovr, vga);
                    }
                }
            }
            if n < 0 {
                if (x2 + 10 > x_view) && (x2 < x_view + nh * w + 10) {
                    self.trace_cloud(x2, y, -n, RIGHT as u8, ovr, attr, vga);
                    if !(x1 > x_view - 10) {
                        self.trace_cloud(x2 - xsize, y, xsize, RIGHT as u8, attr, ovr, vga);
                    }
                }
                let mut size = 0;
                if x1 < x_view + 10 {
                    size = 10;
                }
                if (x1 + 10 > x_view) && (x1 < x_view + nh * w + 10) {
                    self.trace_cloud(x1, y, -n + size, LEFT as u8, attr, ovr, vga);
                }
            }
            i += 1;
        }
    }

    /// 公共接口 使用 buffers.x_view 绘制云 保持向后兼容
    pub fn put_clouds(&mut self, offset: i32, n: i32, vga: &mut VGA, buffers: &Buffers) {
        self.put_clouds_internal(buffers.x_view, offset, n, vga, buffers);
    }

    /// 严格模拟 Pascal TraceCloud 逻辑
    /// X Y N 坐标和长度
    /// Dir Attr Ovr 方向 颜色 覆盖色
    pub fn trace_cloud(
        &mut self,
        x: i32,
        y: i32,
        n: i32,
        dir: u8,
        attr: u8,
        ovr: u8,
        vga: &mut VGA,
    ) {
        // 注意：只通过 vga.get_pixel_world 和 vga.put_pixel_world 访问像素，避免手工做 x_view 偏移
        // Pascal 语义：X 为世界坐标
        // 仅当像素值等于 ovr 时写入 attr
        // ok 表示本行已写入过像素，超出可见范围后可提前结束本行
        //
        //
        //
        //

        let min_x_world = vga.x_view;
        let max_x_world = vga.x_view + 319;

        let dl = attr;
        let bl = ovr;
        let rows: i32 = 19; // CloudHeight
        let len = n.abs();

        // 偏移表：对应 Pascal BACKGR.PAS 内嵌表
        let left_list: [i32; 19] = [
            9, -3, -2, -1, -1, -1, 0, -1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 2, 3,
        ];
        let right_list: [i32; 19] = [
            0, 3, 2, 1, 1, 1, 0, 1, 0, 0, 0, 0, -1, 0, -1, -1, -1, -2, -3,
        ];
        let list = if dir == RIGHT as u8 {
            &right_list
        } else {
            &left_list
        };
        for row in 0..rows {
            let mut ok = false;
            let offset = list[row as usize];

            let y_world = y + row;
            let start_x_world = x + offset;

            let mut i = 0;
            while i < len {
                let x_world = start_x_world + i;
                // Pascal 语义：只有像== Ovr 才会进入裁剪逻辑
                if vga.get_pixel_world(x_world, y_world) == bl {
                    if x_world < min_x_world || x_world > max_x_world {
                        if ok {
                            break;
                        }
                    } else {
                        vga.put_pixel_world(x_world, y_world, dl);
                        ok = true;
                    }
                }
                i += 1;
            }
        }
    }

    /// Rust 严格模拟 Pascal StartClouds 逻辑
    /// 初始化云朵
    ///
    /// Pascal 原版 StartClouds 会临时修改 XView 作为副作用
    /// Rust 版改为传入 x_view，仅用于计算，不修改 buffers.x_view
    pub fn start_clouds(&mut self, x_view: i32, vga: &mut VGA, buffers: &Buffers) {
        if self.clouds == 0 {
            return;
        }
        // Pascal: for i := XView + MaxCloudSize downto XView do
        //   PutClouds(i div CloudSpeed, -CloudSpeed);
        //
        // 不再修改 buffers.x_view，而是把临时 x_view 传入 put_clouds_internal
        for i in (x_view..=x_view + MAX_CLOUD_SIZE).rev() {
            self.put_clouds_at(i, i / CLOUD_SPEED, -CLOUD_SPEED, vga, buffers);
        }
    }

    /// put_clouds 的内部版本，接受临时 x_view 而不修改 buffers
    fn put_clouds_at(
        &mut self,
        temp_x_view: i32,
        x: i32,
        dir: i32,
        vga: &mut VGA,
        buffers: &Buffers,
    ) {
        // put_clouds 逻辑，但使用 temp_x_view 代替 buffers.x_view
        self.put_clouds_internal(temp_x_view, x, dir, vga, buffers);
    }

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

    /// Rust严格模拟Pascal PutBackGr 逻辑
    pub fn put_backgr(
        &self,
        map: &[u8],
        fill: bool,
        vga: &mut VGA,
        buffers: &mut Buffers,
        options: &WorldOptions,
    ) {
        // 重要：Pascal PutBackGr 基于 VGA Mode X 平面写显存；Rust 使用线性 framebuffer
        // 这里不能按平面步进写入，否则会出现覆盖错位
        //
        // 对齐 Pascal 的覆盖语义：只在当前像素处于天空掩码区时写入背景，避免覆盖前景精灵

        let x_start = buffers.x_view / SPEED;
        let horizon = options.horizon as i32;
        let y_base = horizon - HEIGHT; // Pascal: Horizon-HEIGHT
        let y_end = horizon - 1;

        // 简化实现：按整屏重绘背景掩码，行为更稳定
        let screen_w = 320;
        // BackGrMap 是循环表，尾部 0 会在滚屏时被采样到，导致出现竖线
        let mut effective_len = map.len() as i32;
        while effective_len > 0 && map[(effective_len - 1) as usize] == 0 {
            effective_len -= 1;
        }
        if effective_len <= 0 {
            effective_len = map.len().max(1) as i32;
        }
        for sx in 0..screen_w {
            let idx = (x_start + sx).rem_euclid(effective_len) as usize;
            let h = map.get(idx).copied().unwrap_or(0) as i32;

            // 将 map 值理解为形状高度，从 y_base + (HEIGHT - h) 开始向下填充到 y_end
            let top = (y_base + (HEIGHT - h)).clamp(y_base, y_end);

            // 统一使用 world 坐标：world_x = XView + screen_x
            let x_world = buffers.x_view + sx;
            if fill {
                for y in top..=y_end {
                    if vga.get_pixel_world(x_world, y) >= 0xC0 {
                        vga.put_pixel_world(x_world, y, 0xF0);
                    }
                }
            } else {
                for y in top..=y_end {
                    if vga.get_pixel_world(x_world, y) >= 0xC0 {
                        vga.put_pixel_world(x_world, y, 0xF0);
                    }
                }
            }
        }
    }

    /// Rust严格模拟Pascal BrickPalette 逻辑
    /// i: 当前帧或砖块索引
    pub fn brick_palette(&self, i: i32, palette: &mut Palettes, vga: &mut vga256::VGA) {
        let i = i % 20;
        for j in 0..20 {
            if i == j {
                palette.copy_palette(0xFE, 0xE0 + j as usize, vga);
            } else if ((i + 2) % 20) == j {
                palette.copy_palette(0xFF, 0xE0 + j as usize, vga);
            } else {
                palette.copy_palette(0xFD, 0xE0 + j as usize, vga);
            }
        }
    }

    /// Rust严格模拟Pascal LargeBrickPalette 逻辑
    /// i: 当前帧或砖块索引
    pub fn large_brick_palette(&self, i: i32, palette: &mut Palettes, vga: &mut vga256::VGA) {
        let i = i % 32;
        for j in 0..32 {
            if i == j || ((i + 1) % 32) == j {
                palette.copy_palette(0xD6, 0xE0 + j as usize, vga);
            } else if ((i + 3) % 32) == j || ((i + 4) % 32) == j {
                palette.copy_palette(0xD4, 0xE0 + j as usize, vga);
            } else {
                palette.copy_palette(0xD1, 0xE0 + j as usize, vga);
            }
        }
    }

    /// Rust严格模拟Pascal PillarPalette 逻辑
    pub fn pillar_palette(
        &self,
        i: i32,
        palette: &mut Palettes,
        vga: &mut vga256::VGA,
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
                palette.out_palette(idx1, c1 + k as u8, c2 + k as u8, c3 + k as u8, vga);
                palette.out_palette(idx2, c1 + k as u8, c2 + k as u8, c3 + k as u8, vga);
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
            palette.out_palette(idx, c1, c2, c3, vga);
        }
        // 第二段 Base2
        base = options.backgr_color2;
        let [mut c1, mut c2, mut c3] = palette.get_rgb(base);
        c1 /= 4;
        c2 /= 4;
        c3 /= 4;
        for j in (SHADOW_END + 1)..60 {
            let idx = 0xC0 + ((i + j) % 60) as usize;
            palette.out_palette(idx, c1, c2, c3, vga);
        }
    }

    /// Rust严格模拟Pascal WindowPalette 逻辑
    /// i: 当前帧或窗口索引
    pub fn window_palette(&self, i: i32, palette: &mut Palettes, vga: &mut vga256::VGA) {
        let i = i % 32;
        for j in 0..6 {
            let idx = 0xE0 + ((i + j) % 32) as usize;
            palette.copy_palette(1, idx, vga);
        }
        for j in 6..32 {
            let idx = 0xE0 + ((i + j) % 32) as usize;
            palette.copy_palette(16, idx, vga);
        }
    }

    /// Rust严格模拟Pascal DrawBackGr 逻辑
    pub fn draw_backgr(
        &mut self,
        first_time: bool,
        vga: &mut VGA,
        buffers: &mut Buffers,
        options: &WorldOptions,
    ) {
        // Rust严格模拟Pascal DrawBackGr 逻辑
        match self.background {
            1..=3 | 9..=11 => {
                self.put_backgr(&self.backgr_map, first_time, vga, buffers, options);
            }
            _ => {}
        }

        if self.clouds != 0 {
            let i = buffers.x_view / CLOUD_SPEED;
            let dx = buffers.x_view - buffers.last_x_view[vga.current_page() as usize];
            self.put_clouds(i, dx, vga, buffers);
        }
    }

    // Pascal 语义 BackGrMap 为循环表 尾部不应采样到 0 否则滚屏后会出现竖线
    /// y1: 起始y坐标
    /// y2: 结束y坐标
    /// shift: x方向偏移
    /// c: 瑕佺粯鍒剁殑棰滆壊
    /// vga VGA 显存对象 内部会做 world->screen 转换
    pub fn draw_backgr_map(&self, y1: i32, y2: i32, shift: i32, c: u8, vga: &mut vga256::VGA) {
        // Pascal 语义 BackGrMap 为循环表 尾部不应采样到 0 否则滚屏后会出现竖线
        let mut effective_len = self.backgr_map.len() as i32;
        while effective_len > 0 && self.backgr_map[(effective_len - 1) as usize] == 0 {
            effective_len -= 1;
        }
        if effective_len <= 0 {
            effective_len = self.backgr_map.len().max(1) as i32;
        }
        for sx in 0..320 {
            // Pascal 语义 BackGrMap 为循环表 尾部不应采样到 0 否则滚屏后会出现竖线
            let idx = (sx as i32 + shift).rem_euclid(effective_len) as usize;
            let h = self.backgr_map.get(idx).copied().unwrap_or(0) as i32;
            for j in (y1 - h)..=y2 {
                // world_x = XView + screen_x
                let x_world = vga.x_view + sx;
                if vga.get_pixel_world(x_world, j) >= 0xC0 {
                    vga.put_pixel_world(x_world, j, c);
                }
            }
        }
    }

    /// Rust严格模拟Pascal/汇编 DrawPalBackGr 逻辑
    /// 调整背景砖块柱子窗口等的动态调色板
    ///
    /// palette: 调色板对象
    /// vga: VGA 对象
    /// options: 世界参数，仅柱子调色板需要
    ///
    /// 注意 地下场景 sky_type=8 时不更新 0xE0..0xFF 区域的动态调色板 防止闪烁
    /// 若每帧修改这些索引会导致砖墙闪烁
    /// 因此 sky_type==8 时直接返回 不再对 0xE0.. 区域做动态修改
    pub fn draw_pal_backgr(
        &self,
        palette: &mut Palettes,
        vga: &mut vga256::VGA,
        options: Option<&WorldOptions>,
    ) {
        if let Some(opts) = options {
            // 地下天空类型=8) 禁用动砖柱子调色板，防止调色板闪烁成背景闪屏
            if opts.sky_type == 8 {
                return;
            }
        }

        let i = ((vga.x_view as f32) / BRICK_SPEED as f32).round() as i32;
        match self.background {
            4 => self.brick_palette(i, palette, vga),
            5 => self.large_brick_palette(i, palette, vga),
            6 => {
                if let Some(opts) = options {
                    self.pillar_palette(i, palette, vga, opts);
                }
            }
            7 => self.window_palette(i, palette, vga),
            _ => {}
        }
    }

    /// Rust严格模拟Pascal ReadColorMap 逻辑
    pub fn read_color_map(&mut self, buffers: &Buffers, vga: &VGA) {
        let total = (NV * H) as usize;
        if self.color_map.len() < total {
            self.color_map.resize(total, 0);
        }
        let x_view = buffers.x_view;
        for i in 0..total {
            // Pascal: ColorMap[i] := GetPixel(XView + Shift, i) * 256 + GetPixel(XView + Shift + 1, i);
            let c0 = vga.get_pixel_world(x_view + SHIFT, i as i32) as u16;
            let c1 = vga.get_pixel_world(x_view + SHIFT + 1, i as i32) as u16;
            self.color_map[i] = c0 * 256 + c1;
        }
    }

    /// Rust严格模拟Pascal/汇编 DrawBricks 逻辑
    /// x: 左上角x坐标（世界坐标）
    /// y: 左上角y坐标（世界坐标）
    /// vga VGA 显存对象 内部会做 world->screen 转换
    pub fn draw_bricks(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA, sprites: &SpriteDataManager) {
        // PALBRICK_000 是 20x14 的砖块纹理，需要平铺填充到 (w x h) 区域
        let brick_w = 20;
        let brick_h = 14;
        let brick_data = &sprites.PALBRICK_000;
        // 平铺填充砖块纹理
        // 使用 put_image 而非 draw_image：背景砖块应不透明绘制，不跳过 0 值像素
        for ty in (0..h).step_by(brick_h as usize) {
            for tx in (0..w).step_by(brick_w as usize) {
                let actual_w = (brick_w).min(w - tx) as usize;
                let actual_h = (brick_h).min(h - ty) as usize;
                // 使用世界坐标版的不透明绘制方法
                vga.put_image_imagebuffer_partial_world(
                    x + tx, y + ty, actual_w, actual_h, brick_data
                );
            }
        }
    }
    /// 严格模拟 Pascal LargeBricks 逻辑
    pub fn large_bricks(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA) {
        let screen_width = vga.width as i32;

        let mut bl = ((y * screen_width + x) & 0xFF) as u8;
        bl &= 0b0001_1111;
        bl = bl.wrapping_add(0xE0);

        let dl_tmp = ((y + 14) & 0xFF) as u8;
        if (dl_tmp & 0b0001_0000) != 0 {
            bl ^= 16;
        }

        for dy in 0..h {
            let di_y = y + dy;
            let dl = (y + dy) as u8;
            let dl_inner = dl & 0x0F;
            let color: u8;

            if dl_inner == 2 {
                color = 0xD4;
            } else if dl_inner > 2 {
                // 普砖块，颜色随列变化
                let mut al = bl;
                for i in 0..w {
                    let color = (al & 0b0001_1111) | 0xE0;
                    vga.put_pixel_world(x + i, di_y, color);
                    al = al.wrapping_add(1);
                }
                continue;
            } else if dl_inner > 0 {
                color = 0xD1;
            } else {
                color = 0xD6;
                bl ^= 16;
            }

            // 填充一行
            for i in 0..w {
                vga.put_pixel_world(x + i, di_y, color);
            }
        }
    }

    /// Rust严格模拟Pascal Pillar 逻辑
    /// x: 左上角x坐标
    /// y: 左上角y坐标
    /// w: 宽度
    /// h: 高度
    /// vga VGA 显存对象 内部会做 world->screen 转换
    pub fn pillar(&self, x: i32, y: i32, _w: i32, _h: i32, vga: &mut vga256::VGA, sprites: &SpriteDataManager) {
        match (x / 20) % 3 {
            0 => vga.draw_sprite_world(x, y, &sprites.PALPILL_000),
            1 => vga.draw_sprite_world(x, y, &sprites.PALPILL_001),
            2 => vga.draw_sprite_world(x, y, &sprites.PALPILL_002),
            _ => {}
        }
    }

    /// Rust严格模拟Pascal Windows 逻辑
    pub fn windows(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA) {
        let screen_width = vga.width as i32;
        let mut si = y + 22;
        for dy in 0..h {
            let di_y = y + dy;
            let bl = ((y + dy) * screen_width + x) as u8 | 0xC0;
            if (si & 0b0001_1111) >= 0b0000_0011 {
                // 普通窗口填充
                let mut al = bl;
                for i in 0..w {
                    let color = (al & 0b0001_1111) | 0b1110_0000;
                    vga.put_pixel_world(x + i, di_y, color);
                    al = al.wrapping_add(1);
                }
            } else {
                // 特殊分支：固定颜色填充，模拟原版批量写显存的效果
                let color = 0x01;
                let mut i = 0;
                // 每次写入两个像素
                while i + 1 < w {
                    vga.put_pixel_world(x + i, di_y, color);
                    vga.put_pixel_world(x + i + 1, di_y, color);
                    i += 2;
                }
                // 处理剩余一个像素
                if i < w {
                    vga.put_pixel_world(x + i, di_y, color);
                }
            }
            si += 1;
        }
    }

    /// 绘制一块背景区域
    /// - sky_type 在部分取值时使用 smooth_fill 绘制天空渐变
    /// - 其它情况根据 self.background 绘制砖块大砖柱子窗口或按 color_map 逐行填充
    pub fn draw_backgr_block(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        vga: &mut vga256::VGA,
        options: &WorldOptions,
        sprites: &SpriteDataManager,
    ) {
        let sky_type = options.sky_type;
        match sky_type {
            2 | 5 | 9 | 10 | 11 | 12 => {
                self.smooth_fill(
                    x as usize, y as usize, w as usize, h as usize, &options, vga,
                );
            }
            _ => match self.background {
                4 => self.draw_bricks(x, y, w, h, vga, sprites),
                5 => self.large_bricks(x, y, w, h, vga),
                6 => self.pillar(x, y, w, h, vga, sprites),
                7 => self.windows(x, y, w, h, vga),
                _ => {
                    for i in 0..h {
                        let color =
                            self.color_map.get((y + i) as usize).copied().unwrap_or(0) as u8;
                        vga.fill_world(x, y + i, w, 1, color);
                    }
                }
            },
        }
    }

    // 严格对齐 Pascal BACKGR.PAS SmoothFill(X,Y,W,H) 的像素效果 线性实现
    /// x y w h 为世界坐标下的矩形区域
    pub fn smooth_fill(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        options: &WorldOptions,
        vga: &mut vga256::VGA,
    ) {
        // 算法说明：按 6 行为一个周期生成渐变色，并对部分像素做抖动来模拟原版效果
        // cur_y >= horizon 时使用 0xF0，否则从 0xEF 开始随行数递减并下限到 0xE0

        let screen_w = vga.width as i32;
        let screen_h = vga.height as i32;

        let x0 = x as i32;
        let y0 = y as i32;
        let ww = w as i32;
        let hh = h as i32;

        let horizon = options.horizon.saturating_sub(4) as i32;

        let mut cur_y = y0;
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

        for _ in 0..hh {
            // x0 和 cur_y 是世界坐标 DrawSky 传入 x_view 作为 X 因此需要在屏幕坐标下做边界判断
            // 之前用世界 x 与 screen_w 比较 会导致 x_view>0 时整段填充被跳过
            // 新出现的列会残留旧像素 例如 debug overlay 的红色
            let sy = cur_y - vga.y_view;
            if sy >= 0 && sy < screen_h {
                // 1 全行写入 dl
                for dx in 0..ww {
                    let px_world = x0 + dx;
                    let sx = px_world - vga.x_view;
                    if sx < 0 || sx >= screen_w {
                        continue;
                    }
                    vga.put_pixel(sx, sy, dl);
                }

                // 当 dh>=3 且 dl 不是 0xE0 0xF0 时 对部分像素写 dl-1 形成抖动效果
                if dh >= 3 && dl != 0xE0 && dl != 0xF0 {
                    let dl2 = dl.wrapping_sub(1);
                    let parity = dh & 1;
                    for dx in 0..ww {
                        let px_world = x0 + dx;
                        let sx = px_world - vga.x_view;
                        if sx < 0 || sx >= screen_w {
                            continue;
                        }
                        // 抖动 pattern WORLD 坐标为基准（避免随相机滚动产生闪烁）
                        let m = ((px_world % 4 + 4) % 4) as i32;
                        let select = if parity == 0 {
                            m == 0 || m == 2
                        } else {
                            m == 1 || m == 3
                        };
                        if select {
                            vga.put_pixel(sx, sy, dl2);
                        }
                    }
                }
            }

            // 对齐 Pascal 的覆盖语义
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

    // ========== GPU渲染支持方法 ==========

    /// GPU模式：收集云朵精灵命令
    pub fn collect_cloud_sprites(&self, x_view: i32) -> Vec<SpriteCommand> {
        let mut sprites = Vec::new();
        
        // 云朵作为填充矩形渲染（简化版本）
        for i in 1..=(self.clouds as usize) {
            if i < self.cloud_map.len() {
                let cloud = &self.cloud_map[i];
                let cx = cloud[0] - x_view / CLOUD_SPEED;
                let cy = cloud[1];
                
                if cx > -100 && cx < (NH * W + 100) {
                    // 云朵使用默认UV (需要在图集中预留云朵纹理)
                    let uv = SpriteUV { x: 0, y: 0, width: 60, height: 20 };
                    sprites.push(SpriteCommand::new(cx, cy, uv));
                }
            }
        }
        
        sprites
    }

    /// GPU模式：收集smooth_fill填充命令
    pub fn collect_smooth_fills(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        options: &WorldOptions,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();
        let horizon = options.horizon as i32;
        
        let mut cur_y = y;
        let mut dl = 0xE0u8;
        let mut dh = 0i32;
        
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
    pub fn collect_background_data(&self, _x_view: i32, options: &WorldOptions) -> Vec<FillCommand> {
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
    
    /// GPU版smooth_fill - 直接向vga.sprite_batch添加填充命令
    pub fn smooth_fill_gpu(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        options: &WorldOptions,
        vga: &mut vga256::VGA,
    ) {
        let horizon = options.horizon as i32;
        let mut cur_y = y as i32;
        let mut dl = 0xE0u8;
        let mut dh = 0i32;
        
        for _ in 0..h {
            vga.fill_world_gpu(x as i32, cur_y, w as i32, 1, dl);
            
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
    pub fn draw_bricks_gpu(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA) {
        // 简化版本：使用填充色模拟砖块纹理
        let brick_color = 0x48u8; // 砖块基础色
        vga.fill_world_gpu(x, y, w, h, brick_color);
    }
    
    /// GPU版large_bricks - 大砖块填充
    pub fn large_bricks_gpu(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA) {
        // 渐变砖块效果
        for dy in 0..h {
            let color = 0xE0 + ((y + dy) & 0x1F) as u8;
            vga.fill_world_gpu(x, y + dy, w, 1, color);
        }
    }
    
    /// GPU版pillar - 柱子装饰（使用精灵）
    pub fn pillar_gpu(&self, x: i32, y: i32, _w: i32, _h: i32, vga: &mut vga256::VGA) {
        // 柱子使用填充色模拟
        let pillar_color = 0x30u8;
        vga.fill_world_gpu(x, y, W, H, pillar_color);
    }
    
    /// GPU版windows - 窗户填充
    pub fn windows_gpu(&self, x: i32, y: i32, w: i32, h: i32, vga: &mut vga256::VGA) {
        // 窗户背景
        vga.fill_world_gpu(x, y, w, h, 0x18);
    }
}
