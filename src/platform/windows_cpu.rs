//! Windows 平台层 - CPU 软件渲染
//!
//! ## 渲染模式
//!
//! 使用纯CPU软件渲染，通过GDI StretchDIBits显示
//! 兼容Windows XP及以上版本
//!
//! ## 依赖
//! - windows-sys (窗口创建和事件处理)

#![allow(static_mut_refs)]

use super::audio::WaveOutAudio;
use super::common::{CommonTime, FileStorage, FpsCounter, FrameTimer};
use super::{DisplayBackend, FrameResult, InputBackend, KeyCode, KeyEvent, LogBackend, LogLevel};
use crate::cpu::CpuRenderer;
use crate::game_runner::{GAME_HEIGHT, GAME_WIDTH, GameState};

use std::mem::zeroed;
use std::ptr::null_mut;

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
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16, 'R' as u16, 'S' as u16, 'C' as u16,
    'P' as u16, 'U' as u16, 'C' as u16, 'l' as u16, 'a' as u16, 's' as u16, 's' as u16, 0,
];

const WINDOW_TITLE: &[u16] = &[
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16, ' ' as u16, '(' as u16, 'C' as u16,
    'P' as u16, 'U' as u16, ')' as u16, 0,
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
static mut CPU_RENDERER: Option<CpuRenderer> = None;
static mut WINDOW_WIDTH: u32 = INITIAL_WINDOW_WIDTH;
static mut WINDOW_HEIGHT: u32 = INITIAL_WINDOW_HEIGHT;

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
                    WINDOW_WIDTH = width;
                    WINDOW_HEIGHT = height;
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
// CPU 渲染器初始化
// ============================================================================

unsafe fn init_cpu_renderer() -> bool {
    unsafe {
        CPU_RENDERER = Some(CpuRenderer::new(GAME_WIDTH, GAME_HEIGHT));
    }
    println!("[CPU] CPU software renderer initialized");
    println!("[CPU] Framebuffer: {}x{} BGRA", GAME_WIDTH, GAME_HEIGHT);
    true
}

// ============================================================================
// GDI 渲染帧
// ============================================================================

unsafe fn render_frame(hwnd: HWND) {
    unsafe {
        let state = match GAME_STATE.as_mut() {
            Some(s) => s,
            None => return,
        };
        let cpu = match CPU_RENDERER.as_mut() {
            Some(c) => c,
            None => return,
        };

        // 提交渲染数据到CPU渲染器
        state.submit_to_cpu(cpu);

        // 使用GDI显示帧缓冲
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return;
        }

        // BITMAPINFOHEADER 结构（使用Win32命名约定）
        #[repr(C)]
        #[allow(non_snake_case)]
        struct BITMAPINFOHEADER {
            biSize: u32,
            biWidth: i32,
            biHeight: i32,
            biPlanes: u16,
            biBitCount: u16,
            biCompression: u32,
            biSizeImage: u32,
            biXPelsPerMeter: i32,
            biYPelsPerMeter: i32,
            biClrUsed: u32,
            biClrImportant: u32,
        }

        let bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: GAME_WIDTH as i32,
            biHeight: -(GAME_HEIGHT as i32), // 负数表示自上而下的位图
            biPlanes: 1,
            biBitCount: 32, // BGRA
            biCompression: 0, // BI_RGB
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        // 计算保持宽高比的缩放
        let win_w = WINDOW_WIDTH as i32;
        let win_h = WINDOW_HEIGHT as i32;
        let game_w = GAME_WIDTH as i32;
        let game_h = GAME_HEIGHT as i32;

        let scale_x = win_w as f64 / game_w as f64;
        let scale_y = win_h as f64 / game_h as f64;
        let scale = scale_x.min(scale_y);

        let dst_w = (game_w as f64 * scale) as i32;
        let dst_h = (game_h as f64 * scale) as i32;
        let dst_x = (win_w - dst_w) / 2;
        let dst_y = (win_h - dst_h) / 2;

        // 如果有黑边，先清空背景
        if dst_x > 0 || dst_y > 0 {
            let brush = GetStockObject(BLACK_BRUSH);
            let rect = RECT {
                left: 0,
                top: 0,
                right: win_w,
                bottom: win_h,
            };
            FillRect(hdc, &rect, brush);
        }

        // 设置缩放模式为HALFTONE（高质量）
        SetStretchBltMode(hdc, HALFTONE as i32);

        // 缩放绘制帧缓冲到窗口
        StretchDIBits(
            hdc,
            dst_x,
            dst_y,
            dst_w,
            dst_h,
            0,
            0,
            game_w,
            game_h,
            cpu.framebuffer().as_ptr() as *const _,
            &bmi as *const _ as *const BITMAPINFO,
            DIB_RGB_COLORS,
            SRCCOPY,
        );

        ReleaseDC(hwnd, hdc);
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

        let _ = ImmAssociateContext(hwnd, 0);

        if !init_cpu_renderer() {
            eprintln!("[CPU] CPU renderer initialization failed");
            return Err("CPU renderer initialization failed".into());
        }
        println!("[CPU] CPU software rendering mode enabled");

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        // 使用公共帧率控制器和FPS计数器
        let mut frame_timer = FrameTimer::new(60.0);
        let mut fps_counter = FpsCounter::new();
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

            let frame_start = std::time::Instant::now();

            if let Some(state) = GAME_STATE.as_mut() {
                // 设置FPS显示数据
                state.set_fps_display(fps_counter.fps(), fps_counter.frame_time_ms());

                let result = state.frame_update();

                if result == FrameResult::Exit {
                    state.shutdown();
                    RUNNING = false;
                    break;
                }
            }

            render_frame(hwnd);

            // 记录帧时间
            let frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            fps_counter.record_frame(frame_time_ms);

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
