// Android 平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: android-activity + wgpu + cpal + ndk
//
// 重要: 这个模块依赖 android-activity, 其他游戏模块通过 platform.rs 抽象访问

use super::{
    AudioBackend, DisplayBackend, InputBackend,
    KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, RandomBackend, StorageBackend, TimeBackend,
    touch_panel::{TouchAction, TouchPanelInput, ButtonLayout, LAYOUT_SAVE_KEY},
};
use crate::gpu::GpuRenderer;

use std::collections::HashSet;
use std::time::{Duration, Instant};

// FPS 显示使用的字体数据
use crate::txt::SWISS_FONT_GLYPHS;

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

// 游戏分辨率 - 从 game_runner 导入，与其他平台保持一致
// GAME_WIDTH = 320, GAME_HEIGHT = 182 (WINDOWHEIGHT，VGA framebuffer 已经是正确尺寸)
use crate::game_runner::{GAME_WIDTH, GAME_HEIGHT};

// ============================================================================
// Android 输入后端
// ============================================================================

/// Android 输入后端 - 支持触摸和物理键盘
pub struct AndroidInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
    
    // 触摸面板 (使用公共模块)
    touch_panel: TouchPanelInput,
    
    // 是否有物理键盘
    has_physical_keyboard: bool,
}

impl AndroidInput {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
            touch_panel: TouchPanelInput::new(),
            has_physical_keyboard: false,
        }
    }

    /// 更新屏幕尺寸
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.touch_panel.set_screen_size(width, height);
    }

    /// 设置是否有物理键盘
    pub fn set_has_physical_keyboard(&mut self, has: bool) {
        self.has_physical_keyboard = has;
    }

    /// 获取触摸面板引用 (用于渲染)
    pub fn touch_panel(&self) -> &TouchPanelInput {
        &self.touch_panel
    }
    
    /// 获取触摸面板可变引用 (用于编辑布局)
    pub fn touch_panel_mut(&mut self) -> &mut TouchPanelInput {
        &mut self.touch_panel
    }

    /// 是否应该显示虚拟按键
    pub fn should_show_virtual_buttons(&self) -> bool {
        !self.has_physical_keyboard
    }

    /// 处理触摸事件 (将 Android MotionAction 转换为通用 TouchAction)
    pub fn handle_touch(&mut self, pointer_id: usize, x: f32, y: f32, action: MotionAction) {
        let touch_action = match action {
            MotionAction::Down | MotionAction::PointerDown => TouchAction::Down,
            MotionAction::Move => TouchAction::Move,
            MotionAction::Up | MotionAction::PointerUp => TouchAction::Up,
            MotionAction::Cancel => TouchAction::Cancel,
            _ => return, // 忽略其他动作
        };
        
        self.touch_panel.handle_touch(pointer_id, x, y, touch_action);
    }

    /// 处理物理按键事件
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
        // 合并物理键盘事件和触摸面板事件
        let mut events = std::mem::take(&mut self.pending_events);
        events.extend(self.touch_panel.take_pending_events());
        events
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

/// 检查是否是游戏控制键 (用于判断是否有物理键盘)
fn is_game_control_key(keycode: Keycode) -> bool {
    matches!(keycode,
        // 方向键
        Keycode::DpadLeft | Keycode::DpadRight | Keycode::DpadUp | Keycode::DpadDown |
        // WASD
        Keycode::W | Keycode::A | Keycode::S | Keycode::D |
        // 动作键
        Keycode::Space | Keycode::Enter | Keycode::ShiftLeft | Keycode::ShiftRight |
        Keycode::CtrlLeft | Keycode::CtrlRight | Keycode::AltLeft | Keycode::AltRight
    )
}

