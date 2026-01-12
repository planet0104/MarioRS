// VGA256.PAS interface 对应的 Rust 模块
// Rust translation of vga256.pas
// VGA 图形模块 - 纯 framebuffer 实现，不依赖任何窗口库
//
// 架构说明：
// - VGA 模块只负责 framebuffer 操作和调色板管理
// - 窗口创建和显示由 platform_desktop.rs 负责
// - 通过 render_to_rgba() 方法将调色板索引转换为 RGBA 供显示

use crate::palettes::{PalType, Palettes};

// logging macros (error/warn/info/debug) are available via crate-level macro_exports

// Pascal VGA256.PAS 常量
pub const VGA_SEGMENT: u16 = 0xA000;
pub const WINDOWHEIGHT: i32 = 13 * 14;
pub const WINDOWWIDTH: i32 = 16 * 20;
pub const SCREEN_WIDTH: i32 = 320;
pub const SCREEN_HEIGHT: i32 = 200;
pub const VIR_SCREEN_WIDTH: i32 = SCREEN_WIDTH + 2 * 20;
pub const VIR_SCREEN_HEIGHT: i32 = 182;
pub const BYTES_PER_LINE: i32 = VIR_SCREEN_WIDTH / 4;
pub const MISC_OUTPUT: u16 = 0x03C2;
pub const SC_INDEX: u16 = 0x03C4;
pub const GC_INDEX: u16 = 0x03CE;
pub const CRTC_INDEX: u16 = 0x03D4;
pub const VERT_RESCAN: u16 = 0x03DA;
pub const MAP_MASK: u8 = 2;
pub const MEMORY_MODE: u8 = 4;
pub const VERT_RETRACE_MASK: u8 = 8;
pub const MAX_SCAN_LINE: u8 = 9;
pub const START_ADDRESS_HIGH: u8 = 0xC;
pub const START_ADDRESS_LOW: u8 = 0xD;
pub const UNDERLINE: u8 = 0x14;
pub const MODE_CONTROL: u8 = 0x17;
pub const READ_MAP: u8 = 4;
pub const GRAPHICS_MODE: u8 = 5;
pub const MISCELLANEOUS: u8 = 6;
pub const MAX_SCREENS: i32 = 24;
pub const MAX_PAGE: i32 = 1;
pub const PAGE_SIZE: i32 = (VIR_SCREEN_HEIGHT + MAX_SCREENS) * BYTES_PER_LINE;
pub const PAGE_0: i32 = 0;
pub const PAGE_1: i32 = 0x8000;
pub const YBASE: i32 = 9;

#[derive(Clone)]
pub struct BackgroundData {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

pub struct VGA {
    pub framebuffer: Vec<u8>, // 线性palette索引缓冲区
    pub palette: Palettes,
    // Pascal的PushBackGr/PopBackGr保存的数据位于VGA显存地址空间里，和当前绘制页无关；
    // PopBackGr会把"上一次保存的背景数据"写回到当前页(PageOffset)。
    // 因此这里用一个全局复用栈来模拟"显存栈地址"，handle从1开始。
    pub background_stack: Vec<BackgroundData>,
    bg_stack_next: usize,
    pub width: usize,
    pub height: usize,
    pub page: i32,   // 当前页号，兼容Pascal
    pub x_view: i32, // 对应 Pascal XView
    pub y_view: i32, // 对应 Pascal YView
    // Pascal 的 SetYStart/SetYEnd 是"显示窗口裁剪"，不等于相机 YView
    pub y_start: i32,
    pub y_end: i32,
    pub y_offset: i32,          // 对应 Pascal YOffset
    pub page_offset: i32,       // 对应 Pascal PageOffset
    pub in_graphics_mode: bool, // 对应 Pascal InGraphicsMode
    pub stack: [i32; 2],        // 对应 Pascal Stack
}

impl VGA {
    /// 请求窗口重绘（保留接口兼容性，实际由平台层处理）
    pub fn request_redraw(&self) {
        // 窗口重绘现在由 platform_desktop.rs 处理
    }

    /// 将“世界坐标”（Pascal 语义，受 XView/YView 影响）转换为屏幕坐标。
    ///
    /// 说明：当前 Rust 的 framebuffer 是 320x200 的线性屏幕缓冲，不具备真实 Mode X 视口。
    /// 为了逐步对齐 Pascal，我们提供 *_world 系列 API：调用方仍传世界坐标，VGA 内部统一减去视口偏移。
    #[inline]
    pub fn world_to_screen(&self, x_world: i32, y_world: i32) -> (i32, i32) {
        (x_world - self.x_view, y_world - self.y_view)
    }

    /// 世界坐标写像素（会自动减去 XView/YView）。
    #[inline]
    pub fn put_pixel_world(&mut self, x_world: i32, y_world: i32, attr: u8) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.put_pixel(x, y, attr);
    }

