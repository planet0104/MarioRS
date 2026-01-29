// Android 平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: android-activity + wgpu + cpal + ndk
//
// 重要: 这个模块依赖 android-activity, 其他游戏模块通过 platform.rs 抽象访问
//
// 输入架构 (手柄与遥控器完全分离):
// - 手柄: Java GamepadController -> nativeOnGamepadButton/Axis -> joystick_android.rs
// - 遥控器: Java RemoteController -> nativeOnRemoteKey -> joystick_android_tv.rs
// - 虚拟按钮: Java VirtualController -> nativeOnButtonEvent -> 键盘事件队列

use super::common::{CommonRandom, CommonTime, FileStorage, FpsCounter};
use super::joystick_android;
use super::joystick_android_tv;
use super::{
    DisplayBackend, InputBackend, KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, StorageBackend,
};
use crate::gpu::GpuRenderer;
use crate::status::RenderMode;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ============================================================================
// JNI 原生按钮事件队列 (解决多点触摸延迟问题)
// ============================================================================

/// 原生按钮事件
#[derive(Clone, Copy, Debug)]
pub struct NativeButtonEvent {
    pub button_id: i32,
    pub pressed: bool,
}

/// 原生按钮 ID 常量 (与 Java 代码保持一致)
pub mod native_button {
    pub const DPAD_LEFT: i32 = 1;
    pub const DPAD_RIGHT: i32 = 2;
    pub const DPAD_UP: i32 = 3;
    pub const DPAD_DOWN: i32 = 4;
    pub const BTN_A: i32 = 5;
    pub const BTN_B: i32 = 6;
    pub const BTN_X: i32 = 7;
    pub const BTN_Y: i32 = 8;
}

/// 全局事件队列 - 用于 JNI 回调线程和游戏主线程通信
static NATIVE_BUTTON_EVENTS: Mutex<Vec<NativeButtonEvent>> = Mutex::new(Vec::new());

/// 软键盘按键事件
#[derive(Clone, Copy, Debug)]
pub struct SoftKeyEvent {
    pub key_code: i32,
    pub pressed: bool,
}

/// 软键盘事件队列
static SOFT_KEY_EVENTS: Mutex<Vec<SoftKeyEvent>> = Mutex::new(Vec::new());

/// JNI 导出函数 - 由 Java MainActivity 调用 (游戏按钮)
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnButtonEvent(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    button_id: i32,
    pressed: i32,
) {
    if let Ok(mut queue) = NATIVE_BUTTON_EVENTS.lock() {
        queue.push(NativeButtonEvent { button_id, pressed: pressed != 0 });
    }
}

// ============================================================================
// 手柄专用 JNI 接口 (与遥控器完全分离)
// Java GamepadController -> Rust joystick_android 模块
// ============================================================================

/// JNI 导出函数 - 手柄按钮事件
/// 由 Java GamepadController.handleKeyEvent() 调用
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnGamepadButton(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    log_debug(&format!("[JNI Gamepad] button key_code={}, pressed={}", key_code, pressed != 0));
    // 直接转发到手柄模块
    joystick_android::on_gamepad_button(key_code, pressed != 0);
    joystick_android::on_gamepad_connected();
}

/// JNI 导出函数 - 手柄摇杆轴事件
/// 由 Java GamepadController.handleMotionEvent() 调用
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnGamepadAxis(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    axis_id: i32,
    value: f32,
) {
    // 直接转发到手柄模块
    joystick_android::on_gamepad_axis(axis_id, value);
}

// ============================================================================
// 遥控器专用 JNI 接口 (与手柄完全分离)
// Java RemoteController -> Rust joystick_android_tv 模块
// ============================================================================

/// JNI 导出函数 - 遥控器按键事件
/// 由 Java RemoteController.handleKeyEvent() 调用
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnRemoteKey(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    log_debug(&format!("[JNI Remote] key_code={}, pressed={}", key_code, pressed != 0));
    // 直接转发到遥控器模块
    joystick_android_tv::on_tv_remote_key(key_code, pressed != 0);
    
    // 同时加入软键盘事件队列 (用于键盘输入处理，如菜单导航)
    if let Ok(mut queue) = SOFT_KEY_EVENTS.lock() {
        queue.push(SoftKeyEvent { key_code, pressed: pressed != 0 });
    }
}

// ============================================================================
// 兼容性: 保留旧的 nativeOnKeyEvent (已弃用，仅用于软键盘)
// ============================================================================

/// JNI 导出函数 - 软键盘按键 (已弃用，保留兼容性)
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnKeyEvent(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    // 旧接口，转发到软键盘队列
    if let Ok(mut queue) = SOFT_KEY_EVENTS.lock() {
        queue.push(SoftKeyEvent { key_code, pressed: pressed != 0 });
    }
}

/// 从队列中取出所有待处理的软键盘事件
pub fn take_soft_key_events() -> Vec<SoftKeyEvent> {
    if let Ok(mut queue) = SOFT_KEY_EVENTS.lock() {
        std::mem::take(&mut *queue)
    } else {
        Vec::new()
    }
}

/// 从队列中取出所有待处理的原生按钮事件
pub fn take_native_button_events() -> Vec<NativeButtonEvent> {
    if let Ok(mut queue) = NATIVE_BUTTON_EVENTS.lock() {
        std::mem::take(&mut *queue)
    } else {
        Vec::new()
    }
}

/// 将原生按钮事件转换为平台按键事件
pub fn native_button_to_key_event(event: &NativeButtonEvent) -> PlatformKeyEvent {
    let key = match event.button_id {
        native_button::DPAD_LEFT => PlatformKeyCode::Left,
        native_button::DPAD_RIGHT => PlatformKeyCode::Right,
        native_button::DPAD_UP => PlatformKeyCode::Up,
        native_button::DPAD_DOWN => PlatformKeyCode::Down,
        native_button::BTN_A => PlatformKeyCode::AltLeft,      // A = Jump
        native_button::BTN_B => PlatformKeyCode::ControlLeft,  // B = Run
        native_button::BTN_X => PlatformKeyCode::Space,        // X = Fire
        native_button::BTN_Y => PlatformKeyCode::ShiftLeft,    // Y = Special
        _ => PlatformKeyCode::Unknown,
    };
    PlatformKeyEvent {
        key,
        pressed: event.pressed,
    }
}

