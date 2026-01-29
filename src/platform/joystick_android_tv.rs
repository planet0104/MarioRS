//! Android TV 遥控器支持模块 (与手柄完全分离)
//!
//! 实现 Android TV 平台下使用遥控器控制游戏的功能
//! 只支持6个按键: 上/下/左/右/OK/返回
//!
//! 架构说明:
//! - 此模块独立于手柄模块 (joystick_android.rs)
//! - Java RemoteController 通过 nativeOnRemoteKey 更新状态
//! - 游戏主循环通过 read_tv_remote_state() 读取状态
//! - 与手柄逻辑完全分离，互不影响
//!
//! 遥控器特性 (与手柄不同):
//! - 自动加速模式: 由 Java RemoteController 管理
//! - 延迟释放: 由 Java RemoteController 管理
//! - 只有6个按键，无摇杆
//!
//! 按键映射:
//! - 上键: 菜单向上 / 游戏中跳跃
//! - 下键: 菜单向下 / 游戏中钻管道+发射子弹
//! - 左键: 左移动
//! - 右键: 右移动
//! - OK键: 菜单确认 / 游戏中跳跃
//! - 返回键: ESC

use std::sync::Mutex;

// ============================================================================
// Android KeyCode 常量
// ============================================================================

/// Android 按键码常量
pub mod keycode {
    pub const DPAD_UP: i32 = 19;
    pub const DPAD_DOWN: i32 = 20;
    pub const DPAD_LEFT: i32 = 21;
    pub const DPAD_RIGHT: i32 = 22;
    pub const DPAD_CENTER: i32 = 23;  // OK键
    pub const ENTER: i32 = 66;
    pub const BACK: i32 = 4;
}

// ============================================================================
// 全局 TV 遥控器状态 (线程安全)
// ============================================================================

/// 全局 TV 遥控器状态
static TV_REMOTE_STATE: Mutex<TvRemoteGlobalState> = Mutex::new(TvRemoteGlobalState::new());

/// TV 遥控器全局状态
struct TvRemoteGlobalState {
    // 方向键状态
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    // 确认/跳跃
    ok: bool,
    // 返回
    back: bool,
    // 是否检测到遥控器
    detected: bool,
    // 上键按下边沿 (用于触发跳跃)
    up_pressed_once: bool,
    // OK键按下边沿 (用于触发跳跃/确认)
    ok_pressed_once: bool,
}

impl TvRemoteGlobalState {
    const fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            ok: false,
            back: false,
            detected: false,
            up_pressed_once: false,
            ok_pressed_once: false,
        }
    }
}

/// TV 遥控器输入快照 (用于游戏主循环读取)
#[derive(Debug, Clone, Copy, Default)]
pub struct TvRemoteSnapshot {
    /// 方向上 (菜单向上)
    pub up: bool,
    /// 方向下 (菜单向下/钻管道)
    pub down: bool,
    /// 方向左 (左移动)
    pub left: bool,
    /// 方向右 (右移动)
    pub right: bool,
    /// OK键 (确认/跳跃)
    pub ok: bool,
    /// 返回键 (ESC)
    pub back: bool,
    /// 是否检测到遥控器
    pub detected: bool,
    /// 上键按下边沿 (用于游戏中跳跃)
    pub up_pressed_once: bool,
    /// OK键按下边沿 (用于跳跃/确认)
    pub ok_pressed_once: bool,
}

// ============================================================================
// JNI 回调函数 - 供 Java 层调用
// ============================================================================

