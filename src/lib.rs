// Mario RS - 库入口
// 导出游戏核心模块供外部工具和示例使用

// 平台抽象层
pub mod platform;

// AI 调试桥（仅调试 feature 启用时编译）
#[cfg(feature = "debug-bridge")]
pub mod debug_bridge;

// Android 入口点 - 使用 android-activity crate
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

// 声明 Android 入口点 (Rust 2024 edition 要求 unsafe 属性)
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(app: AndroidApp) {
    platform::android_main(app);
}

// GPU渲染模块 - 包含渲染数据类型和命令
// 注意：该模块始终编译，因为数据类型被游戏逻辑使用
// wgpu特定的渲染器在模块内部条件编译
pub mod gpu;

// CPU软件渲染模块
// - Windows: 用于XP兼容 (cpu-backend feature)
// - Android: 始终编译，作为 GPU 失败时的 fallback
// - 微信小游戏: CPU渲染版本 (wxgame-cpu-backend feature)
#[cfg(any(feature = "cpu-backend", feature = "wxgame-cpu-backend", target_os = "android"))]
pub mod cpu;

// 游戏核心模块
pub mod backgr;
pub mod blocks;
pub mod buffers;
pub mod config;
pub mod context;
pub mod enemies;
pub mod figures;
pub mod game_runner;
pub mod glitter;
pub mod joystick;
pub mod keyboard;
pub mod logging;
pub mod mario;
pub mod mpal256;
pub mod music;
pub mod palettes;
pub mod persist;
pub mod play;
pub mod players;
pub mod render_state;
pub mod renderer;
pub mod sprite_assets;
pub mod sprites;
pub mod stars;
pub mod status;
pub mod tmpobj;
pub mod txt;
pub mod utils;
pub mod worlds;
