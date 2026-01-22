// Android 平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: android-activity + wgpu + cpal + ndk
//
// 重要: 这个模块依赖 android-activity, 其他游戏模块通过 platform.rs 抽象访问

use super::{
    DisplayBackend, InputBackend,
    KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, StorageBackend,
    touch_panel::{TouchAction, TouchPanelInput, ButtonLayout, LAYOUT_SAVE_KEY},
};
use super::common::{
    CommonTime, CommonRandom, FileStorage, FrameTimer, FpsCounter,
    draw_fps_to_overlay_rgba,
};
use crate::gpu::GpuRenderer;

use std::collections::HashSet;
use std::time::{Duration, Instant};
use std::path::PathBuf;

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

use crate::game_runner::{GAME_WIDTH, GAME_HEIGHT};

// ============================================================================
// 类型别名 - 使用公共模块实现
// ============================================================================

pub type DesktopTime = CommonTime;
pub type DesktopRandom = CommonRandom;

// ============================================================================
// Android 输入后端
// ============================================================================

/// Android 输入后端 - 支持触摸和物理键盘
pub struct AndroidInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
    touch_panel: TouchPanelInput,
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

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.touch_panel.set_screen_size(width, height);
    }

    pub fn set_has_physical_keyboard(&mut self, has: bool) {
        self.has_physical_keyboard = has;
    }

    pub fn touch_panel(&self) -> &TouchPanelInput {
        &self.touch_panel
    }
    
    pub fn touch_panel_mut(&mut self) -> &mut TouchPanelInput {
        &mut self.touch_panel
    }

    pub fn should_show_virtual_buttons(&self) -> bool {
        !self.has_physical_keyboard
    }

    pub fn handle_touch(&mut self, pointer_id: usize, x: f32, y: f32, action: MotionAction) {
        let touch_action = match action {
            MotionAction::Down | MotionAction::PointerDown => TouchAction::Down,
            MotionAction::Move => TouchAction::Move,
            MotionAction::Up | MotionAction::PointerUp => TouchAction::Up,
            MotionAction::Cancel => TouchAction::Cancel,
            _ => return,
        };
        
        self.touch_panel.handle_touch(pointer_id, x, y, touch_action);
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

/// 检查是否是游戏控制键
fn is_game_control_key(keycode: Keycode) -> bool {
    matches!(keycode,
        Keycode::DpadLeft | Keycode::DpadRight | Keycode::DpadUp | Keycode::DpadDown |
        Keycode::W | Keycode::A | Keycode::S | Keycode::D |
        Keycode::Space | Keycode::Enter | Keycode::ShiftLeft | Keycode::ShiftRight |
        Keycode::CtrlLeft | Keycode::CtrlRight | Keycode::AltLeft | Keycode::AltRight
    )
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

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        use std::ffi::c_void;
        use wgpu::rwh::{
            AndroidDisplayHandle, AndroidNdkWindowHandle, HasDisplayHandle, HasWindowHandle,
            RawDisplayHandle, RawWindowHandle,
        };
        struct AndroidHandle {
            a_native_window: *mut c_void,
        }
        impl HasWindowHandle for AndroidHandle {
            fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
                let mut handle = AndroidNdkWindowHandle::empty();
                handle.a_native_window = self.a_native_window;
                handle.api_version = 0;
                let raw = RawWindowHandle::AndroidNdk(handle);
                Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(raw) })
            }
        }
        impl HasDisplayHandle for AndroidHandle {
            fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
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
    let msg = CString::new(message.replace('\0', "")).unwrap_or_else(|_| CString::new("(invalid message)").unwrap());
    
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

pub use super::common::{random_i32, random_usize, random_u32, random_u8, random_f32, now_ms};

use std::cell::RefCell;

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
    log_info("MarioRS Android starting...");

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
    
    // 使用公共帧率控制器
    let mut frame_timer = FrameTimer::new(60.0);
    let mut fps_counter = FpsCounter::new();
    let mut running = true;

    while running {
        app.poll_events(Some(Duration::ZERO), |event| {
            match event {
                PollEvent::Main(main_event) => {
                    match main_event {
                        MainEvent::InitWindow { .. } => {
                            log_info("=== Native window initialized ===");
                            if let Some(window) = app.native_window() {
                                let win_width = window.width();
                                let win_height = window.height();
                                
                                log_info(&format!("[Screen] NativeWindow size: {}x{}", win_width, win_height));
                                log_info(&format!("[Screen] NativeWindow aspect ratio: {:.3}", win_width as f32 / win_height as f32));
                                log_info(&format!("[Game] Game framebuffer size: {}x{}", GAME_WIDTH, GAME_HEIGHT));
                                log_info(&format!("[Game] Game aspect ratio: {:.3}", GAME_WIDTH as f32 / GAME_HEIGHT as f32));
                                
                                input.set_screen_size(win_width as f32, win_height as f32);
                                display.set_native_window(Some(window));

                                if game_state.is_none() {
                                    game_state = Some(GameState::new());
                                    log_info("Game state initialized");
                                }
                            }
                        }
                        MainEvent::TerminateWindow { .. } => {
                            log_info("Native window terminated");
                            display.set_native_window(None);
                        }
                        MainEvent::WindowResized { .. } | MainEvent::ContentRectChanged { .. } => {
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

        // 处理输入事件
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let read_event = iter.next(|event| {
                    match event {
                        InputEvent::KeyEvent(key_event) => {
                            let keycode = key_event.key_code();
                            input.handle_key(keycode, key_event.action());
                            if is_game_control_key(keycode) {
                                input.set_has_physical_keyboard(true);
                            }
                        }
                        InputEvent::MotionEvent(motion_event) => {
                            let action = motion_event.action();
                            let pointer_count = motion_event.pointer_count();
                            
                            let action_pointer_index = match action {
                                MotionAction::PointerDown | MotionAction::PointerUp => {
                                    Some(motion_event.pointer_index())
                                }
                                _ => None,
                            };
                            
                            for i in 0..pointer_count {
                                let pointer = motion_event.pointer_at_index(i);
                                
                                let pointer_action = if let Some(action_idx) = action_pointer_index {
                                    if i == action_idx { action } else { MotionAction::Move }
                                } else {
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

        // 帧率限制
        if !frame_timer.should_render() {
            frame_timer.wait_if_needed();
            continue;
        }
        frame_timer.advance();

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            let frame_start = Instant::now();
            
            for event in input.poll_events() {
                state.handle_key_event(&event);
            }

            let result = state.frame_update();

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
                // 使用公共模块的 FPS 绘制函数
                draw_fps_to_overlay_rgba(&mut overlay_vec, ow, oh, fps_counter.fps(), fps_counter.frame_time_ms());
                overlay_vec_opt = Some(overlay_vec);
            }

            if let Some(gpu_renderer) = display.gpu_renderer_mut() {
                state.submit_to_gpu(gpu_renderer);

                if let Some(overlay_vec) = overlay_vec_opt.as_ref() {
                    gpu_renderer.upload_overlay_rgba(ow, oh, overlay_vec);
                } else {
                    gpu_renderer.clear_overlay();
                }
            }

            let _ = display.present();
            
            // 使用公共模块的 FPS 计数器
            let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            fps_counter.record_frame(frame_time_ms);
            
            // 保存触摸面板布局
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

/// 游戏运行入口
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}
