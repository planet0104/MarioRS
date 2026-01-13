// Figures module - handles sprite management and world rendering
// Converted from FIGURES.PAS

use crate::backgr::BackGr;
use crate::buffers::{Buffers, CAN_HOLD_YOU, EX, EY1, ImageBuffer, NV, WorldBuffer, WorldOptions};
use crate::palettes::Palettes;
use crate::sprites::SpriteDataManager;
use crate::vga256::VGA;

/// 调试开关：是否打印特定tile位置的精灵文件名
/// 设置为true时，会打印指定坐标的tile绘制信息
const DEBUG_PRINT_TILE_SPRITES: bool = false;

pub const N1: usize = 3;
pub const N2: usize = 14;

pub struct Figures {
    pub fig_list: [[ImageBuffer; N2]; N1],
    pub bricks: [ImageBuffer; 4],
    pub sky: u8,
    /// P1-2: 实例级 trace 标志（替代静态 AtomicBool）
    pub trace_enabled: bool,
}

impl Figures {
    pub fn new(fig_list: [[ImageBuffer; N2]; N1], bricks: [ImageBuffer; 4], sky: u8) -> Self {
        Self {
            fig_list,
            bricks,
            sky,
            trace_enabled: false, // P1-2: 默认关闭
        }
    }

    /// P1-2修复：设置精灵跟踪
    pub fn set_trace_enabled(&mut self, enabled: bool) {
        self.trace_enabled = enabled;
    }

    /// 内联 trace helper
    #[inline]
    fn trace_sprite(&self, x: i32, y: i32, name: &str) {
        if self.trace_enabled {
            println!("[DRAW_TILE] x={x} y={y} sprite={name}");
        }
    }

    /// Rust移植：将两张草地图片合成一张，规则同Pascal ConvertGrass
    /// p0: 输出缓冲区（可变）
    /// p1: 输入缓冲区1
    /// p2: 输入缓冲区2
    /// 转换草地精灵颜色
    /// 严格对齐 Pascal FIGURES.PAS ConvertGrass 过程
    ///
    /// Pascal 原版逻辑:
    /// ```pascal
    /// procedure Convert;
    /// begin
    ///   C0 := C1;
    ///   if C1 = C2 then Exit;
    ///   if C1 = 2 then begin C0 := 153; if C2 = 0 then Exit; C0 := 155; end
    ///   else if C1 = 3 then begin C0 := 154; if C2 = 0 then Exit; C0 := 156; end
    ///   else { C1 不是 2 也不是 3 } if C2 = 2 then C0 := 157 else C0 := 155;
    /// end;
    /// ```
    pub fn convert_grass(&self, p0: &mut ImageBuffer, p1: &ImageBuffer, p2: &ImageBuffer) {
        let h = crate::buffers::H;
        let w = crate::buffers::W;
        for i in 0..h as usize {
            for j in 0..w as usize {
                let c1 = p1[i][j];
                let c2 = p2[i][j];
                // Pascal: C0 := C1; if C1 = C2 then Exit;
                let c0 = if c1 == c2 {
                    c1
                } else if c1 == 2 {
                    // Pascal: C0 := 153; if C2 = 0 then Exit; C0 := 155;
                    if c2 == 0 { 153 } else { 155 }
                } else if c1 == 3 {
                    // Pascal: C0 := 154; if C2 = 0 then Exit; C0 := 156;
                    if c2 == 0 { 154 } else { 156 }
                } else {
                    // Pascal: else { C1 不是 2 也不是 3 } if C2 = 2 then C0 := 157 else C0 := 155;
                    // 注意: Pascal 这里对所有非2非3的C1值都执行同样的逻辑(包括0和其他值)
                    if c2 == 2 { 157 } else { 155 }
                };
                p0[i][j] = c0;
            }
        }
    }

    /// 对精灵图像进行颜色重映射
    ///
    /// # 参数
    /// * `src` - 源图像缓冲区，如果不提供dst，将直接修改此缓冲区
    /// * `dst` - 可选的目标缓冲区，如果提供则将结果写入此缓冲区
    /// * `c` - 颜色偏移值
    ///
    /// # 实现细节
    /// - 对于像素值 <= 0x10 的部分保持不变
    /// - 对于像素值 > 0x10 的部分:
    ///   1. 取低3位(& 0x07)
    ///   2. 加上颜色偏移值 c
    pub fn recolor<const WW: usize, const HH: usize>(
        &self,
        src: &mut [[u8; WW]; HH],
        dst: Option<&mut [[u8; WW]; HH]>,
        c: u8,
    ) {
        // [RECOLOR_DEBUG] 添加调试日志查看重着色过程
        let mut changed_count = 0;
        let mut sample_before = 0u8;
        let mut sample_after = 0u8;
        
        match dst {
            Some(dst_buf) => {
                for y in 0..HH {
                    for x in 0..WW {
                        let val = src[y][x];
                        let new_val = if val <= 0x10 { val } else { (val & 0x07) + c };
                        dst_buf[y][x] = new_val;
                        if val != new_val {
                            if changed_count == 0 {
                                sample_before = val;
                                sample_after = new_val;
                            }
                            changed_count += 1;
                        }
                    }
                }
            }
            None => {
                for y in 0..HH {
                    for x in 0..WW {
                        if src[y][x] > 0x10 {
                            if changed_count == 0 {
                                sample_before = src[y][x];
                            }
                            src[y][x] = (src[y][x] & 0x07) + c;
                            if changed_count == 0 {
                                sample_after = src[y][x];
                            }
                            changed_count += 1;
                        }
                    }
                }
            }
        }
    }

    /// 使用两个颜色值对精灵图像进行颜色重映射
    ///
    /// # 参数
    /// * `src` - 源图像缓冲区，如果不提供dst，将直接修改此缓冲区
    /// * `dst` - 可选的目标缓冲区，如果提供则将结果写入此缓冲区
    /// * `c1` - 第一个颜色偏移值，用于低值像素
    /// * `c2` - 第二个颜色偏移值，用于高值像素
    ///
    /// # 实现细节
    /// - 对于像素值 <= 0x10 的部分保持不变
    /// - 对于像素值 > 0x10 的部分:
    ///   1. 取低4位(& 0x0F)作为val
    ///   2. 如果val < 8: 加上c1
    ///   3. 如果val >= 8: 取低3位(& 0x07)后加上c2
    pub fn recolor2<const WW: usize, const HH: usize>(
        &self,
        src: &mut [[u8; WW]; HH],
        dst: Option<&mut [[u8; WW]; HH]>,
        c1: u8,
        c2: u8,
    ) {
        match dst {
            Some(dst_buf) => {
                for y in 0..HH {
                    for x in 0..WW {
                        let val = src[y][x];
                        dst_buf[y][x] = if val <= 0x10 {
                            val
                        } else {
                            let val = val & 0x0F;
                            if val < 8 { val + c1 } else { (val & 0x07) + c2 }
                        };
                    }
                }
            }
            None => {
                for y in 0..HH {
                    for x in 0..WW {
                        if src[y][x] > 0x10 {
                            let val = src[y][x] & 0x0F;
                            src[y][x] = if val < 8 { val + c1 } else { (val & 0x07) + c2 };
                        }
                    }
                }
            }
        }
    }

    /// 替换图像中的指定颜色
    ///
    /// # 参数
    /// * `src` - 源图像缓冲区，如果不提供dst，将直接修改此缓冲区
    /// * `dst` - 可选的目标缓冲区，如果提供则将结果写入此缓冲区
    /// * `n1` - 要替换的颜色值
    /// * `n2` - 替换后的新颜色值
    pub fn replace<const WW: usize, const HH: usize>(
        &self,
        src: &mut [[u8; WW]; HH],
        dst: Option<&mut [[u8; WW]; HH]>,
        n1: u8,
        n2: u8,
    ) {
        match dst {
            Some(dst_buf) => {
                for y in 0..HH {
                    for x in 0..WW {
                        dst_buf[y][x] = if src[y][x] == n1 { n2 } else { src[y][x] };
                    }
                }
            }
            None => {
                for y in 0..HH {
                    for x in 0..WW {
                        if src[y][x] == n1 {
                            src[y][x] = n2;
                        }
                    }
                }
            }
        }
    }

