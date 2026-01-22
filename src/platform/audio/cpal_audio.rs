//! 跨平台音频后端 - 基于 cpal
//!
//! 支持平台: macOS, Linux, Android, iOS
//! 功能: 方波生成、序列播放、音量控制

use super::super::AudioBackend;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

// ============================================================================
// 内部类型
// ============================================================================

/// 音频命令：用于在主线程和音频线程之间通信
#[derive(Clone)]
enum AudioCommand {
    Beep {
        frequency: f32,
        duration_samples: u64,
    },
    Sequence {
        notes: Vec<(f32, u64)>,
    },
    Stop,
}

/// 音频状态：在音频回调线程中使用
struct AudioState {
    commands: Vec<AudioCommand>,
    current_freq: f32,
    remaining_samples: u64,
    phase: f32,
    volume: f32,
    enabled: bool,
    sample_rate: f32,
    sequence_notes: Vec<(f32, u64)>,
    sequence_index: usize,
}

impl AudioState {
    fn new(sample_rate: f32) -> Self {
        Self {
            commands: Vec::new(),
            current_freq: 0.0,
            remaining_samples: 0,
            phase: 0.0,
            volume: 0.6,
            enabled: true,
            sample_rate,
            sequence_notes: Vec::new(),
            sequence_index: 0,
        }
    }

    fn process_commands(&mut self) {
        for cmd in self.commands.drain(..) {
            match cmd {
                AudioCommand::Beep {
                    frequency,
                    duration_samples,
                } => {
                    self.current_freq = frequency;
                    self.remaining_samples = duration_samples;
                    self.phase = 0.0;
                    self.sequence_notes.clear();
                    self.sequence_index = 0;
                }
                AudioCommand::Sequence { notes } => {
                    self.sequence_notes = notes;
                    self.sequence_index = 0;
                    if !self.sequence_notes.is_empty() {
                        let (freq, samples) = self.sequence_notes[0];
                        self.current_freq = freq;
                        self.remaining_samples = samples;
                        self.phase = 0.0;
                    }
                }
                AudioCommand::Stop => {
                    self.current_freq = 0.0;
                    self.remaining_samples = 0;
                    self.sequence_notes.clear();
                    self.sequence_index = 0;
                }
            }
        }
    }

    fn next_sample(&mut self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.remaining_samples == 0 {
            if !self.sequence_notes.is_empty() {
                self.sequence_index += 1;
                if self.sequence_index < self.sequence_notes.len() {
                    let (freq, samples) = self.sequence_notes[self.sequence_index];
                    self.current_freq = freq;
                    self.remaining_samples = samples;
                    self.phase = 0.0;
                } else {
                    self.sequence_notes.clear();
                    self.sequence_index = 0;
                    self.current_freq = 0.0;
                }
            } else {
                self.current_freq = 0.0;
            }
        }

        if self.current_freq <= 0.0 || self.remaining_samples == 0 {
            return 0.0;
        }

        let step = self.current_freq / self.sample_rate;
        let value = if self.phase < 0.5 { 1.0 } else { -1.0 };
        self.phase += step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.remaining_samples = self.remaining_samples.saturating_sub(1);
        value * self.volume * 0.3
    }
}

// ============================================================================
// CpalAudio
// ============================================================================

/// 基于 cpal 的跨平台音频后端
pub struct CpalAudio {
    state: Arc<Mutex<AudioState>>,
    _stream: Option<cpal::Stream>,
    sample_rate: f32,
}

impl CpalAudio {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("[Audio] 未找到音频输出设备");
                return Self::fallback();
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Audio] 获取音频配置失败: {}", e);
                return Self::fallback();
            }
        };

        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        let state = Arc::new(Mutex::new(AudioState::new(sample_rate)));
        let state_clone = Arc::clone(&state);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream_f32(&device, config.into(), state_clone, channels)
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream_i16(&device, config.into(), state_clone, channels)
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream_u16(&device, config.into(), state_clone, channels)
            }
            _ => {
                eprintln!("[Audio] 不支持的采样格式");
                return Self {
                    state,
                    _stream: None,
                    sample_rate,
                };
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Audio] 创建音频流失败: {}", e);
                return Self {
                    state,
                    _stream: None,
                    sample_rate,
                };
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("[Audio] 启动音频流失败: {}", e);
        }

        Self {
            state,
            _stream: Some(stream),
            sample_rate,
        }
    }

    fn fallback() -> Self {
        Self {
            state: Arc::new(Mutex::new(AudioState::new(48000.0))),
            _stream: None,
            sample_rate: 48000.0,
        }
    }

    fn build_stream_f32(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        state: Arc<Mutex<AudioState>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError> {
        device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                if let Ok(mut s) = state.lock() {
                    s.process_commands();
                    for frame in data.chunks_mut(channels) {
                        let sample = s.next_sample();
                        for ch in frame.iter_mut() {
                            *ch = sample;
                        }
                    }
                }
            },
            |err| eprintln!("[Audio] 音频流错误: {}", err),
            None,
        )
    }

    fn build_stream_i16(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        state: Arc<Mutex<AudioState>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError> {
        device.build_output_stream(
            &config,
            move |data: &mut [i16], _| {
                if let Ok(mut s) = state.lock() {
                    s.process_commands();
                    for frame in data.chunks_mut(channels) {
                        let sample = (s.next_sample() * 32767.0) as i16;
                        for ch in frame.iter_mut() {
                            *ch = sample;
                        }
                    }
                }
            },
            |err| eprintln!("[Audio] 音频流错误: {}", err),
            None,
        )
    }

    fn build_stream_u16(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        state: Arc<Mutex<AudioState>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError> {
        device.build_output_stream(
            &config,
            move |data: &mut [u16], _| {
                if let Ok(mut s) = state.lock() {
                    s.process_commands();
                    for frame in data.chunks_mut(channels) {
                        let sample = ((s.next_sample() + 1.0) * 0.5 * 65535.0) as u16;
                        for ch in frame.iter_mut() {
                            *ch = sample;
                        }
                    }
                }
            },
            |err| eprintln!("[Audio] 音频流错误: {}", err),
            None,
        )
    }

    fn ms_to_samples(&self, ms: u32) -> u64 {
        (ms as f64 * self.sample_rate as f64 / 1000.0).round() as u64
    }
}

impl Default for CpalAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for CpalAudio {
    fn beep(&mut self, frequency: u32, duration_ms: u32) {
        if let Ok(mut state) = self.state.lock() {
            if !state.enabled || frequency == 0 {
                return;
            }
            state.commands.push(AudioCommand::Beep {
                frequency: frequency as f32,
                duration_samples: self.ms_to_samples(duration_ms),
            });
        }
    }

    fn play_sequence(&mut self, notes: &[(u32, u32)]) {
        if let Ok(mut state) = self.state.lock() {
            if !state.enabled || notes.is_empty() {
                return;
            }
            let notes: Vec<(f32, u64)> = notes
                .iter()
                .map(|(freq, ms)| (*freq as f32, self.ms_to_samples(*ms)))
                .collect();
            state.commands.push(AudioCommand::Sequence { notes });
        }
    }

    fn stop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.commands.push(AudioCommand::Stop);
        }
    }

    fn set_volume(&mut self, volume: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.volume = volume.clamp(0.0, 1.0);
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.enabled = enabled;
        }
    }

    fn is_enabled(&self) -> bool {
        self.state.lock().map(|s| s.enabled).unwrap_or(false)
    }
}
