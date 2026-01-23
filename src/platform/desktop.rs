// PC 桌面端平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: wgpu + winit + cpal
//
// 重要:只有这个模块依赖 winit,其他游戏模块通过 platform.rs 抽象访问

use super::common::{CommonRandom, CommonTime, FileStorage, FpsCounter, FrameTimer};
use super::{
    DisplayBackend, InputBackend, KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel,
};
use crate::gpu::GpuRenderer;

use std::sync::Arc;

// ============================================================================
// Winit 和 wgpu 相关导入
// ============================================================================

use wgpu::Backends;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::{Icon, Window, WindowId};

// Windows平台：禁用Alt键触发的系统菜单（防止游戏卡死）
#[cfg(target_os = "windows")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

// ============================================================================
// 类型别名 - 使用公共模块实现
// ============================================================================

pub type DesktopTime = CommonTime;
pub type DesktopRandom = CommonRandom;
pub type DesktopStorage = FileStorage;

// ============================================================================
// 窗口图标 - 从游戏精灵生成
// ============================================================================

/// 从assets文件夹加载窗口图标
fn create_window_icon() -> Option<Icon> {
    const ICON_DATA: &[u8] = include_bytes!("../../assets/mario_icon_preview.png");

    let img = image::load_from_memory(ICON_DATA).ok()?;
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let rgba_data = rgba_img.into_raw();

    Icon::from_rgba(rgba_data, width, height).ok()
}

/// Windows平台：禁用系统菜单，防止Alt键触发模态菜单循环导致游戏卡死
/// 
/// 问题原因：Windows的WS_SYSMENU窗口样式会在按下Alt键时进入模态菜单循环，
/// 这会阻塞winit的事件循环，导致游戏看起来"卡死"。
/// 解决方案：创建窗口后移除WS_SYSMENU样式。
#[cfg(target_os = "windows")]
fn disable_system_menu(window: &Window) {
    use std::ffi::c_void;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_SYSMENU,
    };

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            let hwnd = win32_handle.hwnd.get() as *mut c_void;
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                // 移除 WS_SYSMENU 样式，禁用Alt键触发的系统菜单
                let new_style = style & !(WS_SYSMENU as isize);
                SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
            }
        }
    }
}

// ============================================================================
// 显示后端 - 使用 wgpu + winit 进行GPU渲染
// ============================================================================

pub struct DesktopDisplay {
    window: Option<Arc<Window>>,
    width: u32,
    height: u32,
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device: Option<Arc<wgpu::Device>>,
    wgpu_queue: Option<Arc<wgpu::Queue>>,
    wgpu_config: Option<wgpu::SurfaceConfiguration>,
    gpu_renderer: Option<GpuRenderer>,
}

impl DesktopDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            window: None,
            width,
            height,
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

    /// 使用 ActiveEventLoop 创建窗口
    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use winit::dpi::LogicalSize;

        let size = LogicalSize::new(self.width as f64, self.height as f64);
        let icon = create_window_icon();

        let window_attributes = Window::default_attributes()
            .with_title("Mario")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_resizable(false) // 禁用窗口调整大小和最大化（像素游戏固定尺寸）
            .with_window_icon(icon)
            .with_visible(false);

        let window = Arc::new(event_loop.create_window(window_attributes)?);

        // Windows平台：禁用系统菜单，防止Alt键触发模态菜单循环导致游戏卡死
        #[cfg(target_os = "windows")]
        {
            disable_system_menu(&window);
        }

        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))?;

        // 使用默认限制，支持更大的纹理尺寸（8192+），以支持高分辨率全屏
        let (device, queue) =
            futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("mario_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            }))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let gpu_renderer = GpuRenderer::new(device.clone(), queue.clone(), config.format);

        self.window = Some(window);
        self.wgpu_surface = Some(surface);
        self.wgpu_device = Some(device);
        self.wgpu_queue = Some(queue);
        self.wgpu_config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);
        Ok(())
    }

    pub fn show_window(&mut self) {
        if let Some(window) = &self.window {
            window.set_visible(true);
            window.request_redraw();
        }
    }

    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }

    // 默认wgpu限制下的最大纹理尺寸（支持高分辨率全屏）
    const MAX_TEXTURE_SIZE: u32 = 8192;

    pub fn resize(
        &mut self,
        new_width: u32,
        new_height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(surface), Some(device), Some(config)) =
            (&self.wgpu_surface, &self.wgpu_device, &mut self.wgpu_config)
        {
            // 限制尺寸不超过最大纹理尺寸
            config.width = new_width.max(1).min(Self::MAX_TEXTURE_SIZE);
            config.height = new_height.max(1).min(Self::MAX_TEXTURE_SIZE);
            surface.configure(device, config);
        }
        Ok(())
    }
}

