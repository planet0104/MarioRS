//! Android 手柄支持模块 (与遥控器完全分离)
//!
//! 使用 Android InputDevice API 读取 USB/蓝牙手柄输入
//! 与 Windows joystick_win.rs 接口保持一致
//!
//! 架构说明:
//! - Java GamepadController 通过 nativeOnGamepadButton 传递按钮事件
//! - Java GamepadController 通过 nativeOnGamepadAxis 传递摇杆轴数据
//! - 游戏主循环通过 read_gamepad_state() 读取状态
//! - 与遥控器模块 (joystick_android_tv.rs) 完全分离
//!
//! 手柄特性 (与遥控器不同):
//! - 支持摇杆 (左/右)
//! - 支持扳机 (LT/RT)
//! - 支持多个按钮 (A/B/X/Y/LB/RB/SELECT/START)
//! - 无延迟释放，直接响应

use std::sync::Mutex;

// ============================================================================
// Android Gamepad 按键码常量 (KeyEvent.KEYCODE_BUTTON_*)
// ============================================================================

pub mod button_code {
    pub const BUTTON_A: i32 = 96;
    pub const BUTTON_B: i32 = 97;
    pub const BUTTON_X: i32 = 99;
    pub const BUTTON_Y: i32 = 100;
    pub const BUTTON_L1: i32 = 102;  // LB
    pub const BUTTON_R1: i32 = 103;  // RB
    pub const BUTTON_L2: i32 = 104;  // LT (数字按钮)
    pub const BUTTON_R2: i32 = 105;  // RT (数字按钮)
    pub const BUTTON_SELECT: i32 = 109;  // SELECT / BACK
    pub const BUTTON_START: i32 = 108;   // START
    pub const BUTTON_THUMBL: i32 = 106;  // 左摇杆按下
    pub const BUTTON_THUMBR: i32 = 107;  // 右摇杆按下
    // DPAD 按键 (部分手柄使用按键而非轴)
    pub const DPAD_UP: i32 = 19;
    pub const DPAD_DOWN: i32 = 20;
    pub const DPAD_LEFT: i32 = 21;
    pub const DPAD_RIGHT: i32 = 22;
}

// ============================================================================
// 全局手柄状态 (线程安全)
// ============================================================================

static GAMEPAD_STATE: Mutex<GamepadGlobalState> = Mutex::new(GamepadGlobalState::new());

/// 手柄全局状态
struct GamepadGlobalState {
    // 是否检测到手柄
    connected: bool,
    
    // 左摇杆轴 (-1.0 到 1.0)
    left_x: f32,
    left_y: f32,
    
    // 右摇杆轴 (-1.0 到 1.0)
    right_x: f32,
    right_y: f32,
    
    // 扳机 (0.0 到 1.0)
    left_trigger: f32,
    right_trigger: f32,
    
    // DPAD 方向 (HAT)
    hat_x: f32,  // -1.0 左, 0 中, 1.0 右
    hat_y: f32,  // -1.0 上, 0 中, 1.0 下
    
    // 主要按钮 (A/B/X/Y)
    button_a: bool,
    button_b: bool,
    button_x: bool,
    button_y: bool,
    
    // 肩键 (LB/RB)
    button_lb: bool,
    button_rb: bool,
    
    // 扳机按钮 (部分手柄有数字扳机按钮)
    button_lt: bool,
    button_rt: bool,
    
    // 功能键
    button_select: bool,
    button_start: bool,
    
    // 摇杆按下
    button_thumbl: bool,
    button_thumbr: bool,
    
    // DPAD 按键状态 (用于没有HAT轴的手柄)
    dpad_up: bool,
    dpad_down: bool,
    dpad_left: bool,
    dpad_right: bool,
}

impl GamepadGlobalState {
    const fn new() -> Self {
        Self {
            connected: false,
            left_x: 0.0,
            left_y: 0.0,
            right_x: 0.0,
            right_y: 0.0,
            left_trigger: 0.0,
            right_trigger: 0.0,
            hat_x: 0.0,
            hat_y: 0.0,
            button_a: false,
            button_b: false,
            button_x: false,
            button_y: false,
            button_lb: false,
            button_rb: false,
            button_lt: false,
            button_rt: false,
            button_select: false,
            button_start: false,
            button_thumbl: false,
            button_thumbr: false,
            dpad_up: false,
            dpad_down: false,
            dpad_left: false,
            dpad_right: false,
        }
    }
}

// ============================================================================
// 手柄输入状态 (返回给调用方，与 joystick_win.rs 兼容)
// ============================================================================