/// 处理 TV 遥控器按键事件 (由 Java RemoteController 调用)
/// 
/// 使用原始 Android KeyCode:
/// - DPAD_UP = 19
/// - DPAD_DOWN = 20
/// - DPAD_LEFT = 21
/// - DPAD_RIGHT = 22
/// - DPAD_CENTER = 23 (OK键)
/// - ENTER = 66
/// - BACK = 4
pub fn on_tv_remote_key(key_code: i32, pressed: bool) {
    if let Ok(mut state) = TV_REMOTE_STATE.lock() {
        match key_code {
            keycode::DPAD_UP => {
                // 标记检测到遥控器
                state.detected = true;
                // 上键按下边沿
                if pressed && !state.up {
                    state.up_pressed_once = true;
                }
                state.up = pressed;
            }
            keycode::DPAD_DOWN => {
                state.detected = true;
                state.down = pressed;
            }
            keycode::DPAD_LEFT => {
                state.detected = true;
                state.left = pressed;
            }
            keycode::DPAD_RIGHT => {
                state.detected = true;
                state.right = pressed;
            }
            keycode::DPAD_CENTER | keycode::ENTER => {
                state.detected = true;
                // OK键按下边沿
                if pressed && !state.ok {
                    state.ok_pressed_once = true;
                }
                state.ok = pressed;
            }
            keycode::BACK => {
                state.detected = true;
                state.back = pressed;
            }
            _ => {}
        }
    }
}

/// 读取 TV 遥控器状态快照 (游戏主循环调用)
/// 
/// 注意: 会消费 pressed_once 边沿状态
pub fn read_tv_remote_state() -> TvRemoteSnapshot {
    if let Ok(mut state) = TV_REMOTE_STATE.lock() {
        let snapshot = TvRemoteSnapshot {
            up: state.up,
            down: state.down,
            left: state.left,
            right: state.right,
            ok: state.ok,
            back: state.back,
            detected: state.detected,
            up_pressed_once: state.up_pressed_once,
            ok_pressed_once: state.ok_pressed_once,
        };
        
        // 消费边沿状态
        state.up_pressed_once = false;
        state.ok_pressed_once = false;
        
        snapshot
    } else {
        TvRemoteSnapshot::default()
    }
}

/// 读取 TV 遥控器状态快照 (不消费 pressed_once)
pub fn peek_tv_remote_state() -> TvRemoteSnapshot {
    if let Ok(state) = TV_REMOTE_STATE.lock() {
        TvRemoteSnapshot {
            up: state.up,
            down: state.down,
            left: state.left,
            right: state.right,
            ok: state.ok,
            back: state.back,
            detected: state.detected,
            up_pressed_once: state.up_pressed_once,
            ok_pressed_once: state.ok_pressed_once,
        }
    } else {
        TvRemoteSnapshot::default()
    }
}

/// 重置 TV 遥控器状态 (Activity 暂停/恢复时调用)
pub fn reset_tv_remote_state() {
    if let Ok(mut state) = TV_REMOTE_STATE.lock() {
        state.up = false;
        state.down = false;
        state.left = false;
        state.right = false;
        state.ok = false;
        state.back = false;
        state.up_pressed_once = false;
        state.ok_pressed_once = false;
        // 不重置 detected 状态
    }
}

/// 检查是否检测到 TV 遥控器
pub fn is_tv_remote_detected() -> bool {
    if let Ok(state) = TV_REMOTE_STATE.lock() {
        state.detected
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_up_key() {
        // 重置状态
        reset_tv_remote_state();
        
        // 按下上键
        on_tv_remote_key(keycode::DPAD_UP, true);
        let snap = read_tv_remote_state();
        assert!(snap.up);
        assert!(snap.detected);
        
        // pressed_once 应该被消费
        let snap2 = read_tv_remote_state();
        assert!(snap2.up);
        assert!(!snap2.up_pressed_once);
        
        // 释放上键
        on_tv_remote_key(keycode::DPAD_UP, false);
        let snap3 = read_tv_remote_state();
        assert!(!snap3.up);
    }

    #[test]
    fn test_remote_ok_key() {
        reset_tv_remote_state();
        
        // 按下OK键
        on_tv_remote_key(keycode::DPAD_CENTER, true);
        let snap = read_tv_remote_state();
        assert!(snap.ok);
        assert!(snap.ok_pressed_once);
        
        // pressed_once 应该被消费
        let snap2 = read_tv_remote_state();
        assert!(!snap2.ok_pressed_once);
    }
}
