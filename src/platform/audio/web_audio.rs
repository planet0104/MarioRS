//! Web 音频后端 - 基于 Web Audio API
//!
//! 使用 `web_sys::AudioContext` 与 `OscillatorNode` 实现方波播放与序列播放，
//! 不依赖额外的第三方 crate（仅依赖 wasm-bindgen / web-sys，它们已在 wasm 目标中声明）。

use super::super::AudioBackend;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{AudioContext, OscillatorNode, GainNode};

use std::cell::RefCell;
use std::rc::Rc;

/// WebAudio: 管理 AudioContext、GainNode，并保存正在播放/已调度的 Oscillator 引用，
/// 以防止它们在播放期间被回收。
pub struct WebAudio {
    ctx: AudioContext,
    gain: GainNode,
    enabled: bool,
    volume: f32,
    // 保存正在调度或播放的 Oscillator 对象，类型为 JsValue
    scheduled: Rc<RefCell<Vec<JsValue>>>,
}

impl WebAudio {
    pub fn new() -> Self {
        let ctx = AudioContext::new().expect("无法创建 AudioContext");

        let gain = ctx.create_gain().expect("无法创建 GainNode");
        gain.gain().set_value(0.6);

        // 将增益连接到 destination
        let _ = gain.connect_with_audio_node(&ctx.destination()).ok();

        Self {
            ctx,
            gain,
            enabled: true,
            volume: 0.6,
            scheduled: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// 尝试恢复/解锁 AudioContext（在用户交互时调用）
    pub fn resume(&self) {
        // AudioContext::resume() 返回一个 Promise，忽略结果即可
        let _ = self.ctx.resume();
    }

    fn schedule_oscillator(&mut self, freq: f32, start_time: f64, duration_s: f64) {
        if !self.enabled || freq <= 0.0 {
            return;
        }

        let osc = self
            .ctx
            .create_oscillator()
            .expect("无法创建 Oscillator")
            .dyn_into::<OscillatorNode>()
            .expect("类型转换失败");

        // 方波：直接通过 Reflect 设置 `type` 属性（避免方法名不一致）
        let _ = js_sys::Reflect::set(
            osc.as_ref(),
            &JsValue::from_str("type"),
            &JsValue::from_str("square"),
        );
        osc.frequency().set_value(freq);

        // 连接到增益
        let _ = osc.connect_with_audio_node(&self.gain).ok();

        // 保持对对象的引用，防止被 GC
        let js_osc = osc.clone().dyn_into::<JsValue>().unwrap();
        self.scheduled.borrow_mut().push(js_osc.clone());

        // 当节点结束时从 scheduled 中移除
        let scheduled_clone = self.scheduled.clone();
        let js_osc_clone = js_osc.clone();
        let onended = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let mut vec = scheduled_clone.borrow_mut();
            if let Some(idx) = vec.iter().position(|v| v == &js_osc_clone) {
                vec.remove(idx);
            }
        }) as Box<dyn FnMut(_)>);

        osc.set_onended(Some(onended.as_ref().unchecked_ref()));
        onended.forget();

        // 使用 AudioContext 的时间基准进行调度
        let start = start_time.max(self.ctx.current_time());
        let stop = start + duration_s;

        // start/stop 的绑定方法在 web-sys 中以 `start_with_when` / `stop_with_when` 命名
        let _ = osc.start_with_when(start).ok();
        let _ = osc.stop_with_when(stop).ok();
    }
}

impl Default for WebAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for WebAudio {
    fn beep(&mut self, frequency: u32, duration_ms: u32) {
        if !self.enabled || frequency == 0 || duration_ms == 0 {
            return;
        }

        let now = self.ctx.current_time();
        let dur = (duration_ms as f64) / 1000.0;
        self.schedule_oscillator(frequency as f32, now, dur);
    }

    fn play_sequence(&mut self, notes: &[(u32, u32)]) {
        if !self.enabled || notes.is_empty() {
            return;
        }

        let mut cursor = self.ctx.current_time();
        for &(freq, ms) in notes {
            let dur = (ms as f64) / 1000.0;
            if freq == 0 || ms == 0 {
                cursor += dur;
                continue;
            }

            self.schedule_oscillator(freq as f32, cursor, dur);
            cursor += dur;
        }
    }

    fn stop(&mut self) {
        // 停止所有已调度/正在播放的振荡器
        let now = self.ctx.current_time();
        let mut vec = self.scheduled.borrow_mut();
        for jsv in vec.iter() {
            if let Ok(osc) = jsv.clone().dyn_into::<OscillatorNode>() {
                let _ = osc.stop_with_when(now).ok();
                let _ = osc.disconnect();
            }
        }
        vec.clear();
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.gain.gain().set_value(self.volume);
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
