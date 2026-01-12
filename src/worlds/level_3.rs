// level_3.rs - 关卡 3-1 (Level 3a)
// 从 Pascal WORLDS.PAS 移植的地图数据
//
// 注意
// 1 WORLDS.PAS 内包含扩展字符，直接复制容易出现替换字符，从而丢失真实字节
// 2 本文件的地图常量通过离线工具 src/bin/dump_pascal_level3a_map.rs 从原始字节导出

use crate::buffers::{WorldOptions, H, W, NV, MAX_WORLD_SIZE};

// ============================================================================
// Level 3a 地图数据（主关卡）
// ============================================================================

pub const LEVEL_3A_MAP: &[&[u8]] = &[
    b"AA           ",
    b"AA\xF7        ?\xE0",
    b"AA\xF7    I     ",
    b"AA     I\x89    ",
    b"AA      \xF4    ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"AA      \xF5    ",
    b"         I   ",
    b"         I\x89  ",
    b"             ",
    b"             ",
    b"             ",
    b"AA     I\x89    ",
    b"AA     I     ",
    b"AA           ",
    b"AACCC*       ",
    b"AA\xF7CCCCCCCC* ",
    b"AA\xF7CCCCCCCC* ",
    b"AACCCCCCCCC* ",
    b"AACCCCCCC*   ",
    b"AA           ",
    b"AA?\xE0         ",
    b"             ",
    b"             ",
    b"             ",
    b"             ",
    b"20           ",
    b"31      \xF4    ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"AA      \xF5    ",
    b"AACCCC*      ",
    b"AACCCCCCCCC* ",
    b"AA\xF7CCCCCCCC* ",
    b"AA\xF7CCCCCCCC*\x89",
    b"AACCCCCCC*   ",
    b"AAACCCCCC*   ",
    b"AAA\x87         ",
    b"AAA          ",
    b"220 \x85        ",
    b"331          ",
    b"222220 \x85     ",
    b"333331       ",
    b"             ",
    b"             ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\xF7   ?    ? ",
    b"AA    ? \x89  ? ",
    b"AA    ?    ?\xE0",
    b"AA    ?    ? ",
    b"AA    ?    ? ",
    b"AA          \x89",
    b"AA    K \xF4    ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"AAI\x87    \xF5    ",
    b"          J* ",
    b"          J* ",
    b"          J* ",
    b"          J* ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA\x87          ",
    b"AACCC*       ",
    b"AACCCCCCCCC* ",
    b"AACCCCCCCCC* ",
    b"AACCCCCCCCC*\x89",
    b"AA    K      ",
    b"AA    K      ",
    b"AA\x87   K      ",
    b"AA           ",
    b"2220 \x85    J* ",
    b"3331      J* ",
    b"          J* ",
    b"          J* ",
    b"AJ      \xF4    ",
    b"A\xF6\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"A\x87      \xF5    ",
    b"A      I     ",
    b"A      I     ",
    b"A      I     ",
    b"A      I\x89    ",
    b"A      I?    ",
    b"A\x87           ",
    b"A?\xE1          ",
    b"        I220 ",
    b"        I331 ",
    b"220 \x85   I    ",
    b"331     I    ",
    b"22220        ",
    b"33331        ",
    b"AA  ?\xE0   I   ",
    b"AA\x87 ?    I\x89  ",
    b"AACCC\xF7       ",
    b"AACCC\xF7       ",
    b"AACCC\xF7       ",
    b"AACCC        ",
    b"AA\x87          ",
    b"AA          I",
    b"2222220 \x85   I",
    b"3333331    \xFCI",
    b"22220 \x85     I",
    b"33331       I",
    b"AA         \xFCI",
    b"AA       0222",
    b"AA\xF7      1333",
    b"AA\xF7        \xFCI",
    b"            I",
    b"            I",
    b"           \xFCI",
    b"AA      \xF4    ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"AA\x89     \xF5    ",
    b"AACCC*       ",
    b"AACCCCCC*    ",
    b"AACCCCCCCCC* ",
    b"AAACCCCCCCC* ",
    b"AAAAAACCCCC* ",
    b"AAAAAA\x89      ",
    b"AAAAAA\xF7      ",
    b"AAAAAA\xF7      ",
    b"AAAA         ",
    b"AA           ",
    b"AA      $    ",
    b"AA     \xF4     ",
    b"AA\xF6\xF6\xF6\xFE\xF6\xF9\xFA\xF4   ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF5\xF6\xF9\xFA  ",
    b"AA      \xF4\xF5$  ",
    b"AA\xF6\xF6\xF6\xF6\xF6\xF6\xF9\xFA   ",
    b"AA\x88     \xF5    ",
    b"AA\xF7          ",
    b"AA\xF7        $ ",
    b"AA\xF7          ",
    b"AA2220\xE7      ",
    b"AA3331\xE7      ",
    b"AA         $ ",
    b"             ",
    b"\xAFAAAAAAAAAAA ",
    b"AAAAAAAAAAAA ",
    b"AAAAAAAAAAAA ",
    b"AAAAAAAAAAAA ",
    b"AAAAAA       ",
    b"AAAAAACC\xFE\xFE   ",
    b"AAAAAACC   ? ",
    b"AAAACCCC\xF7  ? ",
    b"AACCCCCC\xF7  ? ",
    b"AACCCCCC\x89  ? ",
    b"AA         ?\xE1",
    b"AA2220\xE7    ? ",
    b"AA3331\xE7    ? ",
    b"AA         ? ",
    b"AACCCCC    ? ",
    b"AACCCCC\x89     ",
    b"AA           ",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
];

pub const LEVEL_3B_MAP: &[&[u8]] = &[];

// ============================================================================
// 关卡配置
// ============================================================================

#[derive(Clone, Debug)]
pub struct Level3Options;

impl Level3Options {
    pub fn options_3a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 10,
            wall_type1: 2,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0x18,
            ground_color1: 0xB2,
            ground_color2: 0x70,
            horizon: 140,
            backgr_type: 1,
            backgr_color1: 0x36,
            backgr_color2: 0x30,
            stars: 0,
            clouds: 0,
            design: 1,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0x30,
            wood_color: 0x30,
            xblock_color: 0x68,
            ..WorldOptions::default()
        }
    }

    pub fn opt_3a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (9 * H) as u16,
            sky_type: 12,
            wall_type1: 2,
            wall_type2: 0,
            wall_type3: 0,
            pipe_color: 0x18,
            ground_color1: 0xB2,
            ground_color2: 0x70,
            horizon: 140,
            backgr_type: 1,
            backgr_color1: 0x36,
            backgr_color2: 0x30,
            stars: 0,
            clouds: 0,
            design: 1,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0x30,
            wood_color: 0x30,
            xblock_color: 0x68,
            ..WorldOptions::default()
        }
    }

    pub fn options_3b() -> WorldOptions {
        WorldOptions::default()
    }
}

// ============================================================================
// 关卡实现
// ============================================================================

pub struct Level3;

impl Level3 {
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

