// PC 桌面端平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: pixels + winit + cpal + rand + std::fs
//
// 重要:只有这个模块依赖 winit,其他游戏模块通过 platform.rs 抽象访问

use super::{
    AudioBackend, DisplayBackend, InputBackend, 
    KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, RandomBackend, StorageBackend, TimeBackend,
};
use crate::gpu::GpuRenderer;

// Windows 使用 hashbrown 避免 BCryptGenRandom 依赖(兼容 Win7)
// 但 desktop.rs 主要用于 wgpu-backend(非 Windows GDI),所以保留 std
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Winit 和 wgpu 相关导入
// ============================================================================

use wgpu::Backends;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::{Icon, Window, WindowId};

// ============================================================================
// 窗口图标 - 从游戏精灵生成
// ============================================================================

/// 从assets文件夹加载窗口图标
fn create_window_icon() -> Option<Icon> {
    // 尝试从编译时嵌入的图标数据创建
    // 使用include_bytes!在编译时嵌入PNG图标
    const ICON_DATA: &[u8] = include_bytes!("../../assets/mario_icon_preview.png");
    
    // 使用image crate解码PNG
    let img = image::load_from_memory(ICON_DATA).ok()?;
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let rgba_data = rgba_img.into_raw();
    
    Icon::from_rgba(rgba_data, width, height).ok()
}

// ============================================================================
// 显示后端 - 使用 wgpu + winit 进行GPU渲染
// ============================================================================

pub struct DesktopDisplay {
    window: Option<Arc<Window>>,
    width: u32,
    height: u32,
    // wgpu设备用于GPU渲染
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device: Option<Arc<wgpu::Device>>,
    wgpu_queue: Option<Arc<wgpu::Queue>>,
    wgpu_config: Option<wgpu::SurfaceConfiguration>,
    // GPU渲染器
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
    
    /// 获取GPU渲染器引用
    pub fn gpu_renderer(&self) -> Option<&GpuRenderer> {
        self.gpu_renderer.as_ref()
    }
    
    /// 获取GPU渲染器可变引用
    pub fn gpu_renderer_mut(&mut self) -> Option<&mut GpuRenderer> {
        self.gpu_renderer.as_mut()
    }

    /// 使用 ActiveEventLoop 创建窗口(winit 0.30 要求在事件循环内创建)
    pub fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        use winit::dpi::LogicalSize;
        
        let size = LogicalSize::new(self.width as f64, self.height as f64);
        
        // 创建窗口图标(从游戏精灵生成)
        let icon = create_window_icon();
        
        // 创建窗口（先不可见，避免白色闪烁）
        let window_attributes = Window::default_attributes()
            .with_title("Mario")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_window_icon(icon)
            .with_visible(false);
        
        let window = Arc::new(event_loop.create_window(window_attributes)?);
        let window_size = window.inner_size();
        
