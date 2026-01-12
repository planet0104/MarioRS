//! Windows 音频后端 - 使用 WinMM (waveOut)
//!
//! 单线程非阻塞模式，模拟 DOS PC Speaker 行为：
//! - beep() 提交音频缓冲区后立即返回
//! - 音频由 Windows 内核异步播放
//! - 不使用 Mutex/线程，兼容 Windows XP

#![allow(nonstandard_style)]

use std::ptr;
use super::super::AudioBackend;

// ============================================================================
// WinMM FFI 绑定
// ============================================================================

#[repr(C)]
struct WAVEFORMATEX {
    wFormatTag: u16,
    nChannels: u16,
    nSamplesPerSec: u32,
    nAvgBytesPerSec: u32,
    nBlockAlign: u16,
    wBitsPerSample: u16,
    cbSize: u16,
}

#[repr(C)]
struct WAVEHDR {
    lpData: *mut i8,
    dwBufferLength: u32,
    dwBytesRecorded: u32,
    dwUser: usize,
    dwFlags: u32,
    dwLoops: u32,
    lpNext: *mut WAVEHDR,
    reserved: usize,
}

type HWAVEOUT = *mut core::ffi::c_void;
type MMRESULT = u32;

#[link(name = "winmm")]
unsafe extern "system" {
    fn waveOutOpen(phwo: *mut HWAVEOUT, uDeviceID: u32, pwfx: *const WAVEFORMATEX, dwCallback: usize, dwInstance: usize, dwFlags: u32) -> MMRESULT;
    fn waveOutPrepareHeader(hwo: HWAVEOUT, pwh: *mut WAVEHDR, cbwh: u32) -> MMRESULT;
    fn waveOutWrite(hwo: HWAVEOUT, pwh: *mut WAVEHDR, cbwh: u32) -> MMRESULT;
    fn waveOutUnprepareHeader(hwo: HWAVEOUT, pwh: *mut WAVEHDR, cbwh: u32) -> MMRESULT;
    fn waveOutClose(hwo: HWAVEOUT) -> MMRESULT;
    fn waveOutReset(hwo: HWAVEOUT) -> MMRESULT;
}

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_MAPPER: u32 = u32::MAX;
const WHDR_DONE: u32 = 0x0000_0001;
const WHDR_PREPARED: u32 = 0x0000_0002;
const SAMPLE_RATE: u32 = 22050; // 降低采样率减少 CPU 开销

// ============================================================================
// 音频缓冲区管理
// ============================================================================

/// 单个音频缓冲区槽位
struct AudioBuffer {
    header: WAVEHDR,
    data: Vec<i16>,
    in_use: bool,
}

impl AudioBuffer {
    fn new() -> Self {
        Self {
            header: unsafe { std::mem::zeroed() },
            data: Vec::new(),
            in_use: false,
        }
    }
}

// 使用 4 个缓冲区轮换，足够覆盖连续音效
const NUM_BUFFERS: usize = 4;

// ============================================================================
// WaveOutAudio - 单线程非阻塞实现
// ============================================================================

/// Windows 音频后端（单线程非阻塞）
pub struct WaveOutAudio {
    hwo: HWAVEOUT,
    buffers: [AudioBuffer; NUM_BUFFERS],
    next_buffer: usize,
    volume: f32,
    enabled: bool,
}

impl WaveOutAudio {
    pub fn new() -> Self {
        let mut hwo: HWAVEOUT = ptr::null_mut();
        unsafe {
            let wfx = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_PCM,
                nChannels: 1,
                nSamplesPerSec: SAMPLE_RATE,
                nAvgBytesPerSec: SAMPLE_RATE * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            // CALLBACK_NULL = 0，不使用回调
            let _ = waveOutOpen(&mut hwo, WAVE_MAPPER, &wfx, 0, 0, 0);
        }

        Self {
            hwo,
            buffers: [
                AudioBuffer::new(),
                AudioBuffer::new(),
                AudioBuffer::new(),
                AudioBuffer::new(),
            ],
            next_buffer: 0,
            volume: 0.6,
            enabled: true,
        }
    }

    /// 清理已完成的缓冲区（每帧调用）
    pub fn tick(&mut self) {
        if self.hwo.is_null() {
            return;
        }

        for buf in &mut self.buffers {
            if buf.in_use {
                unsafe {
                    // 检查是否播放完成
                    if (std::ptr::read_volatile(&buf.header.dwFlags) & WHDR_DONE) != 0 {
                        // 取消准备并标记为空闲
                        if (buf.header.dwFlags & WHDR_PREPARED) != 0 {
                            waveOutUnprepareHeader(
                                self.hwo,
                                &mut buf.header,
                                std::mem::size_of::<WAVEHDR>() as u32,
                            );
                        }
                        buf.in_use = false;
                    }
                }
            }
        }
    }