impl DisplayBackend for DesktopDisplay {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        if let (Some(surface), Some(gpu_renderer), Some(config)) =
            (&self.wgpu_surface, &self.gpu_renderer, &self.wgpu_config)
        {
            let output = match surface.get_current_texture() {
                Ok(t) => t,
                Err(e) => {
                    log_error(&format!("[display] get_surface_texture_failed {:?}", e));
                    return Err(e.to_string());
                }
            };
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            gpu_renderer.update_scale(config.width, config.height);
            gpu_renderer.render_to_surface(&view);

            output.present();
            Ok(())
        } else {
            Ok(())
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ============================================================================
// 音频后端 - 使用平台音频模块
// ============================================================================

pub use super::audio::PlatformAudio as DesktopAudio;

// ============================================================================
// 日志后端 - 使用 println (平台特有)
// ============================================================================

pub struct DesktopLog;

impl DesktopLog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DesktopLog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBackend for DesktopLog {
    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => println!("[DEBUG] {}", message),
            LogLevel::Info => println!("[INFO] {}", message),
            LogLevel::Warn => println!("[WARN] {}", message),
            LogLevel::Error => eprintln!("[ERROR] {}", message),
        }
    }
}

// ============================================================================
// 输入后端 - 包含 winit 特有的事件处理
// ============================================================================

use std::collections::HashSet;

pub struct DesktopInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
}

impl DesktopInput {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
        }
    }

    /// 处理 winit 键盘事件
    pub fn handle_winit_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let key = winit_keycode_to_platform(&event.physical_key);

        if pressed {
            self.key_states.insert(key);
        } else {
            self.key_states.remove(&key);
        }

        self.pending_events.push(PlatformKeyEvent { key, pressed });
    }
}

