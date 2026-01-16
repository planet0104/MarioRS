// VGA256 模块 - 纯GPU渲染版本
// 
// 架构说明：
// - 移除CPU framebuffer，所有绘制通过GPU命令收集
// - 保留视口状态管理（x_view, y_view）用于世界坐标转换
// - 保留调色板管理用于GPU着色器

use crate::gpu::sprite_batch::{SpriteBatch, SpriteCommand};
use crate::gpu::texture_atlas::SpriteUV;
use crate::palettes::{PalType, Palettes};

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

/// GPU渲染的VGA模块
/// 移除了CPU framebuffer，使用SpriteBatch收集GPU渲染命令
pub struct VGA {
    pub palette: Palettes,
    pub width: usize,
    pub height: usize,
    pub page: i32,
    pub x_view: i32,
    pub y_view: i32,
    pub y_start: i32,
    pub y_end: i32,
    pub y_offset: i32,
    pub page_offset: i32,
    pub in_graphics_mode: bool,
    pub stack: [i32; 2],
    
    // GPU渲染命令收集器
    pub sprite_batch: SpriteBatch,
}

impl VGA {
    /// 创建VGA对象
    pub fn new(width: usize, height: usize) -> Self {
        let mut palette = Palettes::new();
        palette.palette = [[0; 3]; 256]; // 初始全黑

        let safe = 34 * BYTES_PER_LINE;
        let stack = [PAGE_0 + PAGE_SIZE + safe, PAGE_1 + PAGE_SIZE + safe];

        VGA {
            palette,
            width,
            height,
            page: 0,
            x_view: 0,
            y_view: 0,
            y_start: 0,
            y_end: WINDOWHEIGHT,
            y_offset: 0,
            page_offset: PAGE_0,
            in_graphics_mode: false,
            stack,
            sprite_batch: SpriteBatch::new(),
        }
    }

    /// 创建VGA对象（兼容旧接口）
    pub fn new_offscreen(width: usize, height: usize) -> Self {
        Self::new(width, height)
    }

    // ========================================================================
    // 视口和坐标转换
    // ========================================================================

    /// 将世界坐标转换为屏幕坐标
    #[inline]
    pub fn world_to_screen(&self, x_world: i32, y_world: i32) -> (i32, i32) {
        (x_world - self.x_view, y_world - self.y_view)
    }

    /// 设置视口位置
    pub fn set_view(&mut self, x: i32, y: i32) {
        self.x_view = x;
        self.y_view = y;
    }

    // ========================================================================
    // GPU渲染命令收集
    // ========================================================================

    /// 开始新一帧的GPU渲染
    pub fn begin_gpu_frame(&mut self) {
        self.sprite_batch.clear();
    }

    /// 添加精灵到GPU渲染队列（屏幕坐标）
    pub fn draw_sprite_gpu(&mut self, x: i32, y: i32, uv: SpriteUV) {
        self.sprite_batch.add_sprite(x, y, uv);
    }

