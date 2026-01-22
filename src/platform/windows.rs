//! Windows 平台层 - wgpu GPU 加速渲染
//!
//! ## 渲染模式
//!
//! 使用 wgpu 进行 GPU 硬件加速渲染
//! - 帧渲染时间: 约1.5-3ms
//! - 性能提升: 相比CPU渲染提升3-5倍
//!
//! ## 依赖
//! - windows-sys (窗口创建和事件处理)
//! - wgpu, futures (GPU渲染)

#![allow(static_mut_refs)]

use super::audio::WaveOutAudio;
use super::common::{CommonTime, FileStorage, FrameTimer};
use super::{DisplayBackend, FrameResult, InputBackend, KeyCode, KeyEvent, LogBackend, LogLevel};
use crate::game_runner::{GAME_HEIGHT, GAME_WIDTH, GameState};
use crate::gpu::GpuRenderer;

use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::Arc;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ============================================================================
// 类型别名 - 使用公共模块
// ============================================================================

pub type DesktopTime = CommonTime;
pub type DesktopStorage = FileStorage;
pub type DesktopAudio = WaveOutAudio;

// ============================================================================
// Win32 外部函数
// ============================================================================

#[link(name = "Imm32")]
unsafe extern "system" {
    fn ImmAssociateContext(hWnd: HWND, hIMC: isize) -> isize;
}

#[cfg(feature = "dark-theme")]
#[link(name = "Dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwAttribute: u32,
        pvAttribute: *const std::ffi::c_void,
        cbAttribute: u32,
    ) -> i32;
}

#[link(name = "Advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hKey: isize,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        hKey: isize,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    fn RegCloseKey(hKey: isize) -> i32;
}

const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const KEY_READ: u32 = 0x20019;

#[cfg(feature = "dark-theme")]
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

#[cfg(feature = "dark-theme")]
#[repr(C)]
struct OSVERSIONINFOW {
    dw_os_version_info_size: u32,
    dw_major_version: u32,
    dw_minor_version: u32,
    dw_build_number: u32,
    dw_platform_id: u32,
    sz_csd_version: [u16; 128],
}

#[cfg(feature = "dark-theme")]
#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOW) -> i32;
}

#[cfg(feature = "dark-theme")]
fn is_windows_10_20h1_or_later() -> bool {
    unsafe {
        let mut os_info: OSVERSIONINFOW = std::mem::zeroed();
        os_info.dw_os_version_info_size = std::mem::size_of::<OSVERSIONINFOW>() as u32;

        if RtlGetVersion(&mut os_info) == 0 {
            if os_info.dw_major_version > 10 {
                return true;
            }
            if os_info.dw_major_version == 10 && os_info.dw_build_number >= 19041 {
                return true;
            }
        }
        false
    }
}

#[cfg(feature = "dark-theme")]
fn is_system_dark_mode() -> bool {
    unsafe {
        let subkey: Vec<u16> =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
                .encode_utf16()
                .collect();
        let value_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();

        let mut hkey: isize = 0;
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey);

        if result != 0 {
            return false;
        }

        let mut data: u32 = 1;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let mut value_type: u32 = 0;

        let _ = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );

        RegCloseKey(hkey);

        data == 0
    }
}

// ============================================================================
// 常量定义
// ============================================================================

const INITIAL_SCALE: u32 = 3;
const INITIAL_WINDOW_WIDTH: u32 = GAME_WIDTH * INITIAL_SCALE;
const INITIAL_WINDOW_HEIGHT: u32 = GAME_HEIGHT * INITIAL_SCALE;
const MIN_SCALE: u32 = 1;
const MAX_SCALE: u32 = 6;
const ASPECT_RATIO: f64 = GAME_WIDTH as f64 / GAME_HEIGHT as f64;

const CLASS_NAME: &[u16] = &[
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16, 'R' as u16, 'S' as u16, 'W' as u16,
    'i' as u16, 'n' as u16, 'd' as u16, 'o' as u16, 'w' as u16, 'C' as u16, 'l' as u16, 'a' as u16,
    's' as u16, 's' as u16, 0,
];

const WINDOW_TITLE: &[u16] = &[
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16, 0,
];

