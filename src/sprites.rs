// 马里奥游戏精灵数据和调色板模块
#![allow(non_snake_case)]
// 说明：历史版本此文件为自动生成的"硬编码数组"形式。
// 当前版本改为：编译期嵌入 assets/sprites 目录，再在 Rust 中解析 Pascal db 数据并做 Mode X 去平面化。

use crate::buffers::{ImageBuffer, ImageBuffer12x7, ImageBuffer20x24, ImageBuffer24x20};
use crate::backgr::get_generated_asset;

/// 调色板静态数组，索引0-159，每个元素为RGBA值
pub static PALETTE: [(u8, u8, u8, u8); 160] = [
    (0, 0, 0, 255),       // 索引 0
    (0, 40, 167, 255),    // 索引 1
    (48, 112, 72, 255),   // 索引 2
    (8, 175, 88, 255),    // 索引 3
    (143, 79, 39, 255),   // 索引 4
    (232, 0, 0, 255),     // 索引 5
    (231, 143, 48, 255),  // 索引 6
    (175, 175, 191, 255), // 索引 7
    (96, 96, 112, 255),   // 索引 8
    (80, 96, 240, 255),   // 索引 9
    (40, 240, 63, 255),   // 索引 10
    (127, 191, 255, 255), // 索引 11
    (248, 224, 80, 255),  // 索引 12
    (240, 224, 88, 255),  // 索引 13
    (255, 247, 144, 255), // 索引 14
    (255, 255, 255, 255), // 索引 15
    (0, 40, 167, 255),    // 索引 16 - 修复：将黑色改为与索引1相同的亮蓝色，解决WINDOW精灵不可见问题
    (23, 23, 23, 255),    // 索引 17
    (32, 32, 32, 255),    // 索引 18
    (47, 47, 47, 255),    // 索引 19
    (56, 56, 56, 255),    // 索引 20
    (71, 71, 71, 255),    // 索引 21
    (80, 80, 80, 255),    // 索引 22
    (96, 96, 96, 255),    // 索引 23
    (112, 112, 112, 255), // 索引 24
    (128, 128, 128, 255), // 索引 25
    (144, 144, 144, 255), // 索引 26
    (160, 160, 160, 255), // 索引 27
    (183, 183, 183, 255), // 索引 28
    (200, 200, 200, 255), // 索引 29
    (224, 224, 224, 255), // 索引 30
    (255, 255, 255, 255), // 索引 31
    (55, 48, 63, 255),    // 索引 32
    (63, 64, 88, 255),    // 索引 33
    (71, 80, 119, 255),   // 索引 34
    (79, 96, 144, 255),   // 索引 35
    (87, 112, 175, 255),  // 索引 36
    (95, 127, 200, 255),  // 索引 37
    (103, 136, 231, 255), // 索引 38
    (104, 168, 255, 255), // 索引 39
    (0, 63, 8, 255),      // 索引 40
    (7, 88, 16, 255),     // 索引 41
    (8, 119, 24, 255),    // 索引 42
    (15, 144, 32, 255),   // 索引 43
    (16, 175, 40, 255),   // 索引 44
    (23, 200, 48, 255),   // 索引 45
    (24, 231, 56, 255),   // 索引 46
    (80, 255, 160, 255),  // 索引 47
    (72, 24, 31, 255),    // 索引 48
    (103, 48, 47, 255),   // 索引 49
    (135, 71, 56, 255),   // 索引 50
    (160, 95, 72, 255),   // 索引 51
    (192, 112, 87, 255),  // 索引 52
    (223, 136, 103, 255), // 索引 53
    (248, 159, 111, 255), // 索引 54
    (255, 192, 144, 255), // 索引 55
    (63, 15, 8, 255),     // 索引 56
    (88, 24, 16, 255),    // 索引 57
    (119, 39, 24, 255),   // 索引 58
    (144, 48, 32, 255),   // 索引 59
    (175, 63, 40, 255),   // 索引 60
    (200, 72, 48, 255),   // 索引 61
    (231, 87, 56, 255),   // 索引 62
    (255, 112, 80, 255),  // 索引 63
    (63, 0, 0, 255),      // 索引 64
    (88, 31, 0, 255),     // 索引 65
    (119, 63, 0, 255),    // 索引 66
    (144, 95, 0, 255),    // 索引 67
    (175, 127, 0, 255),   // 索引 68
    (200, 159, 0, 255),   // 索引 69
    (231, 191, 0, 255),   // 索引 70
    (255, 223, 0, 255),   // 索引 71
    (63, 23, 15, 255),    // 索引 72
    (88, 47, 31, 255),    // 索引 73
    (119, 71, 47, 255),   // 索引 74
    (144, 95, 63, 255),   // 索引 75
    (175, 119, 79, 255),  // 索引 76
    (200, 143, 95, 255),  // 索引 77
    (231, 167, 111, 255), // 索引 78
    (255, 215, 136, 255), // 索引 79
    (112, 56, 48, 255),   // 索引 80
    (135, 80, 56, 255),   // 索引 81
    (152, 104, 64, 255),  // 索引 82
    (175, 128, 72, 255),  // 索引 83
    (192, 152, 80, 255),  // 索引 84
    (215, 176, 88, 255),  // 索引 85
    (232, 200, 96, 255),  // 索引 86
    (255, 224, 120, 255), // 索引 87
    (23, 23, 24, 255),    // 索引 88
    (40, 40, 55, 255),    // 索引 89
    (63, 63, 80, 255),    // 索引 90
    (80, 80, 111, 255),   // 索引 91
    (103, 103, 136, 255), // 索引 92
    (120, 120, 167, 255), // 索引 93
    (143, 143, 192, 255), // 索引 94
    (176, 176, 255, 255), // 索引 95
    (15, 24, 23, 255),    // 索引 96
    (23, 47, 39, 255),    // 索引 97
    (31, 64, 55, 255),    // 索引 98
    (39, 87, 71, 255),    // 索引 99
    (47, 104, 87, 255),   // 索引 100
    (55, 127, 103, 255),  // 索引 101
    (63, 144, 119, 255),  // 索引 102
    (80, 192, 152, 255),  // 索引 103
    (24, 24, 31, 255),    // 索引 104
    (55, 55, 63, 255),    // 索引 105
    (80, 80, 95, 255),    // 索引 106
    (111, 111, 127, 255), // 索引 107
    (136, 136, 159, 255), // 索引 108
    (167, 167, 191, 255), // 索引 109
    (192, 192, 223, 255), // 索引 110
    (231, 231, 255, 255), // 索引 111
    (24, 63, 16, 255),    // 索引 112
    (48, 88, 32, 255),    // 索引 113
    (72, 119, 48, 255),   // 索引 114
    (96, 144, 64, 255),   // 索引 115
    (120, 175, 80, 255),  // 索引 116
    (144, 200, 96, 255),  // 索引 117
    (168, 231, 112, 255), // 索引 118
    (216, 255, 143, 255), // 索引 119
    (63, 40, 40, 255),    // 索引 120
    (88, 64, 64, 255),    // 索引 121
    (119, 87, 87, 255),   // 索引 122
    (144, 111, 111, 255), // 索引 123
    (175, 128, 128, 255), // 索引 124
    (200, 152, 152, 255), // 索引 125
    (231, 175, 175, 255), // 索引 126
    (255, 216, 216, 255), // 索引 127
    (63, 63, 24, 255),    // 索引 128
    (88, 88, 48, 255),    // 索引 129
    (119, 119, 72, 255),  // 索引 130
    (144, 144, 96, 255),  // 索引 131
    (175, 175, 120, 255), // 索引 132
    (200, 200, 144, 255), // 索引 133
    (231, 231, 168, 255), // 索引 134
    (255, 255, 216, 255), // 索引 135
    (8, 16, 56, 255),     // 索引 136
    (24, 48, 87, 255),    // 索引 137
    (40, 80, 112, 255),   // 索引 138
    (56, 112, 143, 255),  // 索引 139
    (72, 144, 168, 255),  // 索引 140
    (88, 176, 199, 255),  // 索引 141
    (104, 208, 224, 255), // 索引 142
    (144, 255, 255, 255), // 索引 143
    (72, 16, 56, 255),    // 索引 144
    (96, 32, 87, 255),    // 索引 145
    (127, 48, 112, 255),  // 索引 146
    (151, 64, 143, 255),  // 索引 147
    (176, 80, 168, 255),  // 索引 148
    (200, 96, 199, 255),  // 索引 149
    (231, 112, 224, 255), // 索引 150
    (255, 152, 255, 255), // 索引 151
    (0, 0, 0, 255),       // 索引 152
    (0, 0, 0, 255),       // 索引 153
    (0, 0, 0, 255),       // 索引 154
    (0, 0, 0, 255),       // 索引 155
    (0, 0, 0, 255),       // 索引 156
    (0, 0, 0, 255),       // 索引 157
    (0, 0, 0, 255),       // 索引 158
    (215, 176, 88, 255),  // 索引 159
];

