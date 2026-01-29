#![allow(unused)]

//! 音乐与音效系统，严格对齐 Pascal 版的播放节奏。
//!
//! Pascal 使用 PC Speaker 的 `Sound/NoSound` 组合出旋律，
//! 本模块使用平台音频抽象层播放方波，复刻其节奏与音色。
//!
//! Windows 版本：单线程非阻塞，使用 RefCell 实现内部可变性
//! Linux/Mac 版本：使用 Arc<Mutex> 支持 cpal 回调线程

use crate::platform::{AudioBackend, DesktopAudio};

// Windows: 使用 RefCell 实现内部可变性，保持 &self API
#[cfg(target_os = "windows")]
use std::cell::RefCell;

// Linux/Mac 需要 Mutex 支持 cpal 的音频回调线程
#[cfg(not(target_os = "windows"))]
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Pascal 音符常量：使用 Ord(c) 值映射到频率表
// ---------------------------------------------------------------------------
const C0: u8 = 1;
const D0: u8 = 3;
const E0: u8 = 5;
const F0: u8 = 6;
const G0: u8 = 8;
const A0: u8 = 10;
const B0: u8 = 12;
const C1: u8 = 13;
const D1: u8 = 15;
const E1: u8 = 17;
const F1: u8 = 18;
const G1: u8 = 20;
const A1: u8 = 22;
const B1: u8 = 24;
const C2: u8 = 25;
const D2: u8 = 27;
const E2: u8 = 29;
const F2: u8 = 30;
const G2: u8 = 32;
const A2: u8 = 34;
const B2: u8 = 36;
const C3: u8 = 37;
const D3: u8 = 39;
const E3: u8 = 41;
const F3: u8 = 42;
const G3: u8 = 44;
const A3: u8 = 46;
const B3: u8 = 48;
const C4: u8 = 49;
const D4: u8 = 51;
const E4: u8 = 53;
const F4: u8 = 54;
const G4: u8 = 56;
const A4: u8 = 58;
const B4: u8 = 60;
const C5: u8 = 61;
const D5: u8 = 63;
const E5: u8 = 65;
const F5: u8 = 66;
const G5: u8 = 68;
const A5: u8 = 70;
const B5: u8 = 72;
const C6: u8 = 73;
#[allow(dead_code)]
const D6: u8 = 75;
#[allow(dead_code)]
const E6: u8 = 77;
#[allow(dead_code)]
const F6: u8 = 78;
#[allow(dead_code)]
const G6: u8 = 80;
#[allow(dead_code)]
const A6: u8 = 82;
#[allow(dead_code)]
const B6: u8 = 84;

// ---------------------------------------------------------------------------
// Pascal 音乐序列：1=标记，随后交替存放音符与帧数，0 终止
// ---------------------------------------------------------------------------
const LIFE_MUSIC: &[u8] = &[1, G4, 8, C5, 8, E5, 8, C5, 8, D5, 8, G5, 8, 0];
const GROW_MUSIC: &[u8] = &[
    1, C3, 4, G3, 4, C4, 4, 38, 4, 45, 4, 50, 4, D3, 4, A3, 4, D4, 4, 0,
];
const COIN_MUSIC: &[u8] = &[1, F5, 1, 0];
const PIPE_MUSIC: &[u8] = &[
    1, C1, 0, C1, 8, C0, 0, C0, 16, C1, 0, C1, 8, C0, 0, C0, 16, C1, 0, C1, 8, C0, 0, C0, 16, 0,
];
const FIRE_MUSIC: &[u8] = &[1, E3, 1, A3, 1, 0];
const HIT_MUSIC: &[u8] = &[1, C2, 2, C1, 3, C0, 4, C2, 1, C1, 2, C0, 3, 0];
const DEAD_MUSIC: &[u8] = &[1, C2, 3, C1, 4, C0, 6, 0];
const NOTE_MUSIC: &[u8] = &[1, C0, 3, C1, 2, C2, 1, 0];
const STAR_MUSIC: &[u8] = &[
    1, C3, 4, E3, 4, G3, 4, C4, 4, E4, 4, G4, 4, C5, 4, E5, 4, G5, 4, C6, 4, 0,
];

/// 记录当前播放的音乐序列与位置
struct MusicState {
    music_sequence: Vec<u8>,
    position: usize,
}

// ============================================================================
// Windows 版本：单线程，使用 RefCell 实现内部可变性
// ============================================================================

#[cfg(target_os = "windows")]
struct AudioState {
    audio: DesktopAudio,
    music_state: MusicState,
}

#[cfg(target_os = "windows")]
pub struct MusicPlayer {
    state: RefCell<AudioState>,
    note_frequencies: [i32; 85],
    pub beeper_sound: bool,
}

