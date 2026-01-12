// level_2.rs - 关卡 2-1 (Level 2a)
// 从 Pascal WORLDS.PAS 移植的地图数据

use crate::buffers::{WorldOptions, H, W, NV, MAX_WORLD_SIZE};

// ============================================================================
// Level 2a 地图数据（主关卡）
// ============================================================================

pub const LEVEL_2A_MAP: &[&[u8]] = &[
    b"AAAAAAAAAAAAA",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA         \xFCJ",
    b"AA         \xFCJ",
    b"AA         \xFCJ",
    b"AA220 \x85    \xFCJ",
    b"AA331      \xFCJ",
    b"AA22220 \x85  \xFCJ",
    b"AA33331    \xFCJ",
    b"AA    ?    \xFCJ",
    b"AA    ?\xE0   \xFCJ",
    b"AA20 \x85     \xFCJ",
    b"AA31       \xFCJ",
    b"AA         \xFCJ",
    b"AA         \xFCJ",
    b"#%           ",
    b"#%           ",
    b"#%           ",
    b"AA       AAAA",
    b"AA       AAAA",
    b"AA       AAAA",
    b"AA       AAAA",
    b"AAI      AAAA",
    b"#%I\x88       I ",
    b"#%           ",
    b"#%\x82          ",
    b"#%           ",
    b"#%I        I ",
    b"AAI      AAAA",
    b"AA       AAAA",
    b"AA          A",
    b"AA          A",
    b"AA          A",
    b"AA220\xE8      A",
    b"AA331\xE0      A",
    b"AA222220 \x85  A",
    b"AA333331    A",
    b"            A",
    b"            A",
    b" K          A",
    b"AA          A",
    b"AA          A",
    b"AA     A    A",
    b"AA     A    A",
    b"AA     A    A",
    b"AA     A    A",
    b"AA\x88    A    A",
    b"AA     A    A",
    b"#%     A    A",
    b"#%\x82         A",
    b"#%     A    A",
    b"AA     A    A",
    b"AA     A    A",
    b"AA     A    A",
    b"#%     A    A",
    b"#%\x82         A",
    b"#%     A    A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A ** A",
    b"AA     A\x80** A",
    b"AA     A ** A",
    b"AA     A\x80   A",
    b"AA          A",
    b"AA    A20\xE0\x85 A",
    b"AA    A31\xE3  A",
    b"AA    AAAAAAA",
    b"AA     $\xE3    ",
    b"AA           ",
    b"AA\x88          ",
    b"AA           ",
    b"AA2220\xE0\x86   J ",
    b"AA3331\xE1    J ",
    b"#%         J ",
    b"#%         J\xE0",
    b"#%         J ",
    b"AA22220 \x86  J ",
    b"AA33331    J\x89",
    b"#%         J ",
    b"#%         J ",
    b"#%         J ",
    b"#%  W      J ",
    b"AA220 \x86    J ",
    b"AA331      J ",
    b"#%           ",
    b"#%           ",
    b"#%           ",
    b"AA2220\xE0      ",
    b"AA3331\xE0      ",
    b"AA222220     ",
    b"AA333331     ",
    b"#% I   ?\xE0    ",
    b"#% I         ",
    b"#%\x82          ",
    b"#% I         ",
    b"#% I         ",
    b"#%\x82          ",
    b"#% I         ",
    b"AA I         ",
    b"AA2220\xE8      ",
    b"AA3331\xE2      ",
    b"AA   ?\xE2  ?\xED  ",
    b"AA           ",
    b"AA           ",
    b"AA220 \x85      ",
    b"AA331        ",
    b"AA           ",
    b"AA           ",
    b"AA22220\xE8     ",
    b"AA33331\xE3     ",
    b"#%        *  ",
    b"#%\x82        * ",
    b"#%\x82        * ",
    b"#%        *  ",
    b"AA22220 \x85    ",
    b"AA33331      ",
    b"AA    W      ",
    b"AA ** W      ",
    b"AA ** W      ",
    b"AA ** W      ",
    b"AA ** W     A",
    b"AA    W  WW A",
    b"#%    W  W  A",
    b"#%    W  W  A",
    b"#%    W  W  A",
    b"AA    W  W\x88 A",
    b"AA    W     A",
    b"AA    W     A",
    b"AA    WW    A",
    b"#%\x82         A",
    b"AA    WW    A",
    b"AA    W  *  A",
    b"AA    W  *  A",
    b"AA    W  *  A",
    b"AA    W\x80 *  A",
    b"AA    WW    A",
    b"AA          A",
    b"AA          A",
    b"AA220 \x86     A",
    b"AA331       A",
    b"AA222220 \x86  A",
    b"AA333331    A",
    b"AA          A",
    b"AA          A",
    b"AAI         A",
    b"AAI         A",
    b"#%          A",
    b"#%\x82         A",
    b"#%          A",
    b"#%I         A",
    b"#%I         A",
    b"#%          A",
    b"#%\x82         A",
    b"#%          A",
    b"AAI         A",
    b"AAI         A",
    b"AA          A",
    b"AA          A",
    b"AA\xFE\xFE        A",
    b"AA          A",
    b"AA          A",
    b"AA\x88         A",
    b"AA          A",
    b"AA2220\xE7     A",
    b"AA3331\xE7     A",
    b"AA   ?\xE3     A",
    b"AA\x80         A",
    b"AA\x80         A",
    b"AA          A",
    b"AA          A",
    b"AAAAAAAAAAAAA",
    b"\xAFAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAAAAAAAAAAAA",
    b"AAA    ?\xE1    ",
    b"AAA\xFE\xFE        ",
    b"AAA          ",
    b"AAA  ****    ",
    b"AAA  ****    ",
    b"AAA  ****    ",
    b"AAA  ****    ",
    b"AAA  ****    ",
    b"AAA          ",
    b"AAA          ",
    b"AAA2220\xE7     ",
    b"AAA3331\xE7     ",
    b"AAAAAAAAAAAAA",
    b"\xAEAAAAAAAAAAA\xAD",
    b"\xAFAAAAAAAAAAA\xAD",
    b"      A\x88    A",
    b"      A *** A",
    b"220 \x86   *** A",
    b"331         A",
    b"  A   0222222",
    b"  A   1333333",
    b"  A      \xE8022",
    b"  A\x88     \xE1133",
    b"220       A  ",
    b"331       A  ",
    b"22220 \x85   A  ",
    b"33331     A  ",
    b"          A  ",
    b"          022",
    b"20        133",
    b"31      02222",
    b" A      13333",
    b" A          A",
    b" A\x89  *****  A",
    b" A   *****  A",
    b"20 \x86 *****  A",
    b"31          A",
    b"22220 \x85 \xE00222",
    b"33331   \xE21333",
    b"            A",
    b"            A",
    b"    @    ?\xE0 A",
    b"AAAAAAAAAAAAA",
];

