// Android CPU软件渲染后端
//
// 用于不支持Vulkan/OpenGL ES 3.1的老旧Android设备
// 使用 ANativeWindow_lock/unlock 直接写入像素数据
//
// 架构:
// 游戏逻辑 -> RenderCommand -> CpuRenderer -> RGBA帧缓冲 -> ANativeWindow
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
use crate::cpu::CpuRenderer;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ============================================================================
// JNI 原生按钮事件队列 (复用自 android.rs)
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

/// 全局事件队列
static NATIVE_BUTTON_EVENTS: Mutex<Vec<NativeButtonEvent>> = Mutex::new(Vec::new());

/// 软键盘按键事件
#[derive(Clone, Copy, Debug)]
pub struct SoftKeyEvent {
    pub key_code: i32,
    pub pressed: bool,
}

/// 软键盘事件队列
static SOFT_KEY_EVENTS: Mutex<Vec<SoftKeyEvent>> = Mutex::new(Vec::new());

/// JNI 导出函数 - 游戏按钮事件
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
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnGamepadButton(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    log_debug(&format!("[JNI Gamepad] button key_code={}, pressed={}", key_code, pressed != 0));
    joystick_android::on_gamepad_button(key_code, pressed != 0);
    joystick_android::on_gamepad_connected();
}

/// JNI 导出函数 - 手柄摇杆轴事件
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnGamepadAxis(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    axis_id: i32,
    value: f32,
) {
    joystick_android::on_gamepad_axis(axis_id, value);
}

// ============================================================================
// 遥控器专用 JNI 接口 (与手柄完全分离)
// Java RemoteController -> Rust joystick_android_tv 模块
// ============================================================================

/// JNI 导出函数 - 遥控器按键事件
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnRemoteKey(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    log_debug(&format!("[JNI Remote] key_code={}, pressed={}", key_code, pressed != 0));
    joystick_android_tv::on_tv_remote_key(key_code, pressed != 0);
    
    // 同时加入软键盘事件队列
    if let Ok(mut queue) = SOFT_KEY_EVENTS.lock() {
        queue.push(SoftKeyEvent { key_code, pressed: pressed != 0 });
    }
}

// ============================================================================
// 兼容性: 保留旧的 nativeOnKeyEvent
// ============================================================================

/// JNI 导出函数 - 软键盘按键事件 (已弃用，保留兼容性)
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnKeyEvent(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
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
        native_button::BTN_A => PlatformKeyCode::AltLeft,
        native_button::BTN_B => PlatformKeyCode::ControlLeft,
        native_button::BTN_X => PlatformKeyCode::Space,
        native_button::BTN_Y => PlatformKeyCode::ShiftLeft,
        _ => PlatformKeyCode::Unknown,
    };
    PlatformKeyEvent { key, pressed: event.pressed }
}