// ============================================================================
// P1-1 修复：统一的 Sprite 类型系统（替代 SpriteData + 多种 ImageBuffer）
// ============================================================================

/// 统一的精灵类型（const-generic，编译期确定尺寸）
///
/// 这个设计模仿 Pascal 原版的 `ImageBuffer = array [1..H, 1..W] of Char`，
/// 但利用 Rust 的 const-generic 支持多种尺寸。
///
/// 优势：
/// - 无 Box::leak（无内存泄漏）
/// - 无运行时尺寸检查（编译期保证）
/// - 类型安全（不同尺寸的 sprite 是不同类型）
#[derive(Debug, Clone, Copy)]
pub struct Sprite<const W: usize, const H: usize> {
    pub pixels: [[u8; W]; H],
}

impl<const W: usize, const H: usize> Sprite<W, H> {
    pub const fn new() -> Self {
        Self {
            pixels: [[0; W]; H],
        }
    }

    /// 从加载器创建 sprite
    pub fn from_loader(loader: &SpriteLoader, name: &str) -> Self {
        let pixels = loader.buf::<W, H>(name);
        Self { pixels }
    }

    /// 获取宽度
    pub const fn width(&self) -> usize {
        W
    }

    /// 获取高度
    pub const fn height(&self) -> usize {
        H
    }

    /// 获取像素数据的二维数组引用（安全）
    pub fn pixels(&self) -> &[[u8; W]; H] {
        &self.pixels
    }

    /// 转换为一维切片（用于旧版 VGA API）
    /// 返回新分配的Vec，避免unsafe
    pub fn to_flat_vec(&self) -> Vec<u8> {
        self.pixels.iter().flatten().copied().collect()
    }
}

// 语义化类型别名（对应 Pascal 中不同用途的 sprite）
pub type SpriteRegular = Sprite<20, 14>; // 常规地形/道具（对应 Pascal ImageBuffer）
pub type SpriteTall = Sprite<20, 28>; // 玩家（Mario/Luigi，对应 Pascal PicBuffer）
pub type SpriteSmall = Sprite<12, 7>; // 火球/粒子
pub type SpriteEnemy = Sprite<20, 24>; // 敌人（Koopa）
pub type SpritePlant = Sprite<20, 14>; // PALPILL 柱子装饰（修复：实际尺寸是 20x14，不是 24x20）
pub type SpriteLarge = Sprite<108, 28>; // Intro 大图

// ============================================================================
// Pascal 精灵数据加载器
// ============================================================================

// include_dir removed: assets are provided by generated_assets.rs at build time.
// Fallback runtime embedding via include_dir has been removed because all
// sprite files are converted to binary statics during the build.

/// 精灵加载器：统一封装 include_dir + Pascal db 解析 + Mode X 去平面化 + 长度校验。
///
/// 说明：
/// - Sprites 目录既有二进制 `.000`，也有 Pascal include 文本 `.$00`（db 列表）。
/// - 游戏资源在 Pascal 原版里是编译期 include 进源码；Rust 这里用 include_dir 在编译期嵌入。
/// - 像素数据采用 VGA Mode X 的 4-plane 平面布局，需要去平面化成线性 row-major 才能按 (x,y) 访问。
#[derive(Clone, Copy)]
struct SpriteLoader;

impl SpriteLoader {
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    #[inline]
    fn parse_field_name(field_name: &str) -> (&str, u16) {
        let (base, frame) = field_name
            .rsplit_once('_')
            .unwrap_or_else(|| panic!("bad sprite field name: {field_name}"));
        let frame_u16: u16 = frame
            .parse()
            .unwrap_or_else(|_| panic!("bad sprite frame number: {field_name}"));
        (base, frame_u16)
    }

    fn parse_pascal_asm_db(src: &str) -> Vec<u8> {
        fn parse_byte(tok: &str) -> Option<u8> {
            let t = tok.trim().trim_end_matches(',');
            if t.is_empty() {
                return None;
            }
            if let Some(hex) = t.strip_prefix('$') {
                u8::from_str_radix(hex, 16).ok()
            } else {
                t.parse::<u8>().ok()
            }
        }

        let mut out: Vec<u8> = Vec::new();
        for line in src.lines() {
            let l = line.trim_start();
            if !l.starts_with("db") && !l.starts_with("DB") {
                continue;
            }
            for raw in l[2..].split(|c: char| c == ' ' || c == '\t' || c == ',') {
                if let Some(b) = parse_byte(raw) {
                    out.push(b);
                }
            }
        }
        out
    }

