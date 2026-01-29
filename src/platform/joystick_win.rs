//! Windows 手柄支持模块
//!
//! 使用 Windows Multimedia API (winmm.dll) 读取手柄输入
//! 支持任何在 joy.cpl 中显示的 USB/蓝牙手柄

use std::mem::MaybeUninit;

// ============================================================================
// Windows Joystick API 常量和结构体
// ============================================================================

const JOYERR_NOERROR: u32 = 0;
const JOY_RETURNALL: u32 = 0x00FF;

/// JOYINFOEX 结构体 - 手柄状态信息
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct JOYINFOEX {
    dw_size: u32,
    dw_flags: u32,
    dw_xpos: u32,
    dw_ypos: u32,
    dw_zpos: u32,
    dw_rpos: u32,
    dw_upos: u32,
    dw_vpos: u32,
    dw_buttons: u32,
    dw_button_number: u32,
    dw_pov: u32,
    dw_reserved1: u32,
    dw_reserved2: u32,
}

/// JOYCAPSW 结构体 - 手柄能力信息
#[repr(C)]
struct JOYCAPSW {
    w_mid: u16,
    w_pid: u16,
    sz_pname: [u16; 32],
    w_xmin: u32,
    w_xmax: u32,
    w_ymin: u32,
    w_ymax: u32,
    w_zmin: u32,
    w_zmax: u32,
    w_num_buttons: u32,
    w_period_min: u32,
    w_period_max: u32,
    w_rmin: u32,
    w_rmax: u32,
    w_umin: u32,
    w_umax: u32,
    w_vmin: u32,
    w_vmax: u32,
    w_caps: u32,
    w_max_axes: u32,
    w_num_axes: u32,
    w_max_buttons: u32,
    sz_reg_key: [u16; 32],
    sz_oem_vx_d: [u16; 260],
}

#[link(name = "winmm")]
unsafe extern "system" {
    fn joyGetNumDevs() -> u32;
    fn joyGetPosEx(joy_id: u32, pji: *mut JOYINFOEX) -> u32;
    fn joyGetDevCapsW(joy_id: u32, pjc: *mut JOYCAPSW, cbjc: u32) -> u32;
}

// ============================================================================
// 手柄校准数据
// ============================================================================

/// 手柄轴校准参数
#[derive(Debug, Clone, Copy)]
struct AxisCalibration {
    min: u32,
    max: u32,
    center: u32,
    threshold: u32,
}

impl AxisCalibration {
    fn new(min: u32, max: u32) -> Self {
        let center = (min + max) / 2;
        let range = max - min;
        // 死区阈值: 25% (可根据需要调整)
        let threshold = range / 4;
        Self {
            min,
            max,
            center,
            threshold,
        }
    }

    /// 检查轴值是否在负方向 (左/上)
    #[inline]
    fn is_negative(&self, value: u32) -> bool {
        value < self.center.saturating_sub(self.threshold)
    }

    /// 检查轴值是否在正方向 (右/下)
    #[inline]
    fn is_positive(&self, value: u32) -> bool {
        value > self.center + self.threshold
    }
}

// ============================================================================
// 手柄输入状态 (返回给调用方)
// ============================================================================

/// 手柄输入状态
///
/// 按钮布局 (通用 USB 手柄):
/// - button_x (0): X
/// - button_a (1): A
/// - button_b (2): B
/// - button_y (3): Y
/// - button_lb (4): LB / L
/// - button_rb (5): RB / R
/// - button_select (8): SELECT / BACK
/// - button_start (9): START
#[derive(Debug, Clone, Copy, Default)]
pub struct JoystickInput {
    pub connected: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    // 主要按钮 (X/A/B/Y)
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
    // 兼容旧接口 (跳跃 / 加速)
    pub button1: bool,   // 跳跃: A 或 B
    pub button2: bool,   // 加速: LB 或 RB
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
// Windows 手柄管理器
// ============================================================================

/// Windows 手柄管理器
///
/// 封装 Windows Multimedia Joystick API，提供简单的手柄读取接口
pub struct WinJoystick {
    /// 当前活动的手柄 ID (None = 未检测到手柄)
    active_id: Option<u32>,
    /// X 轴校准
    x_axis: AxisCalibration,
    /// Y 轴校准
    y_axis: AxisCalibration,
    /// 上次检测手柄的时间 (用于定期重新扫描)
    last_scan_time: u64,
    /// 按钮映射: 跳跃按钮索引
    jump_button: u32,
    /// 按钮映射: 动作按钮索引
    action_button: u32,
}

impl WinJoystick {
    /// 创建新的手柄管理器
    pub fn new() -> Self {
        let mut joystick = Self {
            active_id: None,
            x_axis: AxisCalibration::new(0, 65535),
            y_axis: AxisCalibration::new(0, 65535),
            last_scan_time: 0,
            jump_button: 0,   // 按钮 0 (通常是 A/X)
            action_button: 1, // 按钮 1 (通常是 B/O)
        };
        // 初始化时扫描手柄
        joystick.scan_devices();
        joystick
    }