/// 将软键盘事件转换为平台按键事件
pub fn soft_key_to_platform_event(event: &SoftKeyEvent) -> PlatformKeyEvent {
    // Android KeyEvent 常量
    const KEYCODE_BACK: i32 = 4;
    const KEYCODE_TAB: i32 = 61;
    const KEYCODE_SPACE: i32 = 62;
    const KEYCODE_ENTER: i32 = 66;
    const KEYCODE_GRAVE: i32 = 68;
    const KEYCODE_SEMICOLON: i32 = 74;
    const KEYCODE_A: i32 = 29;
    const KEYCODE_Z: i32 = 54;
    const KEYCODE_0: i32 = 7;
    const KEYCODE_9: i32 = 16;
    const KEYCODE_ESCAPE: i32 = 111;
    
    let key = match event.key_code {
        KEYCODE_BACK | KEYCODE_ESCAPE => PlatformKeyCode::Escape,
        KEYCODE_TAB | KEYCODE_GRAVE | KEYCODE_SEMICOLON => PlatformKeyCode::Tab,
        KEYCODE_SPACE => PlatformKeyCode::Space,
        KEYCODE_ENTER => PlatformKeyCode::Enter,
        29 => PlatformKeyCode::KeyA,
        30 => PlatformKeyCode::KeyB,
        31 => PlatformKeyCode::KeyC,
        32 => PlatformKeyCode::KeyD,
        33 => PlatformKeyCode::KeyE,
        34 => PlatformKeyCode::KeyF,
        35 => PlatformKeyCode::KeyG,
        36 => PlatformKeyCode::KeyH,
        37 => PlatformKeyCode::KeyI,
        38 => PlatformKeyCode::KeyJ,
        39 => PlatformKeyCode::KeyK,
        40 => PlatformKeyCode::KeyL,
        41 => PlatformKeyCode::KeyM,
        42 => PlatformKeyCode::KeyN,
        43 => PlatformKeyCode::KeyO,
        44 => PlatformKeyCode::KeyP,
        45 => PlatformKeyCode::KeyQ,
        46 => PlatformKeyCode::KeyR,
        47 => PlatformKeyCode::KeyS,
        48 => PlatformKeyCode::KeyT,
        49 => PlatformKeyCode::KeyU,
        50 => PlatformKeyCode::KeyV,
        51 => PlatformKeyCode::KeyW,
        52 => PlatformKeyCode::KeyX,
        53 => PlatformKeyCode::KeyY,
        54 => PlatformKeyCode::KeyZ,
        7 => PlatformKeyCode::Digit0,
        8 => PlatformKeyCode::Digit1,
        9 => PlatformKeyCode::Digit2,
        10 => PlatformKeyCode::Digit3,
        11 => PlatformKeyCode::Digit4,
        12 => PlatformKeyCode::Digit5,
        13 => PlatformKeyCode::Digit6,
        14 => PlatformKeyCode::Digit7,
        15 => PlatformKeyCode::Digit8,
        16 => PlatformKeyCode::Digit9,
        _ => PlatformKeyCode::Unknown,
    };
    PlatformKeyEvent { key, pressed: event.pressed }
}

// ============================================================================
// Android Activity 相关导入
// ============================================================================

use android_activity::{
    AndroidApp, InputStatus, MainEvent, PollEvent,
    input::{InputEvent, KeyAction, Keycode, MotionAction},
};
use ndk::native_window::NativeWindow;

use crate::game_runner::{GAME_HEIGHT, GAME_WIDTH};

pub type DesktopTime = CommonTime;
pub type DesktopRandom = CommonRandom;

// ============================================================================
// Android 输入后端
// ============================================================================

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
        self.pending_events.push(PlatformKeyEvent { key: platform_key, pressed });
    }
}

impl Default for AndroidInput {
    fn default() -> Self { Self::new() }
}

impl InputBackend for AndroidInput {
    fn poll_events(&mut self) -> Vec<PlatformKeyEvent> {
        std::mem::take(&mut self.pending_events)
    }
    fn is_key_pressed(&self, key: PlatformKeyCode) -> bool {
        self.key_states.contains(&key)
    }
    fn should_close(&self) -> bool { self.should_close }
    fn request_close(&mut self) { self.should_close = true; }
}

pub type DesktopInput = AndroidInput;

/// Android 按键码转换
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
        Keycode::Tab | Keycode::Grave | Keycode::Semicolon => PlatformKeyCode::Tab,
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
        Keycode::DpadCenter => PlatformKeyCode::Enter,
        _ => PlatformKeyCode::Unknown,
    }
}

// ============================================================================
// 显示后端 - Android CPU软件渲染
// ============================================================================

pub struct AndroidDisplay {
    width: u32,
    height: u32,
    native_window: Option<NativeWindow>,
    cpu_renderer: Option<CpuRenderer>,
}