    fn load_raw_sprite_bytes(&self, base: &str, frame: u16) -> Vec<u8> {
        // 优先使用 .$00 这类 Pascal include 文件
        let ext_inc = format!("${:02}", frame);
        let ext_bin = format!("{:03}", frame);
        let inc_name = format!("{base}.{ext_inc}");
        let bin_name = format!("{base}.{ext_bin}");
        // Try generated assets (produced by build.rs) first
        if let Some(bytes) = get_generated_asset(&inc_name) {
            return bytes.to_vec();
        }
        if let Some(bytes) = get_generated_asset(&bin_name) {
            return bytes.to_vec();
        }

        // No runtime fallback: generated assets must contain the files.
        panic!(
            "sprite file not found (must be generated): {} or {}",
            inc_name,
            bin_name
        );
    }

    #[inline]
    fn verify_len(&self, label: &str, got: usize, expected: usize) {
        assert!(
            got == expected,
            "ModeX sprite length mismatch: {} got={} expected={}",
            label,
            got,
            expected
        );
    }

    fn modex_deplane(&self, label: &str, bytes: &[u8], w: usize, h: usize) -> Vec<u8> {
        // Pascal VGA256 Mode X 平面格式：
        // **平面优先**（所有plane0数据，然后plane1，然后plane2，然后plane3）
        //
        // 格式：[plane0: all_rows] [plane1: all_rows] [plane2: all_rows] [plane3: all_rows]
        // 每个平面有 (w/4) * h 字节
        
        assert!(
            w % 4 == 0,
            "ModeX requires width divisible by 4: {} w={}",
            label,
            w
        );
        self.verify_len(label, bytes.len(), w * h);

        let bytes_per_line = w / 4;
        let plane_size = bytes_per_line * h;
        let mut out = vec![0u8; w * h];
        
        for y in 0..h {
            for x in 0..w {
                let plane = x & 3;
                let bx = x >> 2;
                let src_idx = plane * plane_size + y * bytes_per_line + bx;
                out[y * w + x] = bytes[src_idx];
            }
        }
        
        out
    }

    fn pixels<const WW: usize, const HH: usize>(&self, field_name: &str) -> Vec<u8> {
        let (base, frame) = Self::parse_field_name(field_name);
        let bytes = self.load_raw_sprite_bytes(base, frame);
        
        // 所有精灵使用标准的 ModeX 平面优先格式（包括 WOOD）
        // Pascal 的 DrawImage 实现确认了这一点
        self.modex_deplane(field_name, &bytes, WW, HH)
    }
    
    fn buf<const WW: usize, const HH: usize>(&self, field_name: &str) -> [[u8; WW]; HH] {
        let pixels = self.pixels::<WW, HH>(field_name);
        let mut out = [[0u8; WW]; HH];
        for y in 0..HH {
            for x in 0..WW {
                out[y][x] = pixels[y * WW + x];
            }
        }
        out
    }

    /// 加载为 const-generic Sprite（替代 leak_sprite_data，消除 Box::leak）
    fn load_sprite<const W: usize, const H: usize>(&self, base: &str, frame: u16) -> Sprite<W, H> {
        let field_name = format!("{}_{:03}", base, frame);
        let pixels = self.buf::<W, H>(&field_name);
        Sprite { pixels }
    }
}

// ============================================================================
// 注意：已消除 Box::leak、SpriteData 和 once_cell 依赖。
// 当前工程的精灵渲染统一使用：
// - SpriteDataManager：游戏运行期绘制（const-generic 固定尺寸）
// - Sprite<W, H> 静态常量：Intro 大图和背景纹理
// ============================================================================

pub struct SpriteDataManager {
    pub BROWN_000: ImageBuffer,
    pub BROWN_001: ImageBuffer,
    pub BROWN_002: ImageBuffer,
    pub BROWN_003: ImageBuffer,
    pub BROWN_004: ImageBuffer,

    pub SWMAR_000: crate::buffers::PicBuffer,
    pub SWMAR_001: crate::buffers::PicBuffer,

    pub SJMAR_000: crate::buffers::PicBuffer,
    pub SJMAR_001: crate::buffers::PicBuffer,

    pub LWMAR_000: crate::buffers::PicBuffer,
    pub LWMAR_001: crate::buffers::PicBuffer,
    pub LJMAR_000: crate::buffers::PicBuffer,
    pub LJMAR_001: crate::buffers::PicBuffer,
    pub FWMAR_000: crate::buffers::PicBuffer,
    pub FWMAR_001: crate::buffers::PicBuffer,
    pub FJMAR_000: crate::buffers::PicBuffer,
    pub FJMAR_001: crate::buffers::PicBuffer,

    pub LWLUI_000: crate::buffers::PicBuffer,
    pub LWLUI_001: crate::buffers::PicBuffer,
    pub LJLUI_000: crate::buffers::PicBuffer,
    pub LJLUI_001: crate::buffers::PicBuffer,
    pub SWLUI_000: crate::buffers::PicBuffer,
    pub SWLUI_001: crate::buffers::PicBuffer,
    pub SJLUI_000: crate::buffers::PicBuffer,
    pub SJLUI_001: crate::buffers::PicBuffer,
    pub FWLUI_000: crate::buffers::PicBuffer,
    pub FWLUI_001: crate::buffers::PicBuffer,
    pub FJLUI_000: crate::buffers::PicBuffer,
    pub FJLUI_001: crate::buffers::PicBuffer,

    pub PIPE_000: ImageBuffer,
    pub PIPE_001: ImageBuffer,
    pub PIPE_002: ImageBuffer,
    pub PIPE_003: ImageBuffer,

    pub GREEN_000: ImageBuffer,
    pub GREEN_001: ImageBuffer,
    pub GREEN_002: ImageBuffer,
    pub GREEN_003: ImageBuffer,
    pub GREEN_004: ImageBuffer,

    pub SAND_000: ImageBuffer,
    pub SAND_001: ImageBuffer,
    pub SAND_002: ImageBuffer,
    pub SAND_003: ImageBuffer,
    pub SAND_004: ImageBuffer,

    pub GRASS_000: ImageBuffer,
    pub GRASS_001: ImageBuffer,
    pub GRASS_002: ImageBuffer,
    pub GRASS_003: ImageBuffer,
    pub GRASS_004: ImageBuffer,

    pub DES_000: ImageBuffer,
    pub DES_001: ImageBuffer,
    pub DES_002: ImageBuffer,
    pub DES_003: ImageBuffer,
    pub DES_004: ImageBuffer,

    pub BRICK0_000: ImageBuffer,
    pub BRICK0_001: ImageBuffer,
    pub BRICK0_002: ImageBuffer,

    pub BRICK1_000: ImageBuffer,
    pub BRICK1_001: ImageBuffer,
    pub BRICK1_002: ImageBuffer,

    pub BRICK2_000: ImageBuffer,
    pub BRICK2_001: ImageBuffer,
    pub BRICK2_002: ImageBuffer,

    pub GRASS1_000: ImageBuffer,
    pub GRASS1_001: ImageBuffer,
    pub GRASS1_002: ImageBuffer,

    pub GRASS2_000: ImageBuffer,
    pub GRASS2_001: ImageBuffer,
    pub GRASS2_002: ImageBuffer,

