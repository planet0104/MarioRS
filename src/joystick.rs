// JOYSTICK.PAS interface 对应的 Rust 模块
//
// P2-2 修复：使用安全的结构体封装替代 static mut
// - 消除线程不安全的 static mut
// - 提供易于测试的 API
// - 支持未来的输入系统整合

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
#[derive(Debug, Clone)]
pub struct JoystickState {
    pub detected: bool,
    pub enabled: bool,
    pub calibrated: bool,
    pub wait_button: bool,
    pub button_pressed: bool,
    pub button1: bool,
    pub button2: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub rec: JoyRec,
}

impl Default for JoystickState {
    fn default() -> Self {
        Self::new()
    }
}

impl JoystickState {
    /// 创建新的手柄状态（默认为未检测到/未启用）
    pub fn new() -> Self {
        Self {
            detected: false,
            enabled: false,
            calibrated: false,
            wait_button: false,
            button_pressed: false,
            button1: false,
            button2: false,
            left: false,
            right: false,
            up: false,
            down: false,
            rec: JoyRec::default(),
        }
    }

    /// 读取手柄状态（对应 Pascal ReadJoystick）
    pub fn read(&mut self) {
        // 占位实现 - 当前项目不使用真实手柄
        // 未来可以在这里集成实际的手柄 API
    }

    /// 重置手柄状态（对应 Pascal ResetJoystick）
    pub fn reset(&mut self) {
        self.button_pressed = false;
        self.button1 = false;
        self.button2 = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.down = false;
    }

    /// 校准手柄（对应 Pascal Calibrate）
    pub fn calibrate(&mut self) {
        // 占位实现 - 当前项目不使用真实手柄
        self.calibrated = true;
    }

    /// 检查是否有方向键被按下
    pub fn has_direction(&self) -> bool {
        self.left || self.right || self.up || self.down
    }

    /// 检查是否有按钮被按下
    pub fn has_button(&self) -> bool {
        self.button1 || self.button2
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