/// 将软键盘事件转换为平台按键事件
/// 使用 Android KeyEvent 常量值 (与 android-activity Keycode 枚举值相同)
pub fn soft_key_to_platform_event(event: &SoftKeyEvent) -> PlatformKeyEvent {
    // Android KeyEvent 常量 (https://developer.android.com/reference/android/view/KeyEvent)
    const KEYCODE_BACK: i32 = 4;      // 返回键 -> Escape
    const KEYCODE_TAB: i32 = 61;
    const KEYCODE_SPACE: i32 = 62;
    const KEYCODE_ENTER: i32 = 66;
    const KEYCODE_DEL: i32 = 67;  // Backspace
    const KEYCODE_GRAVE: i32 = 68;  // ` 反引号
    const KEYCODE_SEMICOLON: i32 = 74;  // ; / : 分号/冒号
    const KEYCODE_A: i32 = 29;
    const KEYCODE_B: i32 = 30;
    const KEYCODE_C: i32 = 31;
    const KEYCODE_D: i32 = 32;
    const KEYCODE_E: i32 = 33;
    const KEYCODE_F: i32 = 34;
    const KEYCODE_G: i32 = 35;
    const KEYCODE_H: i32 = 36;
    const KEYCODE_I: i32 = 37;
    const KEYCODE_J: i32 = 38;
    const KEYCODE_K: i32 = 39;
    const KEYCODE_L: i32 = 40;
    const KEYCODE_M: i32 = 41;
    const KEYCODE_N: i32 = 42;
    const KEYCODE_O: i32 = 43;
    const KEYCODE_P: i32 = 44;
    const KEYCODE_Q: i32 = 45;
    const KEYCODE_R: i32 = 46;
    const KEYCODE_S: i32 = 47;
    const KEYCODE_T: i32 = 48;
    const KEYCODE_U: i32 = 49;
    const KEYCODE_V: i32 = 50;
    const KEYCODE_W: i32 = 51;
    const KEYCODE_X: i32 = 52;
    const KEYCODE_Y: i32 = 53;
    const KEYCODE_Z: i32 = 54;
    const KEYCODE_0: i32 = 7;
    const KEYCODE_1: i32 = 8;
    const KEYCODE_2: i32 = 9;
    const KEYCODE_3: i32 = 10;
    const KEYCODE_4: i32 = 11;
    const KEYCODE_5: i32 = 12;
    const KEYCODE_6: i32 = 13;
    const KEYCODE_7: i32 = 14;
    const KEYCODE_8: i32 = 15;
    const KEYCODE_9: i32 = 16;
    const KEYCODE_ESCAPE: i32 = 111;
    
    let key = match event.key_code {
        // 返回键 -> Escape (用于Intro界面返回菜单、结束Demo等)
        KEYCODE_BACK => PlatformKeyCode::Escape,
        KEYCODE_TAB => PlatformKeyCode::Tab,
        KEYCODE_SPACE => PlatformKeyCode::Space,
        KEYCODE_ENTER => PlatformKeyCode::Enter,
        KEYCODE_ESCAPE => PlatformKeyCode::Escape,
        // 反引号和分号/冒号映射到 Tab (方便输入作弊码)
        KEYCODE_GRAVE | KEYCODE_SEMICOLON => PlatformKeyCode::Tab,
        KEYCODE_A => PlatformKeyCode::KeyA,
        KEYCODE_B => PlatformKeyCode::KeyB,
        KEYCODE_C => PlatformKeyCode::KeyC,
        KEYCODE_D => PlatformKeyCode::KeyD,
        KEYCODE_E => PlatformKeyCode::KeyE,
        KEYCODE_F => PlatformKeyCode::KeyF,
        KEYCODE_G => PlatformKeyCode::KeyG,
        KEYCODE_H => PlatformKeyCode::KeyH,
        KEYCODE_I => PlatformKeyCode::KeyI,
        KEYCODE_J => PlatformKeyCode::KeyJ,
        KEYCODE_K => PlatformKeyCode::KeyK,
        KEYCODE_L => PlatformKeyCode::KeyL,
        KEYCODE_M => PlatformKeyCode::KeyM,
        KEYCODE_N => PlatformKeyCode::KeyN,
        KEYCODE_O => PlatformKeyCode::KeyO,
        KEYCODE_P => PlatformKeyCode::KeyP,
        KEYCODE_Q => PlatformKeyCode::KeyQ,
        KEYCODE_R => PlatformKeyCode::KeyR,
        KEYCODE_S => PlatformKeyCode::KeyS,
        KEYCODE_T => PlatformKeyCode::KeyT,
        KEYCODE_U => PlatformKeyCode::KeyU,
        KEYCODE_V => PlatformKeyCode::KeyV,
        KEYCODE_W => PlatformKeyCode::KeyW,
        KEYCODE_X => PlatformKeyCode::KeyX,
        KEYCODE_Y => PlatformKeyCode::KeyY,
        KEYCODE_Z => PlatformKeyCode::KeyZ,
        KEYCODE_0 => PlatformKeyCode::Digit0,
        KEYCODE_1 => PlatformKeyCode::Digit1,
        KEYCODE_2 => PlatformKeyCode::Digit2,
        KEYCODE_3 => PlatformKeyCode::Digit3,
        KEYCODE_4 => PlatformKeyCode::Digit4,
        KEYCODE_5 => PlatformKeyCode::Digit5,
        KEYCODE_6 => PlatformKeyCode::Digit6,
        KEYCODE_7 => PlatformKeyCode::Digit7,
        KEYCODE_8 => PlatformKeyCode::Digit8,
        KEYCODE_9 => PlatformKeyCode::Digit9,
        _ => PlatformKeyCode::Unknown,
    };
    PlatformKeyEvent {
        key,
        pressed: event.pressed,
    }
}

// ============================================================================
// Android Activity 相关导入
// ============================================================================

use android_activity::{
    AndroidApp, InputStatus, MainEvent, PollEvent,
    input::{InputEvent, KeyAction, Keycode, Source},
};
use ndk::native_window::NativeWindow;

// ============================================================================
// 手柄输入辅助函数
// ============================================================================

/// 检查 keycode 是否为手柄专用按键 (不包含 DPAD，因为 DPAD 可能来自遥控器)
fn is_gamepad_only_keycode(keycode: Keycode) -> bool {
    matches!(
        keycode,
        // 手柄主要按钮
        Keycode::ButtonA
            | Keycode::ButtonB
            | Keycode::ButtonC
            | Keycode::ButtonX
            | Keycode::ButtonY
            | Keycode::ButtonZ
            // 肩键和扳机
            | Keycode::ButtonL1
            | Keycode::ButtonR1
            | Keycode::ButtonL2
            | Keycode::ButtonR2
            // 功能键
            | Keycode::ButtonSelect
            | Keycode::ButtonStart
            | Keycode::ButtonMode
            // 摇杆按下
            | Keycode::ButtonThumbl
            | Keycode::ButtonThumbr
    )
}

