// level_5.rs - 关卡 5-1 (Level 5a) 和 5-2 (Level 5b)
// 从 Pascal WORLDS.PAS 移植的地图数据
//
// 注意：
// 1. WORLDS.PAS 内包含扩展字符，直接复制容易出现替换字符，从而丢失真实字节
// 2. 本文件的地图常量通过工具 tools/dump_level5_map.py 从原始字节导出
// 3. 所有字节严格对应 Pascal 原版，使用 Latin-1 (ISO-8859-1) 编码

use crate::buffers::{WorldOptions, H, W, NV, MAX_WORLD_SIZE};

// ============================================================================
// Level 5a 地图数据（主关卡）
// ============================================================================

pub const LEVEL_5A_MAP: &[&[u8]] = &[
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA           ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA           ",
    b"AA220        ",
    b"AA331        ",
    b"AA2220       ",
    b"AA3331       ",
    b"AA           ",
    b"AA\x88          ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA\xF0\xF0\xF0        ",
    b"AA   ?\xE3      ",
    b"AA2220\xE0\x85     ",
    b"AA3331\xE1      ",
    b"AA           ",
    b"AA\xF0\xF0     *   ",
    b"AA\xF0\xF0\xF0\xF0\xF0  *   ",
    b"AA\x88      *   ",
    b"             ",
    b"             ",
    b"AA2220       ",
    b"AA3331       ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0          ",
    b"AA\xF0\xF0 ?   ?   ",
    b"AA   ?   ?\xE0  ",
    b"AA   ?   ?   ",
    b"AA           ",
    b"AAX\x89         ",
    b"             ",
    b"             ",
    b"AA  X\x89       ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA    X\x89     ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA   X\x89      ",
    b"AA\xF0\xF0         ",
    b"AA\xF0          ",
    b"AA2220\xE8\x85     ",
    b"AA3331\xE1      ",
    b"AA           ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF0\xF0         ",
    b"AA           ",
    b"AA%%%%%      ",
    b"AA#####%%    ",
    b"AA#######    ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\x88          ",
    b"AA\xF0\xF0         ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA2220\xE1\x85     ",
    b"AA3331\xE0      ",
    b"          *  ",
    b"          *  ",
    b"AA220     *  ",
    b"AA331     *  ",
    b"AA22220      ",
    b"AA33331      ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF0\xF0\xF0 X\x89     ",
    b"AA    X      ",
    b"AA           ",
    b"AA  X        ",
    b"AA  X\x89       ",
    b"AA\xF0\xF0         ",
    b"AA\xF0\xF0\xF0   X\x89   ",
    b"AA      X    ",
    b"             ",
    b"AA2222220\xE8   ",
    b"AA3333331\xE1   ",
    b"AA           ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF0\xF0\xF0   ?\xE0   ",
    b"AA\x89     ?    ",
    b"AA           ",
    b"AA           ",
    b"             ",
    b"             ",
    b"             ",
    b"           $\xE1",
    b"        $\xED   ",
    b"AA   $       ",
    b"AA\xF0\xF0\xF0        ",
    b"AA%%%%%%%%   ",
    b"AA%%%%%%%#   ",
    b"AA#######    ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA           ",
    b"AA           ",
    b"AAX\x89         ",
    b"AA           ",
    b"AAXXX\x89       ",
    b"AA           ",
    b"AAXXXXX\x89   ?\xE0",
    b"AA220        ",
    b"AA331        ",
    b"             ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA           ",
    b"AA22220      ",
    b"AA33331      ",
    b"AA2220       ",
    b"AA3331       ",
    b"AA           ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF0\xF0         ",
    b"AA           ",
    b"             ",
    b"             ",
    b"             ",
    b"AA           ",
    b"AA           ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\x89          ",
    b"AA           ",
    b"AA\xF0\xF0         ",
    b"AA\xF0          ",
    b"AA  XX\x89 ***  ",
    b"AA           ",
    b"AA\xF0\xF0\xF0        ",
    b"AA   XX\x89 *** ",
    b"AA           ",
    b"AA\xF0\xF0         ",
    b"AA\xF0\xF0\xF0 XX\x89 ***",
    b"AA\xF0          ",
    b"AA           ",
    b"AA220        ",
    b"AA331        ",
    b"AA           ",
    b"AA\xF0          ",
    b"AA\xF0\xF0\xF0        ",
    b"AA           ",
    b"AA\xFE\xFE         ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\x88          ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA\xF0\xF0\xF0\xF0\xF0\xF0     ",
    b"AA\xF0\xF0         ",
    b"AA           ",
    b"AA2220\xE7      ",
    b"AA3331\xE7      ",
    b"AA           ",
    b"AA           ",
];

