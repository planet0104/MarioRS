// Mario RS - 库入口
// 导出游戏核心模块供外部工具和示例使用

// 平台抽象层
pub mod platform;

// 游戏核心模块
pub mod buffers;
pub mod config;
pub mod context;
pub mod enemies;
pub mod game_runner;
pub mod play;
pub mod players;
pub mod utils;
pub mod persist;
pub mod logging;
pub mod vga256;
pub mod backgr;
pub mod blocks;
pub mod figures;
pub mod glitter;
pub mod joystick;
pub mod keyboard;
pub mod mario;
pub mod mpal256;
pub mod music;
pub mod palettes;
pub mod renderer;
pub mod sprites;
pub mod stars;
pub mod status;
pub mod tmpobj;
pub mod txt;
pub mod worlds;