/// 检查 keycode 是否为 DPAD 方向键
fn is_dpad_keycode(keycode: Keycode) -> bool {
    matches!(
        keycode,
        Keycode::DpadUp | Keycode::DpadDown | Keycode::DpadLeft | Keycode::DpadRight
    )
}

/// 检查 MotionEvent 的 source 是否为手柄/摇杆
fn is_gamepad_motion_source(source: Source) -> bool {
    // 直接比较 Source 枚举值
    source == Source::Gamepad || source == Source::Joystick
}

/// 处理手柄摇杆 MotionEvent
/// 
/// 目前只标记手柄已连接，轴值处理依赖按键事件 (DPAD 等)
/// 摇杆的模拟轴支持可以在后续版本中添加
fn handle_gamepad_motion(_motion_event: &android_activity::input::MotionEvent) {
    // 标记手柄已连接
    joystick_android::on_gamepad_connected();
    
    // TODO: 添加摇杆轴值读取
    // android-activity 0.6 的 Pointer API 需要进一步研究
    // 当前版本主要依赖手柄的 DPAD 按键和按钮事件
}

// ============================================================================
// 常量定义
// ============================================================================

use crate::game_runner::{GAME_HEIGHT, GAME_WIDTH};

// ============================================================================
// 类型别名 - 使用公共模块实现
// ============================================================================

pub type DesktopTime = CommonTime;
pub type DesktopRandom = CommonRandom;

// ============================================================================
// Android 输入后端
// ============================================================================

/// Android 输入后端 - 支持物理键盘和原生 Java 按钮
/// 触摸输入由 Java 原生按钮处理，通过 JNI 传递事件
pub struct AndroidInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
}

impl AndroidInput {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
        }
    }

    pub fn handle_key(&mut self, keycode: Keycode, action: KeyAction) {
        let platform_key = android_keycode_to_platform(keycode);
        let pressed = action == KeyAction::Down;

        if pressed {
            self.key_states.insert(platform_key);
        } else {
            self.key_states.remove(&platform_key);
        }

        self.pending_events.push(PlatformKeyEvent {
            key: platform_key,
            pressed,
        });
    }

}

