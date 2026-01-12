// 游戏运行器模块
//
// 将游戏初始化和主循环逻辑从平台层分离出来
// 平台层只负责提供运行环境，游戏逻辑在此模块中

use crate::mario::MarioGame;
use crate::platform::FrameResult;
use crate::vga256::{VGA, SCREEN_WIDTH, WINDOWHEIGHT};

// 导出游戏窗口尺寸供平台层使用（平台层不应直接引用 vga256 模块）
pub const GAME_WIDTH: u32 = SCREEN_WIDTH as u32;
pub const GAME_HEIGHT: u32 = WINDOWHEIGHT as u32;

/// 游戏应用状态
pub struct GameState {
    pub vga: VGA,
    pub game: MarioGame,
}

impl GameState {
    /// 创建新的游戏状态
    pub fn new() -> Self {
        let mut vga = VGA::new_offscreen(
            SCREEN_WIDTH as usize,
            WINDOWHEIGHT as usize,
        );
        let mut game = MarioGame::new();
        game.init_palette(&mut vga);
        
        Self { vga, game }
    }
    
    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key_event: &crate::platform::KeyEvent) {
        self.game.handle_key_event(key_event);
    }
    
    /// 帧更新
    pub fn frame_update(&mut self) -> FrameResult {
        self.game.frame_update(&mut self.vga)
    }
    
    /// 将VGA framebuffer渲染到RGBA缓冲区
    pub fn render_to_rgba(&self, output: &mut [u8]) {
        self.vga.render_to_rgba(output);
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