    pub GRASS3_000: ImageBuffer,
    pub GRASS3_001: ImageBuffer,
    pub GRASS3_002: ImageBuffer,

    pub PALM0_000: ImageBuffer,
    pub PALM0_001: ImageBuffer,
    pub PALM0_002: ImageBuffer,

    pub PALM1_000: ImageBuffer,
    pub PALM1_001: ImageBuffer,
    pub PALM1_002: ImageBuffer,

    pub PALM2_000: ImageBuffer,
    pub PALM2_001: ImageBuffer,
    pub PALM2_002: ImageBuffer,

    pub PALM3_000: ImageBuffer,
    pub PALM3_001: ImageBuffer,
    pub PALM3_002: ImageBuffer,

    pub WOOD_000: ImageBuffer,
    pub WOOD_000_ORIG: ImageBuffer, // 原始数据副本，用于每次recolor前恢复
    pub XBLOCK_000: ImageBuffer,
    pub XBLOCK_000_ORIG: ImageBuffer, // 原始数据副本

    pub BLOCK_000: ImageBuffer,
    pub BLOCK_001: ImageBuffer,
    pub BLOCK_001_ORIG: ImageBuffer, // 原始数据副本

    pub COIN_000: ImageBuffer,

    pub EXIT_000: ImageBuffer,
    pub EXIT_001: ImageBuffer,

    pub WPALM_000: ImageBuffer,

    pub FENCE_000: ImageBuffer,
    pub FENCE_001: ImageBuffer,

    pub SMTREE_000: ImageBuffer,
    pub SMTREE_001: ImageBuffer,

    pub TREE_000: ImageBuffer,
    pub TREE_001: ImageBuffer,
    pub TREE_002: ImageBuffer,
    pub TREE_003: ImageBuffer,

    pub WINDOW_001: ImageBuffer,

    pub LAVA_000: ImageBuffer,
    pub LAVA_001: ImageBuffer,
    pub LAVA2_001: ImageBuffer,
    pub LAVA2_002: ImageBuffer,
    pub LAVA2_003: ImageBuffer,
    pub LAVA2_004: ImageBuffer,
    pub LAVA2_005: ImageBuffer,

    pub FALL_000: ImageBuffer,
    pub FALL_001: ImageBuffer,

    pub WINDOW_000: ImageBuffer,
    pub NOTE_000: ImageBuffer,
    pub PIN_000: ImageBuffer,
    pub QUEST_000: ImageBuffer,
    pub QUEST_001: ImageBuffer,
    pub PALBRICK_000: ImageBuffer,
    pub CHAMP_000: ImageBuffer,
    pub POISON_000: ImageBuffer,
    pub LIFE_000: ImageBuffer,
    pub FLOWER_000: ImageBuffer,
    pub STAR_000: ImageBuffer,
    pub FIRE_000: ImageBuffer12x7,
    pub FIRE_001: ImageBuffer12x7,
    pub F_000: ImageBuffer,
    pub F_001: ImageBuffer,
    pub F_002: ImageBuffer,
    pub F_003: ImageBuffer,
    pub GRKOOPA_000: ImageBuffer20x24,
    pub GRKOOPA_001: ImageBuffer20x24,
    pub RDKOOPA_000: ImageBuffer20x24,
    pub RDKOOPA_001: ImageBuffer20x24,
    pub GRKP_000: ImageBuffer,
    pub GRKP_001: ImageBuffer,
    pub RDKP_000: ImageBuffer,
    pub RDKP_001: ImageBuffer,
    pub LIFT1_000: ImageBuffer,
    pub DONUT_000: ImageBuffer,
    pub DONUT_001: ImageBuffer,
    pub PPLANT_000: ImageBuffer24x20,
    pub PPLANT_001: ImageBuffer24x20,
    pub PPLANT_002: ImageBuffer24x20,
    pub PPLANT_003: ImageBuffer24x20,
    pub HIT_000: ImageBuffer24x20,
    pub PART_000: ImageBuffer12x7,
    pub WHHIT_000: ImageBuffer,
    pub WHFIRE_000: ImageBuffer,
    pub CHIBIBO_000: ImageBuffer,
    pub CHIBIBO_001: ImageBuffer,
    pub CHIBIBO_002: ImageBuffer,
    pub CHIBIBO_003: ImageBuffer,
    pub FISH_001: ImageBuffer,
    pub RED_000: ImageBuffer,
    pub RED_001: ImageBuffer,

    // Intro 大图精灵
    pub INTRO_000: Sprite<108, 28>,
    pub INTRO_001: Sprite<24, 28>,
    pub INTRO_002: Sprite<84, 28>,

    // 背景柱子纹理
    pub PALPILL_000: SpritePlant,
    pub PALPILL_001: SpritePlant,
    pub PALPILL_002: SpritePlant,
}

