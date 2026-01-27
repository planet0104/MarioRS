// JOYSTICK.PAS interface 对应的 Rust 模块
//
// P2-2 修复：使用安全的结构体封装替代 static mut
// - 消除线程不安全的 static mut
// - 提供易于测试的 API
// - 支持未来的输入系统整合
//
// 手柄/遥控器支持：
// - Windows: 使用 winmm.dll (Multimedia Joystick API)
// - Android TV: 遥控器按键由 Java 层处理 (MainActivity.mapKeyToGameButton)
//               转换为按钮事件后通过 JNI 传递，无需 Rust 端后端
// - 其他平台: 占位实现 (可扩展)

// Windows 手柄后端
#[cfg(target_os = "windows")]
use crate::platform::joystick_win::WinJoystick;

// JOYSTICK.PAS interface 结构体 - 手柄校准数据
#[derive(Debug, Clone, Copy, Default)]
pub struct JoyRec {
    pub x: u16,
    pub y: u16,
    pub x_center: u16,
    pub y_center: u16,
    pub x_min: u16,
    pub y_min: u16,
    pub x_max: u16,
    pub y_max: u16,
    pub x_left: u16,
    pub y_up: u16,
    pub x_right: u16,
    pub y_down: u16,
}

/// P2-2 修复：手柄状态管理器（替代 static mut）
///
/// 这是一个线程安全、易于测试的手柄状态封装，替代原来的 12 个 static mut 全局变量。
///
/// 优势：
/// - 线程安全（无 static mut）
/// - 易于测试（可创建实例）
/// - 清晰的所有权（由调用方持有）
/// - 支持未来扩展（例如多手柄支持）
pub struct JoystickState {
    pub detected: bool,
    pub enabled: bool,
    pub calibrated: bool,
    pub wait_button: bool,
    pub button_pressed: bool,
    // 主要按钮 (X/A/B/Y) - 按实际索引顺序
    pub button_x: bool,  // 按钮 0
    pub button_a: bool,  // 按钮 1
    pub button_b: bool,  // 按钮 2
    pub button_y: bool,  // 按钮 3
    // 肩键 (LB/RB)
    pub button_lb: bool, // 按钮 4
    pub button_rb: bool, // 按钮 5
    // 功能键
    pub button_select: bool, // 按钮 8
    pub button_start: bool,  // 按钮 9
    // 兼容旧接口 (游戏功能映射)
    pub button1: bool,  // 跳跃: A 或 B
    pub button2: bool,  // 加速: LB 或 RB
    // 方向
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub rec: JoyRec,

    // 平台后端
    #[cfg(target_os = "windows")]
    backend: WinJoystick,
}

impl Default for JoystickState {
    fn default() -> Self {
        Self::new()
    }
}

impl JoystickState {
    /// 创建新的手柄状态（默认为未检测到/未启用）
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        let backend = WinJoystick::new();

        Self {
            detected: false,
            enabled: false,
            calibrated: false,
            wait_button: false,
            button_pressed: false,
            // 主要按钮 (X/A/B/Y)
            button_x: false,
            button_a: false,
            button_b: false,
            button_y: false,
            // 肩键
            button_lb: false,
            button_rb: false,
            // 功能键
            button_select: false,
            button_start: false,
            // 兼容
            button1: false,
            button2: false,
            // 方向
            left: false,
            right: false,
            up: false,
            down: false,
            rec: JoyRec::default(),
            #[cfg(target_os = "windows")]
            backend,
        }
    }

    /// 读取手柄状态（对应 Pascal ReadJoystick）
    pub fn read(&mut self) {
        #[cfg(target_os = "windows")]
        {
            use crate::platform::joystick_win::JoystickInput;

            let input: JoystickInput = self.backend.read();

            // 更新状态
            self.detected = input.connected;
            self.enabled = input.connected;
            // 方向
            self.left = input.left;
            self.right = input.right;
            self.up = input.up;
            self.down = input.down;
            // 主要按钮 (X/A/B/Y)
            self.button_x = input.button_x;
            self.button_a = input.button_a;
            self.button_b = input.button_b;
            self.button_y = input.button_y;
            // 肩键
            self.button_lb = input.button_lb;
            self.button_rb = input.button_rb;
            // 功能键
            self.button_select = input.button_select;
            self.button_start = input.button_start;
            // 兼容旧接口 (button1=跳跃, button2=加速)
            self.button1 = input.button1;  // A 或 B
            self.button2 = input.button2;  // LB 或 RB
            self.button_pressed = input.button_a || input.button_b || input.button_x || input.button_y;
            // 原始数据
            self.rec.x = input.raw_x;
            self.rec.y = input.raw_y;
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 其他平台: 占位实现
            // 可以在这里添加其他平台的手柄支持 (如 gilrs for Linux/macOS)
        }
    }

    /// 重置手柄状态（对应 Pascal ResetJoystick）
    pub fn reset(&mut self) {
        self.button_pressed = false;
        self.button_x = false;
        self.button_a = false;
        self.button_b = false;
        self.button_y = false;
        self.button_lb = false;
        self.button_rb = false;
        self.button_select = false;
        self.button_start = false;
        self.button1 = false;
        self.button2 = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.down = false;
    }

    /// 校准手柄（对应 Pascal Calibrate）
    pub fn calibrate(&mut self) {
        #[cfg(target_os = "windows")]
        {
            // 重新扫描手柄设备，自动获取校准参数
            self.backend.scan_devices();
            self.calibrated = self.backend.is_connected();
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 其他平台 (包括 Android): 无需校准
            // Android TV 遥控器按键由 Java 层处理
            self.calibrated = true;
        }
    }

    /// 检查手柄是否已连接
    pub fn is_connected(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.backend.is_connected()
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 其他平台 (包括 Android): 返回 false
            // Android TV 遥控器输入通过键盘/按钮事件处理，不使用手柄后端
            false
        }
    }

    /// 设置按钮映射 (jump_button, action_button)
    #[cfg(target_os = "windows")]
    pub fn set_button_mapping(&mut self, jump: u32, action: u32) {
        self.backend.set_button_mapping(jump, action);
    }

    /// 设置死区阈值 (0.0 - 1.0)
    #[cfg(target_os = "windows")]
    pub fn set_deadzone(&mut self, deadzone: f32) {
        self.backend.set_deadzone(deadzone);
    }

    /// 检查是否有方向键被按下
    pub fn has_direction(&self) -> bool {
        self.left || self.right || self.up || self.down
    }

    /// 检查是否有按钮被按下
    pub fn has_button(&self) -> bool {
        self.button_a || self.button_b || self.button_x || self.button_y
            || self.button_lb || self.button_rb
    }
}

// ============================================================================
// 向后兼容的全局函数（如果需要的话，未来可以移除）
// ============================================================================

/// 占位函数：读取手柄（向后兼容）
///
/// 注意：新代码应该使用 JoystickState 实例方法
pub fn read_joystick() {
    // 占位实现
}

/// 占位函数：重置手柄（向后兼容）
///
/// 注意：新代码应该使用 JoystickState 实例方法
pub fn reset_joystick() {
    // 占位实现
}

/// 占位函数：校准手柄（向后兼容）
///
/// 注意：新代码应该使用 JoystickState 实例方法
pub fn calibrate() {
    // 占位实现
}