impl AndroidDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            native_window: None,
            cpu_renderer: None,
        }
    }

    pub fn cpu_renderer(&self) -> Option<&CpuRenderer> {
        self.cpu_renderer.as_ref()
    }

    pub fn cpu_renderer_mut(&mut self) -> Option<&mut CpuRenderer> {
        self.cpu_renderer.as_mut()
    }

    pub fn set_native_window(&mut self, window: Option<NativeWindow>) {
        self.native_window = window.clone();
        if window.is_none() {
            self.cpu_renderer = None;
            return;
        }
        
        // 设置窗口缓冲区尺寸为游戏分辨率的整数倍
        // 让Android SurfaceFlinger硬件合成器负责最终缩放，比CPU快得多
        if let Some(ref win) = self.native_window {
            unsafe {
                let win_ptr = win.ptr().as_ptr();
                let native_w = win.width() as i32;
                let native_h = win.height() as i32;
                
                // 计算最佳整数缩放倍数（不超过屏幕尺寸）
                let scale_x = native_w / GAME_WIDTH as i32;
                let scale_y = native_h / GAME_HEIGHT as i32;
                let scale = scale_x.min(scale_y).max(1);
                
                let buf_w = GAME_WIDTH as i32 * scale;
                let buf_h = GAME_HEIGHT as i32 * scale;
                
                log_info(&format!("[CPU] Native: {}x{}, Buffer: {}x{}, Scale: {}x", 
                    native_w, native_h, buf_w, buf_h, scale));
                
                // WINDOW_FORMAT_RGBA_8888 = 1
                let result = ndk_sys::ANativeWindow_setBuffersGeometry(
                    win_ptr,
                    buf_w,
                    buf_h,
                    1, // WINDOW_FORMAT_RGBA_8888
                );
                if result != 0 {
                    log_warn(&format!("[CPU] setBuffersGeometry failed: {}", result));
                } else {
                    log_info(&format!("[CPU] Window buffer set to {}x{} RGBA_8888", buf_w, buf_h));
                }
            }
        }
        
        // 创建CPU渲染器
        self.cpu_renderer = Some(CpuRenderer::new(GAME_WIDTH, GAME_HEIGHT));
        log_info("[CPU] CpuRenderer created");
    }

    pub fn resize(&mut self, _new_width: u32, _new_height: u32) {
        // CPU渲染器使用固定的游戏分辨率，不需要调整
    }

    /// 将帧缓冲写入到 ANativeWindow（优化版本）
    /// 
    /// 优化策略:
    /// 1. 使用整数缩放避免浮点除法
    /// 2. 预计算源坐标查找表
    /// 3. 只清除边框区域而非全屏
    /// 4. 批量像素处理
    pub fn present_framebuffer(&mut self) -> Result<(), String> {
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

            // 计算整数缩放倍数（由于setBuffersGeometry已设置为整数倍，这里应该是精确的）
            let scale = (win_width / fb_width).min(win_height / fb_height).max(1);
            let dst_width = fb_width * scale;
            let dst_height = fb_height * scale;
            let offset_x = (win_width - dst_width) / 2;
            let offset_y = (win_height - dst_height) / 2;

            // 优化: 只清除边框区域（而非全屏清除）
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
            // 左右边框（只处理游戏区域的行）
            for y in offset_y..(offset_y + dst_height) {
                let row = bits.offset((y * stride) as isize);
                // 左边框
                for x in 0..offset_x {
                    *row.offset(x as isize) = black;
                }
                // 右边框
                for x in (offset_x + dst_width)..win_width {
                    *row.offset(x as isize) = black;
                }
            }

            // 整数倍缩放复制（无浮点运算，无颜色转换）
            // CpuRenderer 已直接输出 Android RGBA_8888 格式（ABGR 小端序）
            let fb_ptr = framebuffer.as_ptr() as *const u32;
            
            if scale == 1 {
                // 1:1 复制（最快路径 - 直接内存复制）
                for sy in 0..fb_height {
                    let src_row = fb_ptr.offset((sy * fb_width) as isize);
                    let dst_row = bits.offset(((offset_y + sy) * stride + offset_x) as isize);
                    std::ptr::copy_nonoverlapping(src_row, dst_row, fb_width as usize);
                }
            } else {
                // 整数倍放大（每个源像素复制 scale x scale 次）
                for sy in 0..fb_height {
                    let src_row = fb_ptr.offset((sy * fb_width) as isize);
                    
                    // 先渲染第一行
                    let first_dy = offset_y + sy * scale;
                    let first_row = bits.offset((first_dy * stride + offset_x) as isize);
                    
                    for sx in 0..fb_width {
                        let pixel = *src_row.offset(sx as isize);
                        
                        // 水平方向复制 scale 次
                        let dx_base = sx * scale;
                        for s in 0..scale {
                            *first_row.offset((dx_base + s) as isize) = pixel;
                        }
                    }
                    
                    // 垂直方向复制其余行
                    for dy_offset in 1..scale {
                        let dy = first_dy + dy_offset;
                        let dst_row = bits.offset((dy * stride + offset_x) as isize);
                        std::ptr::copy_nonoverlapping(first_row, dst_row, dst_width as usize);
                    }
                }
            }

            ndk_sys::ANativeWindow_unlockAndPost(win_ptr);
        }
        Ok(())
    }
}

