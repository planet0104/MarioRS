// level_6.rs - 关卡 6-1 (Level 6a) 和 6-2 (Level 6b)
// 从 Pascal WORLDS.PAS 移植的地图数据
//
// 注意：
// 1. WORLDS.PAS 内包含扩展字符，直接复制容易出现替换字符，从而丢失真实字节
// 2. 本文件的地图常量通过工具 tools/dump_level6_map.py 从原始字节导出
// 3. 所有字节严格对应 Pascal 原版，使用 Latin-1 (ISO-8859-1) 编码

use crate::buffers::{WorldOptions, H, W, NV, MAX_WORLD_SIZE};

// ============================================================================
// Level 6a 地图数据（主关卡）- 141 行 × 13 列
// ============================================================================

pub const LEVEL_6A_MAP: &[&[u8]] = &[
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0    ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF7          ",
    b"AA\xF7   J      ",
    b"AA\x87   J      ",
    b"AAA   J      ",
    b"AAA        ?\xE0",
    b"AAAA    J    ",
    b"AAAA    J    ",
    b"AAAA\x87   J    ",
    b"AAAAA\xF7       ",
    b"AAAAA\xF7       ",
    b"AAAAA\xF0\xF0\xF0\xF0    ",
    b"AAAAA\xF0\xF0\xF0\xF0\xF0   ",
    b"AAAAA        ",
    b"AAAA20\xE8\x84     ",
    b"AAAA31\xE1      ",
    b"AAAA         ",
    b"AAA\x87      *  ",
    b"AA        *  ",
    b"AA\xF0\xF0\xF0     *  ",
    b"AA\xF0\xF0      *  ",
    b"AA           ",
    b"AA2220       ",
    b"AA3331       ",
    b"AA22220 \x84    ",
    b"AA33331      ",
    b"AA           ",
    b"AA\x88          ",
    b"AA\xF7          ",
    b"AA\xF7   ?\xE0     ",
    b"AA\xF0\xF0\xF0 ?      ",
    b"AA\xF0\xF0\xF0\xF0?      ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA           ",
    b"AA           ",
    b"AA22220\xE0\x85    ",
    b"AA33331\xE1     ",
    b"AA           ",
    b"AA           ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA\x87          ",
    b"AACCCCCCCCC  ",
    b"AA\xF7CCCCCCCC  ",
    b"AA\xF7CCCCCCCC\x89 ",
    b"AACCCCCCCCC  ",
    b"AAAAACCCCCC  ",
    b"AAAAAAACCCCC ",
    b"AAAAAAACCCCC ",
    b"AAAAAAA\x89     ",
    b"AAAAAAA      ",
    b"ACCC         ",
    b"ACCC\xF7        ",
    b"AACC\xF7        ",
    b"AACC         ",
    b"AACC         ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0\xF0        ",
    b"AAWWWWW\x89     ",
    b"AA\xF7          ",
    b"AA\xF7   $\xE1   * ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0  * ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0\xF0 * ",
    b"AA\x87          ",
    b"AAWWWWW\x89     ",
    b"AA\xF7          ",
    b"AA\xF7        * ",
    b"AA\x87        * ",
    b"AA\xF0\xF0\xF0 $\xED   * ",
    b"AA\xF0\xF0         ",
    b"AAWWWWW\x89     ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF0\xF0\xF0        ",
    b"AA           ",
    b"AA\xF7   W      ",
    b"AA\xF7   W      ",
    b"AA    W\x89     ",
    b"A            ",
    b"  K          ",
    b"         K   ",
    b"  K          ",
    b"         K   ",
    b"  K          ",
    b"A            ",
    b"AA\x87   W      ",
    b"AA    W20\xE1\x85  ",
    b"AA\xF0\xF0\xF0 W31\xE0   ",
    b"AA\xF0\xF0  W220\xE8\x85 ",
    b"AA\x87   W331\xE1  ",
    b"AA\xF7   W      ",
    b"AA\xF7          ",
    b"A            ",
    b"  K      ?\xE3  ",
    b"A            ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0    ",
    b"AA           ",
    b"AAWWWWW      ",
    b"   ?\xE1 K      ",
    b"A     K    * ",
    b"AA    K    * ",
    b"AA    K    * ",
    b"A     K    * ",
    b"A\x87    K    * ",
    b"AA\xF7   K      ",
    b"AA\xF7   W220 \x85 ",
    b"A\x87    W331   ",
    b"A     W20 \x85  ",
    b"A     W31    ",
    b"AA\xF7   K      ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AAW          ",
    b"AA           ",
    b"AA\xFE\xFE         ",
    b"AA\x87          ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0\xF0   ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0\xF0    ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\x87          ",
    b"AA2220\xE7      ",
    b"AA3331\xE7      ",
    b"AA           ",
    b"AA           ",
];