    /// 水平镜像图像
    ///
    /// # 参数
    /// * `src` - 源图像缓冲区，如果不提供dst，将直接修改此缓冲区
    /// * `dst` - 可选的目标缓冲区，如果提供则将结果写入此缓冲区
    /// * `h` - 图像高度
    /// * `w` - 图像宽度
    /// 对src进行原地水平镜像
    pub fn mirror_mut<const WW: usize, const HH: usize>(&self, src: &mut [[u8; WW]; HH]) {
        let temp = src.clone();
        for y in 0..HH {
            for x in 0..WW {
                src[y][x] = temp[y][WW - 1 - x];
            }
        }
    }

    /// 返回一个新的ImageBuffer作为水平镜像结果
    pub fn mirror<const WW: usize, const HH: usize>(&self, src: &[[u8; WW]; HH]) -> [[u8; WW]; HH] {
        let mut dst = [[0u8; WW]; HH];
        for y in 0..HH {
            for x in 0..WW {
                dst[y][x] = src[y][WW - 1 - x];
            }
        }
        dst
    }

    /// 顺时针旋转图像90度
    ///
    /// # 参数
    /// * `src` - 源图像缓冲区，如果不提供dst，将直接修改此缓冲区
    /// * `dst` - 可选的目标缓冲区，如果提供则将结果写入此缓冲区
    /// * `h` - 图像高度
    /// * `w` - 图像宽度
    pub fn rotate<const WW: usize, const HH: usize>(
        &self,
        src: &mut [[u8; WW]; HH],
        dst: Option<&mut [[u8; WW]; HH]>,
    ) {
        // Pascal Rotate: 180度旋转（从最后一个字节开始倒序复制）
        // 汇编代码：从 src[W*H-1] 开始向前读，写入 dst[0] 开始向后写
        // 效果：dest[y][x] = src[H-1-y][W-1-x]
        match dst {
            Some(dst_buf) => {
                for y in 0..HH {
                    for x in 0..WW {
                        dst_buf[y][x] = src[HH - 1 - y][WW - 1 - x];
                    }
                }
            }
            None => {
                let temp = src.clone();
                for y in 0..HH {
                    for x in 0..WW {
                        src[y][x] = temp[HH - 1 - y][WW - 1 - x];
                    }
                }
            }
        }
    }

    /// 设置天空类型
    pub fn init_sky(&mut self, new_sky: u8) {
        self.sky = new_sky;
    }

    /// 初始化管道的颜色
    ///
    /// # 参数
    /// * `new_color` - 新的颜色偏移值，将应用于所有管道图像
    /// * `sprites` - 精灵数据管理器，包含所有管道精灵
    ///
    /// # 实现细节
    /// - 使用 recolor 函数处理所有管道图像
    /// - 处理 PIPE_000 到 PIPE_003 共四个管道精灵
    /// - 直接修改源图像数据，不创建新的缓冲区
    ///
    /// # 对应Pascal代码
    /// ```pascal
    /// procedure InitPipes (NewColor: Byte);
    /// begin
    ///   ReColor (@Pipe000, @Pipe000, NewColor);
    ///   ReColor (@Pipe001, @Pipe001, NewColor);
    ///   ReColor (@Pipe002, @Pipe002, NewColor);
    ///   ReColor (@Pipe003, @Pipe003, NewColor);
    /// end;
    /// ```
    pub fn init_pipes(&self, new_color: u8, sprites: &mut SpriteDataManager) {
        self.recolor(&mut sprites.PIPE_000, None, new_color);
        self.recolor(&mut sprites.PIPE_001, None, new_color);
        self.recolor(&mut sprites.PIPE_002, None, new_color);
        self.recolor(&mut sprites.PIPE_003, None, new_color);
    }

    pub fn init_wall(
        &mut self,
        n: usize,
        wall_type: u8,
        sprites: &SpriteDataManager,
        options: &WorldOptions,
    ) {
        // 索引调整：Pascal的1..N转换为Rust的0..N-1
        let n = n - 1;

        // 根据墙面类型选择和复制源图像
        let base_images = match wall_type {
            0 => [
                sprites.GREEN_000.clone(),
                sprites.GREEN_001.clone(),
                sprites.GREEN_002.clone(),
                sprites.GREEN_003.clone(),
                sprites.GREEN_004.clone(),
            ],
            1 => [
                sprites.SAND_000.clone(),
                sprites.SAND_001.clone(),
                sprites.SAND_002.clone(),
                sprites.SAND_003.clone(),
                sprites.SAND_004.clone(),
            ],
            2 => {
                // 使用地面颜色进行重映射
                let mut images = [
                    sprites.GREEN_000.clone(),
                    sprites.GREEN_001.clone(),
                    sprites.GREEN_002.clone(),
                    sprites.GREEN_003.clone(),
                    sprites.GREEN_004.clone(),
                ];
                // 应用颜色重映射
                for img in images.iter_mut() {
                    self.recolor2(img, None, options.ground_color1, options.ground_color2);
                }
                images
            }
            3 => [
                sprites.BROWN_000.clone(),
                sprites.BROWN_001.clone(),
                sprites.BROWN_002.clone(),
                sprites.BROWN_003.clone(),
                sprites.BROWN_004.clone(),
            ],
            4 => [
                sprites.GRASS_000.clone(),
                sprites.GRASS_001.clone(),
                sprites.GRASS_002.clone(),
                sprites.GRASS_003.clone(),
                sprites.GRASS_004.clone(),
            ],
            5 => [
                sprites.DES_000.clone(),
                sprites.DES_001.clone(),
                sprites.DES_002.clone(),
                sprites.DES_003.clone(),
                sprites.DES_004.clone(),
            ],
            _ => return,
        };

        // 复制基础图像到对应位置（严格对齐 Pascal FigList[N, 1..13] 的索引）
        // Pascal (wall_type=0 示例):
        //   FigList[N,1]  = Green000
        //   FigList[N,2]  = Green001
        //   FigList[N,4]  = Green002
        //   FigList[N,5]  = Green003
        //   FigList[N,10] = Green004
        // Rust 数组是 0-based，但我们保留 index 0 作为未使用占位，直接让 1..13 与 Pascal 对齐。
        // 为避免遗留默认值（例如 main.rs 初始化的 BROWN_000）泄漏，这里也把 [0] 填充为 base_images[0]。
        self.fig_list[n][0] = base_images[0].clone(); // unused slot (safety)
        self.fig_list[n][1] = base_images[0].clone(); // Pascal 1
        self.fig_list[n][2] = base_images[1].clone(); // Pascal 2
        self.fig_list[n][4] = base_images[2].clone(); // Pascal 4
        self.fig_list[n][5] = base_images[3].clone(); // Pascal 5
        self.fig_list[n][10] = base_images[4].clone(); // Pascal 10

        // 生成镜像和旋转的图像（索引同 Pascal）
        // Mirror FigList[N,1] -> FigList[N,3]
        let mut temp = self.fig_list[n][1].clone();
        self.mirror_mut(&mut temp);
        self.fig_list[n][3] = temp;

        // Rotate FigList[N,4] -> FigList[N,6]
        let mut temp = self.fig_list[n][4].clone();
        self.rotate(&mut temp, None);
        self.fig_list[n][6] = temp;

        // Rotate FigList[N,1] -> FigList[N,9]
        let mut temp = self.fig_list[n][1].clone();
        self.rotate(&mut temp, None);
        self.fig_list[n][9] = temp;

        // Rotate FigList[N,2] -> FigList[N,8]
        let mut temp = self.fig_list[n][2].clone();
        self.rotate(&mut temp, None);
        self.fig_list[n][8] = temp;

        // Rotate FigList[N,3] -> FigList[N,7]
        let mut temp = self.fig_list[n][3].clone();
        self.rotate(&mut temp, None);
        self.fig_list[n][7] = temp;

        // 处理最后三个图像
        // Mirror FigList[N,10] -> FigList[N,11]
        let mut temp = self.fig_list[n][10].clone();
        self.mirror_mut(&mut temp);
        self.fig_list[n][11] = temp;

        // Rotate FigList[N,11] -> FigList[N,12]
        let mut temp = self.fig_list[n][11].clone();
        self.rotate(&mut temp, None);
        self.fig_list[n][12] = temp;

        // Mirror FigList[N,12] -> FigList[N,13]
        let mut temp = self.fig_list[n][12].clone();
        self.mirror_mut(&mut temp);
        self.fig_list[n][13] = temp;
    }

