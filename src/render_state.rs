// RenderState 模块 - GPU渲染状态管理
//
// 架构说明：
// - 管理视口状态（x_view, y_view）用于世界坐标转换
// - 管理调色板数据用于GPU着色器
// - 收集GPU渲染命令（SpriteBatch）

use crate::gpu::sprite_batch::{SpriteBatch, SpriteCommand};
use crate::gpu::texture_atlas::SpriteUV;
use crate::palettes::{PalType, Palettes};

// 屏幕尺寸常量
pub const WINDOWHEIGHT: i32 = 13 * 14;
pub const WINDOWWIDTH: i32 = 16 * 20;
pub const SCREEN_WIDTH: i32 = 320;
pub const SCREEN_HEIGHT: i32 = 200;
pub const VIR_SCREEN_WIDTH: i32 = SCREEN_WIDTH + 2 * 20;
pub const VIR_SCREEN_HEIGHT: i32 = 182;
pub const BYTES_PER_LINE: i32 = VIR_SCREEN_WIDTH / 4;

// 页面常量（用于双缓冲状态管理）
pub const MAX_PAGE: i32 = 1;
pub const PAGE_SIZE: i32 = (VIR_SCREEN_HEIGHT + 24) * BYTES_PER_LINE;
pub const PAGE_0: i32 = 0;
pub const PAGE_1: i32 = 0x8000;
pub const YBASE: i32 = 9;

/// GPU渲染状态管理
/// 管理视口、调色板和渲染命令收集
pub struct RenderState {
    pub palette: Palettes,
    pub page: i32,
    pub x_view: i32,
    pub y_view: i32,
    pub y_start: i32,
    pub y_end: i32,
    pub y_offset: i32,
    pub page_offset: i32,
    pub stack: [i32; 2],

    // GPU渲染命令收集器
    pub sprite_batch: SpriteBatch,
}

impl RenderState {
    /// 创建RenderState对象
    pub fn new(_width: usize, _height: usize) -> Self {
        let mut palette = Palettes::new();
        palette.palette = [[0; 3]; 256]; // 初始全黑

        let safe = 34 * BYTES_PER_LINE;
        let stack = [PAGE_0 + PAGE_SIZE + safe, PAGE_1 + PAGE_SIZE + safe];

        RenderState {
            palette,
            page: 0,
            x_view: 0,
            y_view: 0,
            y_start: 0,
            y_end: WINDOWHEIGHT,
            y_offset: 0,
            page_offset: PAGE_0,
            stack,
            sprite_batch: SpriteBatch::new(),
        }
    }

    /// 创建RenderState对象（兼容旧接口）
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
    pub fn draw_sprite_flipped_gpu(
        &mut self,
        x: i32,
        y: i32,
        uv: SpriteUV,
        flip_x: bool,
        flip_y: bool,
    ) {
        self.sprite_batch
            .add_sprite_flipped(x, y, uv, flip_x, flip_y);
    }

    /// 添加翻转的精灵（世界坐标）
    pub fn draw_sprite_flipped_world_gpu(
        &mut self,
        x_world: i32,
        y_world: i32,
        uv: SpriteUV,
        flip_x: bool,
        flip_y: bool,
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch
            .add_sprite_flipped(x, y, uv, flip_x, flip_y);
    }

    /// 添加上下颠倒的精灵（世界坐标）
    pub fn draw_sprite_upside_down_world_gpu(&mut self, x_world: i32, y_world: i32, uv: SpriteUV) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch.add_sprite_upside_down(x, y, uv);
    }

    /// 添加带调色板偏移的精灵（世界坐标）
    pub fn draw_sprite_recolored_world_gpu(
        &mut self,
        x_world: i32,
        y_world: i32,
        uv: SpriteUV,
        palette_offset: i32,
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch
            .add_sprite_recolored(x, y, uv, palette_offset);
    }

    /// 添加部分可见的精灵（用于升起动画，世界坐标）
    pub fn draw_sprite_partial_world_gpu(
        &mut self,
        x_world: i32,
        y_world: i32,
        uv: SpriteUV,
        visible_height: f32,
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.sprite_batch
            .push_sprite_partial(SpriteCommand::new(x, y, uv), visible_height);
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

    /// 添加UI层填充矩形到GPU渲染队列（屏幕坐标）
    /// UI层在所有sprites之后渲染，用于状态栏、暂停文本等
    pub fn fill_ui_gpu(&mut self, x: i32, y: i32, w: i32, h: i32, color_index: u8) {
        use crate::gpu::sprite_batch::FillCommand;
        self.sprite_batch
            .push_ui_fill(FillCommand::new(x, y, w, h, color_index));
    }

    /// 添加UI层填充矩形到GPU渲染队列（世界坐标）
    /// UI层在所有sprites之后渲染，用于状态栏、暂停文本等
    pub fn fill_ui_world_gpu(
        &mut self,
        x_world: i32,
        y_world: i32,
        w: i32,
        h: i32,
        color_index: u8,
    ) {
        let (x, y) = self.world_to_screen(x_world, y_world);
        self.fill_ui_gpu(x, y, w, h, color_index);
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

    /// 清空渲染队列
    pub fn clear(&mut self) {
        self.sprite_batch.clear();
    }

    pub fn set_palette(&mut self, color: u8, red: u8, green: u8, blue: u8) {
        if self.palette.lock_palette {
            return;
        }
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
            c[0] = 0;
            c[1] = 0;
            c[2] = 0;
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

    pub fn set_y_offset(&mut self, new_y_offset: i32) {
        self.y_offset = new_y_offset;
    }
    pub fn get_y_offset(&self) -> i32 {
        self.y_offset
    }
    pub fn set_y_start(&mut self, y_start: i32) {
        self.y_start = y_start;
    }
    pub fn set_y_end(&mut self, y_end: i32) {
        self.y_end = y_end;
    }
    pub fn lock_pal(&mut self) {
        self.palette.lock_pal();
    }
    pub fn unlock_pal(&mut self) {
        self.palette.unlock_pal();
    }
    pub fn clear_vga_mem(&mut self) {
        self.clear();
    }

    pub fn reset_stack(&mut self) {
        let safe = 34 * BYTES_PER_LINE;
        self.stack[0] = PAGE_0 + PAGE_SIZE + safe;
        self.stack[1] = PAGE_1 + PAGE_SIZE + safe;
    }

    // ========================================================================
    // 调色板包装方法
    // ========================================================================

    pub fn palette_blink_wrapper(&mut self, options: &crate::buffers::WorldOptions) {
        if self.palette.lock_palette {
            return;
        }
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
}