// ============================================================================
// Level 6b 地图数据（副关卡）- 50 行 × 13 列
// ============================================================================

pub const LEVEL_6B_MAP: &[&[u8]] = &[
    b"AAA          ",
    b"AAA          ",
    b"AAA2220\xE8     ",
    b"AAA3331\xE0     ",
    b"AAA          ",
    b"AAA\xF0\xF0\xF0\xF0\xF0     ",
    b"AAA\xF0\xF0\xF0\xF0      ",
    b"AAA\xF7      ** ",
    b"AAA\xF7  K  ****",
    b"AAA\xF0\xF0\xF0    ** ",
    b"AAA\xF0\xF0\xF0\xF0      ",
    b"AAA\x87         ",
    b"AAACCCC      ",
    b"AAACCCC\x87     ",
    b"AA\xF7CCCCC     ",
    b"AA\xF7CCCCC     ",
    b"AA\xF7CCCCC     ",
    b"AACCCCCCC    ",
    b"AAAAAACCC    ",
    b"AAAAAA\xF7CC    ",
    b"AAAAAA\xF7CC    ",
    b"AAAAAACCC    ",
    b"AAACCCCCC    ",
    b"ACCCCCCCC    ",
    b"ACCCCC       ",
    b"ACCCCC       ",
    b"AACCCC       ",
    b"AA\x87          ",
    b"AA   K     ?\xE0",
    b"AA\xF0\xF0         ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AAX\x89         ",
    b"AA   K       ",
    b"AA\x87          ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA           ",
    b"AA\xF7       ** ",
    b"AA\xF7  K   ****",
    b"AA\xF0\xF0\xF0     ** ",
    b"AA\x87          ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AA2220\xE1      ",
    b"AA3331\xE1      ",
    b"AA           ",
    b"AA           ",
];

// ============================================================================
// 关卡配置
// ============================================================================

#[derive(Clone, Debug)]
pub struct Level6Options;

impl Level6Options {
    /// Options_6a - Level 6a 的第一套配置
    pub fn options_6a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 10,
            wall_type1: 0,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0x30,
            ground_color1: 0x4B,
            ground_color2: 0,
            horizon: 124,
            backgr_type: 10,
            backgr_color1: 0x36,
            backgr_color2: 0x30,
            stars: 0,
            clouds: 0,
            design: 2,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0xB0,
            wood_color: 0x48,
            xblock_color: 0xA0,
            ..WorldOptions::default()
        }
    }

    /// Opt_6a - Level 6a 的第二套配置
    pub fn opt_6a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 3,
            wall_type1: 0,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0x30,
            ground_color1: 0x4B,
            ground_color2: 0,
            horizon: 124,
            backgr_type: 10,
            backgr_color1: 0x36,
            backgr_color2: 0x30,
            stars: 0,
            clouds: 0,
            design: 2,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0xB0,
            wood_color: 0x48,
            xblock_color: 0xA0,
            ..WorldOptions::default()
        }
    }

    /// Options_6b - Level 6b 配置
    pub fn options_6b() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 11,
            wall_type1: 2,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0x30,
            ground_color1: 0xB0,
            ground_color2: 0x71,
            horizon: 124,
            backgr_type: 9,
            backgr_color1: 0x36,
            backgr_color2: 0x30,
            stars: 0,
            clouds: 0,
            design: 2,
            c2r: 10,
            c2g: 20,
            c2b: 8,
            c3r: 20,
            c3g: 40,
            c3b: 16,
            brick_color: 0xB0,
            wood_color: 0x48,
            xblock_color: 0x30,
            ..WorldOptions::default()
        }
    }
}

// ============================================================================
// 关卡实现
// ============================================================================

pub struct Level6;

impl Level6 {
    pub fn new() -> Self {
        Self {}
    }

    // run 方法已删除 - 使用新的状态机驱动的 play.frame_update() 方法

    fn convert_map_data(map_bytes: &[&[u8]]) -> [[char; NV as usize]; MAX_WORLD_SIZE as usize + 1] {
        let mut map = [['\0'; NV as usize]; MAX_WORLD_SIZE as usize + 1];

        for (col, line) in map_bytes.iter().enumerate() {
            let x = col + 1;
            if x > MAX_WORLD_SIZE as usize {
                break;
            }
            for (y, &byte_val) in line.iter().enumerate() {
                if y >= NV as usize {
                    break;
                }
                map[x][y] = byte_val as char;
            }
        }

        map
    }
}