    /// 添加精灵到GPU渲染队列（世界坐标）
    pub fn draw_sprite_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_sprite(x, y, uv);
    }

    /// 添加翻转的精灵（屏幕坐标）
    pub fn draw_sprite_flipped_gpu(&mut self, x: i32, y: i32, uv: SpriteUV, flip_x: bool, flip_y: bool) {
        self.sprite_batch.add_sprite_flipped(x, y, uv, flip_x, flip_y);
    }

    /// 添加翻转的精灵（世界坐标）
    pub fn draw_sprite_flipped_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV, flip_x: bool, flip_y: bool) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_sprite_flipped(x, y, uv, flip_x, flip_y);
    }

    /// 添加上下颠倒的精灵（世界坐标）
    pub fn draw_sprite_upside_down_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_sprite_upside_down(x, y, uv);
    }

    /// 添加带调色板偏移的精灵（世界坐标）
    pub fn draw_sprite_recolored_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV, palette_offset: i32) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_sprite_recolored(x, y, uv, palette_offset);
    }

    /// 添加部分可见的精灵（用于升起动画，世界坐标）
    pub fn draw_sprite_partial_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV, visible_height: f32) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.push_sprite_partial(SpriteCommand::new(x, y, uv), visible_height);
    }

    /// 添加填充矩形到GPU渲染队列（屏幕坐标）
    pub fn fill_gpu(&mut self, x: i32, y: i32, w: i32, h: i32, color_index: u8) {
        self.sprite_batch.add_fill(x, y, w, h, color_index);
    }

    /// 添加填充矩形到GPU渲染队列（世界坐标）
    pub fn fill_world_gpu(&mut self, x_world: i32, y_world: i32, w: i32, h: i32, color_index: u8) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_fill(x, y, w, h, color_index);
    }

    /// 设置当前调色板索引
    pub fn set_gpu_palette(&mut self, index: u32) {
        self.sprite_batch.set_palette(index);
    }

    /// 获取收集的精灵批次
    pub fn get_sprite_batch(&self) -> &SpriteBatch {
        &self.sprite_batch
    }

    /// 获取可变的精灵批次
    pub fn get_sprite_batch_mut(&mut self) -> &mut SpriteBatch {
        &mut self.sprite_batch
    }

    // ========================================================================
    // 兼容旧API的空操作方法（GPU模式下不执行任何操作）
    // 这些方法保留是为了让现有代码编译通过，后续逐步移除
    // ========================================================================

    /// 请求窗口重绘（空操作）
    pub fn request_redraw(&self) {}

    /// 翻页（保留页面状态管理）
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

    /// 获取当前页号
    pub fn current_page(&self) -> i32 {
        self.page
    }

    /// 获取页偏移量
    pub fn get_page_offset(&self) -> i32 {
        self.page_offset
    }

    /// 清空（GPU模式下清空渲染队列）
    pub fn clear(&mut self) {
        self.sprite_batch.clear();
    }

    pub fn detect_vga() -> bool { true }
    pub fn get_mode() -> u8 { 0x13 }
    pub fn set_mode(&mut self, new_mode: u8) {
        if new_mode == 0x13 { self.in_graphics_mode = true; }
    }
    pub fn set_width(&mut self, _new_width: i32) {}

    pub fn init_vga(&mut self) {
        self.clear_palette();
        self.set_mode(0x13);
        self.clear_palette();
        self.in_graphics_mode = true;
    }

    pub fn old_mode(&mut self) {
        self.clear_palette();
        self.set_mode(0x03);
        self.in_graphics_mode = false;
    }

    pub fn wait_display(&self) {}
    pub fn wait_retrace(&self) {}
    pub fn present(&mut self) {}

    pub fn set_palette(&mut self, color: u8, red: u8, green: u8, blue: u8) {
        if self.palette.lock_palette { return; }
        let idx = color as usize;
        if idx < self.palette.palette.len() {
            self.palette.palette[idx] = [red, green, blue];
        }
    }

    pub fn read_palette(&mut self, palette: &PalType) {
        for i in 0..256 {
            self.set_palette(i as u8, palette[i][0], palette[i][1], palette[i][2]);
        }
    }

    pub fn clear_palette(&mut self) {
        for c in self.palette.palette.iter_mut() {
            c[0] = 0; c[1] = 0; c[2] = 0;
        }
    }

    pub fn set_viewport(&mut self, x: i32, y: i32, page_nr: i32) {
        self.x_view = x;
        self.y_view = y;
        self.page = page_nr;
    }

    pub fn show_page(&mut self) {
        self.set_viewport(self.x_view, self.y_view, self.page);
    }

    pub fn border(&mut self, _attr: u8) {}
    pub fn set_y_offset(&mut self, new_y_offset: i32) { self.y_offset = new_y_offset; }
    pub fn get_y_offset(&self) -> i32 { self.y_offset }
    pub fn set_y_start(&mut self, y_start: i32) { self.y_start = y_start; }
    pub fn set_y_end(&mut self, y_end: i32) { self.y_end = y_end; }
    pub fn lock_pal(&mut self) { self.palette.lock_pal(); }
    pub fn unlock_pal(&mut self) { self.palette.unlock_pal(); }
    pub fn clear_vga_mem(&mut self) { self.clear(); }

    pub fn reset_stack(&mut self) {
        let safe = 34 * BYTES_PER_LINE;
        self.stack[0] = PAGE_0 + PAGE_SIZE + safe;
        self.stack[1] = PAGE_1 + PAGE_SIZE + safe;
    }

    // ========================================================================
    // 调色板包装方法
    // ========================================================================

    pub fn palette_blink_wrapper(&mut self, options: &crate::buffers::WorldOptions) {
        if self.palette.lock_palette { return; }
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.blink_palette(self, options);
        self.palette = pal;
    }

    pub fn palette_fade_down_wrapper(&mut self, n: u8) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.fade_down(n, self);
        self.palette = pal;
    }

    pub fn palette_fade_up_wrapper(&mut self, n: u8) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.fade_up(n, self);
        self.palette = pal;
    }

    pub fn palette_init(&mut self, p: &crate::palettes::PalType) {
        self.palette.new_palette(p);
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.read_palette(self, p);
        self.palette = pal;
    }

    pub fn palette_clear(&mut self) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.clear_palette(self);
        self.palette = pal;
    }

    pub fn palette_fade_step(&mut self) {
        if let Some(temp_pal) = self.palette.fade_step() {
            if !self.palette.lock_palette {
                for i in 0..256 {
                    self.palette.palette[i] = temp_pal[i];
                }
            }
        }
    }

    pub fn palette_init_grass(&mut self, options: &crate::buffers::WorldOptions) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.init_grass(self, options);
        self.palette = pal;
    }

    pub fn palette_out(&mut self, color: usize, r: u8, g: u8, b: u8) {
        let was_locked = self.palette.lock_palette;
        let mut pal = std::mem::take(&mut self.palette);
        pal.lock_palette = was_locked;
        self.palette.lock_palette = was_locked;
        pal.out_palette(color, r, g, b, self);
        self.palette = pal;
    }

    // ========================================================================
    // 水平滚动（GPU模式下只更新视口，不移动像素）
    // ========================================================================

    pub fn scroll_screen_x(&mut self, _dx: i32) {
        // GPU模式下不需要移动像素，每帧完整重绘
    }

    // ========================================================================
    // 以下是保留的兼容性空方法，供编译通过
    // 实际绘制已移至GPU
    // ========================================================================

    /// 像素写入（空操作，GPU模式下使用fill_gpu）
    pub fn put_pixel(&mut self, _x: i32, _y: i32, _attr: u8) {}
    pub fn put_pixel_world(&mut self, _x: i32, _y: i32, _attr: u8) {}
    
    /// 像素读取（始终返回0）
    pub fn get_pixel(&self, _x: i32, _y: i32) -> u8 { 0 }
    pub fn get_pixel_world(&self, _x: i32, _y: i32) -> u8 { 0 }
    
    /// 填充矩形（转发到GPU）
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, attr: u8) {
        self.fill_gpu(x, y, w, h, attr);
    }
    pub fn fill_world(&mut self, x_world: i32, y_world: i32, w: i32, h: i32, attr: u8) {
        self.fill_world_gpu(x_world, y_world, w, h, attr);
    }

    /// 图像绘制（空操作，使用draw_sprite_*_gpu代替）
    pub fn draw_image(&mut self, _xpos: i32, _ypos: i32, _width: i32, _height: i32, _bitmap: &[u8]) {}
    pub fn draw_image_world(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &[u8]) {}
    pub fn put_image(&mut self, _xpos: i32, _ypos: i32, _width: i32, _height: i32, _bitmap: &[u8]) {}
    pub fn put_image_world(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &[u8]) {}
    
    pub fn draw_image_imagebuffer<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    pub fn draw_image_imagebuffer_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    pub fn put_image_imagebuffer<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    pub fn put_image_imagebuffer_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    
    pub fn draw_image_imagebuffer_partial<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _w: usize, _h: usize, _bitmap: &[[u8; WW]; HH]) {}
    pub fn put_image_imagebuffer_partial<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _w: usize, _h: usize, _bitmap: &[[u8; WW]; HH]) {}
    pub fn put_image_imagebuffer_partial_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _w: usize, _h: usize, _bitmap: &[[u8; WW]; HH]) {}
    
    pub fn draw_part_imagebuffer<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _y1: usize, _y2: usize, _bitmap: &[[u8; WW]; HH]) {}
    pub fn draw_part_imagebuffer_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _y1: usize, _y2: usize, _bitmap: &[[u8; WW]; HH]) {}
    
    pub fn up_side_down_imagebuffer<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    pub fn up_side_down_imagebuffer_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH]) {}
    pub fn up_side_down(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &[u8]) {}
    
    pub fn draw_part(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _y1: i32, _y2: i32, _bitmap: &[u8]) {}
    
    pub fn recolor_image<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH], _color: i32) {}
    pub fn recolor_image_world<const WW: usize, const HH: usize>(&mut self, _x: i32, _y: i32, _bitmap: &[[u8; WW]; HH], _color: i32) {}
    pub fn recolor_image_pascal(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &[u8], _diff: u8) {}
    
    pub fn draw_sprite<const W: usize, const H: usize>(&mut self, _x: i32, _y: i32, _sprite: &crate::sprites::Sprite<W, H>) {}
    pub fn draw_sprite_world<const W: usize, const H: usize>(&mut self, _x: i32, _y: i32, _sprite: &crate::sprites::Sprite<W, H>) {}
    
    pub fn get_image(&self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &mut [u8]) {}
    pub fn get_image_world(&self, _x: i32, _y: i32, _w: i32, _h: i32, _bitmap: &mut [u8]) {}
    pub fn get_image_imagebuffer<const WW: usize, const HH: usize>(&self, _x: i32, _y: i32, _buf: &mut [[u8; WW]; HH]) {}
    pub fn get_image_imagebuffer_world<const WW: usize, const HH: usize>(&self, _x: i32, _y: i32, _buf: &mut [[u8; WW]; HH]) {}
    
    pub fn draw_bitmap(&mut self, _x: i32, _y: i32, _bitmap: &[u8], _attr: u8) {}

    /// 背景保存/恢复（空操作，GPU每帧完整重绘）
    pub fn push_backgr(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Vec<u8> { Vec::new() }
    pub fn push_backgr_world(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Vec<u8> { Vec::new() }
    pub fn push_backgr_address(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) -> i32 { 0 }
    pub fn push_backgr_address_world(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) -> i32 { 0 }
    pub fn pop_backgr(&mut self, _buf: &Vec<u8>) {}
    pub fn pop_backgr_address(&mut self, _address: i32) {}

    // ========================================================================
    // render_to_rgba保留用于过渡期（返回黑色画面）
    // ========================================================================
    
    /// 渲染到RGBA（过渡用，返回黑色）
    pub fn render_to_rgba(&self, output: &mut [u8]) {
        // GPU模式下framebuffer为空，填充黑色
        for chunk in output.chunks_exact_mut(4) {
            chunk[0] = 0; chunk[1] = 0; chunk[2] = 0; chunk[3] = 255;
        }
    }
}

// 兼容旧代码的BackgroundData结构
#[derive(Clone)]
pub struct BackgroundData {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}