// ============================================================================
// 全局状态
// ============================================================================

static mut GAME_STATE: Option<GameState> = None;
static mut AUDIO: Option<DesktopAudio> = None;
static mut RUNNING: bool = true;
static mut IS_FULLSCREEN: bool = false;
static mut SAVED_WINDOW_STYLE: u32 = 0;
static mut SAVED_WINDOW_RECT: RECT = RECT {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};
static mut GPU_RENDERER: Option<GpuRenderer> = None;
static mut GPU_SURFACE: Option<wgpu::Surface<'static>> = None;
static mut GPU_SURFACE_CONFIG: Option<wgpu::SurfaceConfiguration> = None;

const VK_F11: u16 = 0x7A;

// ============================================================================
// 按键转换
// ============================================================================

fn vk_to_keycode(vk: u16) -> Option<KeyCode> {
    match vk {
        VK_LEFT => Some(KeyCode::Left),
        VK_RIGHT => Some(KeyCode::Right),
        VK_UP => Some(KeyCode::Up),
        VK_DOWN => Some(KeyCode::Down),
        VK_SPACE => Some(KeyCode::Space),
        VK_MENU => Some(KeyCode::AltLeft),
        VK_LMENU => Some(KeyCode::AltLeft),
        VK_RMENU => Some(KeyCode::AltRight),
        VK_CONTROL => Some(KeyCode::ControlLeft),
        VK_LCONTROL => Some(KeyCode::ControlLeft),
        VK_RCONTROL => Some(KeyCode::ControlRight),
        VK_SHIFT => Some(KeyCode::ShiftLeft),
        VK_LSHIFT => Some(KeyCode::ShiftLeft),
        VK_RSHIFT => Some(KeyCode::ShiftRight),
        VK_RETURN => Some(KeyCode::Enter),
        VK_ESCAPE => Some(KeyCode::Escape),
        VK_TAB => Some(KeyCode::Tab),
        0x50 => Some(KeyCode::KeyP),
        0x53 => Some(KeyCode::KeyS),
        0x46 => Some(KeyCode::KeyF),
        0x4A => Some(KeyCode::KeyJ),
        0x4C => Some(KeyCode::KeyL),
        0x57 => Some(KeyCode::KeyW),
        0x43 => Some(KeyCode::KeyC),
        0x4D => Some(KeyCode::KeyM),
        0x4F => Some(KeyCode::KeyO),
        0x31 => Some(KeyCode::Digit1),
        0x32 => Some(KeyCode::Digit2),
        0x33 => Some(KeyCode::Digit3),
        0x34 => Some(KeyCode::Digit4),
        0x35 => Some(KeyCode::Digit5),
        0x36 => Some(KeyCode::Digit6),
        0x37 => Some(KeyCode::Digit7),
        0x38 => Some(KeyCode::Digit8),
        _ => None,
    }
}

// ============================================================================
// 全屏切换
// ============================================================================