impl SpriteDataManager {
    pub fn new() -> Self {
        let loader = SpriteLoader::new();
        let b20x14 = |name: &str| loader.buf::<20, 14>(name);
        let b20x28 = |name: &str| loader.buf::<20, 28>(name);
        let b20x24 = |name: &str| loader.buf::<20, 24>(name);
        let b24x20 = |name: &str| loader.buf::<24, 20>(name);
        let b12x7 = |name: &str| loader.buf::<12, 7>(name);

        Self {
            BROWN_000: b20x14("BROWN_000"),
            BROWN_001: b20x14("BROWN_001"),
            BROWN_002: b20x14("BROWN_002"),
            BROWN_003: b20x14("BROWN_003"),
            BROWN_004: b20x14("BROWN_004"),

            SWMAR_000: b20x28("SWMAR_000"),
            SWMAR_001: b20x28("SWMAR_001"),
            SJMAR_000: b20x28("SJMAR_000"),
            SJMAR_001: b20x28("SJMAR_001"),
            LWMAR_000: b20x28("LWMAR_000"),
            LWMAR_001: b20x28("LWMAR_001"),
            LJMAR_000: b20x28("LJMAR_000"),
            LJMAR_001: b20x28("LJMAR_001"),
            FWMAR_000: b20x28("FWMAR_000"),
            FWMAR_001: b20x28("FWMAR_001"),
            FJMAR_000: b20x28("FJMAR_000"),
            FJMAR_001: b20x28("FJMAR_001"),

            LWLUI_000: b20x28("LWLUI_000"),
            LWLUI_001: b20x28("LWLUI_001"),
            LJLUI_000: b20x28("LJLUI_000"),
            LJLUI_001: b20x28("LJLUI_001"),
            SWLUI_000: b20x28("SWLUI_000"),
            SWLUI_001: b20x28("SWLUI_001"),
            SJLUI_000: b20x28("SJLUI_000"),
            SJLUI_001: b20x28("SJLUI_001"),
            FWLUI_000: b20x28("FWLUI_000"),
            FWLUI_001: b20x28("FWLUI_001"),
            FJLUI_000: b20x28("FJLUI_000"),
            FJLUI_001: b20x28("FJLUI_001"),

            PIPE_000: b20x14("PIPE_000"),
            PIPE_001: b20x14("PIPE_001"),
            PIPE_002: b20x14("PIPE_002"),
            PIPE_003: b20x14("PIPE_003"),

            GREEN_000: b20x14("GREEN_000"),
            GREEN_001: b20x14("GREEN_001"),
            GREEN_002: b20x14("GREEN_002"),
            GREEN_003: b20x14("GREEN_003"),
            GREEN_004: b20x14("GREEN_004"),

            SAND_000: b20x14("SAND_000"),
            SAND_001: b20x14("SAND_001"),
            SAND_002: b20x14("SAND_002"),
            SAND_003: b20x14("SAND_003"),
            SAND_004: b20x14("SAND_004"),

            GRASS_000: b20x14("GRASS_000"),
            GRASS_001: b20x14("GRASS_001"),
            GRASS_002: b20x14("GRASS_002"),
            GRASS_003: b20x14("GRASS_003"),
            GRASS_004: b20x14("GRASS_004"),

            DES_000: b20x14("DES_000"),
            DES_001: b20x14("DES_001"),
            DES_002: b20x14("DES_002"),
            DES_003: b20x14("DES_003"),
            DES_004: b20x14("DES_004"),

            BRICK0_000: b20x14("BRICK0_000"),
            BRICK0_001: b20x14("BRICK0_001"),
            BRICK0_002: b20x14("BRICK0_002"),
            BRICK1_000: b20x14("BRICK1_000"),
            BRICK1_001: b20x14("BRICK1_001"),
            BRICK1_002: b20x14("BRICK1_002"),
            BRICK2_000: b20x14("BRICK2_000"),
            BRICK2_001: b20x14("BRICK2_001"),
            BRICK2_002: b20x14("BRICK2_002"),

            GRASS1_000: b20x14("GRASS1_000"),
            GRASS1_001: b20x14("GRASS1_001"),
            GRASS1_002: b20x14("GRASS1_002"),
            GRASS2_000: b20x14("GRASS2_000"),
            GRASS2_001: b20x14("GRASS2_001"),
            GRASS2_002: b20x14("GRASS2_002"),
            GRASS3_000: b20x14("GRASS3_000"),
            GRASS3_001: b20x14("GRASS3_001"),
            GRASS3_002: b20x14("GRASS3_002"),

            PALM0_000: b20x14("PALM0_000"),
            PALM0_001: b20x14("PALM0_001"),
            PALM0_002: b20x14("PALM0_002"),
            PALM1_000: b20x14("PALM1_000"),
            PALM1_001: b20x14("PALM1_001"),
            PALM1_002: b20x14("PALM1_002"),
            PALM2_000: b20x14("PALM2_000"),
            PALM2_001: b20x14("PALM2_001"),
            PALM2_002: b20x14("PALM2_002"),
            PALM3_000: b20x14("PALM3_000"),
            PALM3_001: b20x14("PALM3_001"),
            PALM3_002: b20x14("PALM3_002"),

            WOOD_000: b20x14("WOOD_000"),
            WOOD_000_ORIG: b20x14("WOOD_000"), // 原始数据副本
            XBLOCK_000: b20x14("XBLOCK_000"),
            XBLOCK_000_ORIG: b20x14("XBLOCK_000"), // 原始数据副本
            BLOCK_000: b20x14("BLOCK_000"),
            BLOCK_001: b20x14("BLOCK_001"),
            BLOCK_001_ORIG: b20x14("BLOCK_001"), // 原始数据副本
            COIN_000: b20x14("COIN_000"),
            EXIT_000: b20x14("EXIT_000"),
            EXIT_001: b20x14("EXIT_001"),
            WPALM_000: b20x14("WPALM_000"),
            FENCE_000: b20x14("FENCE_000"),
            FENCE_001: b20x14("FENCE_001"),
            SMTREE_000: b20x14("SMTREE_000"),
            SMTREE_001: b20x14("SMTREE_001"),
            TREE_000: b20x14("TREE_000"),
            TREE_001: b20x14("TREE_001"),
            TREE_002: b20x14("TREE_002"),
            TREE_003: b20x14("TREE_003"),
            WINDOW_001: b20x14("WINDOW_001"),
            LAVA_000: b20x14("LAVA_000"),
            LAVA_001: b20x14("LAVA_001"),
            LAVA2_001: b20x14("LAVA2_001"),
            LAVA2_002: b20x14("LAVA2_002"),
            LAVA2_003: b20x14("LAVA2_003"),
            LAVA2_004: b20x14("LAVA2_004"),
            LAVA2_005: b20x14("LAVA2_005"),
            FALL_000: b20x14("FALL_000"),
            FALL_001: b20x14("FALL_001"),
            WINDOW_000: b20x14("WINDOW_000"),
            NOTE_000: b20x14("NOTE_000"),
            PIN_000: b20x14("PIN_000"),
            QUEST_000: b20x14("QUEST_000"),
            QUEST_001: b20x14("QUEST_001"),
            PALBRICK_000: b20x14("PALBRICK_000"),
            CHAMP_000: b20x14("CHAMP_000"),
            POISON_000: b20x14("POISON_000"),
            LIFE_000: b20x14("LIFE_000"),
            FLOWER_000: b20x14("FLOWER_000"),
            STAR_000: b20x14("STAR_000"),
            FIRE_000: b12x7("FIRE_000"),
            FIRE_001: b12x7("FIRE_001"),
            F_000: b20x14("F_000"),
            F_001: b20x14("F_001"),
            F_002: b20x14("F_002"),
            F_003: b20x14("F_003"),

            GRKOOPA_000: b20x24("GRKOOPA_000"),
            GRKOOPA_001: b20x24("GRKOOPA_001"),
            RDKOOPA_000: b20x24("RDKOOPA_000"),
            RDKOOPA_001: b20x24("RDKOOPA_001"),
            GRKP_000: b20x14("GRKP_000"),
            GRKP_001: b20x14("GRKP_001"),
            RDKP_000: b20x14("RDKP_000"),
            RDKP_001: b20x14("RDKP_001"),

            LIFT1_000: b20x14("LIFT1_000"),
            DONUT_000: b20x14("DONUT_000"),
            DONUT_001: b20x14("DONUT_001"),

            PPLANT_000: b24x20("PPLANT_000"),
            PPLANT_001: b24x20("PPLANT_001"),
            PPLANT_002: b24x20("PPLANT_002"),
            PPLANT_003: b24x20("PPLANT_003"),
            HIT_000: b24x20("HIT_000"),
            PART_000: b12x7("PART_000"),
            WHHIT_000: b20x14("WHHIT_000"),
            WHFIRE_000: b20x14("WHFIRE_000"),

            CHIBIBO_000: b20x14("CHIBIBO_000"),
            CHIBIBO_001: b20x14("CHIBIBO_001"),
            CHIBIBO_002: b20x14("CHIBIBO_002"),
            CHIBIBO_003: b20x14("CHIBIBO_003"),
            FISH_001: b20x14("FISH_001"),
            RED_000: b20x14("RED_000"),
            RED_001: b20x14("RED_001"),

            // Intro 大图精灵
            INTRO_000: loader.load_sprite("INTRO", 0),
            INTRO_001: loader.load_sprite("INTRO", 1),
            INTRO_002: loader.load_sprite("INTRO", 2),

            // 背景柱子纹理
            PALPILL_000: loader.load_sprite("PALPILL", 0),
            PALPILL_001: loader.load_sprite("PALPILL", 1),
            PALPILL_002: loader.load_sprite("PALPILL", 2),
        }
    }
}