impl DisplayBackend for AndroidDisplay {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }

    fn present(&mut self) -> Result<(), String> {
        self.present_framebuffer()
    }

    fn request_redraw(&self) {
        // Android 使用连续渲染模式
    }
}

pub type DesktopDisplay = AndroidDisplay;

// ============================================================================
// 存储后端
// ============================================================================

pub struct AndroidStorage {
    inner: FileStorage,
}

impl AndroidStorage {
    pub fn new() -> Self {
        Self { inner: FileStorage::new() }
    }

    pub fn with_app(app: &AndroidApp) -> Self {
        let base_path = if let Some(path) = app.internal_data_path() {
            path.to_path_buf()
        } else {
            PathBuf::from(".")
        };
        Self { inner: FileStorage::with_base_path(base_path) }
    }
}

impl Default for AndroidStorage {
    fn default() -> Self { Self::new() }
}

impl StorageBackend for AndroidStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> { self.inner.load(key) }
    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> { self.inner.save(key, data) }
    fn remove(&mut self, key: &str) -> Result<(), String> { self.inner.remove(key) }
    fn exists(&self, key: &str) -> bool { self.inner.exists(key) }
}

pub type DesktopStorage = AndroidStorage;

// ============================================================================
// 日志后端
// ============================================================================

fn android_log_write(priority: i32, message: &str) {
    use std::ffi::CString;
    let tag = CString::new("MarioRS-CPU").unwrap_or_else(|_| CString::new("Mario").unwrap());
    let msg = CString::new(message.replace('\0', ""))
        .unwrap_or_else(|_| CString::new("(invalid message)").unwrap());
    unsafe {
        ndk_sys::__android_log_write(priority, tag.as_ptr(), msg.as_ptr());
    }
}

pub struct AndroidLog;

impl AndroidLog {
    pub fn new() -> Self { Self }
    pub fn init() {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("MarioRS-CPU"),
        );
        android_log_write(4, "AndroidLog initialized (CPU mode)");
    }
}

