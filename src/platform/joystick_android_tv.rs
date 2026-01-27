//! Android TV 遥控器支持模块
//!
//! 实现 Android TV 平台下使用遥控器控制游戏的功能
//! 支持多种遥控器类型的按键映射，包括主推按键和备选按键

use std::collections::HashSet;

// ============================================================================
// Android TV 遥控器按键码常量 (KeyEvent.KEYCODE_*)
// ============================================================================

/// Android 按键码常量
pub mod keycode {
    // 方向键 (D-Pad)
    pub const DPAD_UP: i32 = 19;
    pub const DPAD_DOWN: i32 = 20;
    pub const DPAD_LEFT: i32 = 21;
    pub const DPAD_RIGHT: i32 = 22;
    pub const DPAD_CENTER: i32 = 23;  // OK/Select 键

    // 确认和返回
    pub const ENTER: i32 = 66;
    pub const BACK: i32 = 4;

    // 彩色按键 (部分遥控器支持)
    pub const PROG_RED: i32 = 183;
    pub const PROG_GREEN: i32 = 184;
    pub const PROG_YELLOW: i32 = 185;
    pub const PROG_BLUE: i32 = 186;

    // 音量键
    pub const VOLUME_UP: i32 = 24;
    pub const VOLUME_DOWN: i32 = 25;

    // 媒体控制键
    pub const MEDIA_PLAY_PAUSE: i32 = 85;
    pub const MEDIA_PLAY: i32 = 126;
    pub const MEDIA_PAUSE: i32 = 127;
    pub const MEDIA_STOP: i32 = 86;
    pub const MEDIA_FAST_FORWARD: i32 = 90;
    pub const MEDIA_REWIND: i32 = 89;
    pub const MEDIA_NEXT: i32 = 87;
    pub const MEDIA_PREVIOUS: i32 = 88;

    // 数字键
    pub const NUM_0: i32 = 7;
    pub const NUM_1: i32 = 8;
    pub const NUM_2: i32 = 9;
    pub const NUM_3: i32 = 10;
    pub const NUM_4: i32 = 11;
    pub const NUM_5: i32 = 12;
    pub const NUM_6: i32 = 13;
    pub const NUM_7: i32 = 14;
    pub const NUM_8: i32 = 15;
    pub const NUM_9: i32 = 16;

    // 频道键
    pub const CHANNEL_UP: i32 = 166;
    pub const CHANNEL_DOWN: i32 = 167;

    // 菜单键
    pub const MENU: i32 = 82;
    pub const INFO: i32 = 165;

    // Fire TV 特殊键
    pub const BUTTON_A: i32 = 96;
    pub const BUTTON_B: i32 = 97;
    pub const BUTTON_X: i32 = 99;
    pub const BUTTON_Y: i32 = 100;
}

// ============================================================================
// 游戏动作枚举
// ============================================================================

/// 游戏动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    /// 移动 - 上
    MoveUp,
    /// 移动 - 下
    MoveDown,
    /// 移动 - 左
    MoveLeft,
    /// 移动 - 右
    MoveRight,
    /// 跳跃
    Jump,
    /// 发射子弹
    Fire,
    /// 加速跑
    Run,
    /// 确认/开始
    Confirm,
    /// 返回/暂停
    Back,
    /// 菜单
    Menu,
}

// ============================================================================
// Android TV 遥控器输入状态
// ============================================================================

/// Android TV 遥控器输入状态
#[derive(Debug, Clone, Default)]
pub struct TvRemoteInput {
    pub connected: bool,
    // 方向
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    // 游戏动作
    pub jump: bool,
    pub fire: bool,
    pub run: bool,
    // 功能键
    pub confirm: bool,
    pub back: bool,
    pub menu: bool,
    // 兼容旧接口 (与 JoystickInput 保持一致)
    pub button1: bool,  // 跳跃
    pub button2: bool,  // 加速
    pub button_x: bool,
    pub button_y: bool,
    pub button_a: bool,
    pub button_b: bool,
}

impl TvRemoteInput {
    pub fn disconnected() -> Self {
        Self::default()
    }
}

// ============================================================================
// Android TV 遥控器管理器
// ============================================================================

/// Android TV 遥控器管理器
///
/// 处理 Android TV 遥控器按键输入，支持多种遥控器类型
/// 通过多按键映射提高兼容性
pub struct AndroidTvRemote {
    /// 当前按下的按键集合
    pressed_keys: HashSet<i32>,
    /// 最后检测时间
    last_update_time: u64,
}