// P1-1 修复完成：已删除 get_sprite() 函数和废弃的 SpriteData
// 现在统一使用 Sprite<W, H> 和 const-generic 类型

// ============================================================================
// GPU 纹理图集构建
// ============================================================================

use crate::gpu::texture_atlas::{TextureAtlas, SpriteUV};

// 精灵ID枚举 - 用于快速查找图集中的精灵
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum SpriteId {
    // 地形精灵
    BROWN_000, BROWN_001, BROWN_002, BROWN_003, BROWN_004,
    PIPE_000, PIPE_001, PIPE_002, PIPE_003,
    GREEN_000, GREEN_001, GREEN_002, GREEN_003, GREEN_004,
    SAND_000, SAND_001, SAND_002, SAND_003, SAND_004,
    GRASS_000, GRASS_001, GRASS_002, GRASS_003, GRASS_004,
    DES_000, DES_001, DES_002, DES_003, DES_004,
    BRICK0_000, BRICK0_001, BRICK0_002,
    BRICK1_000, BRICK1_001, BRICK1_002,
    BRICK2_000, BRICK2_001, BRICK2_002,
    GRASS1_000, GRASS1_001, GRASS1_002,
    GRASS2_000, GRASS2_001, GRASS2_002,
    GRASS3_000, GRASS3_001, GRASS3_002,
    PALM0_000, PALM0_001, PALM0_002,
    PALM1_000, PALM1_001, PALM1_002,
    PALM2_000, PALM2_001, PALM2_002,
    PALM3_000, PALM3_001, PALM3_002,
    
    // 方块精灵
    WOOD_000, XBLOCK_000,
    BLOCK_000, BLOCK_001,
    COIN_000,
    EXIT_000, EXIT_001,
    WPALM_000,
    FENCE_000, FENCE_001,
    SMTREE_000, SMTREE_001,
    TREE_000, TREE_001, TREE_002, TREE_003,
    WINDOW_000, WINDOW_001,
    LAVA_000, LAVA_001,
    LAVA2_001, LAVA2_002, LAVA2_003, LAVA2_004, LAVA2_005,
    FALL_000, FALL_001,
    NOTE_000, PIN_000,
    QUEST_000, QUEST_001,
    PALBRICK_000,
    
    // 道具精灵
    CHAMP_000, POISON_000, LIFE_000, FLOWER_000, STAR_000,
    
    // 火球/粒子
    FIRE_000, FIRE_001, PART_000,
    F_000, F_001, F_002, F_003,
    
    // 敌人精灵
    GRKOOPA_000, GRKOOPA_001,
    RDKOOPA_000, RDKOOPA_001,
    GRKP_000, GRKP_001,
    RDKP_000, RDKP_001,
    CHIBIBO_000, CHIBIBO_001, CHIBIBO_002, CHIBIBO_003,
    FISH_001,
    PPLANT_000, PPLANT_001, PPLANT_002, PPLANT_003,
    
    // 平台/其他
    LIFT1_000,
    DONUT_000, DONUT_001,
    HIT_000,
    WHHIT_000, WHFIRE_000,
    RED_000, RED_001,
    
    // 玩家精灵 (Mario)
    SWMAR_000, SWMAR_001,
    SJMAR_000, SJMAR_001,
    LWMAR_000, LWMAR_001,
    LJMAR_000, LJMAR_001,
    FWMAR_000, FWMAR_001,
    FJMAR_000, FJMAR_001,
    
    // 玩家精灵 (Luigi)
    LWLUI_000, LWLUI_001,
    LJLUI_000, LJLUI_001,
    SWLUI_000, SWLUI_001,
    SJLUI_000, SJLUI_001,
    FWLUI_000, FWLUI_001,
    FJLUI_000, FJLUI_001,
    
    // 背景精灵
    PALPILL_000, PALPILL_001, PALPILL_002,
    
    // Intro 大图
    INTRO_000, INTRO_001, INTRO_002,
    
    // 总数标记
    COUNT,
}

// 精灵UV查找表
pub struct SpriteAtlas {
    pub atlas: TextureAtlas,
    uvs: Vec<SpriteUV>,
}

impl SpriteAtlas {
    // 获取精灵UV
    pub fn get(&self, id: SpriteId) -> SpriteUV {
        self.uvs[id as usize]
    }
    
    // 获取图集数据
    pub fn data(&self) -> &[u8] {
        self.atlas.data()
    }
    
    // 获取图集尺寸
    pub fn size(&self) -> u32 {
        self.atlas.size
    }
    
    /// 获取敌人Chibibo精灵UV (frame: 0=正常, 1=扁平, sub_tp: 子类型)
    pub fn get_chibibo(&self, frame: usize, sub_tp: i32) -> SpriteUV {
        // Chibibo有两帧动画
        match (frame, sub_tp) {
            (0, _) => self.get(SpriteId::CHIBIBO_000),
            (1, _) => self.get(SpriteId::CHIBIBO_001),
            (2, _) => self.get(SpriteId::CHIBIBO_002), // flat
            _ => self.get(SpriteId::CHIBIBO_003),
        }
    }
    
    /// 获取Koopa精灵UV (color: 0=绿, 1=红; frame: 0/1动画帧)
    pub fn get_koopa(&self, color: usize, frame: usize) -> SpriteUV {
        match (color, frame % 2) {
            (0, 0) => self.get(SpriteId::GRKOOPA_000),
            (0, 1) => self.get(SpriteId::GRKOOPA_001),
            (1, 0) => self.get(SpriteId::RDKOOPA_000),
            (1, 1) => self.get(SpriteId::RDKOOPA_001),
            _ => self.get(SpriteId::GRKOOPA_000),
        }
    }
    
    /// 获取Koopa龟壳精灵UV
    pub fn get_koopa_shell(&self, color: usize, frame: usize) -> SpriteUV {
        match (color, frame % 2) {
            (0, 0) => self.get(SpriteId::GRKP_000),
            (0, 1) => self.get(SpriteId::GRKP_001),
            (1, 0) => self.get(SpriteId::RDKP_000),
            (1, 1) => self.get(SpriteId::RDKP_001),
            _ => self.get(SpriteId::GRKP_000),
        }
    }
    
