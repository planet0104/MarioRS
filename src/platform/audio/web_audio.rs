//! Web 音频后端 - 占位符
//!
//! Web 平台的音频由原生 JavaScript Web Audio API 实现
//! 这个模块只是一个空实现，用于保持 Rust 代码的编译兼容性
//!
//! 实际音频播放应该通过 wasm-bindgen 调用 JS 函数实现

use super::super::AudioBackend;

/// Web 音频后端（空实现）
pub struct WebAudio {
    enabled: bool,
    volume: f32,
}

impl WebAudio {
    pub fn new() -> Self {
        Self {
            enabled: true,
            volume: 0.6,
        }
    }
}

impl Default for WebAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for WebAudio {
    fn beep(&mut self, _frequency: u32, _duration_ms: u32) {
        // TODO: 通过 wasm-bindgen 调用 JS Web Audio API
    }

    fn play_sequence(&mut self, _notes: &[(u32, u32)]) {
        // TODO: 通过 wasm-bindgen 调用 JS Web Audio API
    }

    fn stop(&mut self) {
        // TODO: 停止 JS 音频播放
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}