/// 手柄输入状态
///
/// 按钮布局 (标准 Android 手柄):
/// - button_a: A (跳跃)
/// - button_b: B (跳跃备选)
/// - button_x: X (发射)
/// - button_y: Y
/// - button_lb: LB/L1 (加速)
/// - button_rb: RB/R1 (加速备选)
/// - button_select: SELECT/BACK
/// - button_start: START
#[derive(Debug, Clone, Copy, Default)]
pub struct JoystickInput {
    pub connected: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    // 主要按钮 (A/B/X/Y)
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,
    // 肩键 (LB/RB)
    pub button_lb: bool,
    pub button_rb: bool,
    // 功能键
    pub button_select: bool,
    pub button_start: bool,
    // 兼容旧接口 (跳跃 / 加速)
    pub button1: bool,  // 跳跃: A 或 B
    pub button2: bool,  // 加速: LB 或 RB
    // 原始数据
    pub raw_x: u16,
    pub raw_y: u16,
    pub raw_buttons: u32,
}

impl JoystickInput {
    /// 创建断开连接的状态
    pub fn disconnected() -> Self {
        Self::default()
    }
}

// ============================================================================
// JNI 回调函数 - 供 Java 层调用
// ============================================================================

/// 处理手柄摇杆轴事件 (由 Java GamepadController 调用)
///
/// axis_id:
/// - 0: AXIS_X (左摇杆X)
/// - 1: AXIS_Y (左摇杆Y)
/// - 2: AXIS_Z (右摇杆X)
/// - 3: AXIS_RZ (右摇杆Y)
/// - 4: AXIS_LTRIGGER (左扳机)
/// - 5: AXIS_RTRIGGER (右扳机)
/// - 6: AXIS_HAT_X (DPAD X)
/// - 7: AXIS_HAT_Y (DPAD Y)
pub fn on_gamepad_axis(axis_id: i32, value: f32) {
    if let Ok(mut state) = GAMEPAD_STATE.lock() {
        state.connected = true;
        
        match axis_id {
            0 => state.left_x = value,
            1 => state.left_y = value,
            2 => state.right_x = value,
            3 => state.right_y = value,
            4 => state.left_trigger = value,
            5 => state.right_trigger = value,
            6 => state.hat_x = value,
            7 => state.hat_y = value,
            _ => {}
        }
    }
}

/// 处理手柄按钮事件 (由 Java GamepadController 调用)
pub fn on_gamepad_button(key_code: i32, pressed: bool) {
    if let Ok(mut state) = GAMEPAD_STATE.lock() {
        state.connected = true;
        
        match key_code {
            button_code::BUTTON_A => state.button_a = pressed,
            button_code::BUTTON_B => state.button_b = pressed,
            button_code::BUTTON_X => state.button_x = pressed,
            button_code::BUTTON_Y => state.button_y = pressed,
            button_code::BUTTON_L1 => state.button_lb = pressed,
            button_code::BUTTON_R1 => state.button_rb = pressed,
            button_code::BUTTON_L2 => state.button_lt = pressed,
            button_code::BUTTON_R2 => state.button_rt = pressed,
            button_code::BUTTON_SELECT => state.button_select = pressed,
            button_code::BUTTON_START => state.button_start = pressed,
            button_code::BUTTON_THUMBL => state.button_thumbl = pressed,
            button_code::BUTTON_THUMBR => state.button_thumbr = pressed,
            button_code::DPAD_UP => state.dpad_up = pressed,
            button_code::DPAD_DOWN => state.dpad_down = pressed,
            button_code::DPAD_LEFT => state.dpad_left = pressed,
            button_code::DPAD_RIGHT => state.dpad_right = pressed,
            _ => {}
        }
    }
}

/// 标记手柄已连接
pub fn on_gamepad_connected() {
    if let Ok(mut state) = GAMEPAD_STATE.lock() {
        state.connected = true;
    }
}

/// 标记手柄已断开
pub fn on_gamepad_disconnected() {
    if let Ok(mut state) = GAMEPAD_STATE.lock() {
        *state = GamepadGlobalState::new();
    }
}

// ============================================================================
// 公共读取接口
// ============================================================================

/// 死区阈值 (25%)
const DEADZONE: f32 = 0.25;

/// 应用死区
#[inline]
fn apply_deadzone(value: f32) -> f32 {
    if value.abs() < DEADZONE {
        0.0
    } else {
        value
    }
}