#[cfg(target_os = "windows")]
impl MusicPlayer {
    pub fn new() -> Self {
        let audio = DesktopAudio::new();

        // 依照 Pascal `Music` 单元的算法预计算半音阶频率表
        const HALF_NOTE: f64 = 1.059463094;
        const MAX_OCT: usize = 7;
        let mut note_frequencies = [0i32; 85];
        let mut r_tmp = HALF_NOTE * 55.0;

        for i in 1..=MAX_OCT * 12 {
            note_frequencies[i] = r_tmp.round() as i32;
            r_tmp *= HALF_NOTE;
        }

        Self {
            state: RefCell::new(AudioState {
                audio,
                music_state: MusicState {
                    music_sequence: Vec::new(),
                    position: 0,
                },
            }),
            note_frequencies,
            beeper_sound: true,
        }
    }

    /// 每帧调用，清理已完成的音频缓冲区
    pub fn tick(&self) {
        self.state.borrow_mut().audio.tick();
    }

    /// 对应 Pascal `StartMusic`，载入一段全新的音乐序列
    pub fn start_music(&self, sequence: &[u8]) {
        if !self.beeper_sound {
            return;
        }
        let mut state = self.state.borrow_mut();
        state.music_state.music_sequence = sequence.to_vec();
        state.music_state.position = 0;
    }

    /// 对应 Pascal `PlayMusic`，需在每帧调用一次来驱动节奏
    pub fn play_music(&self) {
        if !self.beeper_sound {
            return;
        }

        let mut state = self.state.borrow_mut();
        let ms = &mut state.music_state;

        if ms.music_sequence.is_empty() || ms.position >= ms.music_sequence.len() {
            return;
        }

        let c = ms.music_sequence[ms.position];
        if c > 1 {
            ms.music_sequence[ms.position] -= 1;
        } else {
            ms.position += 1;
            if ms.position >= ms.music_sequence.len() {
                return;
            }

            let note_index = ms.music_sequence[ms.position];
            if note_index > 0 {
                let freq = self.note_frequencies[note_index as usize];
                state.audio.beep(freq as u32, 50);
                state.music_state.position += 1;
            } else {
                ms.music_sequence.clear();
                ms.position = 0;
            }
        }
    }

    /// 对应 Pascal `StopMusic`，立即终止当前序列
    pub fn stop_music(&self) {
        let mut state = self.state.borrow_mut();
        state.music_state.music_sequence.clear();
        state.music_state.position = 0;
    }

    /// 对应 Pascal `PauseMusic`
    pub fn pause_music(&self) {}

    /// 对应 Pascal `Beep`，播放指定频率的短促方波
    pub fn beep(&self, freq: u32) {
        if !self.beeper_sound || freq == 0 {
            return;
        }
        let duration_ms = if freq == 110 { 30 } else { 50 };
        self.state.borrow_mut().audio.beep(freq, duration_ms);
    }

    // --- 便捷方法 ---
    pub fn play_life(&self) {
        self.start_music(LIFE_MUSIC);
    }
    pub fn play_coin(&self) {
        self.start_music(COIN_MUSIC);
    }
    pub fn play_fire(&self) {
        self.start_music(FIRE_MUSIC);
    }
    pub fn play_hit(&self) {
        self.start_music(HIT_MUSIC);
    }
    pub fn play_dead(&self) {
        self.start_music(DEAD_MUSIC);
    }
    pub fn play_note(&self) {
        self.start_music(NOTE_MUSIC);
    }
    pub fn play_star(&self) {
        self.start_music(STAR_MUSIC);
    }
    pub fn play_grow(&self) {
        self.start_music(GROW_MUSIC);
    }
    pub fn play_pipe(&self) {
        self.start_music(PIPE_MUSIC);
    }

    pub fn play_powerup_rise(&self) {
        if !self.beeper_sound {
            return;
        }
        const J_SEQUENCE: [i32; 7] = [0, 12, 10, 8, 6, 4, 2];
        let notes: Vec<(u32, u32)> = J_SEQUENCE
            .iter()
            .map(|j| {
                let freq = 130 - 20 * j;
                let wrapped = freq.rem_euclid(1 << 16) as u32;
                (wrapped, 40)
            })
            .collect();
        self.state.borrow_mut().audio.play_sequence(&notes);
    }

    pub fn play_bump(&self) {
        self.beep(110);
    }
    pub fn play_coin_beep(&self) {
        self.beep(2420);
    }
    pub fn play_death_start(&self) {
        self.beep(220);
    }
    pub fn play_collision(&self) {
        self.beep(30);
    }

    pub fn beeper_on(&mut self) {
        self.beeper_sound = true;
        self.state.borrow_mut().audio.set_enabled(true);
    }

    pub fn beeper_off(&mut self) {
        self.beeper_sound = false;
        self.state.borrow_mut().audio.set_enabled(false);
    }
}

// ============================================================================
// Linux/Mac 版本：使用 Mutex 支持 cpal 回调线程
// ============================================================================