    /// 获取食人花精灵UV
    pub fn get_plant(&self, frame: usize) -> SpriteUV {
        match frame % 4 {
            0 => self.get(SpriteId::PPLANT_000),
            1 => self.get(SpriteId::PPLANT_001),
            2 => self.get(SpriteId::PPLANT_002),
            _ => self.get(SpriteId::PPLANT_003),
        }
    }
    
    /// 获取火球精灵UV
    pub fn get_fireball(&self, frame: usize) -> SpriteUV {
        match frame % 2 {
            0 => self.get(SpriteId::FIRE_000),
            _ => self.get(SpriteId::FIRE_001),
        }
    }
    
    /// 获取玩家精灵UV
    /// player: 0=Mario, 1=Luigi
    /// state: 0=small walk, 1=small jump, 2=large walk, 3=large jump, 4=fire walk, 5=fire jump
    /// frame: 动画帧 0/1
    pub fn get_player(&self, player: usize, state: usize, frame: usize) -> SpriteUV {
        let f = frame % 2;
        match (player, state, f) {
            // Mario
            (0, 0, 0) => self.get(SpriteId::SWMAR_000),
            (0, 0, 1) => self.get(SpriteId::SWMAR_001),
            (0, 1, 0) => self.get(SpriteId::SJMAR_000),
            (0, 1, 1) => self.get(SpriteId::SJMAR_001),
            (0, 2, 0) => self.get(SpriteId::LWMAR_000),
            (0, 2, 1) => self.get(SpriteId::LWMAR_001),
            (0, 3, 0) => self.get(SpriteId::LJMAR_000),
            (0, 3, 1) => self.get(SpriteId::LJMAR_001),
            (0, 4, 0) => self.get(SpriteId::FWMAR_000),
            (0, 4, 1) => self.get(SpriteId::FWMAR_001),
            (0, 5, 0) => self.get(SpriteId::FJMAR_000),
            (0, 5, 1) => self.get(SpriteId::FJMAR_001),
            // Luigi
            (1, 0, 0) => self.get(SpriteId::SWLUI_000),
            (1, 0, 1) => self.get(SpriteId::SWLUI_001),
            (1, 1, 0) => self.get(SpriteId::SJLUI_000),
            (1, 1, 1) => self.get(SpriteId::SJLUI_001),
            (1, 2, 0) => self.get(SpriteId::LWLUI_000),
            (1, 2, 1) => self.get(SpriteId::LWLUI_001),
            (1, 3, 0) => self.get(SpriteId::LJLUI_000),
            (1, 3, 1) => self.get(SpriteId::LJLUI_001),
            (1, 4, 0) => self.get(SpriteId::FWLUI_000),
            (1, 4, 1) => self.get(SpriteId::FWLUI_001),
            (1, 5, 0) => self.get(SpriteId::FJLUI_000),
            (1, 5, 1) => self.get(SpriteId::FJLUI_001),
            // 默认Mario small walk
            _ => self.get(SpriteId::SWMAR_000),
        }
    }
}

