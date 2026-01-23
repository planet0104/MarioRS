// 游戏运行器模块
//
// 将游戏初始化和主循环逻辑从平台层分离出来
// 平台层只负责提供运行环境，游戏逻辑在此模块中

#[cfg(feature = "wgpu-backend")]
use crate::gpu::GpuRenderer;
#[cfg(feature = "cpu-backend")]
use crate::cpu::CpuRenderer;
use crate::mario::MarioGame;
use crate::platform::FrameResult;
use crate::render_state::{RenderState, SCREEN_WIDTH, WINDOWHEIGHT};

// 导出游戏窗口尺寸供平台层使用（平台层不应直接引用 render_state 模块）
pub const GAME_WIDTH: u32 = SCREEN_WIDTH as u32;
pub const GAME_HEIGHT: u32 = WINDOWHEIGHT as u32;

/// 游戏应用状态
pub struct GameState {
    pub render_state: RenderState,
    pub game: MarioGame,

    // GPU资源上传优化标志
    /// 上次上传的图集版本号（用于检测变化）
    last_atlas_version: u64,
    /// 上一帧的调色板数据（用于检测变化）
    last_palette: [[u8; 3]; 256],
}

impl GameState {
    /// 创建新的游戏状态
    pub fn new() -> Self {
        eprintln!("[DEBUG] GameState::new: 创建VGA");
        let mut render_state =
            RenderState::new_offscreen(SCREEN_WIDTH as usize, WINDOWHEIGHT as usize);
        eprintln!("[DEBUG] GameState::new: VGA创建完成，创建MarioGame");
        let mut game = MarioGame::new();
        eprintln!("[DEBUG] GameState::new: MarioGame创建完成");
        game.init_palette(&mut render_state);
        eprintln!("[DEBUG] GameState::new: 调色板初始化完成");

        Self {
            render_state,
            game,
            last_atlas_version: 0, // 初始版本不匹配，确保首次上传
            last_palette: [[0u8; 3]; 256],
        }
    }

    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key_event: &crate::platform::KeyEvent) {
        self.game.handle_key_event(key_event);
    }

    /// 帧更新
    pub fn frame_update(&mut self) -> FrameResult {
        self.game.frame_update(&mut self.render_state)
    }

    /// 设置FPS显示数据（由平台层调用）
    /// FPS将显示在游戏状态栏中，使用GPU渲染
    pub fn set_fps_display(&mut self, fps: u32, frame_time_ms: f32) {
        self.game.set_fps_display(fps, frame_time_ms);
    }

    /// 使GPU资源缓存失效，强制下次submit_to_gpu时重新上传所有资源
    /// 用于从后台恢复时，GPU渲染器被重建的情况
    #[cfg(feature = "wgpu-backend")]
    pub fn invalidate_gpu_resources(&mut self) {
        self.last_atlas_version = 0;
        self.last_palette = [[0u8; 3]; 256];
    }

    /// 获取GPU精灵批次数据用于渲染
    #[cfg(feature = "wgpu-backend")]
    pub fn get_sprite_instances(&self) -> Vec<crate::gpu::SpriteInstance> {
        self.render_state.get_sprite_batch().sprite_instances()
    }

    /// 获取GPU填充矩形数据用于渲染
    #[cfg(feature = "wgpu-backend")]
    pub fn get_fill_rects(&self) -> Vec<crate::gpu::FillRect> {
        self.render_state.get_sprite_batch().fill_rects()
    }

    /// 获取GPU UI层填充矩形数据用于渲染
    #[cfg(feature = "wgpu-backend")]
    pub fn get_ui_fill_rects(&self) -> Vec<crate::gpu::FillRect> {
        self.render_state.get_sprite_batch().ui_fill_rects()
    }

    /// 获取当前调色板数据用于GPU渲染
    /// 返回256色RGBA格式数据
    pub fn get_palette_rgba(&self) -> [[u8; 4]; 256] {
        let mut rgba = [[0u8; 4]; 256];
        let palette = &self.render_state.palette.palette;
        for i in 0..256 {
            // 内部调色板使用 6-bit VGA 值 (0-63)，输出到 GPU 时转换为 8-bit (0-255)
            rgba[i][0] = ((palette[i][0] as u16) * 255 / 63).min(255) as u8;
            rgba[i][1] = ((palette[i][1] as u16) * 255 / 63).min(255) as u8;
            rgba[i][2] = ((palette[i][2] as u16) * 255 / 63).min(255) as u8;
            rgba[i][3] = 255; // 不透明
        }
        rgba
    }

    /// 获取精灵图集数据用于GPU上传
    /// 返回(data, width, height)
    pub fn get_atlas_data(&self) -> (&[u8], u32, u32) {
        let atlas = &self.game.atlas;
        let size = atlas.size();
        (atlas.data(), size, size)
    }

    /// 准备渲染数据（不执行渲染）- GPU后端
    ///
    /// 性能优化：
    /// 1. 只在图集版本变化时上传（关卡切换时build_atlas会递增版本号）
    /// 2. 只在调色板变化时上传（淡入淡出效果时）
    /// 3. 直接传递SpriteBatch引用避免Vec分配
    ///
    /// 调用此方法后，平台层应调用 gpu.render_frame_and_present() 一次性完成渲染
    #[cfg(feature = "wgpu-backend")]
    pub fn submit_to_gpu(&mut self, gpu: &mut GpuRenderer) {
        // 检查图集版本是否变化（关卡切换时会重建图集）
        let current_atlas_version = self.game.atlas.version();
        if current_atlas_version != self.last_atlas_version {
            let (atlas_data, atlas_w, atlas_h) = self.get_atlas_data();
            gpu.upload_atlas(atlas_data, atlas_w, atlas_h);
            self.last_atlas_version = current_atlas_version;
        }

        // 检查调色板是否变化
        let current_palette = &self.render_state.palette.palette;
        if current_palette != &self.last_palette {
            let palette_rgba = self.get_palette_rgba();
            gpu.upload_palette(0, &palette_rgba);
            self.last_palette = *current_palette;
        }

        // 开始新一帧渲染（只准备数据，不执行渲染）
        gpu.begin_frame();

        // 直接从SpriteBatch获取数据并提交（避免额外Vec分配）
        let batch = self.render_state.get_sprite_batch();

        // 添加填充矩形（背景层）
        for fill in batch.fills_iter() {
            gpu.draw_fill(fill.to_fill_rect());
        }

        // 添加精灵（实体层）
        for sprite in batch.sprites_iter() {
            gpu.draw_sprite(sprite.to_instance());
        }

        // 添加直接实例
        for inst in batch.instances_iter() {
            gpu.draw_sprite(*inst);
        }

        // 添加UI层填充矩形（状态栏等，在sprites之后渲染）
        for fill in batch.ui_fills_iter() {
            gpu.draw_ui_fill(fill.to_fill_rect());
        }

        // 注意：不再调用 render_frame()
        // 由平台层调用 render_frame_and_present() 完成渲染和呈现
    }

    /// 准备渲染数据（不执行渲染）- CPU后端
    ///
    /// 将渲染命令提交到CPU渲染器，渲染到BGRA帧缓冲
    /// 调用此方法后，平台层应使用GDI显示帧缓冲
    #[cfg(feature = "cpu-backend")]
    pub fn submit_to_cpu(&mut self, cpu: &mut CpuRenderer) {
        // 检查图集版本是否变化
        let current_atlas_version = self.game.atlas.version();
        if current_atlas_version != self.last_atlas_version {
            let (atlas_data, atlas_size, _) = self.get_atlas_data();
            cpu.upload_atlas(atlas_data, atlas_size);
            self.last_atlas_version = current_atlas_version;
        }

        // 检查调色板是否变化
        let current_palette = &self.render_state.palette.palette;
        if current_palette != &self.last_palette {
            let palette_rgba = self.get_palette_rgba();
            cpu.upload_palette(&palette_rgba);
            self.last_palette = *current_palette;
        }

        // 清空帧缓冲
        cpu.clear();

        // 直接从SpriteBatch获取数据并渲染
        let batch = self.render_state.get_sprite_batch();

        // 渲染填充矩形（背景层）
        for fill in batch.fills_iter() {
            cpu.draw_fill(fill.x, fill.y, fill.width, fill.height, fill.color_index);
        }

        // 渲染精灵（实体层）
        for sprite in batch.sprites_iter() {
            let uv = &sprite.uv;
            cpu.draw_sprite(
                sprite.x,
                sprite.y,
                uv.x,
                uv.y,
                uv.width,
                uv.height,
                sprite.flip_x,
                sprite.flip_y,
                sprite.opaque,
                sprite.palette_offset,
            );
        }

        // 渲染直接实例（需要从SpriteInstance转换回UV坐标）
        // 注意：instances使用归一化UV，需要转换回像素坐标
        let atlas_size = self.game.atlas.size();
        for inst in batch.instances_iter() {
            let uv_x = (inst.uv_offset[0] * atlas_size as f32) as u32;
            let uv_y = (inst.uv_offset[1] * atlas_size as f32) as u32;
            let uv_w = inst.size[0] as u32;
            let uv_h = inst.size[1] as u32;
            cpu.draw_sprite(
                inst.position[0],
                inst.position[1],
                uv_x,
                uv_y,
                uv_w,
                uv_h,
                inst.flip[0] != 0.0,
                inst.flip[1] != 0.0,
                inst.opaque != 0.0,
                inst.palette_offset as i32,
            );
        }

        // 渲染UI层填充矩形（状态栏等）
        for fill in batch.ui_fills_iter() {
            cpu.draw_fill(fill.x, fill.y, fill.width, fill.height, fill.color_index);
        }
    }

    /// 请求退出
    pub fn request_quit(&mut self) {
        self.game.request_quit();
    }

    /// 关闭游戏（保存配置等清理工作）
    pub fn shutdown(&mut self) {
        self.game.shutdown();
    }

    /// 获取显示宽度
    pub fn width(&self) -> u32 {
        SCREEN_WIDTH as u32
    }

    /// 获取显示高度
    pub fn height(&self) -> u32 {
        WINDOWHEIGHT as u32
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// 打印游戏启动信息
pub fn print_startup_info() {
    println!("========================================");
    println!("    Mario RS - Rust重制版马里奥游戏");
    println!("========================================");
    println!();
    println!("    游戏控制说明：");
    println!("    方向键     - 移动");
    println!("    Alt/空格   - 跳跃");
    println!("    P键        - 暂停（暂停后按Tab输入作弊码）");
    println!("    S键        - 切换状态栏");
    println!("    ESC        - 退出");
    println!("========================================");
}