#[cfg(not(target_os = "windows"))]
pub struct MusicPlayer {
    audio: Arc<Mutex<DesktopAudio>>,
    note_frequencies: [i32; 85],
    music_state: Arc<Mutex<MusicState>>,
    pub beeper_sound: bool,
}

#[cfg(not(target_os = "windows"))]
impl MusicPlayer {
    pub fn new() -> Self {
        let audio = DesktopAudio::new();

        const HALF_NOTE: f64 = 1.059463094;
        const MAX_OCT: usize = 7;
        let mut note_frequencies = [0i32; 85];
        let mut r_tmp = HALF_NOTE * 55.0;

        for i in 1..=MAX_OCT * 12 {
            note_frequencies[i] = r_tmp.round() as i32;
            r_tmp *= HALF_NOTE;
        }

        Self {
            audio: Arc::new(Mutex::new(audio)),
            note_frequencies,
            music_state: Arc::new(Mutex::new(MusicState {
                music_sequence: Vec::new(),
                position: 0,
            })),
            beeper_sound: true,
        }
    }

    /// 每帧调用（Linux/Mac 版本为空操作，cpal 自动管理）
    pub fn tick(&mut self) {}

    pub fn start_music(&self, sequence: &[u8]) {
        if !self.beeper_sound {
            return;
        }
        let mut state = self.music_state.lock().unwrap();
        state.music_sequence = sequence.to_vec();
        state.position = 0;
    }

    pub fn play_music(&self) {
        if !self.beeper_sound {
            return;
        }

        let mut state = self.music_state.lock().unwrap();

        if state.music_sequence.is_empty() || state.position >= state.music_sequence.len() {
            return;
        }

        let c = state.music_sequence[state.position];
        if c > 1 {
            let pos = state.position;
            state.music_sequence[pos] -= 1;
        } else {
            state.position += 1;
            if state.position >= state.music_sequence.len() {
                return;
            }

            let note_index = state.music_sequence[state.position];
            if note_index > 0 {
                let freq = self.note_frequencies[note_index as usize];
                drop(state);
                self.beep(freq as u32);

                let mut state = self.music_state.lock().unwrap();
                state.position += 1;
            } else {
                state.music_sequence.clear();
                state.position = 0;
            }
        }
    }

    pub fn stop_music(&self) {
        let mut state = self.music_state.lock().unwrap();
        state.music_sequence.clear();
        state.position = 0;
    }

    pub fn pause_music(&self) {}

    pub fn beep(&self, freq: u32) {
        if !self.beeper_sound || freq == 0 {
            return;
        }
        let duration_ms = if freq == 110 { 30 } else { 50 };
        if let Ok(mut audio) = self.audio.lock() {
            audio.beep(freq, duration_ms);
        }
    }

    pub fn play_life(&self) {
        self.start_music(LIFE_MUSIC);
    }
    pub fn play_coin(&self) {
        self.start_music(COIN_MUSIC);
    }
    pub fn play_fire(&self) {
        self.start_music(FIRE_MUSIC);
    }
    pub fn play_hit(&self) {
        self.start_music(HIT_MUSIC);
    }
    pub fn play_dead(&self) {
        self.start_music(DEAD_MUSIC);
    }
    pub fn play_note(&self) {
        self.start_music(NOTE_MUSIC);
    }
    pub fn play_star(&self) {
        self.start_music(STAR_MUSIC);
    }
    pub fn play_grow(&self) {
        self.start_music(GROW_MUSIC);
    }
    pub fn play_pipe(&self) {
        self.start_music(PIPE_MUSIC);
    }

    pub fn play_powerup_rise(&self) {
        if !self.beeper_sound {
            return;
        }
        const J_SEQUENCE: [i32; 7] = [0, 12, 10, 8, 6, 4, 2];
        let notes: Vec<(u32, u32)> = J_SEQUENCE
            .iter()
            .map(|j| {
                let freq = 130 - 20 * j;
                let wrapped = freq.rem_euclid(1 << 16) as u32;
                (wrapped, 40)
            })
            .collect();
        if let Ok(mut audio) = self.audio.lock() {
            audio.play_sequence(&notes);
        }
    }

    pub fn play_bump(&self) {
        self.beep(110);
    }
    pub fn play_coin_beep(&self) {
        self.beep(2420);
    }
    pub fn play_death_start(&self) {
        self.beep(220);
    }
    pub fn play_collision(&self) {
        self.beep(30);
    }

    pub fn beeper_on(&mut self) {
        self.beeper_sound = true;
        if let Ok(mut audio) = self.audio.lock() {
            audio.set_enabled(true);
        }
    }

    pub fn beeper_off(&mut self) {
        self.beeper_sound = false;
        if let Ok(mut audio) = self.audio.lock() {
            audio.set_enabled(false);
        }
    }
}