unsafe fn toggle_fullscreen(hwnd: HWND) {
    unsafe {
        if IS_FULLSCREEN {
            SetWindowLongW(hwnd, GWL_STYLE, SAVED_WINDOW_STYLE as i32);
            SetWindowPos(
                hwnd,
                HWND_TOP,
                SAVED_WINDOW_RECT.left,
                SAVED_WINDOW_RECT.top,
                SAVED_WINDOW_RECT.right - SAVED_WINDOW_RECT.left,
                SAVED_WINDOW_RECT.bottom - SAVED_WINDOW_RECT.top,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
            IS_FULLSCREEN = false;
        } else {
            SAVED_WINDOW_STYLE = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            GetWindowRect(hwnd, &mut SAVED_WINDOW_RECT);

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoW(monitor, &mut mi);

            let fullscreen_style = WS_POPUP | WS_VISIBLE;
            SetWindowLongW(hwnd, GWL_STYLE, fullscreen_style as i32);
            SetWindowPos(
                hwnd,
                HWND_TOP,
                mi.rcMonitor.left,
                mi.rcMonitor.top,
                mi.rcMonitor.right - mi.rcMonitor.left,
                mi.rcMonitor.bottom - mi.rcMonitor.top,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
            IS_FULLSCREEN = true;
        }
    }
}

// ============================================================================
// 窗口过程
// ============================================================================

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                if let Some(state) = GAME_STATE.as_mut() {
                    state.request_quit();
                    state.shutdown();
                }
                RUNNING = false;
                PostQuitMessage(0);
                0
            }

            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                0
            }

            WM_ERASEBKGND => 1,

            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = wparam as u16;

                if vk == VK_F11 {
                    toggle_fullscreen(hwnd);
                    return 0;
                }

                if vk == VK_ESCAPE as u16 && IS_FULLSCREEN {
                    toggle_fullscreen(hwnd);
                    return 0;
                }

                if let Some(keycode) = vk_to_keycode(vk) {
                    let event = KeyEvent {
                        key: keycode,
                        pressed: true,
                    };
                    if let Some(state) = GAME_STATE.as_mut() {
                        state.handle_key_event(&event);
                    }
                }
                0
            }

            WM_KEYUP | WM_SYSKEYUP => {
                let vk = wparam as u16;
                if let Some(keycode) = vk_to_keycode(vk) {
                    let event = KeyEvent {
                        key: keycode,
                        pressed: false,
                    };
                    if let Some(state) = GAME_STATE.as_mut() {
                        state.handle_key_event(&event);
                    }
                }
                0
            }

            WM_PAINT => {
                let mut ps: PAINTSTRUCT = zeroed();
                BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
                0
            }

            WM_SIZE => {
                let width = (lparam & 0xFFFF) as u32;
                let height = ((lparam >> 16) & 0xFFFF) as u32;
                if width > 0 && height > 0 {
                    if let Some(gpu) = GPU_RENDERER.as_ref() {
                        if let Some(surface) = GPU_SURFACE.as_ref() {
                            if let Some(config) = GPU_SURFACE_CONFIG.as_mut() {
                                config.width = width;
                                config.height = height;
                                surface.configure(gpu.device.as_ref(), config);
                                gpu.update_scale(width, height);
                            }
                        }
                    }
                }
                0
            }

            WM_SIZING => {
                let rect = &mut *(lparam as *mut RECT);
                let edge = wparam as u32;

                let mut style_rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                AdjustWindowRect(&mut style_rect, style, 0);
                let border_width = (style_rect.right - style_rect.left) as i32;
                let border_height = (style_rect.bottom - style_rect.top) as i32;

                let width = (rect.right - rect.left - border_width) as f64;
                let height = (rect.bottom - rect.top - border_height) as f64;

                let (new_width, new_height) = match edge {
                    WMSZ_LEFT | WMSZ_RIGHT => {
                        let h = (width / ASPECT_RATIO).round() as i32;
                        (width as i32, h)
                    }
                    WMSZ_TOP | WMSZ_BOTTOM => {
                        let w = (height * ASPECT_RATIO).round() as i32;
                        (w, height as i32)
                    }
                    _ => {
                        let aspect = width / height;
                        if aspect > ASPECT_RATIO {
                            let h = (width / ASPECT_RATIO).round() as i32;
                            (width as i32, h)
                        } else {
                            let w = (height * ASPECT_RATIO).round() as i32;
                            (w, height as i32)
                        }
                    }
                };

                match edge {
                    WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT => {
                        rect.left = rect.right - new_width - border_width;
                    }
                    _ => {
                        rect.right = rect.left + new_width + border_width;
                    }
                }
                match edge {
                    WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT => {
                        rect.top = rect.bottom - new_height - border_height;
                    }
                    _ => {
                        rect.bottom = rect.top + new_height + border_height;
                    }
                }

                1
            }

            WM_GETMINMAXINFO => {
                let mmi = &mut *(lparam as *mut MINMAXINFO);

                let mut style_rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                let style = WS_OVERLAPPEDWINDOW;
                AdjustWindowRect(&mut style_rect, style, 0);
                let border_width = (style_rect.right - style_rect.left) as i32;
                let border_height = (style_rect.bottom - style_rect.top) as i32;

                mmi.ptMinTrackSize.x = (GAME_WIDTH * MIN_SCALE) as i32 + border_width;
                mmi.ptMinTrackSize.y = (GAME_HEIGHT * MIN_SCALE) as i32 + border_height;
                mmi.ptMaxTrackSize.x = (GAME_WIDTH * MAX_SCALE) as i32 + border_width;
                mmi.ptMaxTrackSize.y = (GAME_HEIGHT * MAX_SCALE) as i32 + border_height;

                0
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ============================================================================
// GPU 初始化
// ============================================================================

unsafe fn init_gpu_renderer(hwnd: HWND) -> bool {
    use std::num::NonZeroIsize;
    use wgpu::rwh::{
        HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle,
        WindowsDisplayHandle,
    };

    let hinstance = unsafe { GetModuleHandleW(null_mut()) };

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    struct WinHandle {
        hwnd: isize,
        hinstance: isize,
    }
    impl HasWindowHandle for WinHandle {
        fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
            let mut handle = Win32WindowHandle::new(NonZeroIsize::new(self.hwnd).unwrap());
            handle.hinstance = NonZeroIsize::new(self.hinstance);
            let raw = RawWindowHandle::Win32(handle);
            Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(raw) })
        }
    }
    impl HasDisplayHandle for WinHandle {
        fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
            let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
            Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(raw) })
        }
    }

    let win_handle = WinHandle {
        hwnd: hwnd as isize,
        hinstance: hinstance as isize,
    };

    let surface = match instance.create_surface(&win_handle) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[GPU] Cannot create Surface: {:?}", e);
            return false;
        }
    };

    let adapter =
        match futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[GPU] Cannot get GPU adapter: {:?}", e);
                return false;
            }
        };

    let (device, queue) =
        match futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Mario GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        })) {
            Ok((d, q)) => (d, q),
            Err(e) => {
                eprintln!("[GPU] Cannot create GPU device: {:?}", e);
                return false;
            }
        };

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: INITIAL_WINDOW_WIDTH,
        height: INITIAL_WINDOW_HEIGHT,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let gpu_renderer = GpuRenderer::new(device, queue, config.format);
    gpu_renderer.update_scale(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT);

    unsafe {
        GPU_RENDERER = Some(gpu_renderer);
        GPU_SURFACE = Some(std::mem::transmute(surface));
        GPU_SURFACE_CONFIG = Some(config);
    }

    let adapter_info = adapter.get_info();
    println!("[GPU] Backend: {:?}", adapter_info.backend);
    println!(
        "[GPU] Device: {} ({:?})",
        adapter_info.name, adapter_info.device_type
    );
    println!(
        "[GPU] GPU renderer initialized (Surface: {:?})",
        surface_format
    );
    true
}