fn draw_fps_to_overlay_rgba(overlay: &mut [u8], width: u32, height: u32, fps: u32, frame_time_ms: f32) {
    if width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    if overlay.len() < w.saturating_mul(h).saturating_mul(4) {
        return;
    }

    let text = format!("FPS:{} MS:{:.1}", fps, frame_time_ms);
    let mut x_pos = 10usize;
    let y_pos = 10usize;
    let scale = 1usize;

    let draw_glyph = |overlay: &mut [u8],
                      x_pos: usize,
                      y_pos: usize,
                      glyph_w: usize,
                      glyph_h: usize,
                      bitmap: &[u8],
                      color: [u8; 4],
                      dx: usize,
                      dy: usize| {
        for row in 0..glyph_h {
            for col in 0..glyph_w {
                let bit_index = row * glyph_w + col;
                let byte_index = bit_index / 8;
                let bit_offset = bit_index % 8;
                if byte_index >= bitmap.len() {
                    continue;
                }
                let byte = bitmap[byte_index];
                let bit = (byte >> bit_offset) & 1;
                if bit != 1 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x_pos + col * scale + sx + dx;
                        let py = y_pos + row * scale + sy + dy;
                        if px >= w || py >= h {
                            continue;
                        }
                        let idx = (py * w + px) * 4;
                        overlay[idx] = color[0];
                        overlay[idx + 1] = color[1];
                        overlay[idx + 2] = color[2];
                        overlay[idx + 3] = color[3];
                    }
                }
            }
        }
    };

    let shadow = [0u8, 0u8, 0u8, 255u8];
    let white = [255u8, 255u8, 255u8, 255u8];

    for ch in text.chars() {
        let ch_code = ch as usize;
        if ch_code < 32 || ch_code > 129 {
            x_pos += 8;
            continue;
        }
        let glyph_idx = ch_code - 32;
        if glyph_idx >= SWISS_FONT_GLYPHS.len() {
            x_pos += 8;
            continue;
        }
        let glyph = &SWISS_FONT_GLYPHS[glyph_idx];
        let glyph_w = glyph.width() as usize;
        let glyph_h = glyph.height() as usize;
        let bitmap = glyph.bitmap();

        draw_glyph(overlay, x_pos, y_pos, glyph_w, glyph_h, bitmap, shadow, 1, 1);
        draw_glyph(overlay, x_pos, y_pos, glyph_w, glyph_h, bitmap, white, 0, 0);

        x_pos += glyph_w * scale + 2;
    }
}

// ============================================================================
// 显示后端 - Android: wgpu surface + GpuRenderer
// ============================================================================

pub struct AndroidDisplay {
    width: u32,
    height: u32,
    native_window: Option<NativeWindow>,
    // wgpu surface 与设备
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device: Option<std::sync::Arc<wgpu::Device>>,
    wgpu_queue: Option<std::sync::Arc<wgpu::Queue>>,
    wgpu_config: Option<wgpu::SurfaceConfiguration>,
    // GPU 渲染器
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
    
    /// 获取GPU渲染器引用
    pub fn gpu_renderer(&self) -> Option<&GpuRenderer> {
        self.gpu_renderer.as_ref()
    }
    
    /// 获取GPU渲染器可变引用
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

        // wgpu 初始化（与 desktop/windows 路径保持一致）
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        // 从 ANativeWindow 创建 Surface
        use std::ffi::c_void;
        use wgpu::rwh::{
            AndroidDisplayHandle, AndroidNdkWindowHandle, HasDisplayHandle, HasWindowHandle,
            RawDisplayHandle, RawWindowHandle,
        };
        struct AndroidHandle {
            a_native_window: *mut c_void,
        }
        impl HasWindowHandle for AndroidHandle {
            fn window_handle(
                &self,
            ) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
                let mut handle = AndroidNdkWindowHandle::empty();
                handle.a_native_window = self.a_native_window;
                handle.api_version = 0;
                let raw = RawWindowHandle::AndroidNdk(handle);
                Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(raw) })
            }
        }
        impl HasDisplayHandle for AndroidHandle {
            fn display_handle(
                &self,
            ) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
                let raw = RawDisplayHandle::Android(AndroidDisplayHandle::empty());
                Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(raw) })
            }
        }

        let android_handle = AndroidHandle {
            a_native_window: window.ptr().as_ptr() as *mut c_void,
        };

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
            Some(a) => a,
            None => {
                log_error("[GPU] request_adapter_failed");
                return;
            }
        };

        let (device, queue) = match futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Mario Android Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )) {
            Ok(v) => v,
            Err(e) => {
                log_error(&format!("[GPU] request_device_failed {:?}", e));
                return;
            }
        };

        let device = std::sync::Arc::new(device);
        let queue = std::sync::Arc::new(queue);

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
}