impl Default for AndroidLog {
    fn default() -> Self { Self::new() }
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
// 音频后端
// ============================================================================

pub use super::audio::PlatformAudio as AndroidAudio;
pub type DesktopAudio = AndroidAudio;

// ============================================================================
// 全局便捷函数
// ============================================================================

pub use super::common::{now_ms, random_f32, random_i32, random_u8, random_u32, random_usize};

thread_local! {
    static LOG: AndroidLog = AndroidLog::new();
}

pub fn log_debug(msg: &str) { LOG.with(|l| l.debug(msg)); }
pub fn log_info(msg: &str) { LOG.with(|l| l.info(msg)); }
pub fn log_warn(msg: &str) { LOG.with(|l| l.warn(msg)); }
pub fn log_error(msg: &str) { LOG.with(|l| l.error(msg)); }

// ============================================================================
// Android 主入口
// ============================================================================

use crate::game_runner::GameState;
use crate::platform::FrameResult;

/// Android 应用主函数 (CPU版本)
pub fn android_main(app: AndroidApp) {
    AndroidLog::init();
    log_info("[CPU] Android CPU backend starting...");

    let mut display = AndroidDisplay::new(GAME_WIDTH, GAME_HEIGHT);
    log_info("[CPU] Display created");
    let mut input = AndroidInput::new();
    log_info("[CPU] Input created");
    let _storage = AndroidStorage::with_app(&app);
    log_info("[CPU] Storage created");
    let mut game_state: Option<GameState> = None;

    let mut fps_counter = FpsCounter::new();
    let mut running = true;
    let mut last_render_time = Instant::now();
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut frame_count: u64 = 0;

    log_info("[CPU] Entering main loop...");

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
                        let w = window.width();
                        let h = window.height();
                        log_info(&format!("[CPU] Window: {}x{}", w, h));
                        display.set_native_window(Some(window));
                        // 延迟创建游戏状态到主循环中
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
                MainEvent::Destroy => { running = false; }
                MainEvent::Resume { .. } => {
                    if let Some(window) = app.native_window() {
                        display.set_native_window(Some(window));
                    }
                }
                MainEvent::Pause => {}
                _ => {}
            },
            PollEvent::Wake | PollEvent::Timeout => {}
            _ => {}
        });

        // 处理输入事件
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            input.handle_key(key_event.key_code(), key_event.action());
                        }
                        InputEvent::MotionEvent(_) => {}
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

        // 处理软键盘/遥控器事件队列
        // 注意: 遥控器状态已由 nativeOnRemoteKey 直接更新到 joystick_android_tv 模块
        // 这里只负责将事件转换为平台按键事件，用于游戏菜单导航等
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

        // 延迟创建游戏状态（在主循环中创建，避免在事件回调中创建）
        if game_state.is_none() && display.cpu_renderer().is_some() {
            log_info("[CPU] Creating GameState in main loop...");
            game_state = Some(GameState::new());
            log_info("[CPU] GameState created successfully");
        }

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            frame_count += 1;
            if frame_count <= 3 {
                log_info(&format!("[CPU] Frame {} starting...", frame_count));
            }
            
            let frame_start = Instant::now();
            state.set_fps_display(fps_counter.fps(), fps_counter.frame_time_ms());
            
            // 帧更新
            if frame_count <= 3 {
                log_info(&format!("[CPU] Frame {} calling frame_update...", frame_count));
            }
            let result = state.frame_update();
            if frame_count <= 3 {
                log_info(&format!("[CPU] Frame {} frame_update done", frame_count));
            }

            // CPU渲染
            if let Some(cpu_renderer) = display.cpu_renderer_mut() {
                if frame_count <= 3 {
                    log_info(&format!("[CPU] Frame {} calling submit_to_cpu...", frame_count));
                }
                state.submit_to_cpu(cpu_renderer);
                if frame_count <= 3 {
                    log_info(&format!("[CPU] Frame {} submit_to_cpu done", frame_count));
                }
            }

            // 显示帧缓冲
            if frame_count <= 3 {
                log_info(&format!("[CPU] Frame {} calling present...", frame_count));
            }
            match display.present_framebuffer() {
                Ok(_) => {
                    if frame_count <= 3 {
                        log_info(&format!("[CPU] Frame {} present done", frame_count));
                    }
                },
                Err(e) => {
                    log_warn(&format!("[CPU] Frame {} present failed: {}", frame_count, e));
                }
            }

            let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            fps_counter.record_frame(frame_time_ms);

            if result == FrameResult::Exit {
                state.shutdown();
                running = false;
            }
        }
    }
}

/// 游戏运行入口
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}
