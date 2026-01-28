// ============================================================================
// CPU软件渲染器
// ============================================================================
//
// 将RenderCommand渲染到帧缓冲
//
// 特点:
// - 纯CPU渲染，无GPU依赖
// - 支持索引色精灵和调色板
// - 支持翻转、透明、调色板偏移等效果
//
// 像素格式（条件编译）:
// - Windows (cpu-backend): BGRA格式，用于GDI StretchDIBits
// - Android (android-cpu): ABGR格式（小端序），用于ANativeWindow RGBA_8888
//
// ============================================================================

/// 游戏画面宽度
pub const GAME_WIDTH: u32 = 320;
/// 游戏画面高度
pub const GAME_HEIGHT: u32 = 182;

/// CPU软件渲染器
pub struct CpuRenderer {
    /// 帧缓冲 (格式由平台决定: Windows=BGRA, Android=ABGR)
    framebuffer: Vec<u8>,
    /// 精灵图集数据 (R8索引色格式)
    atlas: Vec<u8>,
    /// 图集尺寸
    atlas_size: u32,
    /// 调色板 (256色RGBA)
    palette: [[u8; 4]; 256],
    /// 屏幕宽度
    width: u32,
    /// 屏幕高度
    height: u32,
}

impl CpuRenderer {
    /// 创建新的CPU渲染器
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: vec![0u8; (width * height * 4) as usize],
            atlas: Vec::new(),
            atlas_size: 0,
            palette: [[0u8; 4]; 256],
            width,
            height,
        }
    }

    /// 上传精灵图集
    pub fn upload_atlas(&mut self, data: &[u8], size: u32) {
        self.atlas = data.to_vec();
        self.atlas_size = size;
    }

    /// 上传调色板
    pub fn upload_palette(&mut self, palette: &[[u8; 4]; 256]) {
        self.palette = *palette;
    }

    /// 清空帧缓冲为黑色
    pub fn clear(&mut self) {
        self.framebuffer.fill(0);
    }

    /// 获取帧缓冲数据（用于GDI显示）
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    /// 获取屏幕宽度
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 获取屏幕高度
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 写入单个像素到帧缓冲
    #[inline]
    #[allow(dead_code)]
    fn put_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let offset = ((y as u32 * self.width + x as u32) * 4) as usize;
        if offset + 3 < self.framebuffer.len() {
            #[cfg(feature = "android-cpu")]
            {
                // ABGR格式 (Android RGBA_8888 小端序)
                self.framebuffer[offset] = color[0];     // R
                self.framebuffer[offset + 1] = color[1]; // G
                self.framebuffer[offset + 2] = color[2]; // B
                self.framebuffer[offset + 3] = 0xFF;     // A
            }
            
            #[cfg(not(feature = "android-cpu"))]
            {
                // BGRA格式（Windows GDI原生格式）
                self.framebuffer[offset] = color[2];     // B
                self.framebuffer[offset + 1] = color[1]; // G
                self.framebuffer[offset + 2] = color[0]; // R
                self.framebuffer[offset + 3] = color[3]; // A
            }
        }
    }

    /// 绘制填充矩形（优化版本）
    /// 
    /// 优化策略:
    /// 1. 提前裁剪
    /// 2. 使用行批量填充
    /// 3. 移除循环内边界检查
    pub fn draw_fill(&mut self, x: f32, y: f32, w: f32, h: f32, color_index: u8) {
        let color = self.palette[color_index as usize];
        let x0 = x as i32;
        let y0 = y as i32;
        let x1 = (x + w) as i32;
        let y1 = (y + h) as i32;

        // 裁剪到屏幕范围
        let x_start = x0.max(0) as u32;
        let y_start = y0.max(0) as u32;
        let x_end = (x1 as u32).min(self.width);
        let y_end = (y1 as u32).min(self.height);

        if x_start >= x_end || y_start >= y_end {
            return;
        }

        // 平台特定像素格式
        #[cfg(feature = "android-cpu")]
        let pixel_bytes = [color[0], color[1], color[2], 0xFF]; // ABGR (Android RGBA_8888 小端序: R G B A)
        
        #[cfg(not(feature = "android-cpu"))]
        let pixel_bytes = [color[2], color[1], color[0], color[3]]; // BGRA (Windows GDI)
        
        let fb_stride = self.width * 4;
        let fill_width = (x_end - x_start) as usize;

        // 逐行填充
        for py in y_start..y_end {
            let row_offset = (py * fb_stride + x_start * 4) as usize;
            let row_end = row_offset + fill_width * 4;
            
            // 批量填充整行
            let row_slice = &mut self.framebuffer[row_offset..row_end];
            for chunk in row_slice.chunks_exact_mut(4) {
                chunk.copy_from_slice(&pixel_bytes);
            }
        }
    }

    /// 绘制精灵（优化版本）
    ///
    /// 优化策略:
    /// 1. 提前裁剪完全在屏幕外的精灵
    /// 2. 计算可见区域，只处理可见像素
    /// 3. 预计算行偏移，减少循环内计算
    /// 4. 内联像素写入，避免函数调用开销
    ///
    /// 参数:
    /// - x, y: 屏幕位置
    /// - uv_x, uv_y: 图集中的位置
    /// - uv_w, uv_h: 精灵尺寸
    /// - flip_x, flip_y: 翻转标志
    /// - opaque: 是否不透明绘制（true时索引0也绘制）
    /// - palette_offset: 调色板偏移
    pub fn draw_sprite(
        &mut self,
        x: f32,
        y: f32,
        uv_x: u32,
        uv_y: u32,
        uv_w: u32,
        uv_h: u32,
        flip_x: bool,
        flip_y: bool,
        opaque: bool,
        palette_offset: i32,
    ) {
        if self.atlas.is_empty() || self.atlas_size == 0 {
            return;
        }

        let dst_x = x as i32;
        let dst_y = y as i32;
        let width = self.width as i32;
        let height = self.height as i32;
        let uv_w_i = uv_w as i32;
        let uv_h_i = uv_h as i32;

        // 优化1: 提前裁剪 - 完全在屏幕外则跳过
        if dst_x + uv_w_i <= 0 || dst_x >= width ||
           dst_y + uv_h_i <= 0 || dst_y >= height {
            return;
        }

        // 优化2: 计算可见区域（裁剪到屏幕范围）
        let x_start = if dst_x < 0 { (-dst_x) as u32 } else { 0 };
        let y_start = if dst_y < 0 { (-dst_y) as u32 } else { 0 };
        let x_end = if dst_x + uv_w_i > width { (width - dst_x) as u32 } else { uv_w };
        let y_end = if dst_y + uv_h_i > height { (height - dst_y) as u32 } else { uv_h };

        let atlas_stride = self.atlas_size;
        let fb_stride = self.width * 4;

        // 优化3: 预取调色板偏移计算
        let pal_offset = palette_offset & 0xFF;

        for dy in y_start..y_end {
            // 预计算Y相关偏移
            let src_y = if flip_y { uv_h - 1 - dy } else { dy };
            let py = (dst_y + dy as i32) as u32;
            let fb_row_offset = (py * fb_stride) as usize;
            let atlas_row = (uv_y + src_y) * atlas_stride + uv_x;

            for dx in x_start..x_end {
                // 处理翻转
                let src_x = if flip_x { uv_w - 1 - dx } else { dx };

                // 从图集读取颜色索引（边界检查已通过裁剪保证）
                let atlas_idx = (atlas_row + src_x) as usize;
                let color_idx = self.atlas[atlas_idx];

                // 透明处理
                if color_idx == 0 && !opaque {
                    continue;
                }

                // 应用调色板偏移
                let final_idx = ((color_idx as i32 + pal_offset) & 0xFF) as usize;
                let color = self.palette[final_idx];

                // 优化4: 内联像素写入（避免put_pixel函数调用）
                let px = (dst_x + dx as i32) as u32;
                let offset = fb_row_offset + (px * 4) as usize;
                
                // 平台特定像素格式
                #[cfg(feature = "android-cpu")]
                {
                    // ABGR格式 (Android RGBA_8888 小端序: R G B A)
                    self.framebuffer[offset] = color[0];     // R
                    self.framebuffer[offset + 1] = color[1]; // G
                    self.framebuffer[offset + 2] = color[2]; // B
                    self.framebuffer[offset + 3] = 0xFF;     // A
                }
                
                #[cfg(not(feature = "android-cpu"))]
                {
                    // BGRA格式 (Windows GDI)
                    self.framebuffer[offset] = color[2];     // B
                    self.framebuffer[offset + 1] = color[1]; // G
                    self.framebuffer[offset + 2] = color[0]; // R
                    self.framebuffer[offset + 3] = color[3]; // A
                }
            }
        }
    }

    /// 绘制精灵（部分可见，用于升起动画）
    pub fn draw_sprite_partial(
        &mut self,
        x: f32,
        y: f32,
        uv_x: u32,
        uv_y: u32,
        uv_w: u32,
        uv_h: u32,
        visible_height: f32,
        flip_x: bool,
        flip_y: bool,
        opaque: bool,
        palette_offset: i32,
    ) {
        if visible_height <= 0.0 {
            return;
        }

        let clipped_h = (visible_height as u32).min(uv_h);
        self.draw_sprite(
            x,
            y,
            uv_x,
            uv_y,
            uv_w,
            clipped_h,
            flip_x,
            flip_y,
            opaque,
            palette_offset,
        );
    }
}

impl Default for CpuRenderer {
    fn default() -> Self {
        Self::new(GAME_WIDTH, GAME_HEIGHT)
    }
}
