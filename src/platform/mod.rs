//! 平台抽象层
//!
//! 只抽象真正与平台相关的底层接口：
//! - 显示输出（framebuffer -> 屏幕）
//! - 音频输出
//! - 时间获取
//! - 随机数生成
//! - 持久化存储
//! - 日志输出
//! - 键盘事件获取
//!
//! 调色板、视口控制、背景保存等都是游戏逻辑，在 RenderState/Buffers 模块内用纯 Rust 实现

// ============================================================================
// 子模块声明
// ============================================================================

pub mod audio;
pub mod common;

// Windows 手柄支持 (使用 winmm.dll)
#[cfg(target_os = "windows")]
pub mod joystick_win;

// Android TV 遥控器支持
#[cfg(target_os = "android")]
pub mod joystick_android_tv;

// Windows GDI + wgpu 后端（需要同时启用 gdi-backend 和 wgpu-backend）
#[cfg(all(target_os = "windows", feature = "gdi-backend", feature = "wgpu-backend", not(feature = "cpu-backend")))]
mod windows;

// Windows CPU 软件渲染后端（XP兼容）
#[cfg(all(target_os = "windows", feature = "cpu-backend", not(feature = "wgpu-backend")))]
mod windows_cpu;

#[cfg(feature = "wgpu-backend")]
mod desktop;

// Android 后端（需要 wgpu-backend）
#[cfg(all(target_os = "android", feature = "wgpu-backend", not(feature = "android-cpu")))]
mod android;

// Android CPU 软件渲染后端（老旧设备兼容）
#[cfg(all(target_os = "android", feature = "android-cpu"))]
mod android_cpu;

// ============================================================================
// 显示后端 - 最底层的渲染抽象
// ============================================================================

/// 显示后端接口（纯GPU模式）
///
/// 不同平台的实现：
/// - Desktop: wgpu (GPU加速)
/// - Windows: wgpu (GPU加速)
/// - Android: wgpu (GPU加速)
/// - Web: WebGPU
pub trait DisplayBackend {
    /// 获取显示尺寸
    fn width(&self) -> u32;
    fn height(&self) -> u32;

    /// 提交渲染到屏幕显示
    fn present(&mut self) -> Result<(), String>;

    /// 请求窗口重绘
    fn request_redraw(&self);
}

// ============================================================================
// 音频后端 - 声音输出抽象
// ============================================================================

/// 音频后端接口
///
/// 不同平台的实现：
/// - Windows: WinMM (waveOut)
/// - Desktop: cpal
/// - Web: Web Audio API (原生 JS)
pub trait AudioBackend {
    /// 播放指定频率的方波（模拟 PC Speaker）
    fn beep(&mut self, frequency: u32, duration_ms: u32);

    /// 播放方波序列（用于音乐）
    fn play_sequence(&mut self, notes: &[(u32, u32)]);

    /// 停止所有音频
    fn stop(&mut self);

    /// 设置音量（0.0 - 1.0）
    fn set_volume(&mut self, volume: f32);

    /// 是否启用音频
    fn set_enabled(&mut self, enabled: bool);
    fn is_enabled(&self) -> bool;
}

// ============================================================================
// 时间后端 - 时间获取抽象
// ============================================================================

/// 时间后端接口
pub trait TimeBackend {
    /// 获取当前时间戳（毫秒）
    fn now_ms(&self) -> f64;

    /// 获取自程序启动以来的时间（毫秒）
    fn elapsed_ms(&self) -> f64;

    /// 非阻塞延迟检查：返回是否已过指定时间
    fn has_elapsed(&self, start_ms: f64, duration_ms: f64) -> bool {
        self.now_ms() - start_ms >= duration_ms
    }
}

// ============================================================================
// 随机数后端 - 随机数生成抽象
// ============================================================================

/// 随机数后端接口
pub trait RandomBackend {
    /// 生成 [0, max) 范围内的随机整数
    fn random_range(&mut self, max: i32) -> i32;

    /// 生成 [0, max) 范围内的随机浮点数
    fn random_range_f32(&mut self, max: f32) -> f32;

    /// 生成 [0, 1) 范围内的随机浮点数
    fn random_f32(&mut self) -> f32;

    /// 生成随机布尔值
    fn random_bool(&mut self) -> bool {
        self.random_range(2) == 0
    }
}

// ============================================================================
// 存储后端 - 持久化存储抽象
// ============================================================================

/// 存储后端接口
pub trait StorageBackend {
    /// 读取数据
    fn load(&self, key: &str) -> Option<Vec<u8>>;

    /// 写入数据
    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String>;

    /// 删除数据
    fn remove(&mut self, key: &str) -> Result<(), String>;

    /// 检查数据是否存在
    fn exists(&self, key: &str) -> bool;
}

// ============================================================================
// 日志后端 - 日志输出抽象
// ============================================================================

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 日志后端接口
pub trait LogBackend {
    fn log(&self, level: LogLevel, message: &str);

    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

// ============================================================================
// 输入后端 - 键盘/手柄输入抽象
// ============================================================================

/// 按键码（与平台无关的统一定义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // 方向键
    Left,
    Right,
    Up,
    Down,