impl AndroidTvRemote {
    /// 创建新的遥控器管理器
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            last_update_time: 0,
        }
    }

    /// 处理按键按下事件
    pub fn on_key_down(&mut self, key_code: i32) {
        self.pressed_keys.insert(key_code);
    }

    /// 处理按键释放事件
    pub fn on_key_up(&mut self, key_code: i32) {
        self.pressed_keys.remove(&key_code);
    }

    /// 检查按键是否按下
    #[inline]
    pub fn is_pressed(&self, key_code: i32) -> bool {
        self.pressed_keys.contains(&key_code)
    }

    /// 检查多个按键中是否有任一按下
    #[inline]
    pub fn is_any_pressed(&self, key_codes: &[i32]) -> bool {
        key_codes.iter().any(|&k| self.pressed_keys.contains(&k))
    }

    /// 读取当前输入状态
    ///
    /// 按键映射优先级：
    /// - 跳跃: 红色键 > 播放/暂停键 > 快进键 > A按钮
    /// - 发射: 绿色键 > 快退键 > 音量-键 > X按钮
    /// - 加速: 音量+键 > 数字键1 > B按钮
    pub fn read(&self) -> TvRemoteInput {
        // 方向键
        let up = self.is_pressed(keycode::DPAD_UP);
        let down = self.is_pressed(keycode::DPAD_DOWN);
        let left = self.is_pressed(keycode::DPAD_LEFT);
        let right = self.is_pressed(keycode::DPAD_RIGHT);

        // 跳跃: 红色键 > 播放/暂停键 > 快进键 > A按钮
        let jump = self.is_any_pressed(&[
            keycode::PROG_RED,
            keycode::MEDIA_PLAY_PAUSE,
            keycode::MEDIA_PLAY,
            keycode::MEDIA_FAST_FORWARD,
            keycode::BUTTON_A,
        ]);

        // 发射: 绿色键 > 快退键 > X按钮
        // 注意: 音量-键作为发射备选，但优先级较低
        let fire = self.is_any_pressed(&[
            keycode::PROG_GREEN,
            keycode::MEDIA_REWIND,
            keycode::BUTTON_X,
        ]) || (!self.is_pressed(keycode::VOLUME_UP) && self.is_pressed(keycode::VOLUME_DOWN));

        // 加速: 音量+键 > 数字键1 > B按钮 > Y按钮
        let run = self.is_any_pressed(&[
            keycode::VOLUME_UP,
            keycode::NUM_1,
            keycode::BUTTON_B,
            keycode::BUTTON_Y,
        ]);

        // 确认: OK键 > Enter键 > A按钮
        let confirm = self.is_any_pressed(&[
            keycode::DPAD_CENTER,
            keycode::ENTER,
            keycode::BUTTON_A,
        ]);

        // 返回: Back键 > B按钮
        let back = self.is_any_pressed(&[
            keycode::BACK,
            keycode::BUTTON_B,
        ]);

        // 菜单: Menu键 > 黄色键
        let menu = self.is_any_pressed(&[
            keycode::MENU,
            keycode::PROG_YELLOW,
        ]);

        TvRemoteInput {
            connected: true,
            left,
            right,
            up,
            down,
            jump,
            fire,
            run,
            confirm,
            back,
            menu,
            // 兼容旧接口
            button1: jump,      // 跳跃
            button2: run,       // 加速
            button_x: fire,     // 发射
            button_y: run,      // 加速(备选)
            button_a: jump || confirm,  // 跳跃/确认
            button_b: back,     // 返回
        }
    }

    /// 重置所有按键状态
    pub fn reset(&mut self) {
        self.pressed_keys.clear();
    }

    /// 检查是否有遥控器连接
    /// 
    /// 在 Android TV 上，遥控器总是被认为已连接
    pub fn is_connected(&self) -> bool {
        true
    }

    /// 将 Android 按键码转换为游戏动作
    pub fn key_to_action(key_code: i32) -> Option<GameAction> {
        match key_code {
            keycode::DPAD_UP => Some(GameAction::MoveUp),
            keycode::DPAD_DOWN => Some(GameAction::MoveDown),
            keycode::DPAD_LEFT => Some(GameAction::MoveLeft),
            keycode::DPAD_RIGHT => Some(GameAction::MoveRight),
            
            // 跳跃按键
            keycode::PROG_RED | 
            keycode::MEDIA_PLAY_PAUSE | 
            keycode::MEDIA_PLAY |
            keycode::MEDIA_FAST_FORWARD |
            keycode::BUTTON_A => Some(GameAction::Jump),
            
            // 发射按键
            keycode::PROG_GREEN |
            keycode::MEDIA_REWIND |
            keycode::BUTTON_X => Some(GameAction::Fire),
            
            // 加速按键
            keycode::VOLUME_UP |
            keycode::NUM_1 |
            keycode::BUTTON_B |
            keycode::BUTTON_Y => Some(GameAction::Run),
            
            // 确认按键
            keycode::DPAD_CENTER |
            keycode::ENTER => Some(GameAction::Confirm),
            
            // 返回按键
            keycode::BACK => Some(GameAction::Back),
            
            // 菜单按键
            keycode::MENU |
            keycode::PROG_YELLOW => Some(GameAction::Menu),
            
            _ => None,
        }
    }

    /// 检查按键是否应该被游戏消费(阻止系统处理)
    /// 
    /// 音量键等系统按键需要特殊处理
    pub fn should_consume_key(key_code: i32) -> bool {
        matches!(key_code, 
            keycode::DPAD_UP | keycode::DPAD_DOWN | 
            keycode::DPAD_LEFT | keycode::DPAD_RIGHT |
            keycode::DPAD_CENTER |
            keycode::PROG_RED | keycode::PROG_GREEN | 
            keycode::PROG_YELLOW | keycode::PROG_BLUE |
            keycode::MEDIA_PLAY_PAUSE | keycode::MEDIA_PLAY |
            keycode::MEDIA_FAST_FORWARD | keycode::MEDIA_REWIND |
            keycode::BUTTON_A | keycode::BUTTON_B |
            keycode::BUTTON_X | keycode::BUTTON_Y |
            keycode::NUM_1 | keycode::NUM_2 | keycode::NUM_3
        )
    }
}