    /// 扫描并连接第一个可用的手柄
    pub fn scan_devices(&mut self) -> bool {
        let num_devs = unsafe { joyGetNumDevs() };

        for joy_id in 0..num_devs.min(16) {
            let mut caps: MaybeUninit<JOYCAPSW> = MaybeUninit::uninit();
            let result = unsafe {
                joyGetDevCapsW(
                    joy_id,
                    caps.as_mut_ptr(),
                    std::mem::size_of::<JOYCAPSW>() as u32,
                )
            };

            if result == JOYERR_NOERROR {
                let caps = unsafe { caps.assume_init() };

                // 设置轴校准参数
                self.x_axis = AxisCalibration::new(caps.w_xmin, caps.w_xmax);
                self.y_axis = AxisCalibration::new(caps.w_ymin, caps.w_ymax);
                self.active_id = Some(joy_id);

                #[cfg(debug_assertions)]
                {
                    let name: String = caps
                        .sz_pname
                        .iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                        .collect();
                    eprintln!(
                        "[Joystick] 检测到手柄 {}: {} ({} 按钮, {} 轴)",
                        joy_id, name, caps.w_num_buttons, caps.w_num_axes
                    );
                }

                return true;
            }
        }

        self.active_id = None;
        false
    }

    /// 检查手柄是否已连接
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.active_id.is_some()
    }

    /// 读取手柄状态
    ///
    /// 返回 (成功, 方向状态, 按钮状态)
    pub fn read(&mut self) -> JoystickInput {
        let Some(joy_id) = self.active_id else {
            // 没有活动手柄，定期尝试重新扫描
            self.try_rescan();
            return JoystickInput::disconnected();
        };

        let mut info = JOYINFOEX {
            dw_size: std::mem::size_of::<JOYINFOEX>() as u32,
            dw_flags: JOY_RETURNALL,
            dw_xpos: 0,
            dw_ypos: 0,
            dw_zpos: 0,
            dw_rpos: 0,
            dw_upos: 0,
            dw_vpos: 0,
            dw_buttons: 0,
            dw_button_number: 0,
            dw_pov: 0,
            dw_reserved1: 0,
            dw_reserved2: 0,
        };

        let result = unsafe { joyGetPosEx(joy_id, &mut info) };

        if result != JOYERR_NOERROR {
            // 手柄可能已断开
            self.active_id = None;
            return JoystickInput::disconnected();
        }

        // 读取所有按钮状态 (根据实际测试的按钮映射)
        // X=[0] A=[1] B=[2] Y=[3] LB=[4] RB=[5] SELECT=[8] START=[9]
        let buttons = info.dw_buttons;
        let button_x = (buttons & (1 << 0)) != 0;      // X
        let button_a = (buttons & (1 << 1)) != 0;      // A
        let button_b = (buttons & (1 << 2)) != 0;      // B
        let button_y = (buttons & (1 << 3)) != 0;      // Y
        let button_lb = (buttons & (1 << 4)) != 0;     // LB
        let button_rb = (buttons & (1 << 5)) != 0;     // RB
        let button_select = (buttons & (1 << 8)) != 0; // SELECT
        let button_start = (buttons & (1 << 9)) != 0;  // START

        JoystickInput {
            connected: true,
            left: self.x_axis.is_negative(info.dw_xpos),
            right: self.x_axis.is_positive(info.dw_xpos),
            up: self.y_axis.is_negative(info.dw_ypos),
            down: self.y_axis.is_positive(info.dw_ypos),
            // 主要按钮 (X/A/B/Y)
            button_x,
            button_a,
            button_b,
            button_y,
            // 肩键
            button_lb,
            button_rb,
            // 功能键
            button_select,
            button_start,
            // 游戏功能映射:
            // - 跳跃: A 或 B (button1)
            // - 加速/发射: LB 或 RB (button2)
            button1: button_a || button_b,
            button2: button_lb || button_rb,
            // 原始数据
            raw_x: info.dw_xpos as u16,
            raw_y: info.dw_ypos as u16,
            raw_buttons: buttons,
        }
    }

    /// 尝试重新扫描手柄 (节流: 每秒最多一次)
    fn try_rescan(&mut self) {
        // 简单的时间检查，避免频繁扫描
        // 使用 QueryPerformanceCounter 会更准确，但这里简化处理
        let current = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if current > self.last_scan_time + 2 {
            self.last_scan_time = current;
            self.scan_devices();
        }
    }

    /// 设置按钮映射
    pub fn set_button_mapping(&mut self, jump: u32, action: u32) {
        self.jump_button = jump;
        self.action_button = action;
    }

    /// 设置死区阈值 (0.0 - 1.0)
    pub fn set_deadzone(&mut self, deadzone: f32) {
        let deadzone = deadzone.clamp(0.0, 0.5);
        let x_range = self.x_axis.max - self.x_axis.min;
        let y_range = self.y_axis.max - self.y_axis.min;
        self.x_axis.threshold = (x_range as f32 * deadzone) as u32;
        self.y_axis.threshold = (y_range as f32 * deadzone) as u32;
    }
}

impl Default for WinJoystick {
    fn default() -> Self {
        Self::new()
    }
}