impl DisplayBackend for AndroidDisplay {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        let (surface, gpu_renderer, config) = match (&self.wgpu_surface, &mut self.gpu_renderer, &self.wgpu_config) {
            (Some(s), Some(g), Some(c)) => (s, g, c),
            _ => return Ok(()),
        };

        let output = match surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => return Err(e.to_string()),
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        gpu_renderer.update_scale(config.width, config.height);
        gpu_renderer.render_to_surface(&view);

        output.present();
        Ok(())
    }

    fn request_redraw(&self) {
        // Android 使用连续渲染模式
    }
}

impl AndroidDisplay {
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let (surface, device, config) = match (&self.wgpu_surface, &self.wgpu_device, &mut self.wgpu_config) {
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
}

// ============================================================================
// 时间后端 - 使用 std::time
// ============================================================================

pub struct AndroidTime {
    start: Instant,
}

impl AndroidTime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for AndroidTime {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeBackend for AndroidTime {
    fn now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

// ============================================================================
// 随机数后端 - 使用 rand
// ============================================================================

use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

pub struct AndroidRandom {
    rng: SmallRng,
}

impl AndroidRandom {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

impl Default for AndroidRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBackend for AndroidRandom {
    fn random_range(&mut self, max: i32) -> i32 {
        if max <= 0 { return 0; }
        self.rng.gen_range(0..max)
    }

    fn random_range_f32(&mut self, max: f32) -> f32 {
        if max <= 0.0 { return 0.0; }
        self.rng.gen_range(0.0..max)
    }

    fn random_f32(&mut self) -> f32 {
        self.rng.gen_range(0.0..1.0)
    }
}

// ============================================================================
// 存储后端 - 使用 Android Internal Storage
// ============================================================================

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct AndroidStorage {
    base_path: PathBuf,
}

impl AndroidStorage {
    /// 创建存储后端 (无参数版本)
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from("."),
        }
    }

    /// 创建存储后端 (使用 AndroidApp 获取内部存储路径)
    pub fn with_app(app: &AndroidApp) -> Self {
        let base_path = if let Some(path) = app.internal_data_path() {
            path.to_path_buf()
        } else {
            PathBuf::from(".")
        };
        Self { base_path }
    }

    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

impl Default for AndroidStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for AndroidStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.get_path(key);
        let mut file = File::open(&path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Some(buffer)
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.get_path(key);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        let path = self.get_path(key);
        fs::remove_file(&path).map_err(|e| e.to_string())
    }

    fn exists(&self, key: &str) -> bool {
        self.get_path(key).exists()
    }
}

// ============================================================================
// 日志后端 - 使用 NDK 原生日志 API
// ============================================================================

/// 直接调用 Android NDK 日志 API (确保日志输出)
fn android_log_write(priority: i32, message: &str) {
    use std::ffi::CString;
    
    // 日志 tag
    let tag = CString::new("MarioRS").unwrap_or_else(|_| CString::new("Mario").unwrap());
    // 处理消息中的 null 字符
    let msg = CString::new(message.replace('\0', "")).unwrap_or_else(|_| CString::new("(invalid message)").unwrap());
    
    unsafe {
        // Android log priorities: VERBOSE=2, DEBUG=3, INFO=4, WARN=5, ERROR=6
        ndk_sys::__android_log_write(priority, tag.as_ptr(), msg.as_ptr());
    }
}

pub struct AndroidLog;

impl AndroidLog {
    pub fn new() -> Self {
        Self
    }

