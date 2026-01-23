//! 公共输入状态管理
//!
//! 提供按键状态跟踪和事件缓冲的基础实现

use crate::platform::{KeyCode, KeyEvent};
use std::collections::HashSet;

/// 公共输入状态管理器
///
/// 各平台可以组合使用此结构体来管理按键状态
pub struct InputState {
    /// 当前按下的按键集合
    key_states: HashSet<KeyCode>,
    /// 待处理的按键事件队列
    pending_events: Vec<KeyEvent>,
    /// 是否请求关闭
    should_close: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
        }
    }

    /// 处理按键事件
    ///
    /// 更新按键状态并添加到事件队列
    pub fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.key_states.insert(key);
        } else {
            self.key_states.remove(&key);
        }
        self.pending_events.push(KeyEvent { key, pressed });
    }

    /// 获取并清空待处理的事件队列
    pub fn take_events(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// 添加外部事件到队列 (如触摸面板生成的事件)
    pub fn extend_events(&mut self, events: Vec<KeyEvent>) {
        // 同时更新按键状态
        for event in &events {
            if event.pressed {
                self.key_states.insert(event.key);
            } else {
                self.key_states.remove(&event.key);
            }
        }
        self.pending_events.extend(events);
    }

    /// 检查按键是否按下
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.key_states.contains(&key)
    }

    /// 检查是否应该关闭
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// 请求关闭
    pub fn request_close(&mut self) {
        self.should_close = true;
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