impl Default for AndroidTvRemote {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 按键码辅助函数
// ============================================================================

/// 获取按键名称(用于调试日志)
pub fn get_key_name(key_code: i32) -> &'static str {
    match key_code {
        keycode::DPAD_UP => "DPAD_UP",
        keycode::DPAD_DOWN => "DPAD_DOWN",
        keycode::DPAD_LEFT => "DPAD_LEFT",
        keycode::DPAD_RIGHT => "DPAD_RIGHT",
        keycode::DPAD_CENTER => "DPAD_CENTER",
        keycode::ENTER => "ENTER",
        keycode::BACK => "BACK",
        keycode::PROG_RED => "RED",
        keycode::PROG_GREEN => "GREEN",
        keycode::PROG_YELLOW => "YELLOW",
        keycode::PROG_BLUE => "BLUE",
        keycode::VOLUME_UP => "VOL+",
        keycode::VOLUME_DOWN => "VOL-",
        keycode::MEDIA_PLAY_PAUSE => "PLAY/PAUSE",
        keycode::MEDIA_PLAY => "PLAY",
        keycode::MEDIA_PAUSE => "PAUSE",
        keycode::MEDIA_FAST_FORWARD => "FF",
        keycode::MEDIA_REWIND => "REW",
        keycode::BUTTON_A => "BTN_A",
        keycode::BUTTON_B => "BTN_B",
        keycode::BUTTON_X => "BTN_X",
        keycode::BUTTON_Y => "BTN_Y",
        keycode::NUM_1 => "NUM_1",
        keycode::NUM_2 => "NUM_2",
        keycode::NUM_3 => "NUM_3",
        keycode::MENU => "MENU",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_input() {
        let mut remote = AndroidTvRemote::new();
        
        // 测试方向键
        remote.on_key_down(keycode::DPAD_UP);
        let input = remote.read();
        assert!(input.up);
        assert!(!input.down);
        
        remote.on_key_up(keycode::DPAD_UP);
        let input = remote.read();
        assert!(!input.up);
    }

    #[test]
    fn test_jump_mapping() {
        let mut remote = AndroidTvRemote::new();
        
        // 红色键触发跳跃
        remote.on_key_down(keycode::PROG_RED);
        assert!(remote.read().jump);
        remote.on_key_up(keycode::PROG_RED);
        
        // 播放/暂停键触发跳跃
        remote.on_key_down(keycode::MEDIA_PLAY_PAUSE);
        assert!(remote.read().jump);
        remote.on_key_up(keycode::MEDIA_PLAY_PAUSE);
    }

    #[test]
    fn test_fire_mapping() {
        let mut remote = AndroidTvRemote::new();
        
        // 绿色键触发发射
        remote.on_key_down(keycode::PROG_GREEN);
        assert!(remote.read().fire);
        remote.on_key_up(keycode::PROG_GREEN);
        
        // 快退键触发发射
        remote.on_key_down(keycode::MEDIA_REWIND);
        assert!(remote.read().fire);
    }

    #[test]
    fn test_run_mapping() {
        let mut remote = AndroidTvRemote::new();
        
        // 音量+键触发加速
        remote.on_key_down(keycode::VOLUME_UP);
        assert!(remote.read().run);
    }
}
