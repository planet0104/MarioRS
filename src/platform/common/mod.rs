//! 跨平台公共模块
//!
//! 提取自 android.rs, desktop.rs, windows.rs 中的公共实现
//! 减少重复代码，便于维护和扩展

pub mod time;
pub mod random;
pub mod storage;
pub mod input;
pub mod frame_timer;
pub mod overlay;

// 导出公共类型
pub use time::CommonTime;
pub use random::CommonRandom;
pub use storage::FileStorage;
pub use input::InputState;
pub use frame_timer::FrameTimer;
pub use overlay::draw_fps_to_overlay_rgba;

// 导出全局便捷函数
pub use random::{random_i32, random_usize, random_u32, random_u8, random_f32};
pub use time::now_ms;