unsafe fn render_frame(_hwnd: HWND) {
    unsafe {
        let state = match GAME_STATE.as_ref() {
            Some(s) => s,
            None => return,
        };
        let gpu = match GPU_RENDERER.as_mut() {
            Some(g) => g,
            None => return,
        };
        let surface = match GPU_SURFACE.as_ref() {
            Some(s) => s,
            None => return,
        };

        state.submit_to_gpu(gpu);

        match surface.get_current_texture() {
            Ok(output) => {
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| {
                    println!(
                        "[GPU] Surface texture acquired: {:?}",
                        output.texture.format()
                    );
                });

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                gpu.render_to_surface(&view);
                output.present();
            }
            Err(wgpu::SurfaceError::Lost) => {
                if let Some(config) = GPU_SURFACE_CONFIG.as_ref() {
                    surface.configure(gpu.device.as_ref(), config);
                }
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!("[GPU] Out of GPU memory");
            }
            Err(e) => {
                eprintln!("[GPU] Surface error: {:?}", e);
            }
        }
    }
}

// ============================================================================
// 游戏入口
// ============================================================================

pub fn run_game() -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        #[cfg(debug_assertions)]
        crate::game_runner::print_startup_info();

        GAME_STATE = Some(GameState::new());
        AUDIO = Some(DesktopAudio::new());

        let hinstance = GetModuleHandleW(null_mut());

        let icon_id = 1 as *const u16;

        let hicon = LoadImageW(hinstance, icon_id, IMAGE_ICON, 32, 32, 0) as HICON;
        let hicon_sm = LoadImageW(hinstance, icon_id, IMAGE_ICON, 16, 16, 0) as HICON;

        let hicon = if hicon.is_null() {
            LoadIconW(null_mut(), IDI_APPLICATION)
        } else {
            hicon
        };
        let hicon_sm = if hicon_sm.is_null() { hicon } else { hicon_sm };

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: hicon,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(BLACK_BRUSH),
            lpszMenuName: null_mut(),
            lpszClassName: CLASS_NAME.as_ptr(),
            hIconSm: hicon_sm,
        };

        RegisterClassExW(&wc);

        let style = WS_OVERLAPPEDWINDOW;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: INITIAL_WINDOW_WIDTH as i32,
            bottom: INITIAL_WINDOW_HEIGHT as i32,
        };
        AdjustWindowRect(&mut rect, style, 0);

        let window_width = rect.right - rect.left;
        let window_height = rect.bottom - rect.top;

        let hwnd = CreateWindowExW(
            0,
            CLASS_NAME.as_ptr(),
            WINDOW_TITLE.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            window_width,
            window_height,
            null_mut(),
            null_mut(),
            hinstance,
            null_mut(),
        );

        if hwnd.is_null() {
            return Err("Failed to create window".into());
        }

        #[cfg(feature = "dark-theme")]
        if is_windows_10_20h1_or_later() {
            let dark_mode: u32 = if is_system_dark_mode() { 1 } else { 0 };
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }

        let _ = ImmAssociateContext(hwnd, 0);

        if !init_gpu_renderer(hwnd) {
            eprintln!("[GPU] GPU initialization failed");
            return Err("GPU initialization failed".into());
        }
        println!("[GPU] GPU rendering mode enabled");

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        // 使用公共帧率控制器
        let mut frame_timer = FrameTimer::new(60.0);
        let mut msg: MSG = zeroed();

        while RUNNING {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    RUNNING = false;
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if !RUNNING {
                break;
            }

            if let Some(state) = GAME_STATE.as_mut() {
                let result = state.frame_update();

                if result == FrameResult::Exit {
                    state.shutdown();
                    RUNNING = false;
                    break;
                }
            }

            render_frame(hwnd);

            // 使用公共帧率控制
            frame_timer.wait_if_needed();
            frame_timer.advance();
        }

        Ok(())
    }
}