impl Default for DesktopInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBackend for DesktopInput {
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

/// 将 winit KeyCode 转换为平台无关的 KeyCode
fn winit_keycode_to_platform(physical_key: &PhysicalKey) -> PlatformKeyCode {
    match physical_key {
        PhysicalKey::Code(keycode) => match keycode {
            WinitKeyCode::ArrowLeft => PlatformKeyCode::Left,
            WinitKeyCode::ArrowRight => PlatformKeyCode::Right,
            WinitKeyCode::ArrowUp => PlatformKeyCode::Up,
            WinitKeyCode::ArrowDown => PlatformKeyCode::Down,
            WinitKeyCode::Space => PlatformKeyCode::Space,
            WinitKeyCode::AltLeft => PlatformKeyCode::AltLeft,
            WinitKeyCode::AltRight => PlatformKeyCode::AltRight,
            WinitKeyCode::ControlLeft => PlatformKeyCode::ControlLeft,
            WinitKeyCode::ControlRight => PlatformKeyCode::ControlRight,
            WinitKeyCode::ShiftLeft => PlatformKeyCode::ShiftLeft,
            WinitKeyCode::ShiftRight => PlatformKeyCode::ShiftRight,
            WinitKeyCode::Escape => PlatformKeyCode::Escape,
            WinitKeyCode::Enter => PlatformKeyCode::Enter,
            WinitKeyCode::Tab => PlatformKeyCode::Tab,
            WinitKeyCode::F1 => PlatformKeyCode::F1,
            WinitKeyCode::F2 => PlatformKeyCode::F2,
            WinitKeyCode::F11 => PlatformKeyCode::F11,
            WinitKeyCode::Backspace => PlatformKeyCode::Backspace,
            WinitKeyCode::KeyA => PlatformKeyCode::KeyA,
            WinitKeyCode::KeyB => PlatformKeyCode::KeyB,
            WinitKeyCode::KeyC => PlatformKeyCode::KeyC,
            WinitKeyCode::KeyD => PlatformKeyCode::KeyD,
            WinitKeyCode::KeyE => PlatformKeyCode::KeyE,
            WinitKeyCode::KeyF => PlatformKeyCode::KeyF,
            WinitKeyCode::KeyG => PlatformKeyCode::KeyG,
            WinitKeyCode::KeyH => PlatformKeyCode::KeyH,
            WinitKeyCode::KeyI => PlatformKeyCode::KeyI,
            WinitKeyCode::KeyJ => PlatformKeyCode::KeyJ,
            WinitKeyCode::KeyK => PlatformKeyCode::KeyK,
            WinitKeyCode::KeyL => PlatformKeyCode::KeyL,
            WinitKeyCode::KeyM => PlatformKeyCode::KeyM,
            WinitKeyCode::KeyN => PlatformKeyCode::KeyN,
            WinitKeyCode::KeyO => PlatformKeyCode::KeyO,
            WinitKeyCode::KeyP => PlatformKeyCode::KeyP,
            WinitKeyCode::KeyQ => PlatformKeyCode::KeyQ,
            WinitKeyCode::KeyR => PlatformKeyCode::KeyR,
            WinitKeyCode::KeyS => PlatformKeyCode::KeyS,
            WinitKeyCode::KeyT => PlatformKeyCode::KeyT,
            WinitKeyCode::KeyU => PlatformKeyCode::KeyU,
            WinitKeyCode::KeyV => PlatformKeyCode::KeyV,
            WinitKeyCode::KeyW => PlatformKeyCode::KeyW,
            WinitKeyCode::KeyX => PlatformKeyCode::KeyX,
            WinitKeyCode::KeyY => PlatformKeyCode::KeyY,
            WinitKeyCode::KeyZ => PlatformKeyCode::KeyZ,
            WinitKeyCode::Digit0 => PlatformKeyCode::Digit0,
            WinitKeyCode::Digit1 => PlatformKeyCode::Digit1,
            WinitKeyCode::Digit2 => PlatformKeyCode::Digit2,
            WinitKeyCode::Digit3 => PlatformKeyCode::Digit3,
            WinitKeyCode::Digit4 => PlatformKeyCode::Digit4,
            WinitKeyCode::Digit5 => PlatformKeyCode::Digit5,
            WinitKeyCode::Digit6 => PlatformKeyCode::Digit6,
            WinitKeyCode::Digit7 => PlatformKeyCode::Digit7,
            WinitKeyCode::Digit8 => PlatformKeyCode::Digit8,
            WinitKeyCode::Digit9 => PlatformKeyCode::Digit9,
            _ => PlatformKeyCode::Unknown,
        },
        PhysicalKey::Unidentified(_) => PlatformKeyCode::Unknown,
    }
}

// ============================================================================
// 全局便捷函数 - 使用公共模块实现
// ============================================================================

pub use super::common::{now_ms, random_f32, random_i32, random_u8, random_u32, random_usize};

thread_local! {
    static LOG: DesktopLog = DesktopLog::new();
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
// 游戏应用程序 - 封装事件循环
// ============================================================================

use crate::game_runner::{GAME_HEIGHT, GAME_WIDTH, GameState, print_startup_info};
use crate::platform::FrameResult;

/// 游戏应用程序状态
struct GameApp {
    display: DesktopDisplay,
    #[allow(dead_code)]
    input: DesktopInput,
    game_state: Option<GameState>,
    frame_timer: FrameTimer,
    fps_counter: FpsCounter,
    #[allow(dead_code)]
    running: bool,
    is_fullscreen: bool,
}

impl GameApp {
    fn new() -> Self {
        Self {
            display: DesktopDisplay::new(GAME_WIDTH, GAME_HEIGHT),
            input: DesktopInput::new(),
            game_state: None,
            frame_timer: FrameTimer::new(60.0),
            fps_counter: FpsCounter::new(),
            running: true,
            is_fullscreen: false,
        }
    }

    fn toggle_fullscreen(&mut self) {
        use winit::window::Fullscreen;

        if let Some(window) = &self.display.window {
            if self.is_fullscreen {
                window.set_fullscreen(None);
                self.is_fullscreen = false;
            } else {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                self.is_fullscreen = true;
            }
        }
    }
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        eprintln!("[DEBUG] resumed: 开始");
        if !self.display.has_window() {
            eprintln!("[DEBUG] resumed: 创建窗口");
            if let Err(e) = self.display.create_window(event_loop) {
                eprintln!("创建窗口失败: {}", e);
                event_loop.exit();
                return;
            }
            eprintln!("[DEBUG] resumed: 窗口创建完成");

            eprintln!("[DEBUG] resumed: 创建GameState");
            let game_state = GameState::new();
            eprintln!("[DEBUG] resumed: GameState创建完成");

            self.game_state = Some(game_state);

            self.display.show_window();
            eprintln!("[DEBUG] resumed: 窗口已显示");

            print_startup_info();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(state) = &mut self.game_state {
                    state.shutdown();
                }
                self.running = false;
                event_loop.exit();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.display.window {
                    let inner = window.inner_size();
                    if inner.width > 0 && inner.height > 0 {
                        if let Err(e) = self.display.resize(inner.width, inner.height) {
                            eprintln!("调整窗口大小失败: {}", e);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                let platform_key = winit_keycode_to_platform(&key_event.physical_key);
                let is_pressed = key_event.state == ElementState::Pressed;

                if is_pressed && platform_key == PlatformKeyCode::F11 {
                    self.toggle_fullscreen();
                    return;
                }

                if is_pressed && platform_key == PlatformKeyCode::Escape && self.is_fullscreen {
                    self.toggle_fullscreen();
                    return;
                }

                if let Some(state) = &mut self.game_state {
                    let platform_event = crate::platform::KeyEvent {
                        key: platform_key,
                        pressed: is_pressed,
                    };
                    state.handle_key_event(&platform_event);
                }
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    if let Err(e) = self.display.resize(new_size.width, new_size.height) {
                        eprintln!("调整窗口大小失败: {}", e);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // 使用公共帧率控制器
                if !self.frame_timer.should_render() {
                    self.display.request_redraw();
                    return;
                }
                self.frame_timer.advance();

                let frame_start = std::time::Instant::now();

                if let Some(state) = &mut self.game_state {
                    // 设置FPS显示数据（内置到游戏状态栏）
                    state.set_fps_display(self.fps_counter.fps(), self.fps_counter.frame_time_ms());

                    let result = state.frame_update();

                    // 使用合并渲染：一次GPU提交完成所有渲染
                    // 先获取需要的配置信息
                    let surface_config = self
                        .display
                        .wgpu_config
                        .as_ref()
                        .map(|c| (c.width, c.height));

                    if let Some((width, height)) = surface_config {
                        if let Some(gpu_renderer) = self.display.gpu_renderer_mut() {
                            // 准备渲染数据
                            state.submit_to_gpu(gpu_renderer);
                        }

                        // 获取surface纹理并渲染
                        if let Some(surface) = &self.display.wgpu_surface {
                            if let Ok(output) = surface.get_current_texture() {
                                let view = output
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());

                                if let Some(gpu_renderer) = self.display.gpu_renderer_mut() {
                                    gpu_renderer.update_scale(width, height);
                                    gpu_renderer.render_frame_and_present(&view);
                                }

                                output.present();
                            }
                        }
                    }

                    if result == FrameResult::Exit {
                        state.shutdown();
                        self.running = false;
                        event_loop.exit();
                    }
                }

                // 记录帧时间
                let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
                self.fps_counter.record_frame(frame_time_ms);

                self.display.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.display.request_redraw();
    }
}

/// 运行游戏(平台入口函数)
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[DEBUG] run_game: 开始");

    #[cfg(feature = "logging")]
    {
        use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
            .add_directive("wgpu_hal::vulkan=error".parse().unwrap());

        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .ok();
    }

    eprintln!("[DEBUG] run_game: 创建EventLoop");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    eprintln!("[DEBUG] run_game: 创建GameApp");
    let mut app = GameApp::new();
    eprintln!("[DEBUG] run_game: GameApp创建完成，开始运行");
    let _ = event_loop.run_app(&mut app);

    Ok(())
}