// ============================================================================
// Level 5b 地图数据（副关卡）
// ============================================================================

pub const LEVEL_5B_MAP: &[&[u8]] = &[
    b"AAXXXXXXXXXXX",
    b"AA=  X      X",
    b"AA=      ** X",
    b"AA=      ** X",
    b"AA=  X   ** X",
    b"AA=      ** X",
    b"AA=      ** X",
    b"AA=  X      X",
    b"AAXXXXXX    X",
    b"AA     X    X",
    b"AA   ? X    X",
    b"AA  ?\xE0 X    X",
    b"AA     X    X",
    b"#%          X",
    b"#%          X",
    b"#%          X",
    b"AA K        X",
    b"#%          X",
    b"#%          X",
    b"#%          X",
    b"AA     X    X",
    b"AA     X    X",
    b"AA     X  \xE802",
    b"AA     X  \xE013",
    b"AA     X    X",
    b"AA\x80         X",
    b"AA          X",
    b"AA   X    * X",
    b"AA   X    * X",
    b"AA   X    * X",
    b"AA   X    * X",
    b"AA   X    * X",
    b"#%          X",
    b"#%          X",
    b"#%          X",
    b"AA   X      X",
    b"AA   X\x89     X",
    b"AA   X  X\x89  X",
    b"AA      X   X",
    b"AA      X   X",
    b"AA          X",
    b"AA          X",
    b"AA          X",
    b"AA2220\xE1   * X",
    b"AA3331\xE1   * X",
    b"AA          X",
    b"AAXXXXXXXXXXX",
];

// ============================================================================
// 关卡配置
// ============================================================================

#[derive(Clone, Debug)]
pub struct Level5Options;

impl Level5Options {
    /// Options_5a - Level 5a 的第一套配置
    pub fn options_5a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 12,
            wall_type1: 0,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0xB0,
            ground_color1: 0x58,
            ground_color2: 0,
            horizon: 148,
            backgr_type: 3,
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
            brick_color: 0x30,
            wood_color: 0x30,
            xblock_color: 0x30,
            ..WorldOptions::default()
        }
    }

    /// Opt_5a - Level 5a 的第二套配置
    pub fn opt_5a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 9,
            wall_type1: 6,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0xB0,
            ground_color1: 0x58,
            ground_color2: 0,
            horizon: 148,
            backgr_type: 1,
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
            brick_color: 0x30,
            wood_color: 0x30,
            xblock_color: 0x30,
            ..WorldOptions::default()
        }
    }

    /// Options_5b - Level 5b 配置
    pub fn options_5b() -> WorldOptions {
        WorldOptions {
            init_x: (9 * W + 10) as u16,
            init_y: (0 * H) as u16,
            sky_type: 6,
            wall_type1: 102,
            wall_type2: 101,
            wall_type3: 0,
            pipe_color: 0x50,
            ground_color1: 0x48,
            ground_color2: 0,
            horizon: 12,
            backgr_type: 6,
            backgr_color1: 0x34,
            backgr_color2: 0x4C,
            stars: 0,
            clouds: 0,
            design: 5,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0x48,
            wood_color: 0x30,
            xblock_color: 0xB0,
            ..WorldOptions::default()
        }
    }
}

// ============================================================================
// 关卡实现
// ============================================================================

pub struct Level5;

impl Level5 {
    pub fn new() -> Self {
        Self {}
    }

    // run 方法已删除 - 使用新的状态机驱动的 play.frame_update() 方法

    fn convert_map_data(map_bytes: &[&[u8]]) -> [[char; NV as usize]; (MAX_WORLD_SIZE as usize + 1)] {
        let mut map = [['\0'; NV as usize]; (MAX_WORLD_SIZE as usize + 1)];

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