// ============================================================================
// Level 2b 地图数据（地下室/奖励关卡）- 空关卡
// ============================================================================

pub const LEVEL_2B_MAP: &[&[u8]] = &[
    // Level 2b 是空的
];

// ============================================================================
// 关卡配置
// ============================================================================

#[derive(Clone, Debug)]
pub struct Level2Options;

impl Level2Options {
    /// Options_2a - 主关卡配置（地下风格）
    pub fn options_2a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (0 * H) as u16,
            sky_type: 8,      // 地下天空
            wall_type1: 102,
            wall_type2: 101,
            wall_type3: 0,
            pipe_color: 0x50,
            ground_color1: 0x48,
            ground_color2: 0,
            horizon: 136,
            backgr_type: 4,   // 地下背景
            backgr_color1: 0x34,
            backgr_color2: 0x4C,
            stars: 0,
            clouds: 0,
            design: 4,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0x48,
            wood_color: 0x30,
            xblock_color: 0x68,
            ..WorldOptions::default()
        }
    }

    /// Opt_2a - 备用配置
    pub fn opt_2a() -> WorldOptions {
        WorldOptions {
            init_x: (2 * W + 10) as u16,
            init_y: (0 * H) as u16,
            sky_type: 6,      // 不同的天空
            wall_type1: 102,
            wall_type2: 101,
            wall_type3: 0,
            pipe_color: 0x50,
            ground_color1: 0x48,
            ground_color2: 0,
            horizon: 136,
            backgr_type: 6,   // 不同的背景
            backgr_color1: 0x65,
            backgr_color2: 0x1A,
            stars: 0,
            clouds: 0,
            design: 4,
            c2r: 10,
            c2g: 23,
            c2b: 8,
            c3r: 22,
            c3g: 35,
            c3b: 20,
            brick_color: 0x48,
            wood_color: 0x30,
            xblock_color: 0x68,
            ..WorldOptions::default()
        }
    }

    /// Options_2b - 地下室配置（空关卡）
    pub fn options_2b() -> WorldOptions {
        WorldOptions::default()
    }
}

// ============================================================================
// 关卡实现
// ============================================================================

pub struct Level2;

impl Level2 {
    /// 创建新的关卡 2 实例
    pub fn new() -> Self {
        Self {}
    }

    // run 方法已删除 - 使用新的状态机驱动的 play.frame_update() 方法

    /// 将 &[&[u8]] 地图数据转换为 [[char; NV]; MAX_WORLD_SIZE] 格式
    ///
    /// Pascal地图格式：
    /// - 每个 db 字符串是一列（X方向）的数据
    /// - 字符串中的每个字节是该列中Y方向的一个tile
    ///
    /// Rust格式：
    /// - map[x][y] 访问格式
    /// - X范围：1..地图列数+1（因为X+1偏移）
    /// - Y范围：0..NV
    ///
    /// 注意：Y 翻转发生在 Buffers::read_world 内部（W^[X, NV-i]），这里必须保持"原始列数据"不翻转。
    fn convert_map_data(map_bytes: &[&[u8]]) -> [[char; NV as usize]; (MAX_WORLD_SIZE as usize + 1)] {
        let mut map = [['\0'; NV as usize]; (MAX_WORLD_SIZE as usize + 1)];

        for (col, line) in map_bytes.iter().enumerate() {
            let x = col + 1; // 关键：列偏移，严格对齐 Pascal 的 X+1 访问
            if x > MAX_WORLD_SIZE as usize {
                break;
            }
            for (y, &byte_val) in line.iter().enumerate() {
                if y >= NV as usize {
                    break;
                }
                // 保持 0..255 单字节值，与 Pascal/ISO-8859-1 一致（例如 0xE1/0xE7 等）
                map[x][y] = byte_val as char;
            }
        }

        map
    }
}