    /// 世界坐标读像素（会自动减去 XView/YView）。
    #[inline]
    pub fn get_pixel_world(&self, x_world: i32, y_world: i32) -> u8 {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.get_pixel(x, y)
    }

    /// 世界坐标填充矩形（会自动减去 XView/YView）。
    #[inline]
    pub fn fill_world(&mut self, x_world: i32, y_world: i32, w: i32, h: i32, attr: u8) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.fill(x, y, w, h, attr);
    }

    /// 世界坐标绘制图像（0 透明跳过）。
    #[inline]
    pub fn draw_image_world(&mut self, x_world: i32, y_world: i32, w: i32, h: i32, bitmap: &[u8]) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.draw_image(x, y, w, h, bitmap);
    }

    /// 世界坐标写入图像（0 不透明不跳过）。
    #[inline]
    pub fn put_image_world(&mut self, x_world: i32, y_world: i32, w: i32, h: i32, bitmap: &[u8]) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.put_image(x, y, w, h, bitmap);
    }

    /// 世界坐标绘制 ImageBuffer（0 透明跳过）。
    #[inline]
    pub fn draw_image_imagebuffer_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.draw_image_imagebuffer(x, y, bitmap);
    }

    /// 世界坐标绘制 Sprite（安全，无unsafe）
    pub fn draw_sprite_world<const W: usize, const H: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        sprite: &crate::sprites::Sprite<W, H>,
    ) {
        self.draw_image_imagebuffer_world(x_world, y_world, sprite.pixels());
    }

    /// 屏幕坐标绘制 Sprite（安全，无unsafe）
    pub fn draw_sprite<const W: usize, const H: usize>(
        &mut self,
        x: i32,
        y: i32,
        sprite: &crate::sprites::Sprite<W, H>,
    ) {
        self.draw_image_imagebuffer(x, y, sprite.pixels());
    }

    /// 世界坐标写入 ImageBuffer（0 不透明不跳过）。
    #[inline]
    pub fn put_image_imagebuffer_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.put_image_imagebuffer(x, y, bitmap);
    }

    /// 世界坐标绘制 ImageBuffer 的部分行（0 透明跳过）。
    #[inline]
    pub fn draw_part_imagebuffer_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        y1: usize,
        y2: usize,
        bitmap: &[[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.draw_part_imagebuffer(x, y, y1, y2, bitmap);
    }

    /// 世界坐标上下颠倒绘制 ImageBuffer（0 透明跳过）。
    #[inline]
    pub fn up_side_down_imagebuffer_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.up_side_down_imagebuffer(x, y, bitmap);
    }

    /// 世界坐标保存背景区块（等价 Pascal PushBackGr 的坐标语义）。
    #[inline]
    pub fn push_backgr_world(&self, x_world: i32, y_world: i32, w: i32, h: i32) -> Vec<u8> {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.push_backgr(x, y, w, h)
    }

    /// 世界坐标保存背景区块（地址句柄版）。
    #[inline]
    pub fn push_backgr_address_world(&mut self, x_world: i32, y_world: i32, w: i32, h: i32) -> i32 {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.push_backgr_address(x, y, w, h)
    }

    /// 世界坐标读取区域像素到一维缓冲（仿 Pascal GetImage 的坐标语义）。
    #[inline]
    pub fn get_image_world(&self, x_world: i32, y_world: i32, w: i32, h: i32, bitmap: &mut [u8]) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.get_image(x, y, w, h, bitmap);
    }

    /// 世界坐标读取区域像素到二维 ImageBuffer。
    #[inline]
    pub fn get_image_imagebuffer_world<const WW: usize, const HH: usize>(
        &self,
        x_world: i32,
        y_world: i32,
        buf: &mut [[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.get_image_imagebuffer(x, y, buf);
    }

    /// 世界坐标变色绘制（对齐 Pascal RecolorImage 的坐标语义）。
    #[inline]
    pub fn recolor_image_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        bitmap: &[[u8; WW]; HH],
        color: i32,
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.recolor_image(x, y, bitmap, color);
    }
    /// 创建 VGA 对象
    /// 
    /// 注意：窗口创建和显示由 platform_desktop.rs 负责
    /// VGA 只负责 framebuffer 和调色板管理
    pub fn new(width: usize, height: usize) -> Self {
        Self::new_offscreen(width, height)
    }

    /// 设置视口位置，对应 Pascal 的 SetView(X, Y)
    pub fn set_view(&mut self, x: i32, y: i32) {
        self.x_view = x;
        self.y_view = y;
    }

    /// 水平滚动当前 framebuffer（屏幕坐标），用于模拟 Pascal Mode X 的硬件视口滚屏。
    ///
    /// 说明：本项目的 Rust framebuffer 是固定 320x200 的线性缓冲，改变 XView 只会影响后续绘制的位置，
    /// 不会自动移动已绘制的像素。因此增量滚屏需要先把上一帧像素按 dx 平移，再重绘新露出的条带。
    pub fn scroll_screen_x(&mut self, dx: i32) {
        if dx == 0 {
            return;
        }
        let w = self.width;
        let h = self.height;
        if w == 0 || h == 0 {
            return;
        }

        // 限制到屏幕宽度内，避免越界
        let mut dx = dx;
        if dx >= w as i32 {
            dx = w as i32;
        }
        if dx <= -(w as i32) {
            dx = -(w as i32);
        }

        if dx > 0 {
            let d = dx as usize;
            for y in 0..h {
                let row_start = y * w;
                let row_end = row_start + w;
                let row = &mut self.framebuffer[row_start..row_end];
                row.copy_within(0..(w - d), d);
                row[0..d].fill(0);
            }
        } else {
            let d = (-dx) as usize;
            for y in 0..h {
                let row_start = y * w;
                let row_end = row_start + w;
                let row = &mut self.framebuffer[row_start..row_end];
                row.copy_within(d..w, 0);
                row[(w - d)..w].fill(0);
            }
        }
    }

    /// 创建 VGA 对象（纯 framebuffer 模式）
    pub fn new_offscreen(width: usize, height: usize) -> Self {
        let framebuffer = vec![0u8; width * height];
        let palette = Palettes::new();

        let safe = 34 * BYTES_PER_LINE;
        let stack = [PAGE_0 + PAGE_SIZE + safe, PAGE_1 + PAGE_SIZE + safe];

        VGA {
            framebuffer,
            palette,
            background_stack: Vec::new(),
            bg_stack_next: 1,
            width,
            height,
            page: 0,
            x_view: 0,
            y_view: 0,
            y_start: 0,
            y_end: WINDOWHEIGHT,  // 使用游戏可视区域高度 182
            y_offset: 0,
            page_offset: PAGE_0,
            in_graphics_mode: false,
            stack,
        }
    }

    /// 将调色板索引 framebuffer 渲染到 RGBA 缓冲区
    /// 
    /// 供 platform_desktop.rs 调用，将 VGA 内容显示到窗口
    pub fn render_to_rgba(&self, output: &mut [u8]) {
        use crate::palettes::{PE_BLACK_WHITE, PE_EGA_MODE, PE_NO_EFFECT};
        
        let effect = self.palette.palette_effect;
        
        for (i, &color_idx) in self.framebuffer.iter().enumerate() {
            let rgb = self.palette.get_rgb(color_idx);
            let mut r = rgb[0].saturating_mul(4);
            let mut g = rgb[1].saturating_mul(4);
            let mut b = rgb[2].saturating_mul(4);
            
            // 应用调色板效果
            if effect != PE_NO_EFFECT {
                match effect {
                    PE_BLACK_WHITE => {
                        let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
                        r = gray;
                        g = gray;
                        b = gray;
                    }
                    PE_EGA_MODE => {
                        r = r & 0xF0;
                        g = g & 0xF0;
                        b = b & 0xF0;
                    }
                    _ => {}
                }
            }
            
            let offset = i * 4;
            if offset + 3 < output.len() {
                output[offset] = r;
                output[offset + 1] = g;
                output[offset + 2] = b;
                output[offset + 3] = 255;
            }
        }
    }

    /// 翻页，对应 Pascal 的 SwapPages
    pub fn swap_pages(&mut self) {
        match self.page {
            0 => {
                self.page = 1;
                self.page_offset = PAGE_1 + self.y_offset * BYTES_PER_LINE;
            }
            1 => {
                self.page = 0;
                self.page_offset = PAGE_0 + self.y_offset * BYTES_PER_LINE;
            }
            _ => {}
        }
    }

    /// 获取当前页号（兼容Pascal CurrentPage）
    pub fn current_page(&self) -> i32 {
        self.page
    }

    /// 获取页偏移量，对应Pascal GetPageOffset
    pub fn get_page_offset(&self) -> i32 {
        self.page_offset
    }

    /// 清空像素缓冲区（显存），全部置为0
    pub fn clear(&mut self) {
        self.framebuffer.fill(0);
    }

    /// 检测VGA，对应Pascal DetectVGA
    pub fn detect_vga() -> bool {
        // 现代系统总是返回true，或者实现具体的检测逻辑
        true
    }

    /// 获取视频模式，对应Pascal GetMode
    pub fn get_mode() -> u8 {
        // 现代系统模拟，返回13h模式
        0x13
    }

    /// 设置视频模式，对应Pascal SetMode
    pub fn set_mode(&mut self, new_mode: u8) {
        // 现代系统中这是一个空操作或记录模式
        // 在真实硬件中这会调用BIOS中断
        if new_mode == 0x13 {
            self.in_graphics_mode = true;
        }
    }

    /// 设置屏幕宽度，对应Pascal SetWidth
    pub fn set_width(&mut self, new_width: i32) {
        // 现代系统中这是一个空操作
        // 在真实硬件中这会设置CRTC寄存器
    }

    /// 初始化VGA图形模式（320x200 256色），对应 Pascal 的 InitVGA 过程。
    pub fn init_vga(&mut self) {
        // 对应 Pascal: ClearPalette; SetMode($13); ClearPalette; SetWidth(BYTES_PER_LINE shr 1);
        self.clear_palette();
        self.set_mode(0x13);
        self.clear_palette();
        self.set_width(BYTES_PER_LINE >> 1);
        self.clear_vga_mem();
        self.in_graphics_mode = true;
    }

    /// 恢复旧模式，对应Pascal OldMode
    pub fn old_mode(&mut self) {
        if self.in_graphics_mode {
            self.clear_vga_mem();
            self.clear_palette();
            self.show_page();
        }
        self.set_mode(0x03); // 文本模式
        self.in_graphics_mode = false;
    }

    /// 等待显示期间，对应Pascal WaitDisplay
    pub fn wait_display(&self) {
        // 现代系统中这通常是空操作，或者添加适当的延迟
        // 在真实VGA硬件中会等待垂直回扫期间
    }

    /// 等待回扫，对应Pascal WaitRetrace  
    pub fn wait_retrace(&self) {
        // 现代系统中这通常是空操作，或者添加适当的延迟
        // 在真实VGA硬件中会等待垂直回扫
    }

    /// 恢复背景区块（PopBackGr），将 buf 区块数据写回 framebuffer
    pub fn pop_backgr(&mut self, buf: &Vec<u8>) {
        if buf.len() < 4 {
            return;
        }
        // 解析区块参数
        let x = buf[0] as i32;
        let y = buf[1] as i32;
        let w = buf[2] as i32;
        let h = buf[3] as i32;
        let data = &buf[4..];
        if data.len() < (w * h) as usize {
            return;
        }
        // 写回 framebuffer
        for row in 0..h {
            let dst_y = y + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..w {
                let dst_x = x + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * w + col) as usize;
                let color = data[src_idx];
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// Pascal版本的PopBackGr，使用地址参数
    pub fn pop_backgr_address(&mut self, address: i32) {
        if address == 0 {
            return;
        }
        // address 作为全局句柄(从1开始，0表示无效)
        let idx = (address - 1) as usize;
        if idx >= self.background_stack.len() {
            return;
        }
        // 避免同时存在不可变借用与可变借用
        let bg = self.background_stack[idx].clone();
        let x = bg.x;
        let y = bg.y;
        let w = bg.width;
        let h = bg.height;
        let data = bg.data;
        if w <= 0 || h <= 0 {
            return;
        }
        if data.len() < (w * h) as usize {
            return;
        }
        for row in 0..h {
            let dst_y = y + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..w {
                let dst_x = x + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * w + col) as usize;
                let color = data[src_idx];
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 保存背景区块（PushBackGr），返回 Vec<u8>，前4字节为 x, y, w, h，后面为像素数据
    pub fn push_backgr(&self, x: i32, y: i32, w: i32, h: i32) -> Vec<u8> {
        // 只保存屏幕内的区块
        if y + h < 0 || y >= self.height as i32 || w <= 0 || h <= 0 {
            crate::error!(
                "Invalid block dimensions: x={}, y={}, w={}, h={}",
                x, y, w, h
            );
            return Vec::new();
        }
        // 区块参数（x, y, w, h）各1字节，后面是像素数据
        let mut buf = Vec::with_capacity(4 + (w * h) as usize);
        buf.push(x as u8);
        buf.push(y as u8);
        buf.push(w as u8);
        buf.push(h as u8);
        // 采集区块像素
        for row in 0..h {
            let src_y = y + row;
            for col in 0..w {
                let src_x = x + col;
                let color = if src_x < 0
                    || src_x as usize >= self.width
                    || src_y < 0
                    || src_y as usize >= self.height
                {
                    0
                } else {
                    let idx = src_y as usize * self.width + src_x as usize;
                    self.framebuffer[idx]
                };
                buf.push(color);
            }
        }
        buf
    }

    /// Pascal版本的PushBackGr，返回地址
    pub fn push_backgr_address(&mut self, x: i32, y: i32, w: i32, h: i32) -> i32 {
        // Pascal语义：返回0表示无效；这里使用现代实现保存到background_stack，并返回句柄(>=1)
        if w <= 0 || h <= 0 {
            return 0;
        }
        // Pascal限制：if not ((Y + H >= 0) and (Y < 200)) then Exit;
        if !((y + h >= 0) && (y < SCREEN_HEIGHT)) {
            return 0;
        }

        let mut data = Vec::with_capacity((w * h) as usize);
        for row in 0..h {
            let src_y = y + row;
            for col in 0..w {
                let src_x = x + col;
                let color = if src_x < 0
                    || src_x as usize >= self.width
                    || src_y < 0
                    || src_y as usize >= self.height
                {
                    0
                } else {
                    let idx = src_y as usize * self.width + src_x as usize;
                    self.framebuffer[idx]
                };
                data.push(color);
            }
        }

        let handle = self.bg_stack_next.max(1);
        let idx = handle - 1;
        let bg = BackgroundData {
            x,
            y,
            width: w,
            height: h,
            data,
        };
        if idx < self.background_stack.len() {
            self.background_stack[idx] = bg;
        } else {
            self.background_stack.push(bg);
        }
        self.bg_stack_next = handle.saturating_add(1);
        handle as i32
    }

    /// 刷新显示（保留接口兼容性）
    /// 
    /// 实际的显示由 platform_desktop.rs 通过 render_to_rgba() 完成
    pub fn present(&mut self) {
        // 显示由平台层处理，这里不做任何操作
        // 保留此方法是为了兼容现有代码调用
    }

    /// 在framebuffer绘制单个像素
    pub fn put_pixel(&mut self, x: i32, y: i32, attr: u8) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let idx = y as usize * self.width + x as usize;
        self.framebuffer[idx] = attr;
    }

    /// 获取framebuffer的像素值
    pub fn get_pixel(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return 0;
        }
        let idx = y as usize * self.width + x as usize;
        self.framebuffer[idx]
    }

    /// 从framebuffer中获取指定区域的像素数据，存入bitmap缓冲区（仿Pascal GetImage）
    /// bitmap缓冲区需预先分配好足够空间（width*height）
    pub fn get_image(&self, xpos: i32, ypos: i32, width: i32, height: i32, bitmap: &mut [u8]) {
        let w = self.width as i32;
        let h = self.height as i32;
        let width = width.max(0);
        let height = height.max(0);
        for row in 0..height {
            let src_y = ypos + row;
            if src_y < 0 || src_y >= h {
                continue;
            }
            for col in 0..width {
                let src_x = xpos + col;
                let dst_idx = (row * width + col) as usize;
                if src_x < 0 || src_x >= w {
                    if dst_idx < bitmap.len() {
                        bitmap[dst_idx] = 0;
                    }
                    continue;
                }
                let src_idx = src_y as usize * self.width + src_x as usize;
                if dst_idx < bitmap.len() && src_idx < self.framebuffer.len() {
                    bitmap[dst_idx] = self.framebuffer[src_idx];
                }
            }
        }
    }

    /// 从framebuffer中获取指定区域的像素数据，存入二维 ImageBuffer（[[u8; W]; H]）
    pub fn get_image_imagebuffer<const WW: usize, const HH: usize>(
        &self,
        xpos: i32,
        ypos: i32,
        buf: &mut [[u8; WW]; HH],
    ) {
        let w = WW as i32;
        let h = HH as i32;
        for row in 0..h {
            let src_y = ypos + row;
            if src_y < 0 || src_y as usize >= self.height {
                continue;
            }
            for col in 0..w {
                let src_x = xpos + col;
                if src_x < 0 || src_x as usize >= self.width {
                    buf[row as usize][col as usize] = 0;
                    continue;
                }
                let src_idx = src_y as usize * self.width + src_x as usize;
                buf[row as usize][col as usize] = self.framebuffer[src_idx];
            }
        }
    }

    /// 严格移植自 Pascal VGA256.PAS 的 RecolorImage 过程
    /// 作用：对图片区块进行调色变色渲染，src为ImageBuffer，color为调色参数
    /// 仅对非0像素做变色，0透明
    /// 
    /// Pascal汇编：add al, Diff（直接将颜色索引加上Diff值）
    pub fn recolor_image<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        bitmap: &[[u8; WW]; HH],
        color: i32,
    ) {
        for row in 0..HH {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..WW {
                let dst_x = xpos + col as i32;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let c = bitmap[row][col];
                if c == 0 {
                    continue;
                }
                // Pascal: add al, Diff（直接加，不是取低4位）
                let new_c = (c as i32).wrapping_add(color) as u8;
                self.put_pixel(dst_x, dst_y, new_c);
            }
        }
    }

    /// 严格按照Pascal版本实现的RecolorImage
    pub fn recolor_image_pascal(
        &mut self,
        xpos: i32,
        ypos: i32,
        width: i32,
        height: i32,
        bitmap: &[u8],
        diff: u8,
    ) {
        for row in 0..height {
            let dst_y = ypos + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..width {
                let dst_x = xpos + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * width + col) as usize;
                if src_idx >= bitmap.len() {
                    continue;
                }
                let mut color = bitmap[src_idx];
                if color == 0 {
                    continue;
                }
                color = color.wrapping_add(diff);
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 绘制图像区块，bitmap为width*height的palette索引，0透明跳过
    pub fn draw_image(&mut self, xpos: i32, ypos: i32, width: i32, height: i32, bitmap: &[u8]) {
        for row in 0..height {
            let dst_y = ypos + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..width {
                let dst_x = xpos + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * width + col) as usize;
                if src_idx >= bitmap.len() {
                    continue;
                }
                let color = bitmap[src_idx];
                if color == 0 {
                    continue;
                }
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 将 bitmap 区块写入framebuffer（现代化实现，直接按像素写palette索引）
    pub fn put_image(&mut self, xpos: i32, ypos: i32, width: i32, height: i32, bitmap: &[u8]) {
        for row in 0..height {
            let dst_y = ypos + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..width {
                let dst_x = xpos + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * width + col) as usize;
                if src_idx >= bitmap.len() {
                    continue;
                }
                let color = bitmap[src_idx];
                // 注意：put_image在Pascal中不跳过0值像素
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 绘制图像区块，bitmap为二维 ImageBuffer（0透明跳过）
    ///
    /// P2-1 修复：使用安全的二维数组遍历，消除 unsafe from_raw_parts
    pub fn draw_image_imagebuffer<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        // 直接遍历二维数组，避免 unsafe 转换
        for (row, line) in bitmap.iter().enumerate() {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for (col, &color) in line.iter().enumerate() {
                if color == 0 {
                    continue; // 透明像素跳过
                }
                let dst_x = xpos + col as i32;
                if dst_x >= 0 && (dst_x as usize) < self.width {
                    self.put_pixel(dst_x, dst_y, color);
                }
            }
        }
    }

    /// 绘制图像区块的部分区域（用于平铺填充，0 透明跳过）
    /// actual_w, actual_h: 实际要绘制的宽高（可能小于bitmap尺寸）
    pub fn draw_image_imagebuffer_partial<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        actual_w: usize,
        actual_h: usize,
        bitmap: &[[u8; WW]; HH],
    ) {
        let draw_h = actual_h.min(HH);
        let draw_w = actual_w.min(WW);
        
        for row in 0..draw_h {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..draw_w {
                let color = bitmap[row][col];
                if color == 0 {
                    continue; // 透明像素跳过
                }
                let dst_x = xpos + col as i32;
                if dst_x >= 0 && (dst_x as usize) < self.width {
                    self.put_pixel(dst_x, dst_y, color);
                }
            }
        }
    }

    /// 写入图像区块的部分区域（用于平铺填充，不跳过任何像素）
    /// actual_w, actual_h: 实际要绘制的宽高（可能小于bitmap尺寸）
    /// 对齐 Pascal PutImage 语义：0 值像素不跳过
    pub fn put_image_imagebuffer_partial<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        actual_w: usize,
        actual_h: usize,
        bitmap: &[[u8; WW]; HH],
    ) {
        let draw_h = actual_h.min(HH);
        let draw_w = actual_w.min(WW);
        
        for row in 0..draw_h {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..draw_w {
                let color = bitmap[row][col];
                let dst_x = xpos + col as i32;
                if dst_x >= 0 && (dst_x as usize) < self.width {
                    self.put_pixel(dst_x, dst_y, color);
                }
            }
        }
    }

    /// 世界坐标版：写入图像区块的部分区域（不跳过任何像素）
    #[inline]
    pub fn put_image_imagebuffer_partial_world<const WW: usize, const HH: usize>(
        &mut self,
        x_world: i32,
        y_world: i32,
        actual_w: usize,
        actual_h: usize,
        bitmap: &[[u8; WW]; HH],
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.put_image_imagebuffer_partial(x, y, actual_w, actual_h, bitmap);
    }

    /// 将二维 ImageBuffer 区块写入framebuffer（不跳过任何像素）
    ///
    /// 对齐 Pascal VGA256.PAS PutImage: "NULL-bytes are NOT ignored"
    /// 与 draw_image 不同，put_image 会写入所有像素（包括 color 0）
    pub fn put_image_imagebuffer<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        // 直接遍历二维数组，写入所有像素（包括 color 0）
        for (row, line) in bitmap.iter().enumerate() {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for (col, &color) in line.iter().enumerate() {
                // 注意：put_image 不跳过 color 0，与 Pascal PutImage 行为一致
                let dst_x = xpos + col as i32;
                if dst_x >= 0 && (dst_x as usize) < self.width {
                    self.put_pixel(dst_x, dst_y, color);
                }
            }
        }
    }

    /// 绘制二维 ImageBuffer 区块的第 y1~y2 行（含），其余行跳过，0 透明跳过
    pub fn draw_part_imagebuffer<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        y1: usize,
        y2: usize,
        bitmap: &[[u8; WW]; HH],
    ) {
        let y1 = y1.max(0).min(HH - 1);
        let y2 = y2.max(0).min(HH - 1);
        if y1 > y2 || HH == 0 {
            return;
        }
        for row in y1..=y2 {
            let dst_y = ypos + row as i32;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..WW {
                let dst_x = xpos + col as i32;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let color = bitmap[row][col];
                if color == 0 {
                    continue;
                }
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 将二维 ImageBuffer 区块上下颠倒地写入framebuffer（0透明跳过）
    pub fn up_side_down_imagebuffer<const WW: usize, const HH: usize>(
        &mut self,
        xpos: i32,
        ypos: i32,
        bitmap: &[[u8; WW]; HH],
    ) {
        for row in 0..HH {
            let dst_y = ypos + (HH as i32 - 1 - row as i32); // 上下颠倒
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..WW {
                let dst_x = xpos + col as i32;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let color = bitmap[row][col];
                if color == 0 {
                    continue;
                }
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 在framebuffer上上下颠倒地绘制图像区块（0透明跳过）
    /// bitmap为width*height的palette索引，按行存储
    pub fn up_side_down(&mut self, xpos: i32, ypos: i32, width: i32, height: i32, bitmap: &[u8]) {
        for row in 0..height {
            let dst_y = ypos + (height - 1 - row); // 上下颠倒
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..width {
                let dst_x = xpos + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * width + col) as usize;
                if src_idx >= bitmap.len() {
                    continue;
                }
                let color = bitmap[src_idx];
                if color == 0 {
                    continue;
                }
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 绘制 bitmap 的第 y1~y2 行（含），其余行跳过，0 透明跳过
    pub fn draw_part(
        &mut self,
        xpos: i32,
        ypos: i32,
        width: i32,
        height: i32,
        y1: i32,
        y2: i32,
        bitmap: &[u8],
    ) {
        // y1/y2 超界保护
        let y1 = y1.max(0);
        let y2 = y2.min(height - 1);
        if y1 > y2 || height <= 0 {
            return;
        }
        for row in y1..=y2 {
            let dst_y = ypos + row;
            if dst_y < 0 || dst_y as usize >= self.height {
                continue;
            }
            for col in 0..width {
                let dst_x = xpos + col;
                if dst_x < 0 || dst_x as usize >= self.width {
                    continue;
                }
                let src_idx = (row * width + col) as usize;
                if src_idx >= bitmap.len() {
                    continue;
                }
                let color = bitmap[src_idx];
                if color == 0 {
                    continue;
                }
                self.put_pixel(dst_x, dst_y, color);
            }
        }
    }

    /// 填充指定区域为指定颜色
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, attr: u8) {
        for dy in 0..h {
            let py = y + dy;
            if py < 0 || py as usize >= self.height {
                continue;
            }
            for dx in 0..w {
                let px = x + dx;
                if px < 0 || px as usize >= self.width {
                    continue;
                }
                self.put_pixel(px, py, attr);
            }
        }
    }

    /// 设置调色板单色
    pub fn set_palette(&mut self, color: u8, red: u8, green: u8, blue: u8) {
        // 如果调色板被锁定，不写入（防止淡入前闪烁）
        if self.palette.lock_palette {
            return;
        }
        let idx = color as usize;
        if idx < self.palette.palette.len() {
            // Pascal中的调色板值是6位的(0-63)，这里内部存6bit值，渲染时再放大
            self.palette.palette[idx] = [red, green, blue];
        }
    }

    /// 写入整套调色板，对应 Pascal `VGA256.ReadPalette(var NewPalette)`
    ///
    /// 注意1 Pascal 这里的过程名叫 ReadPalette 但实现是写入 DAC
    /// 注意2 Rust 侧保持同名以便对齐 Pascal 调用点
    pub fn read_palette(&mut self, palette: &PalType) {
        // 将传入调色板写入当前 VGA 调色板
        for i in 0..256 {
            self.set_palette(i as u8, palette[i][0], palette[i][1], palette[i][2]);
        }
    }

    /// 清空调色板（全部置为黑色）
    pub fn clear_palette(&mut self) {
        for c in self.palette.palette.iter_mut() {
            c[0] = 0;
            c[1] = 0;
            c[2] = 0;
        }
    }

    /// 绘制1bpp位图，bitmap前2字节为宽高，后续为位图数据，1为attr色，0跳过
    pub fn draw_bitmap(&mut self, x: i32, y: i32, bitmap: &[u8], attr: u8) {
        if bitmap.len() < 2 {
            return;
        }
        let w = bitmap[0] as usize;
        let h = bitmap[1] as usize;
        if w == 0 || h == 0 {
            return;
        }
        let mut bit_idx = 0;
        let mut byte_idx = 2;
        for dy in 0..h {
            for dx in 0..w {
                if bit_idx % 8 == 0 {
                    if byte_idx >= bitmap.len() {
                        return;
                    }
                }
                let byte = bitmap.get(byte_idx).copied().unwrap_or(0);
                let bit = 7 - (bit_idx % 8);
                if (byte & (1 << bit)) != 0 {
                    self.put_pixel(x + dx as i32, y + dy as i32, attr);
                }
                bit_idx += 1;
                if bit_idx % 8 == 0 {
                    byte_idx += 1;
                }
            }
            if bit_idx % 8 != 0 {
                bit_idx += 8 - (bit_idx % 8);
                byte_idx = 2 + (dy + 1) * ((w + 7) / 8);
            }
        }
    }

    /// 设置显存视口偏移，对应 Pascal 的 SetViewport(X, Y, PageNr)
    pub fn set_viewport(&mut self, x: i32, y: i32, page_nr: i32) {
        // 这里只存储参数，供 world->screen 转换使用，不做真实硬件操作
        self.x_view = x;
        self.y_view = y;
        self.page = page_nr;
    }

    /// 翻页显示，对应 Pascal 的 ShowPage
    pub fn show_page(&mut self) {
        // Pascal 的 ShowPage 语义是“把当前页显示出来”。
        // Rust 版使用 pixels 渲染，需要在这里显式 present，否则主循环每帧不会刷新窗口。
        // 注意：SwapPages 的语义是切换绘制页/显示页，这里不应隐式 swap。
        self.set_viewport(self.x_view, self.y_view, self.page);
        self.present();
    }

    /// 设置边框颜色，对应Pascal Border
    pub fn border(&mut self, attr: u8) {
        // 现代系统中这通常是空操作
        // 在真实VGA硬件中会设置边框颜色
    }

    pub fn set_y_offset(&mut self, new_y_offset: i32) {
        self.y_offset = new_y_offset;
    }

    pub fn get_y_offset(&self) -> i32 {
        self.y_offset
    }

    /// 设置 Y 起始扫描线（模拟 Pascal SetYStart）
    pub fn set_y_start(&mut self, y_start: i32) {
        // Pascal: 只影响显示窗口裁剪，不改变相机视口（YView）
        self.y_start = y_start;
    }

    /// 设置 Y 结束扫描线（模拟 Pascal SetYEnd）
    pub fn set_y_end(&mut self, y_end: i32) {
        // Pascal: 只影响显示窗口裁剪，不改变相机视口（YView）
        self.y_end = y_end;
    }

    /// 模拟 Pascal 的 LockPal（调色板锁定，实际可为空操作或加标志位）
    pub fn lock_pal(&mut self) {
        // 在实际硬件下用于防止调色板被修改，这里可为空实现或加锁标志
        self.palette.lock_pal();
    }

    /// 解锁调色板，对应Pascal UnLockPal
    pub fn unlock_pal(&mut self) {
        self.palette.unlock_pal();
    }

    /// 清空 VGA 显存，对应 Pascal 的 ClearVGAMem
    pub fn clear_vga_mem(&mut self) {
        self.clear();
    }

    /// 重置背景堆栈，对应Pascal的ResetStack
    pub fn reset_stack(&mut self) {
        let safe = 34 * BYTES_PER_LINE;
        self.stack[0] = PAGE_0 + PAGE_SIZE + safe;
        self.stack[1] = PAGE_1 + PAGE_SIZE + safe;
        // Pascal ResetStack不会清空已保存的背景数据，只重置“栈顶指针”，以便复用内存。
        self.bg_stack_next = 1;
    }

    /// Wrapper to call `palette.blink_palette` without double mutable borrow of `vga`
    pub fn palette_blink_wrapper(&mut self, options: &crate::buffers::WorldOptions) {
        // 注意：Pascal 原版 BlinkPalette 不会跳过任何 sky_type
        // 瀑布/河流动画使用调色板索引 7-11，与地下室背景 0xE0/0xF0 不冲突

        // 如果调色板被锁定，跳过blink操作
        if self.palette.lock_palette {
            return;
        }

        // 保存锁定状态，因为take后self.palette会变成默认值（unlock状态）
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        // 关键修复：同时设置pal的锁定状态，因为blink_palette中的out_palette检查的是pal.lock_palette
        pal.lock_palette = was_locked;
        // 恢复锁定状态到默认替换的调色板，防止blink期间其他代码通过vga.set_palette写入
        self.palette.lock_palette = was_locked;
        pal.blink_palette(self, options);
        self.palette = pal;
    }

    /// Wrapper to call `palette.fade_down` without double mutable borrow of `vga`
    /// 淡出效果：画面逐渐变暗，最终保持变暗状态
    pub fn palette_fade_down_wrapper(&mut self, n: u8) {
        // 保存锁定状态，因为take后self.palette会变成默认值（unlock状态）
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        // 恢复锁定状态
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.fade_down(n, self);
        // 写回pal，此时pal.palette是变暗的，pal.source_palette是原始值
        self.palette = pal;
    }
    
    /// Wrapper to call `palette.fade_up` without double mutable borrow of `vga`
    /// 淡入效果：画面逐渐变亮，从source_palette恢复
    pub fn palette_fade_up_wrapper(&mut self, n: u8) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.fade_up(n, self);
        // 写回pal，此时pal.palette已恢复为source_palette
        self.palette = pal;
    }
}