        // 创建独立的wgpu实例用于GPU渲染
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::GL | Backends::DX12,
            ..Default::default()
        });
        
        // 创建surface
        let surface = instance.create_surface(window.clone())?;
        
        // 请求适配器
        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).ok_or("找不到合适的GPU适配器")?;
        
        // 请求设备
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("mario_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))?;
        
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        // 配置surface
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
        
        // 创建GPU渲染器
        let gpu_renderer = GpuRenderer::new(device.clone(), queue.clone());
        
        self.window = Some(window);
        self.wgpu_surface = Some(surface);
        self.wgpu_device = Some(device);
        self.wgpu_queue = Some(queue);
        self.wgpu_config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);
        Ok(())
    }
    
    /// 显示窗口(在游戏初始化完成后调用)
    pub fn show_window(&mut self) {
        if let Some(window) = &self.window {
            window.set_visible(true);
            // 显示后立即请求重绘
            window.request_redraw();
        }
    }

    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }
    
    /// 处理窗口大小调整,重新配置wgpu surface
    pub fn resize(&mut self, new_width: u32, new_height: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(surface), Some(device), Some(config)) = 
            (&self.wgpu_surface, &self.wgpu_device, &mut self.wgpu_config) 
        {
            // 更新surface配置
            config.width = new_width.max(1);
            config.height = new_height.max(1);
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
        // 使用独立的wgpu surface进行GPU渲染输出
        if let (Some(surface), Some(gpu_renderer), Some(config)) = 
            (&self.wgpu_surface, &self.gpu_renderer, &self.wgpu_config) 
        {
            // 获取surface纹理
            let output = match surface.get_current_texture() {
                Ok(t) => t,
                Err(e) => {
                    log_error(&format!("[display] get_surface_texture_failed {:?}", e));
                    return Err(e.to_string());
                }
            };
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            // 更新缩放参数
            gpu_renderer.update_scale(config.width, config.height);
            
            // 渲染到窗口surface
            gpu_renderer.render_to_surface(&view);
            
            // 提交
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
// 时间后端 - 使用 std::time
// ============================================================================

pub struct DesktopTime {
    start: Instant,
}

impl DesktopTime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl TimeBackend for DesktopTime {
    fn now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

// ============================================================================
// 随机数后端 - 使用 rand (SmallRng 减小体积)
// ============================================================================

use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

pub struct DesktopRandom {
    rng: SmallRng,
}

impl DesktopRandom {
    pub fn new() -> Self {
        // 使用系统时间作为种子,避免依赖rand的std_rng特性
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

impl RandomBackend for DesktopRandom {
    fn random_range(&mut self, max: i32) -> i32 {
        if max <= 0 {
            return 0;
        }
        self.rng.gen_range(0..max)
    }

    fn random_range_f32(&mut self, max: f32) -> f32 {
        if max <= 0.0 {
            return 0.0;
        }
        self.rng.gen_range(0.0..max)
    }

    fn random_f32(&mut self) -> f32 {
        self.rng.gen_range(0.0..1.0)
    }
}

// ============================================================================
// 存储后端 - 使用 std::fs
// ============================================================================

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct DesktopStorage {
    base_path: PathBuf,
}

impl DesktopStorage {
    pub fn new() -> Self {
        // 优先使用当前工作目录(更稳定,适合开发和发布环境)
        // 如果获取失败,则使用可执行文件所在目录
        let base_path = std::env::current_dir()
            .ok()
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        Self { base_path }
    }

    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

impl StorageBackend for DesktopStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.get_path(key);
        let mut file = File::open(&path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Some(buffer)
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.get_path(key);
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
// 日志后端 - 使用 println
// ============================================================================

pub struct DesktopLog;

impl DesktopLog {
    pub fn new() -> Self {
        Self
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
// 输入后端
// ============================================================================

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
// 全局便捷函数 - 使用线程局部存储
// ============================================================================

use std::cell::RefCell;

thread_local! {
    static RANDOM: RefCell<DesktopRandom> = RefCell::new(DesktopRandom::new());
    static TIME: DesktopTime = DesktopTime::new();
    static LOG: DesktopLog = DesktopLog::new();
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
// 游戏应用程序 - 封装事件循环
// ============================================================================

use crate::game_runner::{GameState, print_startup_info, GAME_WIDTH, GAME_HEIGHT};
use crate::platform::FrameResult;

/// 游戏应用程序状态(平台层只负责事件循环,游戏逻辑在 game_runner 中)
struct GameApp {
    display: DesktopDisplay,
    #[allow(dead_code)]
    input: DesktopInput,
    game_state: Option<GameState>,
    frame_duration: Duration,
    next_frame: Instant,
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
            frame_duration: Duration::from_secs_f64(1.0 / 60.0),
            next_frame: Instant::now(),
            running: true,
            is_fullscreen: false,
        }
    }
    
    /// 切换全屏/窗口模式
    fn toggle_fullscreen(&mut self) {
        use winit::window::Fullscreen;
        
        if let Some(window) = &self.display.window {
            if self.is_fullscreen {
                // 退出全屏
                window.set_fullscreen(None);
                self.is_fullscreen = false;
            } else {
                // 进入全屏(使用无边框全屏模式)
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                self.is_fullscreen = true;
            }
        }
    }
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        eprintln!("[DEBUG] resumed: 开始");
        // 窗口创建(winit 0.30 要求在 resumed 中创建)
        if !self.display.has_window() {
            eprintln!("[DEBUG] resumed: 创建窗口");
            if let Err(e) = self.display.create_window(event_loop) {
                eprintln!("创建窗口失败: {}", e);
                event_loop.exit();
                return;
            }
            eprintln!("[DEBUG] resumed: 窗口创建完成");

            // 初始化游戏状态(游戏逻辑封装在 game_runner 模块中)
            // 注意:窗口在此期间保持不可见,避免白色闪烁
            eprintln!("[DEBUG] resumed: 创建GameState");
            let game_state = GameState::new();
            eprintln!("[DEBUG] resumed: GameState创建完成");
            
            // 上传精灵图集和调色板到GPU
            eprintln!("[DEBUG] resumed: 上传精灵图集");
            if let Some(gpu_renderer) = self.display.gpu_renderer_mut() {
                let (atlas_data, atlas_width, atlas_height) = game_state.get_atlas_data();
                gpu_renderer.upload_atlas(atlas_data, atlas_width, atlas_height);
                
                let palette = game_state.get_palette_rgba();
                gpu_renderer.upload_palette(0, &palette);
            }
            eprintln!("[DEBUG] resumed: 上传完成");
            
            self.game_state = Some(game_state);
            
            // 游戏初始化完成后再显示窗口
            self.display.show_window();
            eprintln!("[DEBUG] resumed: 窗口已显示");
            
            // 打印启动信息
            print_startup_info();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(state) = &mut self.game_state {
                    state.shutdown(); // 保存配置后再退出
                }
                self.running = false;
                event_loop.exit();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                // DPI 或显示器缩放发生变化时,窗口的物理像素尺寸可能变化
                // 这里打日志并兜底调用 resize,避免 surface 尺寸没跟上导致画面偏小或黑边异常
                if let Some(window) = &self.display.window {
                    let inner = window.inner_size();
                    let outer = window.outer_size();
                    if inner.width > 0 && inner.height > 0 {
                        if let Err(e) = self.display.resize(inner.width, inner.height) {
                            eprintln!("调整窗口大小失败: {}", e);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                let platform_key = winit_keycode_to_platform(&key_event.physical_key);
                let is_pressed = key_event.state == ElementState::Pressed;
                
                // F11 切换全屏
                if is_pressed && platform_key == PlatformKeyCode::F11 {
                    self.toggle_fullscreen();
                    return;
                }
                
                // ESC 退出全屏(仅全屏模式下)
                if is_pressed && platform_key == PlatformKeyCode::Escape && self.is_fullscreen {
                    self.toggle_fullscreen();
                    return;
                }
                
                // 转换为平台无关的 KeyEvent 并更新游戏键盘状态
                if let Some(state) = &mut self.game_state {
                    let platform_event = crate::platform::KeyEvent {
                        key: platform_key,
                        pressed: is_pressed,
                    };
                    state.handle_key_event(&platform_event);
                }
            }
            WindowEvent::Resized(new_size) => {
                // 窗口大小改变时,调整 surface texture 尺寸
                // pixels 库会自动处理等比例缩放和 letterbox
                if new_size.width > 0 && new_size.height > 0 {
                    if let Some(window) = &self.display.window {
                    }
                    if let Err(e) = self.display.resize(new_size.width, new_size.height) {
                        eprintln!("调整窗口大小失败: {}", e);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // 帧率限制
                let now = Instant::now();
                if now < self.next_frame {
                    self.display.request_redraw();
                    return;
                }
                self.next_frame = now + self.frame_duration;

                // 游戏帧更新
                if let Some(state) = &mut self.game_state {
                    let result = state.frame_update();

                    // GPU渲染流程
                    if let Some(gpu_renderer) = self.display.gpu_renderer_mut() {
                        // 获取精灵批次数据
                        let sprite_instances = state.get_sprite_instances();
                        let fill_rects = state.get_fill_rects();
                        
                        // 上传调色板
                        let palette = state.get_palette_rgba();
                        gpu_renderer.upload_palette(0, &palette);
                        
                        // 开始新帧
                        gpu_renderer.begin_frame();
                        
                        // 添加填充矩形
                        for fill in fill_rects {
                            gpu_renderer.draw_fill(fill);
                        }
                        
                        // 添加精灵
                        for sprite in sprite_instances {
                            gpu_renderer.draw_sprite(sprite);
                        }
                        
                        // 渲染到GPU纹理
                        gpu_renderer.render_frame();
                    }
                    
                    // 显示 - pixels 会自动进行等比例缩放和添加 letterbox
                    let _ = self.display.present();

                    if result == FrameResult::Exit {
                        state.shutdown(); // 保存配置后再退出
                        self.running = false;
                        event_loop.exit();
                    }
                }

                // 请求下一帧
                self.display.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 请求重绘以保持游戏循环运行
        self.display.request_redraw();
    }
}

/// 运行游戏(平台入口函数)
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[DEBUG] run_game: 开始");
    
    // 初始化日志系统(仅在启用 logging feature 时)
    #[cfg(feature = "logging")]
    {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
        
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
    // winit 0.30: run_app 会消费 event_loop 并在退出时返回
    let _ = event_loop.run_app(&mut app);

    Ok(())
}