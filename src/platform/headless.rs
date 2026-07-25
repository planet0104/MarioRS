//! Headless 平台后端
//!
//! 无窗口、无 GPU、无音频，仅用于 AI 训练/调试。
//! 该模块在 `debug-headless` feature 启用时编译，不影响原有可视化版本。

// 当可视化后端启用时，本模块中的类型仅作为 fallback 存在，允许未使用警告
#![allow(dead_code)]

use super::{AudioBackend, LogBackend, LogLevel};

// ============================================================================
// 音频后端 - 空实现（静音）
// ============================================================================

pub struct HeadlessAudio;

impl HeadlessAudio {
    pub fn new() -> Self {
        Self
    }

    /// 每帧调用（与 DesktopAudio 接口保持一致）
    pub fn tick(&mut self) {}
}

impl Default for HeadlessAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for HeadlessAudio {
    fn beep(&mut self, _frequency: u32, _duration_ms: u32) {}
    fn play_sequence(&mut self, _notes: &[(u32, u32)]) {}
    fn stop(&mut self) {}
    fn set_volume(&mut self, _volume: f32) {}
    fn set_enabled(&mut self, _enabled: bool) {}
    fn is_enabled(&self) -> bool {
        false
    }
}

// ============================================================================
// 日志后端 - 全部输出到 stderr，避免污染 stdout
// ============================================================================

pub struct HeadlessLog;

impl HeadlessLog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeadlessLog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBackend for HeadlessLog {
    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => eprintln!("[DEBUG] {}", message),
            LogLevel::Info => eprintln!("[INFO] {}", message),
            LogLevel::Warn => eprintln!("[WARN] {}", message),
            LogLevel::Error => eprintln!("[ERROR] {}", message),
        }
    }
}

// ============================================================================
// 便捷日志函数（与 desktop.rs 接口保持一致）
// ============================================================================

pub fn log_debug(msg: &str) {
    eprintln!("[DEBUG] {}", msg);
}

pub fn log_info(msg: &str) {
    eprintln!("[INFO] {}", msg);
}

pub fn log_warn(msg: &str) {
    eprintln!("[WARN] {}", msg);
}

pub fn log_error(msg: &str) {
    eprintln!("[ERROR] {}", msg);
}