impl Default for AndroidInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBackend for AndroidInput {
    fn poll_events(&mut self) -> Vec<PlatformKeyEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn is_key_pressed(&self, key: PlatformKeyCode) -> bool {
        self.key_states.contains(&key)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_close(&mut self) {
        self.should_close = true;
    }
}

pub type DesktopInput = AndroidInput;

/// Android 按键码转换为平台按键码
fn android_keycode_to_platform(keycode: Keycode) -> PlatformKeyCode {
    match keycode {
        Keycode::DpadLeft => PlatformKeyCode::Left,
        Keycode::DpadRight => PlatformKeyCode::Right,
        Keycode::DpadUp => PlatformKeyCode::Up,
        Keycode::DpadDown => PlatformKeyCode::Down,
        Keycode::Space => PlatformKeyCode::Space,
        Keycode::Enter => PlatformKeyCode::Enter,
        Keycode::Escape | Keycode::Back => PlatformKeyCode::Escape,
        Keycode::AltLeft => PlatformKeyCode::AltLeft,
        Keycode::AltRight => PlatformKeyCode::AltRight,
        Keycode::CtrlLeft => PlatformKeyCode::ControlLeft,
        Keycode::CtrlRight => PlatformKeyCode::ControlRight,
        Keycode::ShiftLeft => PlatformKeyCode::ShiftLeft,
        Keycode::ShiftRight => PlatformKeyCode::ShiftRight,
        Keycode::Tab => PlatformKeyCode::Tab,
        // 反引号(`)和分号(;/:)映射到Tab，方便在软键盘上输入作弊码 (软键盘没有Tab键)
        Keycode::Grave => PlatformKeyCode::Tab,
        Keycode::Semicolon => PlatformKeyCode::Tab,
        Keycode::A => PlatformKeyCode::KeyA,
        Keycode::B => PlatformKeyCode::KeyB,
        Keycode::C => PlatformKeyCode::KeyC,
        Keycode::D => PlatformKeyCode::KeyD,
        Keycode::E => PlatformKeyCode::KeyE,
        Keycode::F => PlatformKeyCode::KeyF,
        Keycode::G => PlatformKeyCode::KeyG,
        Keycode::H => PlatformKeyCode::KeyH,
        Keycode::I => PlatformKeyCode::KeyI,
        Keycode::J => PlatformKeyCode::KeyJ,
        Keycode::K => PlatformKeyCode::KeyK,
        Keycode::L => PlatformKeyCode::KeyL,
        Keycode::M => PlatformKeyCode::KeyM,
        Keycode::N => PlatformKeyCode::KeyN,
        Keycode::O => PlatformKeyCode::KeyO,
        Keycode::P => PlatformKeyCode::KeyP,
        Keycode::Q => PlatformKeyCode::KeyQ,
        Keycode::R => PlatformKeyCode::KeyR,
        Keycode::S => PlatformKeyCode::KeyS,
        Keycode::T => PlatformKeyCode::KeyT,
        Keycode::U => PlatformKeyCode::KeyU,
        Keycode::V => PlatformKeyCode::KeyV,
        Keycode::W => PlatformKeyCode::KeyW,
        Keycode::X => PlatformKeyCode::KeyX,
        Keycode::Y => PlatformKeyCode::KeyY,
        Keycode::Z => PlatformKeyCode::KeyZ,
        Keycode::Keycode0 => PlatformKeyCode::Digit0,
        Keycode::Keycode1 => PlatformKeyCode::Digit1,
        Keycode::Keycode2 => PlatformKeyCode::Digit2,
        Keycode::Keycode3 => PlatformKeyCode::Digit3,
        Keycode::Keycode4 => PlatformKeyCode::Digit4,
        Keycode::Keycode5 => PlatformKeyCode::Digit5,
        Keycode::Keycode6 => PlatformKeyCode::Digit6,
        Keycode::Keycode7 => PlatformKeyCode::Digit7,
        Keycode::Keycode8 => PlatformKeyCode::Digit8,
        Keycode::Keycode9 => PlatformKeyCode::Digit9,
        Keycode::F1 => PlatformKeyCode::F1,
        Keycode::F2 => PlatformKeyCode::F2,
        Keycode::F11 => PlatformKeyCode::F11,
        // TV遥控器 OK/Select 键 (DpadCenter) 映射为 Enter
        Keycode::DpadCenter => PlatformKeyCode::Enter,
        _ => PlatformKeyCode::Unknown,
    }
}

// ============================================================================
// 显示后端 - Android: wgpu surface + GpuRenderer
// ============================================================================

pub struct AndroidDisplay {
    width: u32,
    height: u32,
    native_window: Option<NativeWindow>,
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device: Option<std::sync::Arc<wgpu::Device>>,
    wgpu_queue: Option<std::sync::Arc<wgpu::Queue>>,
    wgpu_config: Option<wgpu::SurfaceConfiguration>,
    gpu_renderer: Option<GpuRenderer>,
}

impl AndroidDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            native_window: None,
            wgpu_surface: None,
            wgpu_device: None,
            wgpu_queue: None,
            wgpu_config: None,
            gpu_renderer: None,
        }
    }

    pub fn gpu_renderer(&self) -> Option<&GpuRenderer> {
        self.gpu_renderer.as_ref()
    }

    pub fn gpu_renderer_mut(&mut self) -> Option<&mut GpuRenderer> {
        self.gpu_renderer.as_mut()
    }

    /// 设置原生窗口并初始化 GPU 渲染
    /// 返回 true 表示 GPU 初始化成功，false 表示失败（需要 fallback 到 CPU）
    pub fn set_native_window(&mut self, window: Option<NativeWindow>) -> bool {
        self.native_window = window.clone();
        if window.is_none() {
            self.wgpu_surface = None;
            self.wgpu_device = None;
            self.wgpu_queue = None;
            self.wgpu_config = None;
            self.gpu_renderer = None;
            return true; // 清理成功
        }

        let window = window.expect("window is Some");
        let win_width = window.width().max(1) as u32;
        let win_height = window.height().max(1) as u32;

        // 优先使用 Vulkan 后端
        // GLES 后端在某些模拟器上与 GameActivity 存在兼容性问题
        // (eglCreateWindowSurface 失败: NativeWindow already connected to another API)
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        
        log_info("[GPU] Trying Vulkan backend first...");

        use std::ffi::c_void;
        use std::ptr::NonNull;
        use wgpu::rwh::{
            AndroidDisplayHandle, AndroidNdkWindowHandle, HasDisplayHandle, HasWindowHandle,
            RawDisplayHandle, RawWindowHandle,
        };

        // Android 窗口句柄包装器
        struct AndroidHandle {
            a_native_window: NonNull<c_void>,
        }

        // SAFETY: AndroidHandle 仅包含指向 Android native window 的指针
        // 该指针在 surface 生命周期内有效
        unsafe impl Send for AndroidHandle {}
        unsafe impl Sync for AndroidHandle {}

        impl HasWindowHandle for AndroidHandle {
            fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
                let handle = AndroidNdkWindowHandle::new(self.a_native_window);
                let raw = RawWindowHandle::AndroidNdk(handle);
                Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(raw) })
            }
        }
        impl HasDisplayHandle for AndroidHandle {
            fn display_handle(
                &self,
            ) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
                let raw = RawDisplayHandle::Android(AndroidDisplayHandle::new());
                Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(raw) })
            }
        }

        let a_native_window = match NonNull::new(window.ptr().as_ptr() as *mut c_void) {
            Some(ptr) => ptr,
            None => {
                log_error("[GPU] native_window_ptr_is_null");
                return false;
            }
        };

        let android_handle = AndroidHandle { a_native_window };

        let surface = match instance.create_surface(&android_handle) {
            Ok(s) => s,
            Err(e) => {
                log_error(&format!("[GPU] create_surface_failed {:?}", e));
                return false;
            }
        };

        let adapter = match futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, // 兼容老旧设备
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )) {
            Ok(a) => a,
            Err(e) => {
                log_warn(&format!("[GPU] Vulkan adapter not available: {:?}", e));
                log_warn("[GPU] This device may not support Vulkan, or this is an emulator.");
                log_warn("[GPU] Will fallback to CPU software rendering.");
                return false;
            }
        };

        // 输出后端类型日志
        let backend = adapter.get_info().backend;
        log_info(&format!("[GPU] Using backend: {:?}", backend));

        // 直接使用 adapter 的 limits（设备实际支持的值）
        log_info("[GPU] Requesting device...");
        let adapter_limits = adapter.limits();
        log_info(&format!("[GPU] Adapter max_texture_dimension_2d: {}", adapter_limits.max_texture_dimension_2d));

        // 尝试使用 adapter 的 limits 创建设备
        let device_result = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Mario Android Device"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter_limits.clone(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }));

        let (device, queue) = match device_result {
            Ok(v) => {
                log_info("[GPU] Device created successfully with adapter limits");
                v
            },
            Err(e) => {
                log_warn(&format!("[GPU] Failed with adapter limits: {:?}", e));
                // 回退：尝试使用 downlevel_webgl2_defaults
                log_info("[GPU] Trying downlevel_webgl2_defaults...");
                match futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("Mario Android Device Fallback"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    trace: wgpu::Trace::Off,
                })) {
                    Ok(v) => {
                        log_info("[GPU] Device created with downlevel defaults");
                        v
                    },
                    Err(e2) => {
                        log_error(&format!("[GPU] All attempts failed: {:?}", e2));
                        return false;
                    }
                }
            }
        };

        log_info("[GPU] Device created successfully");

        let device = std::sync::Arc::new(device);
        let queue: std::sync::Arc<wgpu::Queue> = std::sync::Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        log_info(&format!("[GPU] Surface formats: {:?}", caps.formats));
        log_info(&format!("[GPU] Alpha modes: {:?}", caps.alpha_modes));
        log_info(&format!("[GPU] Present modes: {:?}", caps.present_modes));

        // 安全获取 surface format
        let surface_format = match caps.formats.first() {
            Some(f) => *f,
            None => {
                log_error("[GPU] No surface formats available");
                return false;
            }
        };

        // 安全获取 alpha mode
        let alpha_mode = caps.alpha_modes.first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        // 选择兼容的 present mode
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes.first().copied().unwrap_or(wgpu::PresentMode::Fifo)
        };

        log_info(&format!("[GPU] Using format: {:?}, alpha: {:?}, present: {:?}", 
            surface_format, alpha_mode, present_mode));

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: win_width,
            height: win_height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        log_info("[GPU] Configuring surface...");
        
        // 配置 surface
        // 注意: 使用 Vulkan 后端可以避免 GLES 在某些模拟器上的兼容性问题
        // (GLES 后端可能因为 NativeWindow 已被 GameActivity 连接而失败)
        surface.configure(&device, &config);
        
        log_info("[GPU] Surface configured successfully");

        log_info("[GPU] Creating GpuRenderer...");
        let mut gpu_renderer = GpuRenderer::new(device.clone(), queue.clone(), config.format);
        gpu_renderer.update_scale(config.width, config.height);
        log_info("[GPU] GpuRenderer created successfully");

        self.wgpu_surface = Some(unsafe { std::mem::transmute(surface) });
        self.wgpu_device = Some(device);
        self.wgpu_queue = Some(queue);
        self.wgpu_config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);
        
        log_info("[GPU] GPU initialization completed successfully");
        true
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let (surface, device, config) =
            match (&self.wgpu_surface, &self.wgpu_device, &mut self.wgpu_config) {
                (Some(s), Some(d), Some(c)) => (s, d, c),
                _ => return,
            };
        config.width = new_width.max(1);
        config.height = new_height.max(1);
        surface.configure(device, config);
        if let Some(gpu) = &mut self.gpu_renderer {
            gpu.update_scale(config.width, config.height);
        }
    }

    /// 重新配置Surface（用于从后台恢复时）
    pub fn reconfigure_surface(&mut self) {
        if let (Some(surface), Some(device), Some(config)) =
            (&self.wgpu_surface, &self.wgpu_device, &self.wgpu_config)
        {
            surface.configure(device, config);
        }
    }
}