    pub fn init() {
        // 初始化 android_logger (用于 log crate 宏)
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("MarioRS"),
        );
        // 使用原生 API 输出初始化消息
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
        // 使用原生 NDK API 直接输出 (更可靠)
        let priority = match level {
            LogLevel::Debug => 3,  // ANDROID_LOG_DEBUG
            LogLevel::Info => 4,   // ANDROID_LOG_INFO
            LogLevel::Warn => 5,   // ANDROID_LOG_WARN
            LogLevel::Error => 6,  // ANDROID_LOG_ERROR
        };
        android_log_write(priority, message);
    }
}

// ============================================================================
// 音频后端 - 使用 cpal
// ============================================================================

pub use super::audio::PlatformAudio as AndroidAudio;

// ============================================================================
// 全局便捷函数 - 使用线程局部存储
// ============================================================================

use std::cell::RefCell;

thread_local! {
    static RANDOM: RefCell<AndroidRandom> = RefCell::new(AndroidRandom::new());
    static TIME: AndroidTime = AndroidTime::new();
    static LOG: AndroidLog = AndroidLog::new();
}

pub fn random_i32(max: i32) -> i32 {
    RANDOM.with(|r| r.borrow_mut().random_range(max))
}

pub fn random_usize(max: usize) -> usize {
    random_i32(max as i32) as usize
}

pub fn random_u32(max: u32) -> u32 {
    random_i32(max as i32) as u32
}

pub fn random_u8(max: u8) -> u8 {
    random_i32(max as i32) as u8
}

pub fn random_f32(max: f32) -> f32 {
    RANDOM.with(|r| r.borrow_mut().random_range_f32(max))
}

pub fn now_ms() -> f64 {
    TIME.with(|t| t.now_ms())
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
// 类型别名 - 与其他平台保持一致的命名
// ============================================================================

pub type DesktopStorage = AndroidStorage;
pub type DesktopAudio = AndroidAudio;
pub type DesktopDisplay = AndroidDisplay;
pub type DesktopInput = AndroidInput;
pub type DesktopTime = AndroidTime;
pub type DesktopRandom = AndroidRandom;
pub type DesktopLog = AndroidLog;

// ============================================================================
// Android 主入口
// ============================================================================

use crate::game_runner::{GameState, print_startup_info};
use crate::platform::FrameResult;

/// Android 应用主函数
pub fn android_main(app: AndroidApp) {
    // 初始化日志
    AndroidLog::init();
    log_info("MarioRS Android starting...");

    // 创建游戏组件
    let mut display = AndroidDisplay::new(GAME_WIDTH, GAME_HEIGHT);
    let mut input = AndroidInput::new();
    let mut storage = AndroidStorage::with_app(&app);
    let mut game_state: Option<GameState> = None;
    
    // 加载保存的触摸面板布局
    if let Some(data) = storage.load(LAYOUT_SAVE_KEY) {
        if let Some(layout) = ButtonLayout::from_bytes(&data) {
            input.touch_panel_mut().apply_layout(&layout);
            log_info("Touch panel layout loaded");
        }
    }
    
    // 帧率控制
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut next_frame = Instant::now();
    let mut running = true;
    
    // FPS 统计
    let mut fps_frame_count = 0u32;
    let mut fps_last_update = Instant::now();
    let mut fps_display = 0u32;           // 显示的 FPS (每秒更新)
    let mut frame_time_display = 0.0f32;  // 显示的帧渲染时间 (每秒更新)
    let mut frame_time_accumulator = 0.0f32; // 帧时间累加器
    let mut frame_start = Instant::now(); // 帧开始时间

    // Android 事件循环
    while running {
        // 使用非阻塞事件轮询 (timeout=0)，避免等待事件浪费帧时间
        app.poll_events(Some(Duration::ZERO), |event| {
            match event {
                PollEvent::Main(main_event) => {
                    match main_event {
                        MainEvent::InitWindow { .. } => {
                            log_info("=== Native window initialized ===");
                            if let Some(window) = app.native_window() {
                                let win_width = window.width();
                                let win_height = window.height();
                                
                                // 打印详细的窗口信息
                                log_info(&format!("[Screen] NativeWindow size: {}x{}", win_width, win_height));
                                log_info(&format!("[Screen] NativeWindow aspect ratio: {:.3}", win_width as f32 / win_height as f32));
                                log_info(&format!("[Game] Game framebuffer size: {}x{}", GAME_WIDTH, GAME_HEIGHT));
                                log_info(&format!("[Game] Game aspect ratio: {:.3}", GAME_WIDTH as f32 / GAME_HEIGHT as f32));
                                
                                input.set_screen_size(win_width as f32, win_height as f32);
                                display.set_native_window(Some(window));

                                // 初始化 GameState，并上传 atlas/palette 到 GPU
                                if game_state.is_none() {
                                    let state = GameState::new();
                                    if let Some(gpu) = display.gpu_renderer_mut() {
                                        let (atlas_data, atlas_w, atlas_h) = state.get_atlas_data();
                                        gpu.upload_atlas(atlas_data, atlas_w, atlas_h);
                                        let palette = state.get_palette_rgba();
                                        gpu.upload_palette(0, &palette);
                                    }
                                    game_state = Some(state);
                                    log_info("Game state initialized");
                                } else if let Some(state) = game_state.as_ref() {
                                    if let Some(gpu) = display.gpu_renderer_mut() {
                                        let (atlas_data, atlas_w, atlas_h) = state.get_atlas_data();
                                        gpu.upload_atlas(atlas_data, atlas_w, atlas_h);
                                        let palette = state.get_palette_rgba();
                                        gpu.upload_palette(0, &palette);
                                    }
                                }
                            }
                        }
                        MainEvent::TerminateWindow { .. } => {
                            log_info("Native window terminated");
                            display.set_native_window(None);
                        }
                        MainEvent::WindowResized { .. } | MainEvent::ContentRectChanged { .. } => {
                            // 窗口尺寸或内容区域变化时更新 touch_panel
                            if let Some(window) = app.native_window() {
                                let width = window.width() as f32;
                                let height = window.height() as f32;
                                log_info(&format!("Window resized: {}x{}", width, height));
                                input.set_screen_size(width, height);
                                display.resize(window.width() as u32, window.height() as u32);
                            }
                        }
                        MainEvent::Destroy => {
                            log_info("App destroy requested");
                            running = false;
                        }
                        _ => {}
                    }
                }
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                _ => {}
            }
        });

        // 处理输入事件 (android-activity 0.6 API)
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            let keycode = key_event.key_code();
                            input.handle_key(keycode, key_event.action());
                            // 只有游戏控制键才视为物理键盘 (忽略音量键等系统按键)
                            if is_game_control_key(keycode) {
                                input.set_has_physical_keyboard(true);
                            }
                        }
                        InputEvent::MotionEvent(motion_event) => {
                            let action = motion_event.action();
                            let pointer_count = motion_event.pointer_count();
                            
                            // 获取 PointerDown/PointerUp 事件对应的 pointer index
                            // 只有该 index 的 pointer 使用原始 action，其他使用 Move
                            let action_pointer_index = match action {
                                MotionAction::PointerDown | MotionAction::PointerUp => {
                                    Some(motion_event.pointer_index())
                                }
                                _ => None,
                            };
                            
                            for i in 0..pointer_count {
                                let pointer = motion_event.pointer_at_index(i);
                                
                                // 确定此 pointer 应该使用的 action
                                let pointer_action = if let Some(action_idx) = action_pointer_index {
                                    if i == action_idx {
                                        // 这个 pointer 是事件的主体
                                        action
                                    } else {
                                        // 其他 pointer 视为 Move（位置更新）
                                        MotionAction::Move
                                    }
                                } else {
                                    // Down/Up/Move/Cancel 等事件应用到所有 pointer
                                    action
                                };
                                
                                input.handle_touch(
                                    pointer.pointer_id() as usize,
                                    pointer.x(),
                                    pointer.y(),
                                    pointer_action,
                                );
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

        // 帧率限制 (使用 sleep 而非忙等待，节省 CPU)
        let now = Instant::now();
        if now < next_frame {
            let sleep_time = next_frame - now;
            if sleep_time > Duration::from_millis(1) {
                std::thread::sleep(sleep_time - Duration::from_millis(1));
            }
            continue;
        }
        next_frame = now + frame_duration;

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            // 记录帧开始时间
            frame_start = Instant::now();
            
            // 处理键盘事件
            for event in input.poll_events() {
                state.handle_key_event(&event);
            }

            // 更新游戏逻辑
            let result = state.frame_update();

            // GPU 渲染流程：Sprite/Fills -> GpuRenderer -> Surface present
            let (ow, oh) = match display.wgpu_config.as_ref() {
                Some(c) => (c.width, c.height),
                None => (0, 0),
            };

            let mut overlay_vec_opt: Option<Vec<u8>> = None;
            if ow > 0 && oh > 0 {
                let mut overlay_vec = if input.should_show_virtual_buttons() {
                    let button_states = input.touch_panel().button_states();
                    match input.touch_panel_mut().renderer_mut().render(&button_states) {
                        Some(s) => s.to_vec(),
                        None => vec![0u8; (ow * oh * 4) as usize],
                    }
                } else {
                    vec![0u8; (ow * oh * 4) as usize]
                };
                draw_fps_to_overlay_rgba(&mut overlay_vec, ow, oh, fps_display, frame_time_display);
                overlay_vec_opt = Some(overlay_vec);
            }

            if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                let sprite_instances = state.get_sprite_instances();
                let fill_rects = state.get_fill_rects();
                let palette = state.get_palette_rgba();

                // 上传图集（BuildWorld 可能会重建/重着色 sprites）
                let (atlas_data, atlas_w, atlas_h) = state.get_atlas_data();
                gpu_renderer.upload_atlas(atlas_data, atlas_w, atlas_h);

                gpu_renderer.upload_palette(0, &palette);
                gpu_renderer.begin_frame();
                for f in fill_rects {
                    gpu_renderer.draw_fill(f);
                }
                for s in sprite_instances {
                    gpu_renderer.draw_sprite(s);
                }
                gpu_renderer.render_frame();

                if let Some(overlay_vec) = overlay_vec_opt.as_ref() {
                    gpu_renderer.upload_overlay_rgba(ow, oh, overlay_vec);
                } else {
                    gpu_renderer.clear_overlay();
                }
            }

            let _ = display.present();
            
            // 累加帧渲染时间
            let current_frame_time = frame_start.elapsed().as_secs_f32() * 1000.0;
            frame_time_accumulator += current_frame_time;
            
            // 更新 FPS 和帧时间统计 (每秒更新一次，显示平均值)
            fps_frame_count += 1;
            let fps_elapsed = fps_last_update.elapsed();
            if fps_elapsed >= Duration::from_secs(1) {
                fps_display = (fps_frame_count as f32 / fps_elapsed.as_secs_f32()) as u32;
                frame_time_display = frame_time_accumulator / fps_frame_count as f32;
                fps_frame_count = 0;
                frame_time_accumulator = 0.0;
                fps_last_update = Instant::now();
            }
            
            // 检查并保存触摸面板布局（在 overlay 借用结束后）
            if input.touch_panel_mut().take_layout_changed() {
                let layout = input.touch_panel().get_layout();
                let data = layout.to_bytes();
                if storage.save(LAYOUT_SAVE_KEY, &data).is_ok() {
                    log_info("Touch panel layout saved");
                }
            }

            if result == FrameResult::Exit {
                state.shutdown();
                running = false;
            }
        }
    }

    log_info("MarioRS Android exiting...");
}

/// 游戏运行入口 (与其他平台保持一致的接口)
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}
