// 游戏运行器模块
//
// 将游戏初始化和主循环逻辑从平台层分离出来
// 平台层只负责提供运行环境，游戏逻辑在此模块中

use crate::gpu::GpuRenderer;
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

        Self { render_state, game }
    }

    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key_event: &crate::platform::KeyEvent) {
        self.game.handle_key_event(key_event);
    }

    /// 帧更新
    pub fn frame_update(&mut self) -> FrameResult {
        self.game.frame_update(&mut self.render_state)
    }

    /// 获取GPU精灵批次数据用于渲染
    pub fn get_sprite_instances(&self) -> Vec<crate::gpu::SpriteInstance> {
        self.render_state.get_sprite_batch().sprite_instances()
    }

    /// 获取GPU填充矩形数据用于渲染
    pub fn get_fill_rects(&self) -> Vec<crate::gpu::FillRect> {
        self.render_state.get_sprite_batch().fill_rects()
    }

    /// 获取GPU UI层填充矩形数据用于渲染
    pub fn get_ui_fill_rects(&self) -> Vec<crate::gpu::FillRect> {
        self.render_state.get_sprite_batch().ui_fill_rects()
    }

    /// 获取当前调色板数据用于GPU渲染
    /// 返回256色RGBA格式数据
    pub fn get_palette_rgba(&self) -> [[u8; 4]; 256] {
        let mut rgba = [[0u8; 4]; 256];
        let palette = &self.render_state.palette.palette;
        for i in 0..256 {
            // VGA调色板是6位色（0-63），需要转换为8位（0-255）
            // 乘以4或使用位移来扩展范围
            rgba[i][0] = (palette[i][0] as u16 * 255 / 63).min(255) as u8;
            rgba[i][1] = (palette[i][1] as u16 * 255 / 63).min(255) as u8;
            rgba[i][2] = (palette[i][2] as u16 * 255 / 63).min(255) as u8;
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

    /// 提交渲染数据到GPU
    ///
    /// 这个方法封装了所有GPU渲染数据的提交逻辑，包括：
    /// 1. 上传调色板
    /// 2. 上传精灵图集
    /// 3. 提交fills、sprites、ui_fills
    /// 4. 渲染到GPU内部纹理
    ///
    /// 平台层只需调用此方法，然后将GPU内部纹理缩放输出到窗口即可
    pub fn submit_to_gpu(&self, gpu: &mut GpuRenderer) {
        // 获取渲染数据
        let sprites = self.get_sprite_instances();
        let fills = self.get_fill_rects();
        let ui_fills = self.get_ui_fill_rects();
        let palette = self.get_palette_rgba();

        // 上传调色板到GPU
        gpu.upload_palette(0, &palette);

        // 上传图集到GPU（BuildWorld 可能会重建/重着色 sprites）
        let (atlas_data, atlas_w, atlas_h) = self.get_atlas_data();
        gpu.upload_atlas(atlas_data, atlas_w, atlas_h);

        // 开始新一帧渲染
        gpu.begin_frame();

        // 添加填充矩形（背景层）
        for fill in fills {
            gpu.draw_fill(fill);
        }

        // 添加精灵（实体层）
        for sprite in sprites {
            gpu.draw_sprite(sprite);
        }

        // 添加UI层填充矩形（状态栏等，在sprites之后渲染）
        for fill in ui_fills {
            gpu.draw_ui_fill(fill);
        }

        // 渲染到GPU内部纹理
        gpu.render_frame();
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