    /// Rust版 InitWalls，对应Pascal InitWalls (W1, W2, W3)
    pub fn init_walls(
        &mut self,
        w1: u8,
        w2: u8,
        w3: u8,
        sprites: &SpriteDataManager,
        options: &WorldOptions,
    ) {
        self.init_wall(1, w1, sprites, options);
        self.init_wall(2, w2, sprites, options);
        self.init_wall(3, w3, sprites, options);
    }

    /// 设置天空调色板（完整移植自Pascal SetSkyPalette）
    /// 需要传入 Palette 和 WorldOptions
    pub fn set_sky_palette(&self, palette: &mut Palettes, options: &WorldOptions) {
        // 修复：直接使用 options.sky_type 确保使用正确的天空类型
        // 之前使用 self.sky 可能导致在某些场景下使用旧的 sky 值
        let sky = options.sky_type;
        match sky {
            0 => {
                palette.change_palette(0xE0, 35, 45, 63);
                palette.change_palette(0xF0, 20, 38, 48);
                palette.change_palette(0xFF, 54, 57, 60);
            }
            1 => {
                palette.change_palette(0xE0, 52, 55, 55);
                palette.change_palette(0xF0, 42, 48, 45);
                palette.change_palette(0xFF, 61, 61, 61);
            }
            2 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 48 - 2 * j, 58 - j, 58);
                }
                palette.change_palette(0xF0, 35, 48, 46);
            }
            3 => {
                palette.change_palette(0xE0, 0, 5, 3);
                palette.change_palette(0xF0, 8, 12, 10);
                palette.change_palette(0xFF, 8, 13, 13);
            }
            4 => {
                palette.change_palette(0xE0, 35, 45, 53);
                palette.change_palette(0xF0, 23, 39, 43);
                palette.change_palette(0xFF, 58, 60, 60);
            }
            5 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 58 - (j / 2), 56 - j, 38 - j);
                }
                palette.change_palette(0xF0, 52, 49, 32);
            }
            6 => {
                if options.backgr_type == 4 {
                    for i in 0xE0..=0xEF {
                        palette.change_palette(i as usize, 22, 15, 11);
                    }
                    palette.change_palette(0xFD, 22, 15, 11);
                    palette.change_palette(0xFE, 19, 12, 8);
                    palette.change_palette(0xFF, 25, 18, 14);
                } else {
                    for i in 0xE0..=0xFF {
                        palette.change_palette(i as usize, 19, 9, 8);
                    }
                    palette.change_palette(0xD1, 19, 9, 8);
                    palette.change_palette(0xD6, 21, 11, 10);
                    palette.change_palette(0xD4, 17, 7, 6);
                }
            }
            7 => {
                if options.backgr_type == 4 {
                    for i in 0xE0..=0xEF {
                        palette.change_palette(i as usize, 18, 18, 22);
                    }
                    palette.change_palette(0xFD, 18, 18, 22);
                    palette.change_palette(0xFF, 23, 23, 27);
                    palette.change_palette(0xFE, 13, 13, 17);
                } else {
                    for i in 0xE0..=0xFF {
                        palette.change_palette(i as usize, 15, 15, 18);
                    }
                    palette.change_palette(0xD1, 15, 15, 18);
                    palette.change_palette(0xD4, 18, 18, 21);
                    palette.change_palette(0xD6, 12, 12, 15);
                }
            }
            8 => {
                // Pascal FIGURES.PAS sky=8:
                //   if BackGrType=4 then 设置 $E0..$EF 以及 $FD/$FE/$FF
                //   else 设置 $E0..$FF 以及 $D1/$D4/$D6
                if options.backgr_type == 4 {
                    for i in 0xE0..=0xEF {
                        palette.change_palette(i as usize, 17, 10, 10);
                    }
                    palette.change_palette(0xFD, 17, 10, 10);
                    palette.change_palette(0xFE, 11, 5, 5);
                    palette.change_palette(0xFF, 20, 14, 14);
                } else {
                    for i in 0xE0..=0xFF {
                        palette.change_palette(i as usize, 15, 5, 5);
                    }
                    palette.change_palette(0xD1, 15, 5, 5);
                    palette.change_palette(0xD4, 20, 10, 10);
                    palette.change_palette(0xD6, 10, 0, 0);
                }
            }
            9 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 63 - (j / 3), 50 - j, 25 - j);
                }
                palette.change_palette(0xF0, 48, 35, 18);
            }
            10 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 27 - j, 43 - j, 63 - j);
                }
                palette.change_palette(0xF0, 58, 58, 63);
            }
            11 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 60 - j, 63 - j, 63 - j);
                }
                palette.change_palette(0xF0, 42, 48, 45);
            }
            12 => {
                for i in 0xE0..=0xEF {
                    let j = i - 0xE0;
                    palette.change_palette(i as usize, 55 - j, 63 - j, 63);
                }
                palette.change_palette(0xF0, 30, 50, 58);
                palette.change_palette(0xF0, 36, 45, 41);
            }
            _ => {}
        }
    }

    /// 绘制天空/背景底色（对齐 Pascal FIGURES.PAS::DrawSky）
    ///
    /// 依赖：
    /// - 填充像素：`vga.fill_world`
    /// - 平滑渐变/砖块等：`backgr`（对应 Pascal BACKGR.PAS 的 SmoothFill/DrawBricks/...）
    pub fn draw_sky(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        vga: &mut VGA,
        options: &WorldOptions,
        backgr: &mut BackGr,
        sprites: &SpriteDataManager,
    ) {
        // Pascal FIGURES.PAS::DrawSky:
        //   if Options.BackGrType = 0 then Fill(..., $E0)
        if options.backgr_type == 0 {
            vga.fill_world(x, y, w, h, 0xE0);
            return;
        }

        match self.sky {
            // Pascal: 0,1,3,4 -> 以 Horizon 分割填充 $E0/$F0
            0 | 1 | 3 | 4 => {
                let horizon = options.horizon as i32;
                let top_h = horizon - y;

                if horizon < y {
                    vga.fill_world(x, y, w, h, 0xF0);
                } else if horizon > y + h - 1 {
                    vga.fill_world(x, y, w, h, 0xE0);
                } else {
                    vga.fill_world(x, y, w, top_h, 0xE0);
                    vga.fill_world(x, horizon, w, h - top_h, 0xF0);
                }
            }

            // Pascal: 2,5,9,10,11,12 -> SmoothFill
            2 | 5 | 9 | 10 | 11 | 12 => {
                backgr.smooth_fill(x as usize, y as usize, w as usize, h as usize, options, vga);
            }

            // Pascal: 6,7,8 -> 根据 BackGrType 画砖/柱子/窗/大砖
            6 | 7 | 8 => {
                // println!("[DRAW_SKY] 地下室场景 sky={}, backgr_type={}, 区域: x={}, y={}, w={}, h={}",
                //            self.sky, options.backgr_type, x, y, w, h);
                match options.backgr_type {
                    4 => backgr.draw_bricks(x, y, w, h, vga, sprites),
                    5 => backgr.large_bricks(x, y, w, h, vga),
                    6 => backgr.pillar(x, y, w, h, vga, sprites),
                    7 => backgr.windows(x, y, w, h, vga),
                    _ => {}
                }
            }

            _ => {}
        }
    }

    /// Rust 严格移植自 Pascal Redraw 过程（变量、分支、流程与Pascal一致）
    pub fn redraw(
        &self,
        x: i32,
        y: i32,
        world_map: &WorldBuffer,
        vga: &mut VGA,
        backgr: &mut BackGr,
        sprites: &mut SpriteDataManager,
        options: &WorldOptions,
        buffers: &Buffers,
    ) {
        // xpos/ypos 为“世界坐标像素”，最终写入 VGA 时必须减去 XView/YView。
        let xpos = x * crate::buffers::W as i32;
        let ypos = y * crate::buffers::H as i32;
        // Pascal: WorldMap^[X,Y] 的有效索引范围包含负数（X:-EX.., Y:-EY1..），通过“内存偏移”实现。
        // Rust 用 Vec 存储时必须显式加上偏移，否则会读到错误位置（通常是0/空格），导致0xF0/0xF7等装饰tile永远不会被处理。
        let get = |x: i32, y: i32| -> u8 {
            let xx = x + EX;
            let yy = y + EY1;
            if xx < 0
                || yy < 0
                || (xx as usize) >= world_map.len()
                || (yy as usize) >= world_map[0].len()
            {
                0
            } else {
                world_map[xx as usize][yy as usize]
            }
        };
        let mut ch = get(x, y);
        let mut fig = None;
        let mut fig_name: Option<&'static str> = None;
        let mut l: bool;
        let mut r: bool;
        let mut ls: bool;
        let mut rs: bool;

        // 调试日志：跟踪 (0,0) 位置的绘制过程
        // if x == 0 && y == 0 {
        //     println!("[TILE_0_0] redraw() called: ch={:#04X} '{}'", ch, if ch >= 32 && ch < 127 { ch as char } else { '?' });
        //     println!("[TILE_0_0] world coords: x={}, y={} => pixel pos: xpos={}, ypos={}", x, y, xpos, ypos);
        //     println!("[TILE_0_0] options: design={}, backgr_type={}, wall_type1={}", options.design, options.backgr_type, options.wall_type1);
        // }

        if x >= 0 && y >= 0 && y < crate::buffers::NV {
            // 背景
            // 注意：Pascal Redraw 每次都会先 DrawSky 做底色（哪怕是地下室）。
            // 之前这里为了避免覆盖精灵做了 skip_sky，会导致地下室初帧“底色没铺”，看起来像黑屏，
            // 直到滚屏触发其它重绘路径后才恢复。
            // 为对齐 Pascal 行为，这里不再跳过。
            if ch != 0 {
                if ch == b'%' && options.design == 4 {
                    self.draw_sky(
                        xpos,
                        ypos,
                        crate::buffers::W as i32,
                        crate::buffers::H as i32 / 2,
                        vga,
                        options,
                        backgr,
                        sprites,
                    );
                } else {
                    self.draw_sky(
                        xpos,
                        ypos,
                        crate::buffers::W as i32,
                        crate::buffers::H as i32,
                        vga,
                        options,
                        backgr,
                        sprites,
                    );
                }
            }
            if ch == b' ' {
                // if x == 0 && y == 0 {
                //     println!("[TILE_0_0] EARLY RETURN: character is SPACE, skipping drawing");
                // }
                return;
            }
            if get(x, y - 1) == 18 {
                fig = Some(&self.fig_list[0][5]);
                fig_name = Some("FIG_LIST[0][5] (special above==18 overlay)");
                self.trace_sprite(x, y, fig_name.unwrap());
                vga.put_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
            }
            fig = None;
            fig_name = None;
            match ch {
                1..=26 => {
                    // Pascal 严格对齐：
                    // Pascal: if Ch > #13 then Ch := Chr(Ord(Ch) - 13)
                    // 后续都使用 FigList[1, ...] (Pascal 1-based = Rust 0-based)
                    let mut ch_modified = ch;
                    
                    // Pascal: if Ch > #13 then Ch := Chr (Ord (Ch) - 13)
                    if ch_modified > 13 {
                        ch_modified = ch_modified - 13;
                    } else {
                        // Pascal: else if WorldMap^ [X - 1, Y] in [#14..#26] then ...
                        let left = get(x - 1, y);
                        if (14..=26).contains(&left) {
                            if [1, 4, 7].contains(&ch_modified) {
                                // Pascal: Fig := @FigList [1, Ord (WorldMap^ [X - 1, Y]) - 13];
                                // Pascal FigList[1, x] = Rust fig_list[0][x]
                                fig = Some(&self.fig_list[0][(left - 13) as usize]);
                                fig_name = Some("FIG_LIST[0, left-13] overlay");
                                self.trace_sprite(x, y, fig_name.unwrap());
                                vga.put_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                            }
                        } else {
                            // Pascal: else if WorldMap^ [X + 1, Y] in [#14..#26] then ...
                            let right = get(x + 1, y);
                            if (14..=26).contains(&right) && [3, 6, 9].contains(&ch_modified) {
                                // Pascal: Fig := @FigList [1, Ord (WorldMap^ [X + 1, Y]) - 13];
                                // Pascal FigList[1, x] = Rust fig_list[0][x]
                                fig = Some(&self.fig_list[0][(right - 13) as usize]);
                                fig_name = Some("FIG_LIST[0, right-13] overlay");
                                self.trace_sprite(x, y, fig_name.unwrap());
                                vga.put_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                            }
                        }
                    }

                    // Pascal: Fig := @FigList [1, Ord (Ch)];
                    // Pascal FigList[1, x] = Rust fig_list[0][x]
                    fig = Some(&self.fig_list[0][ch_modified as usize]);

                    // Pascal: if not (Ch in [#1, #3, #4, #6, #7, #9]) then
                    if ![1, 3, 4, 6, 7, 9].contains(&ch_modified) {
                        fig_name = Some("FIG_LIST[0, ch_modified]");
                        self.trace_sprite(x, y, fig_name.unwrap());
                        vga.put_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                        fig = None;
                        fig_name = None;
                    }
                }
                b'?' => {
                    fig = Some(&sprites.QUEST_000);
                    fig_name = Some("QUEST_000");
                }
                b'@' => {
                    fig = Some(&sprites.QUEST_001);
                    fig_name = Some("QUEST_001");
                }
                b'A' => {
                    l = get(x - 1, y) == b'A';
                    r = get(x + 1, y) == b'A';
                    if (x + y) % 2 == 1 {
                        rs = true;
                        ls = false;
                    } else {
                        ls = true;
                        rs = false;
                    }

                    // 砖块精灵来自 assets/sprites 目录:
                    // wall_type1==100 -> BRICK0_000/001/002
                    // wall_type1==101 -> BRICK1_000/001/002
                    // wall_type1==102 -> BRICK2_000/001/002
                    // (在 build_world 中重新着色为 self.bricks[0..2])
                    if ls && r {
                        fig = Some(&self.bricks[1]);
                        fig_name = Some(match options.wall_type1 {
                            100 => "BRICK0_001 (A stitch)",
                            101 => "BRICK1_001 (A stitch)",
                            102 => "BRICK2_001 (A stitch)",
                            _ => "BRICK?_001 (A stitch)",
                        });
                    } else if rs && l {
                        fig = Some(&self.bricks[2]);
                        fig_name = Some(match options.wall_type1 {
                            100 => "BRICK0_002 (A stitch)",
                            101 => "BRICK1_002 (A stitch)",
                            102 => "BRICK2_002 (A stitch)",
                            _ => "BRICK?_002 (A stitch)",
                        });
                    } else {
                        fig = Some(&self.bricks[0]);
                        fig_name = Some(match options.wall_type1 {
                            100 => "BRICK0_000 (A)",
                            101 => "BRICK1_000 (A)",
                            102 => "BRICK2_000 (A)",
                            _ => "BRICK?_000 (A)",
                        });
                    }
                }
                b'I' => {
                    fig = Some(&sprites.BLOCK_000);
                    fig_name = Some("BLOCK_000");
                }
                b'J' => {
                    fig = Some(&sprites.BLOCK_001);
                    fig_name = Some("BLOCK_001");
                }
                b'K' => {
                    fig = Some(&sprites.NOTE_000);
                    fig_name = Some("NOTE_000");
                }
                b'X' => {
                    fig = Some(&sprites.XBLOCK_000);
                    fig_name = Some("XBLOCK_000");
                }
                b'W' => {
                    fig = Some(&sprites.WOOD_000);
                    fig_name = Some("WOOD_000");
                }
                b'=' => {
                    fig = Some(&sprites.PIN_000);
                    fig_name = Some("PIN_000 (draw/upside down)");
                    self.trace_sprite(x, y, fig_name.unwrap());
                    if CAN_HOLD_YOU.contains(&get(x, y + 1)) {
                        vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                    } else {
                        vga.up_side_down_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                    }
                    fig = None;
                    fig_name = None;
                }
                b'0' => {
                    fig = Some(&sprites.PIPE_000);
                    fig_name = Some("PIPE_000");
                }
                b'1' => {
                    fig = Some(&sprites.PIPE_001);
                    fig_name = Some("PIPE_001");
                }
                b'2' => {
                    fig = Some(&sprites.PIPE_002);
                    fig_name = Some("PIPE_002");
                }
                b'3' => {
                    fig = Some(&sprites.PIPE_003);
                    fig_name = Some("PIPE_003");
                }
                b'*' => {
                    fig = Some(&sprites.COIN_000);
                    fig_name = Some("COIN_000");
                }
                0xFE => {
                    if get(x, y - 1) == 0xFE {
                        fig = Some(&sprites.EXIT_001);
                        fig_name = Some("EXIT_001");
                    } else {
                        fig = Some(&sprites.EXIT_000);
                        fig_name = Some("EXIT_000");
                    }
                }
                0xF7 => {
                    // 严格对齐 Pascal FIGURES.PAS 的草地渲染逻辑
                    // 关键：草地精灵是透明的（使用DrawImage），透明像素会显示下方已绘制的内容
                    // 因此如果草地周围有墙体，需要先绘制墙体作为背景，然后草地覆盖在上面
                    
                    // 0 检查周围是否有墙体（#1..#26），如果有则先绘制墙体背景
                    // 这样草地的透明部分会显示墙体颜色而不是天空颜色
                    let left = get(x - 1, y);
                    let right = get(x + 1, y);
                    
                    // 如果左边或右边有墙体块（C/D被build_wall转换成的#1..#26），先绘制墙体
                    if (1..=26).contains(&left) || (1..=26).contains(&right) {
                        // 关键：草地背景应该使用无边缘的中央墙块（编号5，对应GREEN.003）
                        // 而不是周围墙体的实际编号（可能是带边缘的墙块）
                        // Pascal: 当A+B+L+R=0（四周都被墙体包围）时，WorldMap^[X,Y]:=Chr(5+N)
                        // 对于C/D墙体（N=13），无边缘墙块是 5（对应fig_list[0][5]=GREEN.003）
                        let wall_fig = &self.fig_list[0][5];  // 固定使用编号5（GREEN.003无边缘墙块）
                        self.trace_sprite(x, y, "WALL_BACKGROUND (GREEN.003 for grass)");
                        vga.put_image_imagebuffer_world(xpos, ypos, wall_fig);
                    }
                    
                    // 1 如果上方是树干并且设计为 2 则先叠加一层 SmTree001
                    if get(x, y - 1) == 0xF0 && options.design == 2 {
                        fig = Some(&sprites.SMTREE_001);
                        fig_name = Some("SMTREE_001 (overlay on grass)");
                        self.trace_sprite(x, y, fig_name.unwrap());
                        vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                    }
                    // 2 如果上方是棕榈树干并且设计为 1 则叠加一层 WPalm000
                    if get(x, y - 1) == 0xF6 && options.design == 1 {
                        fig = Some(&sprites.WPALM_000);
                        fig_name = Some("WPALM_000 (overlay on grass)");
                        self.trace_sprite(x, y, fig_name.unwrap());
                        vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                    }
                    
                    // 3 根据左右邻居选择 Grass1 Grass2 Grass3 以实现边缘拼接
                    if x == 0 || get(x - 1, y) == ch {
                        if get(x + 1, y) == ch {
                            fig = Some(&sprites.GRASS2_000);
                            fig_name = Some("GRASS2_000");
                        } else {
                            fig = Some(&sprites.GRASS3_000);
                            fig_name = Some("GRASS3_000");
                        }
                    } else if get(x + 1, y) == ch {
                        fig = Some(&sprites.GRASS1_000);
                        fig_name = Some("GRASS1_000");
                    } else {
                        fig = Some(&sprites.GRASS3_000);
                        fig_name = Some("GRASS3_000");
                    }
                }
                0xF0 => match options.design {
                    1 => {
                        if get(x, y - 1) != ch {
                            fig = Some(&sprites.FENCE_001);
                            fig_name = Some("FENCE_001");
                        } else {
                            fig = Some(&sprites.FENCE_000);
                            fig_name = Some("FENCE_000");
                        }
                    }
                    2 => {
                        if get(x, y - 1) != ch {
                            fig = Some(&sprites.SMTREE_000);
                            fig_name = Some("SMTREE_000");
                        } else {
                            fig = Some(&sprites.SMTREE_001);
                            fig_name = Some("SMTREE_001");
                        }
                        // 关键调试：树干 tile 理论上不应出现大量 0（否则会被 DrawImage 当成透明造成“撕裂”）
                        // 这里只打印你反馈的 Intro 坐标附近，避免刷屏。
                        if self.trace_enabled
                            && matches!((x, y), (3, 9) | (4, 9) | (11, 9) | (12, 9))
                        {
                            let buf = fig.unwrap();
                            let mut zeros = 0usize;
                            for yy in 0..crate::buffers::H as usize {
                                for xx in 0..crate::buffers::W as usize {
                                    if buf[yy][xx] == 0 {
                                        zeros += 1;
                                    }
                                }
                            }
                            let _ = zeros; // 保留变量避免警告
                        }
                    }
                    // 处理Level_1b地下室的装饰字符
                    // 这些字符在Level_1b地图中用于地下室的装饰物
                    // 在Pascal中，这些字符可能直接作为背景图形渲染，而不是精灵
                    0xE8 | 0xE0 | 0xE1 => {
                        // 使用地下室地板装饰 - 与地下室的背景类型4匹配
                        // 当backgr_type == 4时，figures.rs会设置0xE0-0xFF范围的调色板颜色
                        fig = Some(&sprites.BRICK2_000);
                        fig_name = Some("BASEMENT_DECOR");
                    }
                    _ => {}
                },
                0xF6 => {
                    if options.design == 1 {
                        fig = Some(&sprites.WPALM_000);
                        fig_name = Some("WPALM_000");
                    }
                }
                0xFA => {
                    if options.design == 1 {
                        if get(x - 1, y) == 0xF9 {
                            fig = Some(&sprites.PALM3_000);
                            fig_name = Some("PALM3_000 (overlay)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                        } else if get(x + 1, y) == 0xF9 {
                            fig = Some(&sprites.PALM1_000);
                            fig_name = Some("PALM1_000 (overlay)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                        }
                        fig = Some(&sprites.PALM0_000);
                        fig_name = Some("PALM0_000");
                    }
                }
                0xF4 => {
                    if options.design == 1 {
                        if get(x, y + 1) == 0xF6 {
                            fig = Some(&sprites.WPALM_000);
                            fig_name = Some("WPALM_000 (overlay)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                        }
                        fig = Some(&sprites.PALM1_000);
                        fig_name = Some("PALM1_000");
                    }
                }
                0xF9 => {
                    if options.design == 1 {
                        fig = Some(&sprites.PALM2_000);
                        fig_name = Some("PALM2_000");
                    }
                }
                0xF5 => {
                    if options.design == 1 {
                        if get(x, y + 1) == 0xF6 {
                            fig = Some(&sprites.WPALM_000);
                            fig_name = Some("WPALM_000 (overlay)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.draw_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                        }
                        fig = Some(&sprites.PALM3_000);
                        fig_name = Some("PALM3_000");
                    }
                }
                b'#' => match options.design {
                    1 => {
                        fig = Some(&sprites.FALL_000);
                        fig_name = Some("FALL_000");
                    }
                    2 => match get(x, y - 1) {
                        b'#' => {
                            self.trace_sprite(x, y, "TREE_001 (put)");
                            vga.put_image_imagebuffer_world(xpos, ypos, &sprites.TREE_001)
                        }
                        b'%' => {
                            fig = Some(&sprites.TREE_000);
                            fig_name = Some("TREE_000 (put)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.put_image_imagebuffer_world(xpos, ypos, fig.as_ref().unwrap());
                            fig = Some(&sprites.TREE_003);
                            fig_name = Some("TREE_003");
                        }
                        _ => {
                            fig = Some(&sprites.TREE_003);
                            fig_name = Some("TREE_003");
                        }
                    },
                    3 => {
                        fig = Some(&sprites.WINDOW_001);
                        fig_name = Some("WINDOW_001");
                    }
                    4 => {
                        fig = Some(&sprites.LAVA_000);
                        fig_name = Some("LAVA_000");
                    }
                    5 => {
                        vga.fill_world(
                            xpos,
                            ypos,
                            crate::buffers::W as i32,
                            crate::buffers::H as i32,
                            5,
                        );
                    }
                    // 处理Level_1b地下室的装饰字符
                    // 这些字符在Level_1b地图中用于地下室的装饰物
                    // 在Pascal中，这些字符可能直接作为背景图形渲染，而不是精灵
                    0xE8 | 0xE0 | 0xE1 => {
                        // 使用地下室地板装饰 - 与地下室的背景类型4匹配
                        // 当backgr_type == 4时，figures.rs会设置0xE0-0xFF范围的调色板颜色
                        fig = Some(&sprites.BRICK2_000);
                        fig_name = Some("BASEMENT_DECOR");
                    }
                    _ => {}
                },
                b'%' => match options.design {
                    1 => {
                        fig = Some(&sprites.FALL_001);
                        fig_name = Some("FALL_001");
                    }
                    2 => match get(x, y - 1) {
                        b'%' => {
                            self.trace_sprite(x, y, "TREE_000 (put)");
                            vga.put_image_imagebuffer_world(xpos, ypos, &sprites.TREE_000)
                        }
                        b'#' => {
                            fig = Some(&sprites.TREE_001);
                            fig_name = Some("TREE_001 (put)");
                            self.trace_sprite(x, y, fig_name.unwrap());
                            vga.put_image_imagebuffer_world(xpos, ypos, fig.unwrap());
                            fig = Some(&sprites.TREE_002);
                            fig_name = Some("TREE_002");
                        }
                        _ => {
                            fig = Some(&sprites.TREE_002);
                            fig_name = Some("TREE_002");
                        }
                    },
                    3 => {
                        fig = Some(&sprites.WINDOW_000);
                        fig_name = Some("WINDOW_000");
                    }
                    4 => {
                        fig = Some(&sprites.LAVA_001);
                        fig_name = Some("LAVA_001");
                    }
                    5 => {
                        let idx = ((x + (buffers.lava_counter as i32 / 8)) % 5) as u8;
                        fig = Some(match idx {
                            0 => {
                                fig_name = Some("LAVA2_001");
                                &sprites.LAVA2_001
                            }
                            1 => {
                                fig_name = Some("LAVA2_002");
                                &sprites.LAVA2_002
                            }
                            2 => {
                                fig_name = Some("LAVA2_003");
                                &sprites.LAVA2_003
                            }
                            3 => {
                                fig_name = Some("LAVA2_004");
                                &sprites.LAVA2_004
                            }
                            4 => {
                                fig_name = Some("LAVA2_005");
                                &sprites.LAVA2_005
                            }
                            _ => {
                                fig_name = Some("LAVA2_001");
                                &sprites.LAVA2_001
                            }
                        });
                    }
                    // 处理Level_1b地下室的装饰字符
                    // 这些字符在Level_1b地图中用于地下室的装饰物
                    // 在Pascal中，这些字符可能直接作为背景图形渲染，而不是精灵
                    0xE8 | 0xE0 | 0xE1 => {
                        // 使用地下室地板装饰 - 与地下室的背景类型4匹配
                        // 当backgr_type == 4时，figures.rs会设置0xE0-0xFF范围的调色板颜色
                        fig = Some(&sprites.BRICK2_000);
                        fig_name = Some("BASEMENT_DECOR");
                    }
                    _ => {}
                },
                _ => {}
            }
            if let Some(f) = fig {
                if let Some(name) = fig_name {
                    self.trace_sprite(x, y, name);
                } else {
                    self.trace_sprite(x, y, "UNKNOWN");
                }
                
                // Pascal: if Fig <> Nil then DrawImage(...)
                vga.draw_image_imagebuffer_world(xpos, ypos, f);
            }
        }
    }

    /// Rust 严格移植自 Pascal BuildWall 过程（变量、流程、分支与Pascal一致）
    ///
    /// # 参数
    /// * `i` - X 坐标
    /// * `j` - Y 坐标
    /// * `world_map` - 世界地图缓冲区（可变）
    /// * `options` - 世界参数
    /// * `nv` - NV 常量
    /// * `ab`/`cd`/`last_ab`/`last_cd` - 砖块类型状态
    fn build_wall(
        &self,
        i: usize,
        j: usize,
        world_map: &mut WorldBuffer,
        options: &WorldOptions,
        nv: usize,
        ab: &mut u8,
        cd: &mut u8,
        last_ab: u8,
        _last_cd: u8,
    ) {
        // 重要：Pascal 的 WorldMap 支持负索引（X:-EX.., Y:-EY1..），通过指针偏移实现。
        // Rust 用二维 Vec 存储时必须显式加上偏移。
        // BuildWall/BuildWorld 的 X/Y 坐标系是“地图坐标”(0..x_size-1, 0..NV-1)，
        // 所以读写 world_map 时都要 +EX/+EY1。
        let xx = (i as i32 + EX) as usize;
        let yy = (j as i32 + EY1) as usize;
        let c = world_map[xx][yy];
        let mut n = 0u8;
        let mut ch = std::collections::HashSet::new();
        let mut ch_left = std::collections::HashSet::new();
        match c {
            b'A' | b'B' => {
                *ab = c;
                ch.insert(c);
                for v in 1u8..=13u8 {
                    ch.insert(v);
                }
                if last_ab != c {
                    for v in &ch {
                        if ![3, 6, 9].contains(v) {
                            ch_left.insert(*v);
                        }
                    }
                } else {
                    ch_left = ch.clone();
                }
                n = 0;
            }
            b'C' | b'D' => {
                *cd = c;
                ch.insert(c);
                for v in 1u8..=26u8 {
                    ch.insert(v);
                }
                ch.insert(b'A');
                ch.insert(b'B');
                ch.insert(0xE9);
                // 关键：添加草地 0xF7，这样墙体在草地边界不会形成边缘
                // 草地被视为"与墙体连接的地面"，而不是"空气/边缘"
                ch.insert(0xF7);
                ch_left = ch.clone();
                n = 13;
            }
            _ => return,
        }
        let ignore_above = [0xE9];  // Pascal: IgnoreAbove = ['é']
        // A := 1 - Byte ((WorldMap^ [X, Y - 1] in (Ch - IgnoreAbove)) or (Y = 0));
        let a = 1
            - (((j > 0) && {
                let y1 = (j as i32 - 1 + EY1) as usize;
                ch.contains(&world_map[xx][y1]) && !ignore_above.contains(&world_map[xx][y1])
            }) as u8
                | (j == 0) as u8);
        // B := 2 * Byte (Not ((Y = NV - 1) or (WorldMap^ [X, Y + 1] in Ch)));
        let b = 2
            * (!((j == nv - 1) || {
                let y2 = (j as i32 + 1 + EY1) as usize;
                ch.contains(&world_map[xx][y2])
            }) as u8);
        // L := 4 * Byte (Not ((X = 0) or (WorldMap^ [X - 1, Y] in ChLeft)));
        let l = 4
            * (!((i == 0) || {
                let x1 = (i as i32 - 1 + EX) as usize;
                ch_left.contains(&world_map[x1][yy])
            }) as u8);
        // R := 8 * Byte (Not ((X = Options.XSize - 1) or (WorldMap^ [X + 1, Y] in Ch)));
        let r = 8
            * (!((i == options.x_size as usize - 1) || {
                let x2 = (i as i32 + 1 + EX) as usize;
                ch.contains(&world_map[x2][yy])
            }) as u8);
        let sum = a + b + l + r;
        match sum {
            0 => {
                if i > 0
                    && j > 0
                    && !{
                        let x1 = (i as i32 - 1 + EX) as usize;
                        let y1 = (j as i32 - 1 + EY1) as usize;
                        ch.contains(&world_map[x1][y1])
                    }
                {
                    world_map[xx][yy] = 10 + n;
                    return;
                }
                if i < options.x_size as usize - 1
                    && j > 0
                    && !{
                        let x2 = (i as i32 + 1 + EX) as usize;
                        let y1 = (j as i32 - 1 + EY1) as usize;
                        ch.contains(&world_map[x2][y1])
                    }
                {
                    world_map[xx][yy] = 11 + n;
                    return;
                }
                if i > 0
                    && j < nv - 1
                    && !{
                        let x1 = (i as i32 - 1 + EX) as usize;
                        let y2 = (j as i32 + 1 + EY1) as usize;
                        ch.contains(&world_map[x1][y2])
                    }
                {
                    world_map[xx][yy] = 12 + n;
                    return;
                }
                if i < options.x_size as usize - 1
                    && j < nv - 1
                    && !{
                        let x2 = (i as i32 + 1 + EX) as usize;
                        let y2 = (j as i32 + 1 + EY1) as usize;
                        ch.contains(&world_map[x2][y2])
                    }
                {
                    world_map[xx][yy] = 13 + n;
                    return;
                }
                world_map[xx][yy] = 5 + n;
            }
            1 => world_map[xx][yy] = 2 + n,
            2 => world_map[xx][yy] = 8 + n,
            4 => world_map[xx][yy] = 4 + n,
            8 => world_map[xx][yy] = 6 + n,
            5 => world_map[xx][yy] = 1 + n,
            6 => world_map[xx][yy] = 7 + n,
            9 => world_map[xx][yy] = 3 + n,
            10 => world_map[xx][yy] = 9 + n,
            _ => world_map[xx][yy] = 5 + n,
        }
    }

    /// Rust 严格移植自 Pascal BuildWorld 过程（变量、流程、分支与Pascal一致）
    pub fn build_world(
        &mut self,
        world_map: &mut WorldBuffer,
        options: &WorldOptions,
        sprites: &mut SpriteDataManager,
    ) {
        let mut ab: u8 = b' ';
        let mut cd: u8 = b' ';
        let mut ef: u8 = b' ';
        let mut last_ab: u8 = b' ';
        let mut last_cd: u8 = b' ';
        let mut last_ef: u8 = b' ';
        let x_size = options.x_size as usize;
        let nv_usize = NV as usize;

        // 处理特殊字符（注意 world_map 读写都要 +EX/+EY1）
        let get = |wm: &WorldBuffer, x: usize, y: usize| -> u8 {
            let xx = (x as i32 + EX) as usize;
            let yy = (y as i32 + EY1) as usize;
            wm[xx][yy]
        };
        let mut set = |wm: &mut WorldBuffer, x: usize, y: usize, v: u8| {
            let xx = (x as i32 + EX) as usize;
            let yy = (y as i32 + EY1) as usize;
            wm[xx][yy] = v;
        };

        for i in 0..x_size {
            for j in 0..nv_usize {
                match get(world_map, i, j) {
                    0xCF => {  // Pascal: 'Ï' - 问号/宝箱标记
                        if j >= 5 {
                            set(world_map, i, j - 5, b'?');
                        }
                        if j >= 6 {
                            set(world_map, i, j - 6, 0xE9);
                        }
                        set(world_map, i, j, b' ');
                    }
                    0xD0 => {  // Pascal: 'Ð' - 金币标记
                        if j >= 2 {
                            set(world_map, i, j - 2, b'*');
                        }
                        set(world_map, i, j, b' ');
                    }
                    0xD1 => {  // Pascal: 'Ñ' - 向下复制
                        // Pascal:
                        //   k := j + 1;
                        //   for l := j downto -1 do
                        //     WorldMap^[i, l] := WorldMap^[i, k];
                        //
                        // 注意：Pascal 会写入 l=-1（WorldBuffer 支持负索引），Rust 这里用 EY1 偏移复现。
                        let k = j + 1;
                        let xx = (i as i32 + EX) as usize;
                        let src_yy = (k as i32 + EY1) as usize;
                        if xx < world_map.len() && src_yy < world_map[0].len() {
                            let v = world_map[xx][src_yy];
                            // l = j .. 0
                            for l in (0..=j).rev() {
                                let dst_yy = (l as i32 + EY1) as usize;
                                if dst_yy < world_map[0].len() {
                                    world_map[xx][dst_yy] = v;
                                }
                            }
                            // l = -1
                            if EY1 > 0 {
                                let dst_yy = (EY1 - 1) as usize;
                                if dst_yy < world_map[0].len() {
                                    world_map[xx][dst_yy] = v;
                                }
                            }
                        }
                    }
                    0xD2 => {  // Pascal: 'Ò' - 继承上方+设置底部254
                        if j >= 1 {
                            let v = get(world_map, i, j - 1);
                            set(world_map, i, j, v);
                        }
                        // Pascal: WorldMap^[i, NV] := #254
                        set(world_map, i, nv_usize, 254);
                    }
                    0xD3 => {  // Pascal: 'Ó' - 继承上方+设置底部255
                        if j >= 1 {
                            let v = get(world_map, i, j - 1);
                            set(world_map, i, j, v);
                        }
                        // Pascal: WorldMap^[i, NV] := #255
                        set(world_map, i, nv_usize, 255);
                    }
                    _ => {}
                }
            }
        }

        last_ab = b' ';
        last_cd = b' ';
        // last_ef 用于记录上一次 ef 状态，保留是为了对齐 Pascal 行为
        last_ef = b' ';

        let build_wall = options.wall_type1 < 100;
        if build_wall {
            for i in 0..x_size {
                for j in 0..nv_usize {
                    self.build_wall(
                        i, j, world_map, options, nv_usize, &mut ab, &mut cd, last_ab, last_cd,
                    );
                }
                // Pascal 原版的笔误：三行都写成了 LastAB := ...
                // 这导致 LastCD 和 LastEF 永远不会更新
                // 为了严格对齐 Pascal，这里复现这个 bug：
                last_ab = ab;
                last_ab = cd;  // Pascal: LastAB := CD (笔误)
                last_ab = ef;  // Pascal: LastAB := EF (笔误)
            }
        } else {
            let mut bricks = self.bricks.clone();
            match options.wall_type1 {
                100 => {
                    self.recolor(
                        &mut sprites.BRICK0_000,
                        Some(&mut bricks[0]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK0_001,
                        Some(&mut bricks[1]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK0_002,
                        Some(&mut bricks[2]),
                        options.ground_color1,
                    );
                }
                101 => {
                    self.recolor(
                        &mut sprites.BRICK1_000,
                        Some(&mut bricks[0]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK1_001,
                        Some(&mut bricks[1]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK1_002,
                        Some(&mut bricks[2]),
                        options.ground_color1,
                    );
                }
                102 => {
                    self.recolor(
                        &mut sprites.BRICK2_000,
                        Some(&mut bricks[0]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK2_001,
                        Some(&mut bricks[1]),
                        options.ground_color1,
                    );
                    self.recolor(
                        &mut sprites.BRICK2_002,
                        Some(&mut bricks[2]),
                        options.ground_color1,
                    );
                }
                _ => {}
            }
            self.bricks = bricks;
        }

        // [DEBUG] 打印 GRASS 输入精灵的像素值统计（已禁用，避免引入 HashMap 依赖）
        #[allow(dead_code)]
        fn count_pixels(buf: &crate::buffers::ImageBuffer) -> Vec<(u8, usize)> {
            let mut counts = [0usize; 256];
            for row in buf.iter() {
                for px in row.iter() {
                    counts[*px as usize] += 1;
                }
            }
            counts.iter().enumerate()
                .filter(|&(_, c)| *c > 0)
                .map(|(i, c)| (i as u8, *c))
                .collect()
        }
        
        self.convert_grass(
            &mut sprites.GRASS1_000,
            &sprites.GRASS1_001,
            &sprites.GRASS1_002,
        );
        self.convert_grass(
            &mut sprites.GRASS2_000,
            &sprites.GRASS2_001,
            &sprites.GRASS2_002,
        );
        // 注意：Pascal原版参数顺序是 (Grass3000, Grass3002, Grass3001)
        // 即第二个参数是002，第三个参数是001，与GRASS1/GRASS2不同
        self.convert_grass(
            &mut sprites.GRASS3_000,
            &sprites.GRASS3_002,
            &sprites.GRASS3_001,
        );

        self.convert_grass(
            &mut sprites.PALM0_000,
            &sprites.PALM0_001,
            &sprites.PALM0_002,
        );
        self.convert_grass(
            &mut sprites.PALM1_000,
            &sprites.PALM1_001,
            &sprites.PALM1_002,
        );
        self.convert_grass(
            &mut sprites.PALM2_000,
            &sprites.PALM2_001,
            &sprites.PALM2_002,
        );
        self.convert_grass(
            &mut sprites.PALM3_000,
            &sprites.PALM3_001,
            &sprites.PALM3_002,
        );

        // ===== 关键修复：在 recolor 之前从原始数据恢复精灵 =====
        // 因为 recolor 是就地修改，如果 build_world 被多次调用（切换关卡时），
        // 已经被 recolor 过的精灵会被错误地再次 recolor。
        // 解决方案：从原始数据副本恢复后再 recolor。
        
        // 恢复 BLOCK_001 原始数据
        sprites.BLOCK_001 = sprites.BLOCK_001_ORIG.clone();
        self.recolor(&mut sprites.BLOCK_001, None, options.brick_color);
        
        // 恢复 WOOD_000 原始数据
        sprites.WOOD_000 = sprites.WOOD_000_ORIG.clone();
        self.recolor(&mut sprites.WOOD_000, None, options.wood_color);
        
        // 恢复 XBLOCK_000 原始数据
        sprites.XBLOCK_000 = sprites.XBLOCK_000_ORIG.clone();
        self.recolor(&mut sprites.XBLOCK_000, None, options.xblock_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffers::{H, ImageBuffer, W, WorldOptions};
    use crate::mpal256;
    use crate::sprites::SpriteDataManager;
    use image::Rgba;

    // 假设有一个 save_image 函数用于保存ImageBuffer为png
    fn save_image(buf: &ImageBuffer, path: &str) {
        let mut palette = Palettes::new();
        palette.new_palette(mpal256::mpal256_palette());
        let mut img = image::ImageBuffer::<Rgba<u8>, Vec<u8>>::new(W as u32, H as u32);
        for y in 0..H as usize {
            for x in 0..W as usize {
                let color_idx = buf[y][x];
                let rgb = palette.get_rgb(color_idx);
                img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], 255]));
            }
        }
        img.save(path).unwrap();
    }

    #[test]
    fn test_init_wall_to_image() {
        // 构造测试用SpriteDataManager和WorldOptions
        let sprites = SpriteDataManager::new();
        let options = WorldOptions::default();

        // 正确初始化嵌套数组
        let empty_img: ImageBuffer =
            [[0u8; crate::buffers::W as usize]; crate::buffers::H as usize];
        let mut figures = Figures {
            fig_list: [[empty_img.clone(); N2]; N1],
            bricks: [empty_img.clone(); 4],
            sky: 0,
            trace_enabled: false,
        };

        // 测试不同类型的墙
        for wall_type in 0..=5u8 {
            figures.init_wall(1, wall_type, &sprites, &options);
            for idx in 0..N2 {
                let filename = format!("./output/test_walltype{}_fig{}.png", wall_type, idx + 1);
                save_image(&figures.fig_list[0][idx], &filename);
            }
        }
    }

    #[test]
    fn test_set_sky_palette_and_draw_sky() {
        use crate::backgr::BackGr;
        use crate::vga256::VGA;
        use image::Rgba;

        let options = WorldOptions::default();
        let empty_img: ImageBuffer =
            [[0u8; crate::buffers::W as usize]; crate::buffers::H as usize];
        let sprites = SpriteDataManager::new();
        let mut figures = Figures {
            fig_list: [[empty_img.clone(); N2]; N1],
            bricks: [empty_img.clone(); 4],
            sky: 0,
            trace_enabled: false,
        };

        // 测试所有天空类型（0..=12）
        for sky_type in 0..=12u8 {
            figures.init_sky(sky_type);
            let mut palette = Palettes::new();
            palette.new_palette(mpal256::mpal256_palette());
            figures.set_sky_palette(&mut palette, &options);

            // 创建一个 VGA 显存对象和 BackGrState
            let mut vga = VGA::new_offscreen(320, 200);
            vga.palette = palette.clone();
            let max_world_size = 236;
            let w_const = 20;
            let nv = 13;
            let h_const = 14;
            let mut backgr = BackGr::new(max_world_size, w_const, nv, h_const);

            // 绘制天空到 VGA 显存
            figures.draw_sky(
                0,
                0,
                vga.width as i32,
                vga.height as i32,
                &mut vga,
                &options,
                &mut backgr,
                &sprites,
            );

            // 将 VGA 显存转换为动态二维数组
            let mut img_buf = vec![vec![0u8; vga.width]; vga.height];
            for y in 0..vga.height {
                for x in 0..vga.width {
                    img_buf[y][x] = vga.get_pixel(x as i32, y as i32);
                }
            }

            // 保存图片
            let mut img =
                image::ImageBuffer::<Rgba<u8>, Vec<u8>>::new(vga.width as u32, vga.height as u32);
            for y in 0..vga.height {
                for x in 0..vga.width {
                    let color_idx = img_buf[y][x];
                    let rgb = palette.get_rgb(color_idx);
                    img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], 255]));
                }
            }
            let filename = format!("./output/test_skytype{}.png", sky_type);
            img.save(&filename).unwrap();
        }
        // 可人工比对生成的天空图片
    }
}