impl SpriteDataManager {
    // 构建纹理图集
    pub fn build_atlas(&self) -> SpriteAtlas {
        use crate::gpu::ATLAS_SIZE;
        
        let mut atlas = TextureAtlas::new(ATLAS_SIZE);
        let mut uvs = Vec::with_capacity(SpriteId::COUNT as usize);
        
        // 辅助宏：添加精灵到图集
        macro_rules! add_sprite {
            ($field:ident, $w:expr, $h:expr) => {{
                let pixels: Vec<u8> = self.$field.iter().flatten().copied().collect();
                let uv = atlas.add_sprite(stringify!($field), $w, $h, &pixels)
                    .expect(concat!("Failed to add sprite: ", stringify!($field)));
                uvs.push(uv);
            }};
        }
        
        // 地形精灵 (20x14)
        add_sprite!(BROWN_000, 20, 14);
        add_sprite!(BROWN_001, 20, 14);
        add_sprite!(BROWN_002, 20, 14);
        add_sprite!(BROWN_003, 20, 14);
        add_sprite!(BROWN_004, 20, 14);
        add_sprite!(PIPE_000, 20, 14);
        add_sprite!(PIPE_001, 20, 14);
        add_sprite!(PIPE_002, 20, 14);
        add_sprite!(PIPE_003, 20, 14);
        add_sprite!(GREEN_000, 20, 14);
        add_sprite!(GREEN_001, 20, 14);
        add_sprite!(GREEN_002, 20, 14);
        add_sprite!(GREEN_003, 20, 14);
        add_sprite!(GREEN_004, 20, 14);
        add_sprite!(SAND_000, 20, 14);
        add_sprite!(SAND_001, 20, 14);
        add_sprite!(SAND_002, 20, 14);
        add_sprite!(SAND_003, 20, 14);
        add_sprite!(SAND_004, 20, 14);
        add_sprite!(GRASS_000, 20, 14);
        add_sprite!(GRASS_001, 20, 14);
        add_sprite!(GRASS_002, 20, 14);
        add_sprite!(GRASS_003, 20, 14);
        add_sprite!(GRASS_004, 20, 14);
        add_sprite!(DES_000, 20, 14);
        add_sprite!(DES_001, 20, 14);
        add_sprite!(DES_002, 20, 14);
        add_sprite!(DES_003, 20, 14);
        add_sprite!(DES_004, 20, 14);
        add_sprite!(BRICK0_000, 20, 14);
        add_sprite!(BRICK0_001, 20, 14);
        add_sprite!(BRICK0_002, 20, 14);
        add_sprite!(BRICK1_000, 20, 14);
        add_sprite!(BRICK1_001, 20, 14);
        add_sprite!(BRICK1_002, 20, 14);
        add_sprite!(BRICK2_000, 20, 14);
        add_sprite!(BRICK2_001, 20, 14);
        add_sprite!(BRICK2_002, 20, 14);
        add_sprite!(GRASS1_000, 20, 14);
        add_sprite!(GRASS1_001, 20, 14);
        add_sprite!(GRASS1_002, 20, 14);
        add_sprite!(GRASS2_000, 20, 14);
        add_sprite!(GRASS2_001, 20, 14);
        add_sprite!(GRASS2_002, 20, 14);
        add_sprite!(GRASS3_000, 20, 14);
        add_sprite!(GRASS3_001, 20, 14);
        add_sprite!(GRASS3_002, 20, 14);
        add_sprite!(PALM0_000, 20, 14);
        add_sprite!(PALM0_001, 20, 14);
        add_sprite!(PALM0_002, 20, 14);
        add_sprite!(PALM1_000, 20, 14);
        add_sprite!(PALM1_001, 20, 14);
        add_sprite!(PALM1_002, 20, 14);
        add_sprite!(PALM2_000, 20, 14);
        add_sprite!(PALM2_001, 20, 14);
        add_sprite!(PALM2_002, 20, 14);
        add_sprite!(PALM3_000, 20, 14);
        add_sprite!(PALM3_001, 20, 14);
        add_sprite!(PALM3_002, 20, 14);
        
        // 方块精灵 (20x14)
        add_sprite!(WOOD_000, 20, 14);
        add_sprite!(XBLOCK_000, 20, 14);
        add_sprite!(BLOCK_000, 20, 14);
        add_sprite!(BLOCK_001, 20, 14);
        add_sprite!(COIN_000, 20, 14);
        add_sprite!(EXIT_000, 20, 14);
        add_sprite!(EXIT_001, 20, 14);
        add_sprite!(WPALM_000, 20, 14);
        add_sprite!(FENCE_000, 20, 14);
        add_sprite!(FENCE_001, 20, 14);
        add_sprite!(SMTREE_000, 20, 14);
        add_sprite!(SMTREE_001, 20, 14);
        add_sprite!(TREE_000, 20, 14);
        add_sprite!(TREE_001, 20, 14);
        add_sprite!(TREE_002, 20, 14);
        add_sprite!(TREE_003, 20, 14);
        add_sprite!(WINDOW_000, 20, 14);
        add_sprite!(WINDOW_001, 20, 14);
        add_sprite!(LAVA_000, 20, 14);
        add_sprite!(LAVA_001, 20, 14);
        add_sprite!(LAVA2_001, 20, 14);
        add_sprite!(LAVA2_002, 20, 14);
        add_sprite!(LAVA2_003, 20, 14);
        add_sprite!(LAVA2_004, 20, 14);
        add_sprite!(LAVA2_005, 20, 14);
        add_sprite!(FALL_000, 20, 14);
        add_sprite!(FALL_001, 20, 14);
        add_sprite!(NOTE_000, 20, 14);
        add_sprite!(PIN_000, 20, 14);
        add_sprite!(QUEST_000, 20, 14);
        add_sprite!(QUEST_001, 20, 14);
        add_sprite!(PALBRICK_000, 20, 14);
        
        // 道具精灵 (20x14)
        add_sprite!(CHAMP_000, 20, 14);
        add_sprite!(POISON_000, 20, 14);
        add_sprite!(LIFE_000, 20, 14);
        add_sprite!(FLOWER_000, 20, 14);
        add_sprite!(STAR_000, 20, 14);
        
        // 火球/粒子 (12x7)
        add_sprite!(FIRE_000, 12, 7);
        add_sprite!(FIRE_001, 12, 7);
        add_sprite!(PART_000, 12, 7);
        
        // 火焰效果 (20x14)
        add_sprite!(F_000, 20, 14);
        add_sprite!(F_001, 20, 14);
        add_sprite!(F_002, 20, 14);
        add_sprite!(F_003, 20, 14);
        
        // 敌人精灵 (20x24)
        add_sprite!(GRKOOPA_000, 20, 24);
        add_sprite!(GRKOOPA_001, 20, 24);
        add_sprite!(RDKOOPA_000, 20, 24);
        add_sprite!(RDKOOPA_001, 20, 24);
        
        // 龟壳 (20x14)
        add_sprite!(GRKP_000, 20, 14);
        add_sprite!(GRKP_001, 20, 14);
        add_sprite!(RDKP_000, 20, 14);
        add_sprite!(RDKP_001, 20, 14);
        
        // 栗子怪 (20x14)
        add_sprite!(CHIBIBO_000, 20, 14);
        add_sprite!(CHIBIBO_001, 20, 14);
        add_sprite!(CHIBIBO_002, 20, 14);
        add_sprite!(CHIBIBO_003, 20, 14);
        
        // 鱼 (20x14)
        add_sprite!(FISH_001, 20, 14);
        
        // 食人花 (24x20)
        add_sprite!(PPLANT_000, 24, 20);
        add_sprite!(PPLANT_001, 24, 20);
        add_sprite!(PPLANT_002, 24, 20);
        add_sprite!(PPLANT_003, 24, 20);
        
        // 平台/其他 (20x14)
        add_sprite!(LIFT1_000, 20, 14);
        add_sprite!(DONUT_000, 20, 14);
        add_sprite!(DONUT_001, 20, 14);
        
        // 碰撞效果 (24x20)
        add_sprite!(HIT_000, 24, 20);
        
        // 其他 (20x14)
        add_sprite!(WHHIT_000, 20, 14);
        add_sprite!(WHFIRE_000, 20, 14);
        add_sprite!(RED_000, 20, 14);
        add_sprite!(RED_001, 20, 14);
        
        // 玩家精灵 - Mario (20x28)
        add_sprite!(SWMAR_000, 20, 28);
        add_sprite!(SWMAR_001, 20, 28);
        add_sprite!(SJMAR_000, 20, 28);
        add_sprite!(SJMAR_001, 20, 28);
        add_sprite!(LWMAR_000, 20, 28);
        add_sprite!(LWMAR_001, 20, 28);
        add_sprite!(LJMAR_000, 20, 28);
        add_sprite!(LJMAR_001, 20, 28);
        add_sprite!(FWMAR_000, 20, 28);
        add_sprite!(FWMAR_001, 20, 28);
        add_sprite!(FJMAR_000, 20, 28);
        add_sprite!(FJMAR_001, 20, 28);
        
        // 玩家精灵 - Luigi (20x28)
        add_sprite!(LWLUI_000, 20, 28);
        add_sprite!(LWLUI_001, 20, 28);
        add_sprite!(LJLUI_000, 20, 28);
        add_sprite!(LJLUI_001, 20, 28);
        add_sprite!(SWLUI_000, 20, 28);
        add_sprite!(SWLUI_001, 20, 28);
        add_sprite!(SJLUI_000, 20, 28);
        add_sprite!(SJLUI_001, 20, 28);
        add_sprite!(FWLUI_000, 20, 28);
        add_sprite!(FWLUI_001, 20, 28);
        add_sprite!(FJLUI_000, 20, 28);
        add_sprite!(FJLUI_001, 20, 28);
        
        // 背景精灵 (20x14)
        let palpill_pixels_0: Vec<u8> = self.PALPILL_000.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("PALPILL_000", 20, 14, &palpill_pixels_0).unwrap());
        let palpill_pixels_1: Vec<u8> = self.PALPILL_001.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("PALPILL_001", 20, 14, &palpill_pixels_1).unwrap());
        let palpill_pixels_2: Vec<u8> = self.PALPILL_002.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("PALPILL_002", 20, 14, &palpill_pixels_2).unwrap());
        
        // Intro大图 (各种尺寸)
        let intro_pixels_0: Vec<u8> = self.INTRO_000.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("INTRO_000", 108, 28, &intro_pixels_0).unwrap());
        let intro_pixels_1: Vec<u8> = self.INTRO_001.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("INTRO_001", 24, 28, &intro_pixels_1).unwrap());
        let intro_pixels_2: Vec<u8> = self.INTRO_002.pixels.iter().flatten().copied().collect();
        uvs.push(atlas.add_sprite("INTRO_002", 84, 28, &intro_pixels_2).unwrap());
        
        SpriteAtlas { atlas, uvs }
    }
}