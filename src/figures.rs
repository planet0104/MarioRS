// Figures module - handles sprite management and world rendering
// Converted from FIGURES.PAS
// GPU渲染支持：添加了收集渲染指令的方法

use crate::backgr::BackGr;
use crate::buffers::{Buffers, EX, EY1, ImageBuffer, NV, WorldBuffer, WorldOptions};
use crate::palettes::Palettes;
use crate::sprites::SpriteDataManager;
use crate::vga256::VGA;
use crate::gpu::sprite_batch::{FillCommand, SpriteCommand};

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
        let mut _sample_before = 0u8;
        let mut _sample_after = 0u8;
        
        match dst {
            Some(dst_buf) => {
                for y in 0..HH {
                    for x in 0..WW {
                        let val = src[y][x];
                        let new_val = if val <= 0x10 { val } else { (val & 0x07) + c };
                        dst_buf[y][x] = new_val;
                        if val != new_val {
                            if changed_count == 0 {
                                _sample_before = val;
                                _sample_after = new_val;
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
                                _sample_before = src[y][x];
                            }
                            src[y][x] = (src[y][x] & 0x07) + c;
                            if changed_count == 0 {
                                _sample_after = src[y][x];
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
    /// GPU版draw_sky - 使用GPU填充渲染天空/背景
    pub fn draw_sky(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        vga: &mut VGA,
        options: &WorldOptions,
        backgr: &mut BackGr,
        _sprites: &SpriteDataManager,
    ) {
        // 关键：必须以 options.sky_type 为准，不能用 self.sky。
        // 否则在关卡切换或入管渐显时，可能出现“第一帧还按旧天空类型绘制”导致闪烁。
        let sky = options.sky_type;

        // GPU模式：直接使用fill_world_gpu
        if options.backgr_type == 0 {
            vga.fill_world_gpu(x, y, w, h, 0xE0);
            return;
        }

        match sky {
            // 以Horizon分割填充
            0 | 1 | 3 | 4 => {
                let horizon = options.horizon as i32;
                let top_h = horizon - y;

                if horizon < y {
                    vga.fill_world_gpu(x, y, w, h, 0xF0);
                } else if horizon > y + h - 1 {
                    vga.fill_world_gpu(x, y, w, h, 0xE0);
                } else {
                    vga.fill_world_gpu(x, y, w, top_h, 0xE0);
                    vga.fill_world_gpu(x, horizon, w, h - top_h, 0xF0);
                }
            }

            // SmoothFill（渐变背景）
            2 | 5 | 9 | 10 | 11 | 12 => {
                backgr.smooth_fill_gpu(x as usize, y as usize, w as usize, h as usize, options, vga);
            }

            // 地下室场景：画砖/柱子/窗/大砖
            6 | 7 | 8 => {
                match options.backgr_type {
                    4 => backgr.draw_bricks_gpu(x, y, w, h, vga),
                    5 => backgr.large_bricks_gpu(x, y, w, h, vga),
                    6 => backgr.pillar_gpu(x, y, w, h, vga),
                    7 => backgr.windows_gpu(x, y, w, h, vga),
                    _ => {}
                }
            }

            _ => {}
        }
    }

    // ========== GPU渲染支持方法 ==========

    /// GPU模式：收集天空/背景填充命令
    /// 返回填充矩形命令列表，供GPU批量渲染
    pub fn collect_sky_fills(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        options: &WorldOptions,
    ) -> Vec<FillCommand> {
        let mut fills = Vec::new();
        // 关键：必须以 options.sky_type 为准，不能用 self.sky。
        // 否则在关卡切换或入管渐显时，可能出现“第一帧还按旧天空类型绘制”导致闪烁。
        let sky = options.sky_type;
        
        if options.backgr_type == 0 {
            fills.push(FillCommand::new(x, y, w, h, 0xE0));
            return fills;
        }

        match sky {
            0 | 1 | 3 | 4 => {
                let horizon = options.horizon as i32;
                let top_h = horizon - y;

                if horizon < y {
                    fills.push(FillCommand::new(x, y, w, h, 0xF0));
                } else if horizon > y + h - 1 {
                    fills.push(FillCommand::new(x, y, w, h, 0xE0));
                } else {
                    fills.push(FillCommand::new(x, y, w, top_h, 0xE0));
                    fills.push(FillCommand::new(x, horizon, w, h - top_h, 0xF0));
                }
            }
            2 | 5 | 9 | 10 | 11 | 12 => {
                // smooth_fill - 对齐 Oldsrc BACKGR.smooth_fill 的颜色变化（不包含抖动像素）
                // 规则：每6行下降1级，从0xEF开始，最低到0xE0；接近horizon后切换为0xF0
                let horizon = options.horizon.saturating_sub(4) as i32;
                for row in y..(y + h) {
                    let color_idx = if row >= horizon {
                        0xF0
                    } else {
                        let q = (row / 6).max(0) as u8;
                        let mut v = 0xEFu8.wrapping_sub(q);
                        if v < 0xE0 {
                            v = 0xE0;
                        }
                        v
                    };
                    fills.push(FillCommand::new(x, row, w, 1, color_idx));
                }
            }
            6 | 7 | 8 => {
                // 地下室背景
                //
                // Oldsrc 语义：
                // - BackGrType=4: 背景墙面用 PALBRICK_000 平铺（不透明 PutImage 语义）
                // - BackGrType=5/6/7: 分别是大砖/柱子/窗口的背景效果
                //
                // GPU 管线里，BackGrType=4 的砖墙需要以 sprite 平铺实现，
                // 不能用单色填充替代（否则会出现你反馈的“墙面纯色、只有窗口可见”的差异）。
                //
                // 这里对 BackGrType=4 不再返回 fill，由 renderer 负责追加砖墙平铺精灵。
                if options.backgr_type != 4 {
                    fills.push(FillCommand::new(x, y, w, h, 0x18));
                }
            }
            _ => {}
        }
        
        fills
    }

    /// GPU版本：收集单个tile的精灵命令
    /// 替代redraw方法的CPU绘制，用于GPU渲染管线
    pub fn collect_tile_sprite_gpu(
        &self,
        x: i32,
        y: i32,
        world_map: &WorldBuffer,
        _sprites: &SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
        options: &WorldOptions,
        buffers: &Buffers,
    ) -> Vec<SpriteCommand> {
        use crate::sprites::SpriteId;
        let mut commands = Vec::new();
        
        // GPU渲染统一使用屏幕坐标
        let xpos = x * crate::buffers::W as i32 - buffers.x_view;
        let ypos = y * crate::buffers::H as i32 - buffers.y_view;
        
        let get = |x: i32, y: i32| -> u8 {
            let xx = x + EX;
            let yy = y + EY1;
            if xx < 0 || yy < 0 || (xx as usize) >= world_map.len() || (yy as usize) >= world_map[0].len() {
                0
            } else {
                world_map[xx as usize][yy as usize]
            }
        };
        
        if x < 0 || y < 0 || y >= NV {
            return commands;
        }
        
        let ch = get(x, y);
        if ch == b' ' {
            return commands;
        }

        // Oldsrc 特例：如果上方 tile 是 18，则先叠加 FigList[0][5]（不透明）
        // 对齐 Oldsrc FIGURES.redraw 中的特殊覆盖
        if get(x, y - 1) == 18 {
            let (base_id, rotation, flip_x, flip_y) = Self::wall_variant_to_sprite(options.wall_type1, 5);
            let uv = atlas.get(base_id);
            commands.push(
                SpriteCommand::new(xpos, ypos, uv)
                    .with_rotation(rotation)
                    .with_flip(flip_x, flip_y)
                    .with_opaque(true),
            );
        }
        
        // 根据tile字符选择精灵
        let sprite_id: Option<SpriteId> = match ch {
            b'?' => Some(SpriteId::QUEST_000),
            b'@' => Some(SpriteId::QUEST_001),
            b'I' => Some(SpriteId::BLOCK_000),
            b'J' => Some(SpriteId::BLOCK_001),
            b'K' => Some(SpriteId::NOTE_000),
            b'X' => Some(SpriteId::XBLOCK_000),
            b'W' => {
                // 对齐 Oldsrc：地下室墙面是实体砖块，索引0也要绘制（否则会变成纯背景色）
                let uv = atlas.get(SpriteId::WOOD_000);
                commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                None
            }
            b'0' => Some(SpriteId::PIPE_000),
            b'1' => Some(SpriteId::PIPE_001),
            b'2' => Some(SpriteId::PIPE_002),
            b'3' => Some(SpriteId::PIPE_003),
            b'*' => Some(SpriteId::COIN_000),
            0xFE => {
                if get(x, y - 1) == 0xFE {
                    Some(SpriteId::EXIT_001)
                } else {
                    Some(SpriteId::EXIT_000)
                }
            }
            0xF7 => {
                // 草地逻辑对齐 Oldsrc 的 redraw:
                // 1. 若左右邻居存在墙体(1..=26)，先绘制无边缘墙体背景(GREEN_003)
                let left = get(x - 1, y);
                let right = get(x + 1, y);
                if (1..=26).contains(&left) || (1..=26).contains(&right) {
                    let uv = atlas.get(SpriteId::GREEN_003);
                    commands.push(SpriteCommand::new(xpos, ypos, uv));
                }

                // 2. 若上方是树干(0xF0)且 design=2，先叠加一层 SMTREE_001
                if get(x, y - 1) == 0xF0 && options.design == 2 {
                    let uv = atlas.get(SpriteId::SMTREE_001);
                    commands.push(SpriteCommand::new(xpos, ypos, uv));
                }

                // 3. 若上方是棕榈树干(0xF6)且 design=1，先叠加一层 WPALM_000
                // 对齐 Oldsrc: 确保草地透明处显示树干而非天空
                if get(x, y - 1) == 0xF6 && options.design == 1 {
                    let uv = atlas.get(SpriteId::WPALM_000);
                    commands.push(SpriteCommand::new(xpos, ypos, uv));
                }

                // 4. 再绘制草地本体(透明覆盖)
                if x == 0 || get(x - 1, y) == ch {
                    if get(x + 1, y) == ch {
                        Some(SpriteId::GRASS2_000)
                    } else {
                        Some(SpriteId::GRASS3_000)
                    }
                } else if get(x + 1, y) == ch {
                    Some(SpriteId::GRASS1_000)
                } else {
                    Some(SpriteId::GRASS3_000)
                }
            }
            0xF0 => match options.design {
                1 => {
                    if get(x, y - 1) != ch {
                        Some(SpriteId::FENCE_001)
                    } else {
                        Some(SpriteId::FENCE_000)
                    }
                }
                2 => {
                    if get(x, y - 1) != ch {
                        Some(SpriteId::SMTREE_000)
                    } else {
                        Some(SpriteId::SMTREE_001)
                    }
                }
                _ => None,
            },
            0xF6 => {
                if options.design == 1 {
                    Some(SpriteId::WPALM_000)
                } else {
                    None
                }
            }
            0xFA => {
                if options.design == 1 {
                    // 对齐 Oldsrc: 棕榈叶中心精灵
                    // 如果左边是0xF9(右侧棕榈叶位置)，先绘制PALM3作为overlay
                    if get(x - 1, y) == 0xF9 {
                        let uv = atlas.get(SpriteId::PALM3_000);
                        commands.push(SpriteCommand::new(xpos, ypos, uv));
                    // 如果右边是0xF9，先绘制PALM1作为overlay
                    } else if get(x + 1, y) == 0xF9 {
                        let uv = atlas.get(SpriteId::PALM1_000);
                        commands.push(SpriteCommand::new(xpos, ypos, uv));
                    }
                    Some(SpriteId::PALM0_000)
                } else {
                    None
                }
            }
            0xF4 => {
                if options.design == 1 {
                    // 对齐 Oldsrc: 左侧棕榈叶
                    // 如果下方是树干(0xF6)，先绘制WPALM作为overlay确保树干可见
                    if get(x, y + 1) == 0xF6 {
                        let uv = atlas.get(SpriteId::WPALM_000);
                        commands.push(SpriteCommand::new(xpos, ypos, uv));
                    }
                    Some(SpriteId::PALM1_000)
                } else {
                    None
                }
            }
            0xF9 => {
                if options.design == 1 {
                    Some(SpriteId::PALM2_000)
                } else {
                    None
                }
            }
            0xF5 => {
                if options.design == 1 {
                    // 对齐 Oldsrc: 右侧棕榈叶
                    // 如果下方是树干(0xF6)，先绘制WPALM作为overlay确保树干可见
                    if get(x, y + 1) == 0xF6 {
                        let uv = atlas.get(SpriteId::WPALM_000);
                        commands.push(SpriteCommand::new(xpos, ypos, uv));
                    }
                    Some(SpriteId::PALM3_000)
                } else {
                    None
                }
            }
            b'#' => match options.design {
                1 => Some(SpriteId::FALL_000),
                2 => {
                    // 对齐 Oldsrc:
                    // - 上方也是 '#': Put TREE_001 (opaque)
                    // - 上方是 '%': Put TREE_000 (opaque) 然后 Draw TREE_003 (transparent leaves)
                    // - 其它: Oldsrc 只 Draw TREE_003，但 CPU 版不是每帧全量重绘，树干像素会“留在背景”里。
                    //   GPU 全量重绘需要显式补一层树干底图，避免树叶动画帧透明处露出背景。
                    match get(x, y - 1) {
                        b'#' => {
                            let uv = atlas.get(SpriteId::TREE_001);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            None
                        }
                        b'%' => {
                            let uv = atlas.get(SpriteId::TREE_000);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            let uv = atlas.get(SpriteId::TREE_003);
                            commands.push(SpriteCommand::new(xpos, ypos, uv));
                            None
                        }
                        _ => {
                            // TREE_003 的底图应为 TREE_001（TREE001 与 TREE003 使用同一套颜色索引区间）
                            let uv = atlas.get(SpriteId::TREE_001);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            let uv = atlas.get(SpriteId::TREE_003);
                            commands.push(SpriteCommand::new(xpos, ypos, uv));
                            None
                        }
                    }
                }
                3 => {
                    // 对齐 Oldsrc：窗户是覆盖在地下砖墙上的透明图层。
                    // 这里不能用 WOOD_000 当底图，否则会出现你反馈的“窗户上方圆形部分露出木纹”。
                    let uv = atlas.get(SpriteId::PALBRICK_000);
                    commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                    let uv = atlas.get(SpriteId::WINDOW_001);
                    commands.push(SpriteCommand::new(xpos, ypos, uv));
                    None
                }
                4 => Some(SpriteId::LAVA_000),
                _ => None,
            },
            b'%' => match options.design {
                1 => Some(SpriteId::FALL_001),
                2 => {
                    // 对齐 Oldsrc:
                    // - 上方也是 '%': Put TREE_000 (opaque)
                    // - 上方是 '#': Put TREE_001 (opaque) 然后 Draw TREE_002 (transparent leaves)
                    // - 其它: Oldsrc 只 Draw TREE_002，但 CPU 版背景会保留树干像素；GPU 需要补树干底图
                    match get(x, y - 1) {
                        b'%' => {
                            let uv = atlas.get(SpriteId::TREE_000);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            None
                        }
                        b'#' => {
                            let uv = atlas.get(SpriteId::TREE_001);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            let uv = atlas.get(SpriteId::TREE_002);
                            commands.push(SpriteCommand::new(xpos, ypos, uv));
                            None
                        }
                        _ => {
                            // TREE_002 的底图应为 TREE_000（TREE000 与 TREE002 使用同一套颜色索引区间）
                            let uv = atlas.get(SpriteId::TREE_000);
                            commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                            let uv = atlas.get(SpriteId::TREE_002);
                            commands.push(SpriteCommand::new(xpos, ypos, uv));
                            None
                        }
                    }
                }
                3 => {
                    // 对齐 Oldsrc：窗户是覆盖在地下砖墙上的透明图层
                    let uv = atlas.get(SpriteId::PALBRICK_000);
                    commands.push(SpriteCommand::new(xpos, ypos, uv).with_opaque(true));
                    let uv = atlas.get(SpriteId::WINDOW_000);
                    commands.push(SpriteCommand::new(xpos, ypos, uv));
                    None
                }
                4 => Some(SpriteId::LAVA_001),
                5 => {
                    let idx = ((x + (buffers.lava_counter as i32 / 8)) % 5) as u8;
                    Some(match idx {
                        0 => SpriteId::LAVA2_001,
                        1 => SpriteId::LAVA2_002,
                        2 => SpriteId::LAVA2_003,
                        3 => SpriteId::LAVA2_004,
                        _ => SpriteId::LAVA2_005,
                    })
                }
                _ => None,
            },
            b'A' => {
                // 砖块 - 使用默认砖块样式
                let l = get(x - 1, y) == b'A';
                let r = get(x + 1, y) == b'A';
                let stitch = (x + y) % 2 == 1;
                if stitch && r {
                    Some(SpriteId::BRICK0_001)
                } else if !stitch && l {
                    Some(SpriteId::BRICK0_002)
                } else {
                    Some(SpriteId::BRICK0_000)
                }
            }
            b'=' => Some(SpriteId::PIN_000),
            // 墙体精灵 1-26：对齐 Oldsrc 的 redraw 分支（14-26 会先减13再参与选择）
            1..=26 => {
                // 对齐 Oldsrc 的 FigList 变体逻辑：
                // - ch>13: ch:=ch-13
                // - 否则根据左右邻居 14..=26 + 当前 ch in [1,4,7]/[3,6,9] 做 overlay（不透明 put）
                // - 然后绘制 FigList[0][ch_modified]：若 ch_modified 不在 [1,3,4,6,7,9] 则不透明 put，否则透明 draw
                let mut ch_modified = ch;
                if ch_modified > 13 {
                    ch_modified = ch_modified - 13;
                } else {
                    let left = get(x - 1, y);
                    if (14..=26).contains(&left) && [1, 4, 7].contains(&ch_modified) {
                        let overlay_idx = left - 13;
                        let (base_id, rotation, flip_x, flip_y) =
                            Self::wall_variant_to_sprite(options.wall_type1, overlay_idx);
                        let uv = atlas.get(base_id);
                        commands.push(
                            SpriteCommand::new(xpos, ypos, uv)
                                .with_rotation(rotation)
                                .with_flip(flip_x, flip_y)
                                .with_opaque(true),
                        );
                    } else {
                        let right = get(x + 1, y);
                        if (14..=26).contains(&right) && [3, 6, 9].contains(&ch_modified) {
                            let overlay_idx = right - 13;
                            let (base_id, rotation, flip_x, flip_y) =
                                Self::wall_variant_to_sprite(options.wall_type1, overlay_idx);
                            let uv = atlas.get(base_id);
                            commands.push(
                                SpriteCommand::new(xpos, ypos, uv)
                                    .with_rotation(rotation)
                                    .with_flip(flip_x, flip_y)
                                    .with_opaque(true),
                            );
                        }
                    }
                }

                let (base_id, rotation, flip_x, flip_y) =
                    Self::wall_variant_to_sprite(options.wall_type1, ch_modified);
                let uv = atlas.get(base_id);

                // Pascal: if not (Ch in [#1,#3,#4,#6,#7,#9]) then PutImage else DrawImage
                let opaque = ![1, 3, 4, 6, 7, 9].contains(&ch_modified);
                commands.push(
                    SpriteCommand::new(xpos, ypos, uv)
                        .with_rotation(rotation)
                        .with_flip(flip_x, flip_y)
                        .with_opaque(opaque),
                );
                None
            }
            _ => None,
        };
        
        if let Some(id) = sprite_id {
            let uv = atlas.get(id);
            // 默认对齐 DrawImage 语义: 索引0透明
            // Oldsrc 的 PutImage/DrawImage 差异只在明确分支里使用 with_opaque(true) 处理
            commands.push(SpriteCommand::new(xpos, ypos, uv));
        }
        
        commands
    }

    /// 把 FigList[0][idx]（1..=13）映射到基础精灵 + 旋转/翻转（GPU 侧做旋转，避免生成额外贴图）
    /// 只覆盖 Oldsrc InitWalls 生成的变体索引：
    /// 1,2,4,5,10 为基础；3,6,7,8,9,11,12,13 为镜像/旋转组合。
    fn wall_variant_to_sprite(
        wall_type1: u8,
        idx: u8,
    ) -> (crate::sprites::SpriteId, u8, bool, bool) {
        use crate::sprites::SpriteId;

        let (s1, s2, s4, s5, s10) = match wall_type1 {
            0 => (SpriteId::GREEN_000, SpriteId::GREEN_001, SpriteId::GREEN_002, SpriteId::GREEN_003, SpriteId::GREEN_004),
            1 => (SpriteId::SAND_000, SpriteId::SAND_001, SpriteId::SAND_002, SpriteId::SAND_003, SpriteId::SAND_004),
            2 => (SpriteId::GREEN_000, SpriteId::GREEN_001, SpriteId::GREEN_002, SpriteId::GREEN_003, SpriteId::GREEN_004),
            3 => (SpriteId::BROWN_000, SpriteId::BROWN_001, SpriteId::BROWN_002, SpriteId::BROWN_003, SpriteId::BROWN_004),
            4 => (SpriteId::GRASS_000, SpriteId::GRASS_001, SpriteId::GRASS_002, SpriteId::GRASS_003, SpriteId::GRASS_004),
            5 => (SpriteId::DES_000, SpriteId::DES_001, SpriteId::DES_002, SpriteId::DES_003, SpriteId::DES_004),
            _ => (SpriteId::GREEN_000, SpriteId::GREEN_001, SpriteId::GREEN_002, SpriteId::GREEN_003, SpriteId::GREEN_004),
        };

        match idx {
            1 => (s1, 0, false, false),
            2 => (s2, 0, false, false),
            3 => (s1, 0, true, false), // mirror(1)
            4 => (s4, 0, false, false),
            5 => (s5, 0, false, false),
            6 => (s4, 1, false, false), // rotate(4)
            7 => (s1, 1, false, true), // rotate(mirror(1)) == rot90 + flip_y
            8 => (s2, 1, false, false), // rotate(2)
            9 => (s1, 1, false, false), // rotate(1)
            10 => (s10, 0, false, false),
            11 => (s10, 0, true, false), // mirror(10)
            12 => (s10, 1, false, true), // rotate(mirror(10)) == rot90 + flip_y
            13 => (s10, 3, false, false), // mirror(rotate(mirror(10))) == rot270
            _ => (s5, 0, false, false),
        }
    }

    /// GPU版本：收集可见区域的所有tile精灵
    pub fn collect_visible_tiles_gpu(
        &self,
        x_start: i32,
        y_start: i32,
        width: i32,
        height: i32,
        world_map: &WorldBuffer,
        sprites: &SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
        options: &WorldOptions,
        buffers: &Buffers,
    ) -> Vec<SpriteCommand> {
        let mut commands = Vec::new();
        
        for y in y_start..(y_start + height) {
            for x in x_start..(x_start + width) {
                let tile_cmds = self.collect_tile_sprite_gpu(
                    x, y, world_map, sprites, atlas, options, buffers
                );
                commands.extend(tile_cmds);
            }
        }
        
        commands
    }

    // CPU Redraw 路径已彻底删除：纯 GPU 渲染通过 `collect_tile_sprite_gpu/collect_visible_tiles_gpu`
    // 生成 `SpriteCommand`，由 `renderer` 统一提交到 wgpu。

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
        let ef: u8 = b' ';
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
        let set = |wm: &mut WorldBuffer, x: usize, y: usize, v: u8| {
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

// tests removed: pure wgpu mode does not keep CPU framebuffer snapshots.