impl DisplayBackend for AndroidDisplay {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        let (surface, gpu_renderer, config) = match (
            &self.wgpu_surface,
            &mut self.gpu_renderer,
            &self.wgpu_config,
        ) {
            (Some(s), Some(g), Some(c)) => (s, g, c),
            _ => return Ok(()),
        };

        let output = match surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => return Err(e.to_string()),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        gpu_renderer.update_scale(config.width, config.height);
        gpu_renderer.render_to_surface(&view);

        output.present();
        Ok(())
    }

    fn request_redraw(&self) {
        // Android 使用连续渲染模式
    }
}

pub type DesktopDisplay = AndroidDisplay;

// ============================================================================
// 存储后端 - Android 扩展 FileStorage
// ============================================================================

/// 全局 Android 内部存储路径
/// 在 with_app() 首次调用时设置，之后 new() 会使用此路径
static ANDROID_STORAGE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Android 存储后端 - 使用内部存储路径
pub struct AndroidStorage {
    inner: FileStorage,
}

impl AndroidStorage {
    /// 创建存储后端
    /// 如果已通过 with_app() 设置了全局路径，则使用该路径
    /// 否则使用当前工作目录（可能无法写入）
    pub fn new() -> Self {
        // 尝试使用已设置的全局路径
        if let Ok(guard) = ANDROID_STORAGE_PATH.lock() {
            if let Some(ref path) = *guard {
                log_debug(&format!("[Storage] Using cached path: {:?}", path));
                return Self {
                    inner: FileStorage::with_base_path(path.clone()),
                };
            }
        }
        // 回退到默认路径（可能无法写入，但避免崩溃）
        log_warn("[Storage] No Android storage path set, using fallback");
        Self {
            inner: FileStorage::new(),
        }
    }

    /// 使用 AndroidApp 获取内部存储路径并缓存到全局变量
    /// 必须在应用启动时调用一次，确保后续 new() 能获取正确路径
    pub fn with_app(app: &AndroidApp) -> Self {
        let base_path = if let Some(path) = app.internal_data_path() {
            path.to_path_buf()
        } else {
            PathBuf::from(".")
        };
        
        // 缓存到全局变量，供后续 new() 使用
        if let Ok(mut guard) = ANDROID_STORAGE_PATH.lock() {
            log_info(&format!("[Storage] Setting Android storage path: {:?}", base_path));
            *guard = Some(base_path.clone());
        }
        
        Self {
            inner: FileStorage::with_base_path(base_path),
        }
    }
}

impl Default for AndroidStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for AndroidStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        self.inner.load(key)
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        self.inner.save(key, data)
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        self.inner.remove(key)
    }

    fn exists(&self, key: &str) -> bool {
        self.inner.exists(key)
    }
}

pub type DesktopStorage = AndroidStorage;

// ============================================================================
// 日志后端 - 使用 NDK 原生日志 API (Android 特有)
// ============================================================================

fn android_log_write(priority: i32, message: &str) {
    use std::ffi::CString;

    let tag = CString::new("MarioRS").unwrap_or_else(|_| CString::new("Mario").unwrap());
    let msg = CString::new(message.replace('\0', ""))
        .unwrap_or_else(|_| CString::new("(invalid message)").unwrap());

    unsafe {
        ndk_sys::__android_log_write(priority, tag.as_ptr(), msg.as_ptr());
    }
}

pub struct AndroidLog;

impl AndroidLog {
    pub fn new() -> Self {
        Self
    }

    pub fn init() {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("MarioRS"),
        );
        android_log_write(4, "AndroidLog initialized (native API)");
    }
}

impl Default for AndroidLog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBackend for AndroidLog {
    fn log(&self, level: LogLevel, message: &str) {
        let priority = match level {
            LogLevel::Debug => 3,
            LogLevel::Info => 4,
            LogLevel::Warn => 5,
            LogLevel::Error => 6,
        };
        android_log_write(priority, message);
    }
}

pub type DesktopLog = AndroidLog;

// ============================================================================
// 音频后端 - 使用 cpal
// ============================================================================

pub use super::audio::PlatformAudio as AndroidAudio;
pub type DesktopAudio = AndroidAudio;

// ============================================================================
// 全局便捷函数 - 使用公共模块 + Android 日志
// ============================================================================

pub use super::common::{now_ms, random_f32, random_i32, random_u8, random_u32, random_usize};

thread_local! {
    static LOG: AndroidLog = AndroidLog::new();
}

pub fn log_debug(msg: &str) {
    LOG.with(|l| l.debug(msg));
}

pub fn log_info(msg: &str) {
    LOG.with(|l| l.info(msg));
}

pub fn log_warn(msg: &str) {
    LOG.with(|l| l.warn(msg));
}

pub fn log_error(msg: &str) {
    LOG.with(|l| l.error(msg));
}

// ============================================================================
// Android 主入口
// ============================================================================

use crate::game_runner::GameState;
use crate::platform::FrameResult;