    /// 生成方波样本
    fn generate_square_wave(&self, frequency: f32, duration_s: f32) -> Vec<i16> {
        let total_samples = (duration_s * SAMPLE_RATE as f32) as usize;
        let amplitude = (16000.0 * self.volume.clamp(0.0, 1.0)) as i16;
        
        if frequency < 20.0 {
            // 频率太低，返回静音
            return vec![0i16; total_samples];
        }
        
        let samples_per_period = ((SAMPLE_RATE as f32) / frequency).max(2.0) as usize;
        let half = samples_per_period / 2;

        let mut buf = Vec::with_capacity(total_samples);
        for i in 0..total_samples {
            let phase = i % samples_per_period;
            buf.push(if phase < half { amplitude } else { -amplitude });
        }
        buf
    }

    /// 提交音频缓冲区（非阻塞）
    fn submit_buffer(&mut self, samples: Vec<i16>) {
        if self.hwo.is_null() || samples.is_empty() {
            return;
        }

        // 先清理已完成的缓冲区
        self.tick();

        // 找到一个空闲缓冲区
        let start = self.next_buffer;
        for i in 0..NUM_BUFFERS {
            let idx = (start + i) % NUM_BUFFERS;
            if !self.buffers[idx].in_use {
                self.next_buffer = (idx + 1) % NUM_BUFFERS;
                
                let buf = &mut self.buffers[idx];
                buf.data = samples;
                buf.in_use = true;

                unsafe {
                    buf.header = std::mem::zeroed();
                    buf.header.lpData = buf.data.as_ptr() as *mut i8;
                    buf.header.dwBufferLength = (buf.data.len() * 2) as u32;

                    if waveOutPrepareHeader(
                        self.hwo,
                        &mut buf.header,
                        std::mem::size_of::<WAVEHDR>() as u32,
                    ) == 0 {
                        if waveOutWrite(
                            self.hwo,
                            &mut buf.header,
                            std::mem::size_of::<WAVEHDR>() as u32,
                        ) != 0 {
                            // 写入失败，取消准备
                            waveOutUnprepareHeader(
                                self.hwo,
                                &mut buf.header,
                                std::mem::size_of::<WAVEHDR>() as u32,
                            );
                            buf.in_use = false;
                        }
                    } else {
                        buf.in_use = false;
                    }
                }
                return;
            }
        }
        // 所有缓冲区都在使用中，丢弃这个音效
    }
}

impl Default for WaveOutAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WaveOutAudio {
    fn drop(&mut self) {
        if !self.hwo.is_null() {
            unsafe {
                // 停止所有播放
                waveOutReset(self.hwo);
                
                // 取消所有已准备的缓冲区
                for buf in &mut self.buffers {
                    if buf.in_use && (buf.header.dwFlags & WHDR_PREPARED) != 0 {
                        waveOutUnprepareHeader(
                            self.hwo,
                            &mut buf.header,
                            std::mem::size_of::<WAVEHDR>() as u32,
                        );
                    }
                }
                
                waveOutClose(self.hwo);
            }
        }
    }
}

impl AudioBackend for WaveOutAudio {
    fn beep(&mut self, frequency: u32, duration_ms: u32) {
        if !self.enabled || frequency == 0 {
            return;
        }

        let samples = self.generate_square_wave(
            frequency as f32,
            duration_ms as f32 / 1000.0,
        );
        self.submit_buffer(samples);
    }

    fn play_sequence(&mut self, notes: &[(u32, u32)]) {
        if !self.enabled || notes.is_empty() {
            return;
        }

        // 将整个序列合成为一个缓冲区
        let mut all_samples = Vec::new();
        for &(freq, ms) in notes {
            if freq == 0 {
                // 静音
                let silence_samples = (ms as f32 / 1000.0 * SAMPLE_RATE as f32) as usize;
                all_samples.extend(vec![0i16; silence_samples]);
            } else {
                let samples = self.generate_square_wave(freq as f32, ms as f32 / 1000.0);
                all_samples.extend(samples);
            }
        }
        self.submit_buffer(all_samples);
    }

    fn stop(&mut self) {
        if !self.hwo.is_null() {
            unsafe {
                waveOutReset(self.hwo);
            }
            // 标记所有缓冲区为空闲
            for buf in &mut self.buffers {
                buf.in_use = false;
            }
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.stop();
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}
