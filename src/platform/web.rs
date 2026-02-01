#![cfg(target_arch = "wasm32")]

//! Web 平台实现 - 使用 wgpu WebGPU 后端
//!
//! 实现 platform.rs 中定义的所有 Backend traits
//! 使用: wgpu (WebGPU) + wasm-bindgen + web-sys
//!
//! 重要: 此模块只在 WASM 目标平台编译

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, HtmlCanvasElement};

use super::common::CommonRandom;
use super::{
    AudioBackend, DisplayBackend, InputBackend, KeyCode as PlatformKeyCode,
    KeyEvent as PlatformKeyEvent, LogBackend, LogLevel, RandomBackend, StorageBackend, TimeBackend,
};
#[cfg(feature = "wgpu-backend")]
use crate::gpu::GpuRenderer;
use crate::status::RenderMode;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

// ============================================================================
// 类型别名
// ============================================================================

pub type DesktopRandom = CommonRandom;

// ============================================================================
// 日志后端 - 浏览器控制台
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
            LogLevel::Debug | LogLevel::Info => console::log_1(&JsValue::from_str(message)),
            LogLevel::Warn => console::warn_1(&JsValue::from_str(message)),
            LogLevel::Error => console::error_1(&JsValue::from_str(message)),
        }
    }
}

// 日志辅助函数
pub fn log_debug(msg: &str) {
    DesktopLog::default().log(LogLevel::Debug, msg);
}
pub fn log_info(msg: &str) {
    DesktopLog::default().log(LogLevel::Info, msg);
}
pub fn log_warn(msg: &str) {
    DesktopLog::default().log(LogLevel::Warn, msg);
}
pub fn log_error(msg: &str) {
    DesktopLog::default().log(LogLevel::Error, msg);
}

// ============================================================================
// 存储后端 - LocalStorage
// ============================================================================

pub struct DesktopStorage;

impl DesktopStorage {
    pub fn new() -> Self {
        Self
    }

    fn get_storage() -> Option<web_sys::Storage> {
        window()?.local_storage().ok()?
    }
}

impl Default for DesktopStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for DesktopStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let storage = Self::get_storage()?;
        let data_str = storage.get_item(key).ok()??;
        // 将 base64 编码的字符串解码为字节
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data_str.as_bytes())
            .ok()
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let storage = Self::get_storage().ok_or("无法访问 localStorage")?;
        // 将字节编码为 base64 字符串
        use base64::Engine;
        let data_str = base64::engine::general_purpose::STANDARD.encode(data);
        storage
            .set_item(key, &data_str)
            .map_err(|_| "保存数据失败".to_string())
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        let storage = Self::get_storage().ok_or("无法访问 localStorage")?;
        storage
            .remove_item(key)
            .map_err(|_| "删除数据失败".to_string())
    }

    fn exists(&self, key: &str) -> bool {
        if let Some(storage) = Self::get_storage() {
            storage.get_item(key).ok().flatten().is_some()
        } else {
            false
        }
    }
}

// ============================================================================
// 显示后端 - WebGPU 渲染
// ============================================================================

pub struct DesktopDisplay {
    canvas: HtmlCanvasElement,
    width: u32,
    height: u32,
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device: Option<Arc<wgpu::Device>>,
    wgpu_queue: Option<Arc<wgpu::Queue>>,
    wgpu_config: Option<wgpu::SurfaceConfiguration>,
    gpu_renderer: Option<GpuRenderer>,
}