/// Android 应用主函数
pub fn android_main(app: AndroidApp) {
    AndroidLog::init();
    log_info("[GPU] Android GPU backend starting...");

    let mut display = AndroidDisplay::new(GAME_WIDTH, GAME_HEIGHT);
    let mut input = AndroidInput::new();
    let _storage = AndroidStorage::with_app(&app);
    let mut game_state: Option<GameState> = None;

    // FPS 计数器
    let mut fps_counter = FpsCounter::new();
    let mut running = true;

    // GPU 初始化失败标志 - 需要 fallback 到 CPU
    let mut need_cpu_fallback = false;
    
    // GPU资源是否需要重新上传（后台恢复后设为true）
    let mut gpu_resources_invalidated = false;
    // 上一次渲染时间
    let mut last_render_time = Instant::now();
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    while running && !need_cpu_fallback {
        // 计算到下一帧的等待时间，用于 poll_events 超时
        // 使用非常短的超时，确保输入事件能立即被处理
        let elapsed = last_render_time.elapsed();
        let wait_time = if elapsed >= frame_duration {
            Duration::ZERO
        } else {
            // 最多等待 1ms，优先保证输入响应
            (frame_duration - elapsed).min(Duration::from_millis(1))
        };

        app.poll_events(Some(wait_time), |event| match event {
            PollEvent::Main(main_event) => match main_event {
                MainEvent::InitWindow { .. } => {
                    if let Some(window) = app.native_window() {
                        log_info(&format!("[GPU] Window: {}x{}", window.width(), window.height()));
                        
                        // 尝试初始化 GPU 渲染
                        let gpu_ok = display.set_native_window(Some(window));
                        
                        if gpu_ok {
                            log_info("[GPU] GPU rendering initialized successfully");
                            if game_state.is_none() {
                                game_state = Some(GameState::new());
                            } else {
                                gpu_resources_invalidated = true;
                            }
                        } else {
                            log_warn("[GPU] GPU initialization failed, will fallback to CPU rendering");
                            need_cpu_fallback = true;
                        }
                    }
                }
                MainEvent::TerminateWindow { .. } => {
                    display.set_native_window(None);
                }
                MainEvent::WindowResized { .. } | MainEvent::ContentRectChanged { .. } => {
                    if let Some(window) = app.native_window() {
                        display.resize(window.width() as u32, window.height() as u32);
                    }
                }
                MainEvent::Destroy => {
                    // 应用销毁时保存游戏状态
                    if let Some(state) = &mut game_state {
                        log_info("[Android] Destroy event, saving game state...");
                        state.shutdown();
                    }
                    running = false;
                }
                MainEvent::Resume { .. } => {
                    if display.wgpu_surface.is_some() {
                        display.reconfigure_surface();
                        gpu_resources_invalidated = true;
                    } else if let Some(window) = app.native_window() {
                        let gpu_ok = display.set_native_window(Some(window));
                        if gpu_ok {
                            gpu_resources_invalidated = true;
                        } else {
                            log_warn("[GPU] GPU re-initialization failed on Resume, fallback to CPU");
                            need_cpu_fallback = true;
                        }
                    }
                }
                MainEvent::Pause => {
                    // 暂停时保存游戏状态（Android 可能在暂停后直接杀死应用）
                    if let Some(state) = &mut game_state {
                        log_info("[Android] Pause event, saving game state...");
                        state.shutdown();
                    }
                }
                _ => {}
            },
            PollEvent::Wake => {}
            PollEvent::Timeout => {}
            _ => {}
        });
        
        // 检查是否需要 CPU fallback
        if need_cpu_fallback {
            break;
        }

        // 处理 native 层输入事件
        // 注意: 手柄和遥控器输入已由 Java 层分离处理，通过专用 JNI 接口转发
        // 这里只处理可能绕过 Java dispatchKeyEvent 的底层事件 (如物理键盘)
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            let keycode = key_event.key_code();
                            let action = key_event.action();
                            let android_keycode = u32::from(keycode) as i32;
                            let pressed = action == KeyAction::Down;
                            
                            // 手柄专用按键: 转发到 joystick_android 模块
                            // (备用路径，主要路径是 Java GamepadController)
                            if is_gamepad_only_keycode(keycode) {
                                joystick_android::on_gamepad_connected();
                                joystick_android::on_gamepad_button(android_keycode, pressed);
                            } else if is_dpad_keycode(keycode) {
                                // DPAD 按键: 转发到键盘输入系统
                                // (备用路径，主要路径是 Java RemoteController/GamepadController)
                                input.handle_key(keycode, action);
                            } else {
                                // 其他按键: 使用键盘输入逻辑
                                input.handle_key(keycode, action);
                            }
                        }
                        InputEvent::MotionEvent(motion_event) => {
                            // 手柄摇杆事件 (备用路径，主要路径是 Java GamepadController)
                            if is_gamepad_motion_source(motion_event.source()) {
                                handle_gamepad_motion(&motion_event);
                            }
                        }
                        _ => {}
                    }
                    InputStatus::Handled
                });
                if !read_event {
                    break;
                }
            }
        }

        // 处理原生按钮事件 (来自 Java UI 线程)
        if let Some(state) = &mut game_state {
            for native_event in take_native_button_events() {
                let key_event = native_button_to_key_event(&native_event);
                if key_event.key != PlatformKeyCode::Unknown {
                    state.handle_key_event(&key_event);
                }
            }
        }
        
        // 处理软键盘/遥控器事件队列
        // 注意: 遥控器状态已由 nativeOnRemoteKey 直接更新到 joystick_android_tv 模块
        // 这里只负责将事件转换为平台按键事件，用于游戏菜单导航等
        if let Some(state) = &mut game_state {
            for soft_event in take_soft_key_events() {
                // 转换为平台按键事件并处理
                let key_event = soft_key_to_platform_event(&soft_event);
                if key_event.key != PlatformKeyCode::Unknown {
                    state.handle_key_event(&key_event);
                }
            }
        }
        
        // 处理物理键盘事件
        if let Some(state) = &mut game_state {
            for event in input.poll_events() {
                state.handle_key_event(&event);
            }
        }

        // 帧率限制 - 不使用 thread::sleep，而是依赖 poll_events 的超时
        // 这样在等待期间也能响应输入事件
        let now = Instant::now();
        if now.duration_since(last_render_time) < frame_duration {
            // 时间未到，继续循环处理事件
            continue;
        }
        last_render_time = now;

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            if gpu_resources_invalidated {
                state.invalidate_gpu_resources();
                gpu_resources_invalidated = false;
            }

            let frame_start = Instant::now();

            // FPS和渲染模式显示
            state.set_fps_display(fps_counter.fps(), fps_counter.frame_time_ms());
            state.set_render_mode(RenderMode::GPU);

            let result = state.frame_update();

            // 获取 surface 配置信息
            let surface_config = display.wgpu_config.as_ref().map(|c| (c.width, c.height));

            if let Some((config_width, config_height)) = surface_config {
                if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                    // 准备渲染数据
                    state.submit_to_gpu(gpu_renderer);
                }

                // 获取 surface 纹理并渲染
                if let Some(surface) = &display.wgpu_surface {
                    match surface.get_current_texture() {
                        Ok(output) => {
                            // 使用实际纹理尺寸而非配置尺寸
                            // Android 上 NativeWindow 尺寸可能包含系统UI区域
                            // 但实际 surface 纹理尺寸才是正确的渲染目标尺寸
                            let actual_width = output.texture.width();
                            let actual_height = output.texture.height();
                            
                            // 一次性日志: 检查配置尺寸与实际纹理尺寸是否不同
                            static SIZE_LOG_ONCE: std::sync::Once = std::sync::Once::new();
                            SIZE_LOG_ONCE.call_once(|| {
                                if config_width != actual_width || config_height != actual_height {
                                    log_info(&format!(
                                        "[GPU] Size mismatch! Config: {}x{}, Actual texture: {}x{}",
                                        config_width, config_height, actual_width, actual_height
                                    ));
                                } else {
                                    log_info(&format!(
                                        "[GPU] Surface size: {}x{} (config matches texture)",
                                        actual_width, actual_height
                                    ));
                                }
                            });
                            
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                                // 使用实际纹理尺寸更新缩放参数
                                gpu_renderer.update_scale(actual_width, actual_height);

                                // 一次性完成渲染和呈现（单次GPU提交）
                                gpu_renderer.render_frame_and_present(&view);
                            }

                            output.present();
                        }
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            display.reconfigure_surface();
                        }
                        Err(_) => {}
                    }
                }
            }

            // 使用公共模块的 FPS 计数器
            let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            fps_counter.record_frame(frame_time_ms);

            if result == FrameResult::Exit {
                state.shutdown();
                running = false;
            }
        }
    }
    
    // GPU 初始化失败，fallback 到 CPU 软件渲染
    if need_cpu_fallback {
        log_info("[GPU->CPU] Switching to CPU software rendering...");
        run_cpu_fallback(app);
    }
}

