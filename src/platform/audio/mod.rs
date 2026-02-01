//! 音频模块
//!
//! 根据目标平台选择不同的音频后端：
//! - Windows: WinMM (waveOut) - 体积小，无额外依赖
//! - macOS/Linux: cpal - 跨平台音频库
//! - Web: 原生 JavaScript Web Audio API（由前端实现）

// Windows 使用 WinMM
#[cfg(target_os = "windows")]
mod waveout;

#[cfg(target_os = "windows")]
pub use waveout::WaveOutAudio;

// 非 Windows 且 非 WASM 平台使用 cpal
#[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
mod cpal_audio;

#[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
pub use cpal_audio::CpalAudio;

// 统一的音频类型别名
#[cfg(target_os = "windows")]
pub type PlatformAudio = WaveOutAudio;

#[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
pub type PlatformAudio = CpalAudio;

// Web 平台的空实现（音频由 JS 处理）
#[cfg(target_arch = "wasm32")]
mod web_audio;

#[cfg(target_arch = "wasm32")]
pub use web_audio::WebAudio;

#[cfg(target_arch = "wasm32")]
pub type PlatformAudio = WebAudio;