impl DesktopDisplay {
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Result<Self, String> {
        let window = web_sys::window().ok_or("无法获取 window 对象")?;
        let document = window.document().ok_or("无法获取 document 对象")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| format!("找不到 canvas 元素: {}", canvas_id))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "元素不是 canvas")?;

        // 设置 canvas 尺寸
        canvas.set_width(width);
        canvas.set_height(height);

        Ok(Self {
            canvas,
            width,
            height,
            wgpu_surface: None,
            wgpu_device: None,
            wgpu_queue: None,
            wgpu_config: None,
            gpu_renderer: None,
        })
    }

    pub fn gpu_renderer(&self) -> Option<&GpuRenderer> {
        self.gpu_renderer.as_ref()
    }

    pub fn gpu_renderer_mut(&mut self) -> Option<&mut GpuRenderer> {
        self.gpu_renderer.as_mut()
    }

    /// 初始化 WebGPU
    pub async fn init_webgpu(&mut self) -> Result<(), String> {
        log_info("开始初始化 WebGPU...");

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // wgpu API: on wasm create raw window/display handles from the canvas and call unsafe API
        let value: &wasm_bindgen::JsValue = &self.canvas;
        let obj = core::ptr::NonNull::from(value).cast();
        let raw_window_handle = raw_window_handle::WebCanvasWindowHandle::new(obj).into();
        let raw_display_handle = raw_window_handle::WebDisplayHandle::new().into();

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .map_err(|e| format!("创建 surface 失败: {:?}", e))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("请求 adapter 失败: {:?}", e))?;

        log_info(&format!("GPU Adapter: {:?}", adapter.get_info()));

        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("mario_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("无法创建 device: {:?}", e))?;

        // 设置 uncaptured error handler，记录 WebGPU 错误但不 panic
        device.on_uncaptured_error(Arc::new(|err| {
            log_error(&format!("WebGPU 错误: {:?}", err));
        }));

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // 获取 surface 支持的纹理格式，使用第一个支持的格式
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        log_info(&format!("使用 surface 格式: {:?}", surface_format));

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.width.max(1),
            height: self.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Opaque),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // 先创建 GpuRenderer，它会持有设备的 Arc 引用
        // 注意：不使用 clone()，直接传递 Arc，避免 WebGPU 后端的设备身份问题
        let gpu_renderer = GpuRenderer::new(device, queue, config.format);

        // 使用 GpuRenderer 中的设备来配置 surface，确保使用同一个设备引用
        surface.configure(&gpu_renderer.device, &config);

        self.wgpu_surface = Some(surface);
        self.wgpu_device = None; // 不再单独存储设备，统一使用 GpuRenderer 的设备
        self.wgpu_queue = None;
        self.wgpu_config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);

        log_info("WebGPU 初始化完成");
        Ok(())
    }

    pub fn has_webgpu(&self) -> bool {
        self.gpu_renderer.is_some()
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) -> Result<(), String> {
        self.canvas.set_width(new_width);
        self.canvas.set_height(new_height);

        // 使用 GpuRenderer 的设备来重新配置 surface
        if let (Some(surface), Some(gpu_renderer), Some(config)) = (
            &self.wgpu_surface,
            &self.gpu_renderer,
            &mut self.wgpu_config,
        ) {
            config.width = new_width.max(1);
            config.height = new_height.max(1);
            surface.configure(&gpu_renderer.device, config);
        }

        self.width = new_width;
        self.height = new_height;
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
        // 获取 GPU 渲染器引用（用于获取设备和进行渲染）
        let gpu_renderer = match &self.gpu_renderer {
            Some(r) => r,
            None => return Err("GpuRenderer 未初始化".to_string()),
        };

        let surface = match &self.wgpu_surface {
            Some(s) => s,
            None => return Err("WebGPU surface 未初始化".to_string()),
        };

        // 获取尺寸
        let (width, height) = if let Some(config) = &self.wgpu_config {
            (config.width, config.height)
        } else {
            (self.width, self.height)
        };

        // 获取当前帧的 surface texture
        let output = match surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Surface 丢失或过期，使用 GpuRenderer 的设备重新配置
                if let Some(config) = &self.wgpu_config {
                    surface.configure(&gpu_renderer.device, config);
                }
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log_error("WebGPU: GPU 内存不足");
                return Err("GPU 内存不足".to_string());
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log_warn("WebGPU: 获取 surface texture 超时");
                return Ok(());
            }
            Err(wgpu::SurfaceError::Other) => {
                log_warn("WebGPU: 获取 surface texture 发生其他错误");
                return Ok(());
            }
        };

        // 使用明确的标签创建 TextureView
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("surface_view"),
            ..Default::default()
        });

        if let Some(gpu_renderer) = self.gpu_renderer_mut() {
            gpu_renderer.update_scale(width, height);
            gpu_renderer.render_frame_and_present(&view);
        }

        output.present();
        Ok(())
    }

    fn request_redraw(&self) {
        // Web 平台使用 requestAnimationFrame，不需要显式请求重绘
    }
}

