// Android 平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: android-activity + wgpu + cpal + ndk
//
// 重要: 这个模块依赖 android-activity, 其他游戏模块通过 platform.rs 抽象访问

use super::common::{CommonRandom, CommonTime, FileStorage, FpsCounter};
use super::{
    DisplayBackend, InputBackend, KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, StorageBackend,
};
use crate::gpu::GpuRenderer;

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

/// JNI 导出函数 - 由 Java MainActivity 调用 (软键盘按键)
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_mariogame_mario_MainActivity_nativeOnKeyEvent(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    key_code: i32,
    pressed: i32,
) {
    log_info(&format!("[JNI KeyEvent] key_code={}, pressed={}", key_code, pressed != 0));
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
    input::{InputEvent, KeyAction, Keycode, MotionAction},
};
use ndk::native_window::NativeWindow;

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

    pub fn set_native_window(&mut self, window: Option<NativeWindow>) {
        self.native_window = window.clone();
        if window.is_none() {
            self.wgpu_surface = None;
            self.wgpu_device = None;
            self.wgpu_queue = None;
            self.wgpu_config = None;
            self.gpu_renderer = None;
            return;
        }

        let window = window.expect("window is Some");
        let win_width = window.width().max(1) as u32;
        let win_height = window.height().max(1) as u32;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

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
                return;
            }
        };

        let android_handle = AndroidHandle { a_native_window };

        let surface = match instance.create_surface(&android_handle) {
            Ok(s) => s,
            Err(e) => {
                log_error(&format!("[GPU] create_surface_failed {:?}", e));
                return;
            }
        };

        let adapter = match futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )) {
            Ok(a) => a,
            Err(e) => {
                log_error(&format!("[GPU] request_adapter_failed {:?}", e));
                return;
            }
        };

        let (device, queue) =
            match futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Mario Android Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })) {
                Ok(v) => v,
                Err(e) => {
                    log_error(&format!("[GPU] request_device_failed {:?}", e));
                    return;
                }
            };

        let device = std::sync::Arc::new(device);
        let queue: std::sync::Arc<wgpu::Queue> = std::sync::Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: win_width,
            height: win_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let mut gpu_renderer = GpuRenderer::new(device.clone(), queue.clone(), config.format);
        gpu_renderer.update_scale(config.width, config.height);

        self.wgpu_surface = Some(unsafe { std::mem::transmute(surface) });
        self.wgpu_device = Some(device);
        self.wgpu_queue = Some(queue);
        self.wgpu_config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);
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

/// Android 存储后端 - 使用内部存储路径
pub struct AndroidStorage {
    inner: FileStorage,
}

impl AndroidStorage {
    pub fn new() -> Self {
        Self {
            inner: FileStorage::new(),
        }
    }

    /// 使用 AndroidApp 获取内部存储路径
    pub fn with_app(app: &AndroidApp) -> Self {
        let base_path = if let Some(path) = app.internal_data_path() {
            path.to_path_buf()
        } else {
            PathBuf::from(".")
        };
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

    let mut display = AndroidDisplay::new(GAME_WIDTH, GAME_HEIGHT);
    let mut input = AndroidInput::new();
    let storage = AndroidStorage::with_app(&app);
    let mut game_state: Option<GameState> = None;

    // FPS 计数器
    let mut fps_counter = FpsCounter::new();
    let mut running = true;

    // GPU资源是否需要重新上传（后台恢复后设为true）
    let mut gpu_resources_invalidated = false;
    // 上一次渲染时间
    let mut last_render_time = Instant::now();
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    while running {
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
                        log_info(&format!("Window: {}x{}", window.width(), window.height()));
                        display.set_native_window(Some(window));

                        if game_state.is_none() {
                            game_state = Some(GameState::new());
                        } else {
                            gpu_resources_invalidated = true;
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
                    running = false;
                }
                MainEvent::Resume { .. } => {
                    if display.wgpu_surface.is_some() {
                        display.reconfigure_surface();
                        gpu_resources_invalidated = true;
                    } else if let Some(window) = app.native_window() {
                        display.set_native_window(Some(window));
                        gpu_resources_invalidated = true;
                    }
                }
                MainEvent::Pause => {}
                _ => {}
            },
            PollEvent::Wake => {}
            PollEvent::Timeout => {}
            _ => {}
        });

        // 处理输入事件 - 快速消费事件队列
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            input.handle_key(key_event.key_code(), key_event.action());
                        }
                        InputEvent::MotionEvent(_) => {
                            // 触摸事件由 Java 原生按钮处理
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
        
        // 处理软键盘事件 (来自 Java dispatchKeyEvent)
        if let Some(state) = &mut game_state {
            for soft_event in take_soft_key_events() {
                let key_event = soft_key_to_platform_event(&soft_event);
                log_info(&format!("[SoftKey] android_code={}, platform_key={:?}, pressed={}", 
                    soft_event.key_code, key_event.key, key_event.pressed));
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

            // FPS显示内置到游戏状态栏（使用GPU渲染，无需overlay）
            state.set_fps_display(fps_counter.fps(), fps_counter.frame_time_ms());

            let result = state.frame_update();

            // 获取 surface 配置信息
            let surface_config = display.wgpu_config.as_ref().map(|c| (c.width, c.height));

            if let Some((width, height)) = surface_config {
                if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                    // 准备渲染数据
                    state.submit_to_gpu(gpu_renderer);
                }

                // 获取 surface 纹理并渲染
                if let Some(surface) = &display.wgpu_surface {
                    match surface.get_current_texture() {
                        Ok(output) => {
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                                // 更新缩放参数
                                gpu_renderer.update_scale(width, height);

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
}

/// 游戏运行入口
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}