    // 动作键
    Space,
    AltLeft,
    AltRight,
    ControlLeft,
    ControlRight,
    ShiftLeft,
    ShiftRight,

    // 功能键
    Escape,
    Enter,
    Tab,
    F1,
    F2,
    F11,
    Backspace,

    // 字母键
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // 数字键
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // 其他
    Unknown,
}

/// 按键事件
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub key: KeyCode,
    pub pressed: bool,
}

/// 输入后端接口
pub trait InputBackend {
    /// 轮询获取按键事件（非阻塞）
    fn poll_events(&mut self) -> Vec<KeyEvent>;

    /// 检查指定按键当前是否按下
    fn is_key_pressed(&self, key: KeyCode) -> bool;

    /// 检查窗口是否应该关闭
    fn should_close(&self) -> bool;

    /// 设置关闭标志
    fn request_close(&mut self);
}

// ============================================================================
// 平台上下文 - 统一入口
// ============================================================================

/// 平台上下文，包含所有平台相关的后端
pub trait Platform {
    type Display: DisplayBackend;
    type Audio: AudioBackend;
    type Time: TimeBackend;
    type Random: RandomBackend;
    type Storage: StorageBackend;
    type Log: LogBackend;
    type Input: InputBackend;

    fn display(&mut self) -> &mut Self::Display;
    fn audio(&mut self) -> &mut Self::Audio;
    fn time(&self) -> &Self::Time;
    fn random(&mut self) -> &mut Self::Random;
    fn storage(&mut self) -> &mut Self::Storage;
    fn log(&self) -> &Self::Log;
    fn input(&mut self) -> &mut Self::Input;
}

// ============================================================================
// 游戏状态枚举
// ============================================================================

/// 游戏帧结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameResult {
    Continue,
    Exit,
}

/// 游戏阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Initializing,
    Intro,
    Menu,
    ShowPlayerName,
    Playing,
    Paused,
    GameOver,
    Exiting,
}

// ============================================================================
// 平台实现导出 - 根据 feature 和目标平台选择
// ============================================================================

// Windows GDI + wgpu 后端（需要 gdi-backend + wgpu-backend，且未启用 cpu-backend）
#[cfg(all(
    target_os = "windows",
    feature = "gdi-backend",
    feature = "wgpu-backend",
    not(feature = "cpu-backend")
))]
pub use self::windows::{
    DesktopAudio, DesktopDisplay, DesktopInput, DesktopLog, DesktopRandom, DesktopStorage,
    DesktopTime, log_debug, log_error, log_info, log_warn, now_ms, random_f32, random_i32,
    random_u8, random_u32, random_usize, run_game,
};

// Windows CPU 软件渲染后端（XP兼容，仅 Windows + cpu-backend 且未启用 wgpu-backend）
#[cfg(all(
    target_os = "windows",
    feature = "cpu-backend",
    not(feature = "wgpu-backend")
))]
pub use self::windows_cpu::{
    DesktopAudio, DesktopDisplay, DesktopInput, DesktopLog, DesktopRandom, DesktopStorage,
    DesktopTime, log_debug, log_error, log_info, log_warn, now_ms, random_f32, random_i32,
    random_u8, random_u32, random_usize, run_game,
};

// wgpu 后端（跨平台桌面，使用 winit 窗口）
// 注意：
// - 在 Windows 上如果启用了 gdi-backend，则使用 windows.rs 而不是 desktop.rs
// - 在 Android 上使用 android.rs 而不是 desktop.rs
#[cfg(all(
    feature = "wgpu-backend",
    not(all(target_os = "windows", feature = "gdi-backend")),
    not(target_os = "android")
))]
pub use self::desktop::{
    DesktopAudio, DesktopDisplay, DesktopInput, DesktopLog, DesktopRandom, DesktopStorage,
    DesktopTime, log_debug, log_error, log_info, log_warn, now_ms, random_f32, random_i32,
    random_u8, random_u32, random_usize, run_game,
};

// Android 后端（需要 wgpu-backend，且未启用 android-cpu）
#[cfg(all(target_os = "android", feature = "wgpu-backend", not(feature = "android-cpu")))]
pub use self::android::{
    DesktopAudio, DesktopDisplay, DesktopInput, DesktopLog, DesktopRandom, DesktopStorage,
    DesktopTime, android_main, log_debug, log_error, log_info, log_warn, now_ms, random_f32,
    random_i32, random_u8, random_u32, random_usize, run_game,
};

// Android CPU 软件渲染后端（老旧设备）
#[cfg(all(target_os = "android", feature = "android-cpu"))]
pub use self::android_cpu::{
    DesktopAudio, DesktopDisplay, DesktopInput, DesktopLog, DesktopRandom, DesktopStorage,
    DesktopTime, android_main, log_debug, log_error, log_info, log_warn, now_ms, random_f32,
    random_i32, random_u8, random_u32, random_usize, run_game,
};

// Web 平台（未来扩展）
// #[cfg(target_arch = "wasm32")]
// pub mod web;
// #[cfg(target_arch = "wasm32")]
// pub use self::web::{...};