// ============================================================================
// 音频后端 - Web Audio API (通过 JS 实现)
// ============================================================================

pub use super::audio::PlatformAudio as DesktopAudio;

// ============================================================================
// 输入后端 - 键盘事件处理
// ============================================================================

pub struct DesktopInput {
    key_states: HashMap<PlatformKeyCode, bool>,
    event_queue: Vec<PlatformKeyEvent>,
    should_close: bool,
}

impl DesktopInput {
    pub fn new() -> Self {
        Self {
            key_states: HashMap::new(),
            event_queue: Vec::new(),
            should_close: false,
        }
    }

    pub fn handle_keyboard_event(&mut self, event: &web_sys::KeyboardEvent, pressed: bool) {
        let key = web_key_to_platform(&event.code());

        // 更新按键状态
        self.key_states.insert(key, pressed);

        // 添加到事件队列
        self.event_queue.push(PlatformKeyEvent { key, pressed });

        // 阻止默认行为（如空格键滚动页面）
        event.prevent_default();
    }
}

impl InputBackend for DesktopInput {
    fn poll_events(&mut self) -> Vec<PlatformKeyEvent> {
        let events = self.event_queue.clone();
        self.event_queue.clear();
        events
    }

    fn is_key_pressed(&self, key: PlatformKeyCode) -> bool {
        *self.key_states.get(&key).unwrap_or(&false)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_close(&mut self) {
        self.should_close = true;
    }
}

// ============================================================================
// 键盘映射 - Web KeyboardEvent.code -> PlatformKeyCode
// ============================================================================

fn web_key_to_platform(code: &str) -> PlatformKeyCode {
    match code {
        // 方向键
        "ArrowLeft" => PlatformKeyCode::Left,
        "ArrowRight" => PlatformKeyCode::Right,
        "ArrowUp" => PlatformKeyCode::Up,
        "ArrowDown" => PlatformKeyCode::Down,

        // 动作键
        "Space" => PlatformKeyCode::Space,
        "AltLeft" => PlatformKeyCode::AltLeft,
        "AltRight" => PlatformKeyCode::AltRight,
        "ControlLeft" => PlatformKeyCode::ControlLeft,
        "ControlRight" => PlatformKeyCode::ControlRight,
        "ShiftLeft" => PlatformKeyCode::ShiftLeft,
        "ShiftRight" => PlatformKeyCode::ShiftRight,

        // 功能键
        "Escape" => PlatformKeyCode::Escape,
        "Enter" => PlatformKeyCode::Enter,
        "Tab" => PlatformKeyCode::Tab,
        "F1" => PlatformKeyCode::F1,
        "F2" => PlatformKeyCode::F2,
        "F11" => PlatformKeyCode::F11,
        "Backspace" => PlatformKeyCode::Backspace,

        // 字母键
        "KeyA" => PlatformKeyCode::KeyA,
        "KeyB" => PlatformKeyCode::KeyB,
        "KeyC" => PlatformKeyCode::KeyC,
        "KeyD" => PlatformKeyCode::KeyD,
        "KeyE" => PlatformKeyCode::KeyE,
        "KeyF" => PlatformKeyCode::KeyF,
        "KeyG" => PlatformKeyCode::KeyG,
        "KeyH" => PlatformKeyCode::KeyH,
        "KeyI" => PlatformKeyCode::KeyI,
        "KeyJ" => PlatformKeyCode::KeyJ,
        "KeyK" => PlatformKeyCode::KeyK,
        "KeyL" => PlatformKeyCode::KeyL,
        "KeyM" => PlatformKeyCode::KeyM,
        "KeyN" => PlatformKeyCode::KeyN,
        "KeyO" => PlatformKeyCode::KeyO,
        "KeyP" => PlatformKeyCode::KeyP,
        "KeyQ" => PlatformKeyCode::KeyQ,
        "KeyR" => PlatformKeyCode::KeyR,
        "KeyS" => PlatformKeyCode::KeyS,
        "KeyT" => PlatformKeyCode::KeyT,
        "KeyU" => PlatformKeyCode::KeyU,
        "KeyV" => PlatformKeyCode::KeyV,
        "KeyW" => PlatformKeyCode::KeyW,
        "KeyX" => PlatformKeyCode::KeyX,
        "KeyY" => PlatformKeyCode::KeyY,
        "KeyZ" => PlatformKeyCode::KeyZ,

        // 数字键
        "Digit0" => PlatformKeyCode::Digit0,
        "Digit1" => PlatformKeyCode::Digit1,
        "Digit2" => PlatformKeyCode::Digit2,
        "Digit3" => PlatformKeyCode::Digit3,
        "Digit4" => PlatformKeyCode::Digit4,
        "Digit5" => PlatformKeyCode::Digit5,
        "Digit6" => PlatformKeyCode::Digit6,
        "Digit7" => PlatformKeyCode::Digit7,
        "Digit8" => PlatformKeyCode::Digit8,
        "Digit9" => PlatformKeyCode::Digit9,

        _ => PlatformKeyCode::Unknown,
    }
}

// ============================================================================
// Web 时间后端 - 使用 web_sys::Performance
// ============================================================================

pub struct WebTime {
    start: f64,
}

impl WebTime {
    pub fn new() -> Self {
        let start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        Self { start }
    }
}

impl Default for WebTime {
    fn default() -> Self {
        Self::new()
    }
}

impl super::TimeBackend for WebTime {
    fn now_ms(&self) -> f64 {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0) - self.start
    }