/// 读取手柄状态快照 (游戏主循环调用)
pub fn read_gamepad_state() -> JoystickInput {
    if let Ok(state) = GAMEPAD_STATE.lock() {
        if !state.connected {
            return JoystickInput::disconnected();
        }
        
        // 应用死区到摇杆
        let left_x = apply_deadzone(state.left_x);
        let left_y = apply_deadzone(state.left_y);
        
        // 方向判断: 优先使用左摇杆，其次HAT轴，最后DPAD按键
        let left = left_x < -DEADZONE || state.hat_x < -0.5 || state.dpad_left;
        let right = left_x > DEADZONE || state.hat_x > 0.5 || state.dpad_right;
        let up = left_y < -DEADZONE || state.hat_y < -0.5 || state.dpad_up;
        let down = left_y > DEADZONE || state.hat_y > 0.5 || state.dpad_down;
        
        // 构建按钮位掩码 (用于 raw_buttons)
        let mut buttons: u32 = 0;
        if state.button_a { buttons |= 1 << 0; }
        if state.button_b { buttons |= 1 << 1; }
        if state.button_x { buttons |= 1 << 2; }
        if state.button_y { buttons |= 1 << 3; }
        if state.button_lb { buttons |= 1 << 4; }
        if state.button_rb { buttons |= 1 << 5; }
        if state.button_lt { buttons |= 1 << 6; }
        if state.button_rt { buttons |= 1 << 7; }
        if state.button_select { buttons |= 1 << 8; }
        if state.button_start { buttons |= 1 << 9; }
        
        JoystickInput {
            connected: true,
            left,
            right,
            up,
            down,
            button_a: state.button_a,
            button_b: state.button_b,
            button_x: state.button_x,
            button_y: state.button_y,
            button_lb: state.button_lb,
            button_rb: state.button_rb,
            button_select: state.button_select,
            button_start: state.button_start,
            // 游戏功能映射:
            // - 跳跃: A 或 B (button1)
            // - 加速/发射: LB 或 RB (button2)
            button1: state.button_a || state.button_b,
            button2: state.button_lb || state.button_rb,
            // 原始数据 (将浮点转为16位整数，0为中心，范围0-65535)
            raw_x: ((left_x + 1.0) * 32767.5) as u16,
            raw_y: ((left_y + 1.0) * 32767.5) as u16,
            raw_buttons: buttons,
        }
    } else {
        JoystickInput::disconnected()
    }
}

/// 检查是否检测到手柄
pub fn is_gamepad_connected() -> bool {
    if let Ok(state) = GAMEPAD_STATE.lock() {
        state.connected
    } else {
        false
    }
}

/// 重置手柄状态 (Activity 暂停/恢复时调用)
pub fn reset_gamepad_state() {
    if let Ok(mut state) = GAMEPAD_STATE.lock() {
        // 保留 connected 状态，只重置按钮和轴
        state.left_x = 0.0;
        state.left_y = 0.0;
        state.right_x = 0.0;
        state.right_y = 0.0;
        state.left_trigger = 0.0;
        state.right_trigger = 0.0;
        state.hat_x = 0.0;
        state.hat_y = 0.0;
        state.button_a = false;
        state.button_b = false;
        state.button_x = false;
        state.button_y = false;
        state.button_lb = false;
        state.button_rb = false;
        state.button_lt = false;
        state.button_rt = false;
        state.button_select = false;
        state.button_start = false;
        state.button_thumbl = false;
        state.button_thumbr = false;
        state.dpad_up = false;
        state.dpad_down = false;
        state.dpad_left = false;
        state.dpad_right = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_axis() {
        reset_gamepad_state();
        
        // 模拟左摇杆向右
        on_gamepad_axis(0, 0.8);
        let state = read_gamepad_state();
        assert!(state.connected);
        assert!(state.right);
        assert!(!state.left);
    }

    #[test]
    fn test_gamepad_button() {
        reset_gamepad_state();
        
        // 按下A按钮
        on_gamepad_button(button_code::BUTTON_A, true);
        let state = read_gamepad_state();
        assert!(state.button_a);
        assert!(state.button1);  // button1 = A || B
        
        // 释放A按钮
        on_gamepad_button(button_code::BUTTON_A, false);
        let state2 = read_gamepad_state();
        assert!(!state2.button_a);
    }

    #[test]
    fn test_deadzone() {
        reset_gamepad_state();
        
        // 小于死区的值应该被忽略
        on_gamepad_axis(0, 0.1);
        let state = read_gamepad_state();
        assert!(!state.left);
        assert!(!state.right);
        
        // 大于死区的值应该生效
        on_gamepad_axis(0, 0.5);
        let state2 = read_gamepad_state();
        assert!(state2.right);
    }
}