/// 游戏运行入口
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}

// ============================================================================
// CPU 软件渲染 Fallback
// 当 GPU 初始化失败时使用
// ============================================================================

use crate::cpu::CpuRenderer;

/// CPU 渲染显示后端
struct CpuDisplay {
    native_window: Option<NativeWindow>,
    cpu_renderer: Option<CpuRenderer>,
}

impl CpuDisplay {
    fn new() -> Self {
        Self {
            native_window: None,
            cpu_renderer: None,
        }
    }

    fn cpu_renderer(&self) -> Option<&CpuRenderer> {
        self.cpu_renderer.as_ref()
    }

    fn cpu_renderer_mut(&mut self) -> Option<&mut CpuRenderer> {
        self.cpu_renderer.as_mut()
    }

    fn set_native_window(&mut self, window: Option<NativeWindow>) {
        self.native_window = window.clone();
        if window.is_none() {
            self.cpu_renderer = None;
            return;
        }
        
        // 设置窗口缓冲区尺寸为游戏分辨率的整数倍
        if let Some(ref win) = self.native_window {
            unsafe {
                let win_ptr = win.ptr().as_ptr();
                let native_w = win.width() as i32;
                let native_h = win.height() as i32;
                
                // 计算最佳整数缩放倍数
                let scale_x = native_w / GAME_WIDTH as i32;
                let scale_y = native_h / GAME_HEIGHT as i32;
                let scale = scale_x.min(scale_y).max(1);
                
                let buf_w = GAME_WIDTH as i32 * scale;
                let buf_h = GAME_HEIGHT as i32 * scale;
                
                log_info(&format!("[CPU] Native: {}x{}, Buffer: {}x{}, Scale: {}x", 
                    native_w, native_h, buf_w, buf_h, scale));
                
                // WINDOW_FORMAT_RGBA_8888 = 1
                let result = ndk_sys::ANativeWindow_setBuffersGeometry(
                    win_ptr, buf_w, buf_h, 1,
                );
                if result != 0 {
                    log_warn(&format!("[CPU] setBuffersGeometry failed: {}", result));
                } else {
                    log_info(&format!("[CPU] Window buffer set to {}x{} RGBA_8888", buf_w, buf_h));
                }
            }
        }
        
        // 创建 CPU 渲染器
        self.cpu_renderer = Some(CpuRenderer::new(GAME_WIDTH, GAME_HEIGHT));
        log_info("[CPU] CpuRenderer created");
    }

    /// 将帧缓冲写入到 ANativeWindow
    fn present_framebuffer(&self) -> Result<(), String> {
        let window = match &self.native_window {
            Some(w) => w,
            None => return Ok(()),
        };
        let cpu = match &self.cpu_renderer {
            Some(c) => c,
            None => return Ok(()),
        };

        let framebuffer = cpu.framebuffer();
        let fb_width = cpu.width() as i32;
        let fb_height = cpu.height() as i32;

        unsafe {
            let win_ptr = window.ptr().as_ptr();
            let mut buffer: ndk_sys::ANativeWindow_Buffer = std::mem::zeroed();
            let result = ndk_sys::ANativeWindow_lock(win_ptr, &mut buffer, std::ptr::null_mut());
            if result != 0 {
                return Err(format!("ANativeWindow_lock failed: {}", result));
            }

            let win_width = buffer.width;
            let win_height = buffer.height;
            let stride = buffer.stride;
            let bits = buffer.bits as *mut u32;
            
            if bits.is_null() || win_width <= 0 || win_height <= 0 || stride <= 0 {
                ndk_sys::ANativeWindow_unlockAndPost(win_ptr);
                return Err("Invalid buffer".to_string());
            }

            // 计算整数缩放倍数
            let scale = (win_width / fb_width).min(win_height / fb_height).max(1);
            let dst_width = fb_width * scale;
            let dst_height = fb_height * scale;
            let offset_x = (win_width - dst_width) / 2;
            let offset_y = (win_height - dst_height) / 2;

            let black = 0xFF000000u32;
            
            // 上边框
            for y in 0..offset_y {
                let row = bits.offset((y * stride) as isize);
                for x in 0..win_width {
                    *row.offset(x as isize) = black;
                }
            }
            // 下边框
            for y in (offset_y + dst_height)..win_height {
                let row = bits.offset((y * stride) as isize);
                for x in 0..win_width {
                    *row.offset(x as isize) = black;
                }
            }
            // 左右边框
            for y in offset_y..(offset_y + dst_height) {
                let row = bits.offset((y * stride) as isize);
                for x in 0..offset_x {
                    *row.offset(x as isize) = black;
                }
                for x in (offset_x + dst_width)..win_width {
                    *row.offset(x as isize) = black;
                }
            }

            // 整数缩放渲染
            // framebuffer 是 &[u8]，每像素4字节 (ABGR 格式用于 Android RGBA_8888)
            let bytes_per_pixel = 4;
            for src_y in 0..fb_height {
                let row_start = (src_y * fb_width * bytes_per_pixel) as usize;
                for dy in 0..scale {
                    let dst_y = offset_y + src_y * scale + dy;
                    let dst_row = bits.offset((dst_y * stride + offset_x) as isize);
                    
                    for src_x in 0..fb_width {
                        // 读取 4 字节像素 (ABGR -> u32)
                        let px_offset = row_start + (src_x as usize) * 4;
                        let pixel = u32::from_le_bytes([
                            framebuffer[px_offset],
                            framebuffer[px_offset + 1],
                            framebuffer[px_offset + 2],
                            framebuffer[px_offset + 3],
                        ]);
                        for dx in 0..scale {
                            *dst_row.offset((src_x * scale + dx) as isize) = pixel;
                        }
                    }
                }
            }

            ndk_sys::ANativeWindow_unlockAndPost(win_ptr);
        }

        Ok(())
    }
}