    fn elapsed_ms(&self) -> f64 {
        self.now_ms()
    }
}

// 替换 CommonTime 为 WebTime
pub type DesktopTime = WebTime;

// ============================================================================
// 时间和随机数辅助函数
// ============================================================================

thread_local! {
    static TIME: WebTime = WebTime::new();
    static RAND: RefCell<CommonRandom> = RefCell::new(CommonRandom::new());
}

pub fn now_ms() -> f64 {
    TIME.with(|t| t.now_ms())
}

pub fn random_f32(max: f32) -> f32 {
    RAND.with(|r| r.borrow_mut().random_range_f32(max))
}

pub fn random_i32(max: i32) -> i32 {
    RAND.with(|r| r.borrow_mut().random_range(max))
}

pub fn random_u32(max: u32) -> u32 {
    random_i32(max as i32) as u32
}

pub fn random_u8(max: u8) -> u8 {
    random_i32(max as i32) as u8
}

pub fn random_usize(max: usize) -> usize {
    random_i32(max as i32) as usize
}

// ============================================================================
// Web 专用帧率控制
// ============================================================================

/// Web 专用帧率控制器
/// 使用 f64 毫秒而非 std::time::Duration，避免 WASM 时间 API 问题
struct FrameTimer {
    /// 帧间隔（毫秒）
    frame_duration_ms: f64,
    /// 下一帧时间点（毫秒）
    next_frame: f64,
}

impl FrameTimer {
    fn new(target_fps: f64) -> Self {
        let now = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        
        Self {
            frame_duration_ms: 1000.0 / target_fps,
            next_frame: now,
        }
    }

    fn should_render(&self) -> bool {
        let now = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        now >= self.next_frame
    }

