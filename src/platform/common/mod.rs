//! 跨平台公共模块
//!
//! 提取自 android.rs, desktop.rs, windows.rs 中的公共实现
//! 减少重复代码，便于维护和扩展

pub mod frame_timer;
pub mod input;
pub mod random;
pub mod storage;
pub mod time;

// 导出公共类型
pub use frame_timer::{FpsCounter, FrameTimer};
pub use input::InputState;
pub use random::CommonRandom;
pub use storage::FileStorage;
pub use time::CommonTime;

// 导出全局便捷函数
pub use random::{random_f32, random_i32, random_u8, random_u32, random_usize};
pub use time::now_ms;