/// CPU fallback 主循环
/// GPU 初始化失败时由 android_main 调用
fn run_cpu_fallback(app: AndroidApp) {
    log_info("[CPU] Starting CPU fallback rendering...");

    let mut display = CpuDisplay::new();
    let mut input = AndroidInput::new();
    let mut game_state: Option<GameState> = None;

    let mut fps_counter = FpsCounter::new();
    let mut running = true;
    let mut last_render_time = Instant::now();
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    // 检查是否已经有窗口（InitWindow 事件可能在 GPU 初始化期间已触发）
    if let Some(window) = app.native_window() {
        log_info(&format!("[CPU] Window already available: {}x{}", window.width(), window.height()));
        display.set_native_window(Some(window));
    }

    log_info("[CPU] Entering CPU main loop...");

    while running {
        let elapsed = last_render_time.elapsed();
        let wait_time = if elapsed >= frame_duration {
            Duration::ZERO
        } else {
            (frame_duration - elapsed).min(Duration::from_millis(1))
        };

        app.poll_events(Some(wait_time), |event| match event {
            PollEvent::Main(main_event) => match main_event {
                MainEvent::InitWindow { .. } => {
                    log_info("[CPU] InitWindow event received");
                    if let Some(window) = app.native_window() {
                        log_info(&format!("[CPU] Window: {}x{}", window.width(), window.height()));
                        display.set_native_window(Some(window));
                    }
                }
                MainEvent::TerminateWindow { .. } => {
                    display.set_native_window(None);
                }
                MainEvent::Destroy => {
                    // 应用销毁时保存游戏状态
                    if let Some(state) = &mut game_state {
                        log_info("[CPU] Destroy event, saving game state...");
                        state.shutdown();
                    }
                    running = false;
                }
                MainEvent::Resume { .. } => {
                    if let Some(window) = app.native_window() {
                        display.set_native_window(Some(window));
                    }
                }
                MainEvent::Pause => {
                    // 暂停时保存游戏状态（Android 可能在暂停后直接杀死应用）
                    if let Some(state) = &mut game_state {
                        log_info("[CPU] Pause event, saving game state...");
                        state.shutdown();
                    }
                }
                _ => {}
            },
            _ => {}
        });

        // 处理输入事件 (复用现有的输入处理逻辑)
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            let keycode = key_event.key_code();
                            let action = key_event.action();
                            let android_keycode = u32::from(keycode) as i32;
                            let pressed = action == KeyAction::Down;
                            
                            if is_gamepad_only_keycode(keycode) {
                                joystick_android::on_gamepad_connected();
                                joystick_android::on_gamepad_button(android_keycode, pressed);
                            } else {
                                input.handle_key(keycode, action);
                            }
                        }
                        InputEvent::MotionEvent(motion_event) => {
                            if is_gamepad_motion_source(motion_event.source()) {
                                joystick_android::on_gamepad_connected();
                            }
                        }
                        _ => {}
                    }
                    InputStatus::Handled
                });
                if !read_event { break; }
            }
        }

        // 处理原生按钮事件
        if let Some(state) = &mut game_state {
            for native_event in take_native_button_events() {
                let key_event = native_button_to_key_event(&native_event);
                if key_event.key != PlatformKeyCode::Unknown {
                    state.handle_key_event(&key_event);
                }
            }
        }

        // 处理软键盘/遥控器事件
        if let Some(state) = &mut game_state {
            for soft_event in take_soft_key_events() {
                let key_event = soft_key_to_platform_event(&soft_event);
                if key_event.key != PlatformKeyCode::Unknown {
                    state.handle_key_event(&key_event);
                }
            }
        }

        // 处理物理键盘事件
        if let Some(state) = &mut game_state {
            for event in input.poll_events() {
                state.handle_key_event(&event);
            }
        }

        // 帧率限制
        let now = Instant::now();
        if now.duration_since(last_render_time) < frame_duration {
            continue;
        }
        last_render_time = now;

        // 延迟创建游戏状态
        if game_state.is_none() && display.cpu_renderer().is_some() {
            log_info("[CPU] Creating GameState...");
            game_state = Some(GameState::new());
            log_info("[CPU] GameState created successfully");
        }

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            let frame_start = Instant::now();
            // FPS和渲染模式显示
            state.set_fps_display(fps_counter.fps(), fps_counter.frame_time_ms());
            state.set_render_mode(RenderMode::CPU);
            
            let result = state.frame_update();

            // CPU 渲染
            if let Some(cpu_renderer) = display.cpu_renderer_mut() {
                state.submit_to_cpu(cpu_renderer);
            }

            // 显示帧缓冲
            if let Err(e) = display.present_framebuffer() {
                log_warn(&format!("[CPU] Present failed: {}", e));
            }

            let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            fps_counter.record_frame(frame_time_ms);

            if result == FrameResult::Exit {
                state.shutdown();
                running = false;
            }
        }
    }
    
    log_info("[CPU] CPU fallback loop ended");
}