    fn advance(&mut self) {
        let now = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        self.next_frame = now + self.frame_duration_ms;
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(60.0)
    }
}

/// Web 专用帧率统计器
struct FpsCounter {
    frame_count: u32,
    frame_time_accumulator: f32,
    last_update: f64,
    fps_display: u32,
    frame_time_display: f32,
}

impl FpsCounter {
    fn new() -> Self {
        let now = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        
        Self {
            frame_count: 0,
            frame_time_accumulator: 0.0,
            last_update: now,
            fps_display: 0,
            frame_time_display: 0.0,
        }
    }

    fn record_frame(&mut self, frame_time_ms: f32) {
        self.frame_count += 1;
        self.frame_time_accumulator += frame_time_ms;

        let now = window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        
        let elapsed = (now - self.last_update) / 1000.0;
        
        if elapsed >= 1.0 {
            self.fps_display = (self.frame_count as f32 / elapsed as f32) as u32;
            self.frame_time_display = self.frame_time_accumulator / self.frame_count as f32;
            self.frame_count = 0;
            self.frame_time_accumulator = 0.0;
            self.last_update = now;
        }
    }

    fn fps(&self) -> u32 {
        self.fps_display
    }

    fn frame_time_ms(&self) -> f32 {
        self.frame_time_display
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Web 游戏应用程序
// ============================================================================

use crate::game_runner::{GameState, GAME_HEIGHT, GAME_WIDTH};
use crate::platform::FrameResult;

/// Web 游戏应用状态
struct WebGameApp {
    display: DesktopDisplay,
    input: Rc<RefCell<DesktopInput>>,
    audio: DesktopAudio,
    game_state: Option<GameState>,
    frame_timer: FrameTimer,
    fps_counter: FpsCounter,
    running: bool,
}

impl WebGameApp {
    async fn new(canvas_id: &str) -> Result<Self, String> {
        let mut display = DesktopDisplay::new(canvas_id, GAME_WIDTH, GAME_HEIGHT)?;
        display.init_webgpu().await?;

        // 创建 WebAudio 后端实例（AudioContext 将在用户交互时解锁）
        let audio = DesktopAudio::new();

        Ok(Self {
            display,
            input: Rc::new(RefCell::new(DesktopInput::new())),
            audio,
            game_state: None,
            frame_timer: FrameTimer::new(60.0),
            fps_counter: FpsCounter::new(),
            running: true,
        })
    }

    fn init_game(&mut self) {
        log_info("初始化游戏状态...");
        let game_state = GameState::new();
        self.game_state = Some(game_state);
        log_info("游戏状态初始化完成");
    }

    fn frame_update(&mut self) {
        // 使用帧率控制器
        if !self.frame_timer.should_render() {
            return;
        }
        self.frame_timer.advance();

        let frame_start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        if let Some(state) = &mut self.game_state {
            // 设置 FPS 和渲染模式显示
            state.set_fps_display(self.fps_counter.fps(), self.fps_counter.frame_time_ms());
            state.set_render_mode(RenderMode::GPU);

            // 处理输入事件
            let events = self.input.borrow_mut().poll_events();
            for event in events {
                state.handle_key_event(&event);
            }

            // 更新游戏逻辑
            let result = state.frame_update();

            // 提交渲染数据到 GPU
            if let Some(gpu_renderer) = self.display.gpu_renderer_mut() {
                state.submit_to_gpu(gpu_renderer);
            }

            // 渲染到屏幕
            if let Err(e) = self.display.present() {
                log_error(&format!("渲染失败: {}", e));
            }

            if result == FrameResult::Exit {
                state.shutdown();
                self.running = false;
            }
        }

        // 计算帧时间
        if let Some(perf) = web_sys::window().and_then(|w| w.performance()) {
            let frame_time_ms = (perf.now() - frame_start) as f32;
            self.fps_counter.record_frame(frame_time_ms);
        }
    }

    fn is_running(&self) -> bool {
        self.running && !self.input.borrow().should_close()
    }
}

// ============================================================================
// Web 平台入口函数
// ============================================================================

/// 游戏主循环
fn game_loop(app: Rc<RefCell<WebGameApp>>) {
    let app_clone = app.clone();

    let closure = Closure::wrap(Box::new(move || {
        let mut app_ref = app_clone.borrow_mut();

        if app_ref.is_running() {
            app_ref.frame_update();
            // 继续请求下一帧
            drop(app_ref); // 释放借用
            game_loop(app_clone.clone());
        } else {
            log_info("游戏循环结束");
        }
    }) as Box<dyn FnMut()>);

    if let Some(window) = web_sys::window() {
        window
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .expect("无法请求动画帧");
    }

    closure.forget();
}

/// 设置键盘事件监听
fn setup_keyboard_listeners(app: Rc<RefCell<WebGameApp>>) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("无法获取 window")?;
    let document = window.document().ok_or("无法获取 document")?;

    // 克隆 input 引用以便在闭包中使用
    let input = app.borrow().input.clone();

    // 按键按下事件（在首次交互时尝试解锁音频）
    {
        let app_clone = app.clone();
        let input_clone = input.clone();
        let keydown = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            // 在用户交互时尝试恢复 AudioContext
            app_clone.borrow_mut().audio.resume();
            input_clone.borrow_mut().handle_keyboard_event(&event, true);
        }) as Box<dyn FnMut(_)>);

        document.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())?;
        keydown.forget();
    }

    // 按键释放事件
    {
        let input_clone = input.clone();
        let keyup = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            input_clone.borrow_mut().handle_keyboard_event(&event, false);
        }) as Box<dyn FnMut(_)>);

        document.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())?;
        keyup.forget();
    }

    Ok(())
}