// ============================================================================
// 全局便捷函数 - 使用公共模块
// ============================================================================

pub use super::common::{now_ms, random_f32, random_i32, random_u8, random_u32, random_usize};

pub fn log_debug(_msg: &str) {}
pub fn log_info(_msg: &str) {}
pub fn log_warn(_msg: &str) {}
pub fn log_error(msg: &str) {
    eprintln!("{}", msg);
}

// ============================================================================
// 类型别名 (与其他平台兼容)
// ============================================================================

pub struct DesktopDisplay {
    width: u32,
    height: u32,
}

impl DesktopDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl DisplayBackend for DesktopDisplay {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn present(&mut self) -> std::result::Result<(), String> {
        Ok(())
    }
    fn request_redraw(&self) {}
}

pub struct DesktopInput {
    close_requested: bool,
}

impl DesktopInput {
    pub fn new() -> Self {
        Self {
            close_requested: false,
        }
    }
}

impl Default for DesktopInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBackend for DesktopInput {
    fn poll_events(&mut self) -> Vec<crate::platform::KeyEvent> {
        Vec::new()
    }
    fn is_key_pressed(&self, _key: KeyCode) -> bool {
        false
    }
    fn should_close(&self) -> bool {
        self.close_requested
    }
    fn request_close(&mut self) {
        self.close_requested = true;
    }
}

pub struct DesktopRandom;

impl super::RandomBackend for DesktopRandom {
    fn random_range(&mut self, max: i32) -> i32 {
        random_i32(max)
    }
    fn random_range_f32(&mut self, max: f32) -> f32 {
        random_f32(max)
    }
    fn random_f32(&mut self) -> f32 {
        random_f32(1.0)
    }
}

pub struct DesktopLog;

impl LogBackend for DesktopLog {
    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Error => eprintln!("[ERROR] {}", message),
            LogLevel::Warn => eprintln!("[WARN] {}", message),
            _ => {}
        }
    }
}