/// WASM 启动入口 - 由 JavaScript 显式调用
#[wasm_bindgen]
pub fn run_game() {
    // 设置 panic hook，将 Rust panic 输出到浏览器控制台
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    log_info("Mario RS - Web 版本启动");

    wasm_bindgen_futures::spawn_local(async {
        match WebGameApp::new("mario-canvas").await {
            Ok(mut app) => {
                log_info("WebGPU 初始化成功");

                // 初始化游戏
                app.init_game();

                let app_rc = Rc::new(RefCell::new(app));

                // 设置键盘监听（传入 app Rc 以便解锁音频）
                if let Err(e) = setup_keyboard_listeners(app_rc.clone()) {
                    log_error(&format!("设置键盘监听失败: {:?}", e));
                    return;
                }

                log_info("开始游戏主循环");

                // 启动游戏循环
                game_loop(app_rc);
            }
            Err(e) => {
                log_error(&format!("初始化失败: {}", e));
            }
        }
    });
}

/// 提供给非 WASM 入口的包装函数
pub fn run_game_wrapper() -> Result<(), Box<dyn std::error::Error>> {
    // Web 平台的实际入口是上面的 run_game()，这个函数只是为了统一接口
    // 在 Web 平台上，这个函数不会被调用
    Ok(())
}

// ============================================================================
// JS 导出函数（可选）
// ============================================================================

/// 提供给 JS 调用的初始化函数
#[wasm_bindgen]
pub fn init_mario_game(canvas_id: &str) -> js_sys::Promise {
    let canvas_id = canvas_id.to_string();

    wasm_bindgen_futures::future_to_promise(async move {
        match WebGameApp::new(&canvas_id).await {
            Ok(mut app) => {
                app.init_game();
                let app_rc = Rc::new(RefCell::new(app));

                if let Err(e) = setup_keyboard_listeners(app_rc.clone()) {
                    return Err(JsValue::from_str(&format!(
                        "设置键盘监听失败: {:?}",
                        e
                    )));
                }

                game_loop(app_rc);
                Ok(JsValue::from_str("游戏初始化成功"))
            }
            Err(e) => Err(JsValue::from_str(&format!("初始化失败: {}", e))),
        }
    })
}