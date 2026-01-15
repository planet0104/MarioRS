//! Windows 平台层 - 使用 Win32 GDI 原生渲染
//!
//! 优点：体积极小（~300KB），无需 wgpu 依赖
//! 缺点：仅支持 Windows

// 允许对可变静态变量的引用（单线程游戏，安全）
#![allow(static_mut_refs)]

// 使用新的音频模块路径
use super::audio::WaveOutAudio;
pub type DesktopAudio = WaveOutAudio;

use crate::game_runner::{GameState, GAME_HEIGHT, GAME_WIDTH};
use super::{
    AudioBackend, DisplayBackend, InputBackend, LogBackend, LogLevel,
    RandomBackend, StorageBackend, TimeBackend,
    FrameResult, KeyCode, KeyEvent,
};

use std::mem::zeroed;
use std::ptr::null_mut;

// 使用 windows-sys 替代 windows crate，减小体积
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// IME context disassociation to prevent input method interception
#[link(name = "Imm32")]
unsafe extern "system" {
    fn ImmAssociateContext(hWnd: HWND, hIMC: isize) -> isize;
}

// DWM API for dark mode title bar (Vista+, 需要 dark-theme feature)
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

// Registry API for detecting system dark mode
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

// Registry constants
const HKEY_CURRENT_USER: isize = -2147483647i32 as isize; // 0x80000001
const KEY_READ: u32 = 0x20019;

// DWMWA_USE_IMMERSIVE_DARK_MODE (Windows 10 20H1+, Windows 11)
#[cfg(feature = "dark-theme")]
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

// Windows 版本信息结构体
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

// RtlGetVersion - 获取真实的 Windows 版本（不受兼容性清单影响）
#[cfg(feature = "dark-theme")]
#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOW) -> i32;
}

/// 检测是否为 Windows 10 20H1+ 或 Windows 11
/// DWMWA_USE_IMMERSIVE_DARK_MODE 只在 Windows 10 Build 19041+ 才支持
#[cfg(feature = "dark-theme")]
fn is_windows_10_20h1_or_later() -> bool {
    unsafe {
        let mut os_info: OSVERSIONINFOW = std::mem::zeroed();
        os_info.dw_os_version_info_size = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        
        if RtlGetVersion(&mut os_info) == 0 {
            // Windows 10 = 10.0, Windows 11 = 10.0 (build 22000+)
            // 20H1 = Build 19041
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

/// 检测系统是否启用了暗色模式
#[cfg(feature = "dark-theme")]
fn is_system_dark_mode() -> bool {
    unsafe {
        // 注册表路径: HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize
        // 值名称: AppsUseLightTheme (0 = 暗色, 1 = 亮色)
        let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let value_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        
        let mut hkey: isize = 0;
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        
        if result != 0 {
            return false; // 无法打开注册表，默认亮色
        }
        
        let mut data: u32 = 1; // 默认亮色
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
        
        // AppsUseLightTheme: 0 = 暗色, 1 = 亮色
        data == 0
    }
}

// 窗口初始缩放倍数
const INITIAL_SCALE: u32 = 3;
const INITIAL_WINDOW_WIDTH: u32 = GAME_WIDTH * INITIAL_SCALE;
const INITIAL_WINDOW_HEIGHT: u32 = GAME_HEIGHT * INITIAL_SCALE;

// 最小/最大缩放倍数
const MIN_SCALE: u32 = 1;
const MAX_SCALE: u32 = 6;

// 游戏宽高比
const ASPECT_RATIO: f64 = GAME_WIDTH as f64 / GAME_HEIGHT as f64;

// 窗口类名 (UTF-16 编码的字符串)
const CLASS_NAME: &[u16] = &[
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16,
    'R' as u16, 'S' as u16, 'W' as u16, 'i' as u16, 'n' as u16,
    'd' as u16, 'o' as u16, 'w' as u16, 'C' as u16, 'l' as u16,
    'a' as u16, 's' as u16, 's' as u16, 0,
];

// 窗口标题 (UTF-16 编码)
const WINDOW_TITLE: &[u16] = &[
    'M' as u16, 'a' as u16, 'r' as u16, 'i' as u16, 'o' as u16, 0,
];

// 全局状态（Win32 回调需要）
static mut GAME_STATE: Option<GameState> = None;
static mut AUDIO: Option<DesktopAudio> = None;
static mut RUNNING: bool = true;
static mut RGBA_BUFFER: Vec<u8> = Vec::new();

// 当前窗口客户区尺寸（用于动态缩放）
static mut CURRENT_WIDTH: u32 = INITIAL_WINDOW_WIDTH;
static mut CURRENT_HEIGHT: u32 = INITIAL_WINDOW_HEIGHT;

// 全屏状态
static mut IS_FULLSCREEN: bool = false;
static mut SAVED_WINDOW_STYLE: u32 = 0;
static mut SAVED_WINDOW_RECT: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };

// 全屏渲染偏移（用于居中显示）
static mut RENDER_OFFSET_X: i32 = 0;
static mut RENDER_OFFSET_Y: i32 = 0;
static mut FULLSCREEN_WIDTH: u32 = 0;
static mut FULLSCREEN_HEIGHT: u32 = 0;

// 双缓冲相关
static mut BACK_BUFFER_DC: HDC = null_mut();
static mut BACK_BUFFER_BITMAP: HBITMAP = null_mut();
static mut BACK_BUFFER_OLD_BITMAP: HGDIOBJ = null_mut();
static mut BACK_BUFFER_WIDTH: i32 = 0;
static mut BACK_BUFFER_HEIGHT: i32 = 0;

// F11 虚拟键码
const VK_F11: u16 = 0x7A;

/// 将 Win32 虚拟键码转换为平台无关的 KeyCode
fn vk_to_keycode(vk: u16) -> Option<KeyCode> {
    match vk {
        VK_LEFT => Some(KeyCode::Left),
        VK_RIGHT => Some(KeyCode::Right),
        VK_UP => Some(KeyCode::Up),
        VK_DOWN => Some(KeyCode::Down),
        VK_SPACE => Some(KeyCode::Space),
        VK_MENU => Some(KeyCode::AltLeft),      // Alt 键
        VK_LMENU => Some(KeyCode::AltLeft),
        VK_RMENU => Some(KeyCode::AltRight),
        VK_CONTROL => Some(KeyCode::ControlLeft), // Control 键 (通用)
        VK_LCONTROL => Some(KeyCode::ControlLeft),
        VK_RCONTROL => Some(KeyCode::ControlRight),
        VK_SHIFT => Some(KeyCode::ShiftLeft), // Shift 键 (通用)
        VK_LSHIFT => Some(KeyCode::ShiftLeft),
        VK_RSHIFT => Some(KeyCode::ShiftRight),
        VK_RETURN => Some(KeyCode::Enter),
        VK_ESCAPE => Some(KeyCode::Escape),
        VK_TAB => Some(KeyCode::Tab),
        0x50 => Some(KeyCode::KeyP),  // 'P'
        0x53 => Some(KeyCode::KeyS),  // 'S'
        0x46 => Some(KeyCode::KeyF),  // 'F'
        0x4A => Some(KeyCode::KeyJ),  // 'J'
        0x4C => Some(KeyCode::KeyL),  // 'L'
        0x57 => Some(KeyCode::KeyW),  // 'W'
        0x43 => Some(KeyCode::KeyC),  // 'C'
        0x4D => Some(KeyCode::KeyM),  // 'M'
        0x4F => Some(KeyCode::KeyO),  // 'O'
        0x31 => Some(KeyCode::Digit1), // '1'
        0x32 => Some(KeyCode::Digit2), // '2'
        0x33 => Some(KeyCode::Digit3), // '3'
        0x34 => Some(KeyCode::Digit4), // '4'
        0x35 => Some(KeyCode::Digit5), // '5'
        0x36 => Some(KeyCode::Digit6), // '6'
        0x37 => Some(KeyCode::Digit7), // '7'
        0x38 => Some(KeyCode::Digit8), // '8'
        _ => None,
    }
}

/// 切换全屏/窗口模式
unsafe fn toggle_fullscreen(hwnd: HWND) {
    unsafe {
        if IS_FULLSCREEN {
            // 退出全屏：恢复窗口样式和位置
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
            
            // 重置渲染偏移
            RENDER_OFFSET_X = 0;
            RENDER_OFFSET_Y = 0;
            FULLSCREEN_WIDTH = 0;
            FULLSCREEN_HEIGHT = 0;
            
            IS_FULLSCREEN = false;
        } else {
            // 进入全屏：保存当前窗口状态
            SAVED_WINDOW_STYLE = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            GetWindowRect(hwnd, &mut SAVED_WINDOW_RECT);
            
            // 获取当前显示器信息
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoW(monitor, &mut mi);
            
            // 设置无边框样式
            let fullscreen_style = WS_POPUP | WS_VISIBLE;
            SetWindowLongW(hwnd, GWL_STYLE, fullscreen_style as i32);
            
            // 设置窗口大小为显示器大小
            let screen_w = (mi.rcMonitor.right - mi.rcMonitor.left) as u32;
            let screen_h = (mi.rcMonitor.bottom - mi.rcMonitor.top) as u32;
            
            SetWindowPos(
                hwnd,
                HWND_TOP,
                mi.rcMonitor.left,
                mi.rcMonitor.top,
                screen_w as i32,
                screen_h as i32,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
            
            // 计算保持宽高比的渲染区域
            let scale_w = screen_w as f64 / GAME_WIDTH as f64;
            let scale_h = screen_h as f64 / GAME_HEIGHT as f64;
            let scale = scale_w.min(scale_h);
            
            CURRENT_WIDTH = (GAME_WIDTH as f64 * scale) as u32;
            CURRENT_HEIGHT = (GAME_HEIGHT as f64 * scale) as u32;
            
            // 计算居中偏移
            RENDER_OFFSET_X = ((screen_w - CURRENT_WIDTH) / 2) as i32;
            RENDER_OFFSET_Y = ((screen_h - CURRENT_HEIGHT) / 2) as i32;
            FULLSCREEN_WIDTH = screen_w;
            FULLSCREEN_HEIGHT = screen_h;
            
            IS_FULLSCREEN = true;
        }
    }
}

/// Win32 窗口过程
unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                // 窗口销毁时退出消息循环
                if let Some(state) = GAME_STATE.as_mut() {
                    state.request_quit();
                    state.shutdown();
                }
                
                // 清理双缓冲资源
                if !BACK_BUFFER_DC.is_null() {
                    if !BACK_BUFFER_OLD_BITMAP.is_null() {
                        SelectObject(BACK_BUFFER_DC, BACK_BUFFER_OLD_BITMAP);
                    }
                    if !BACK_BUFFER_BITMAP.is_null() {
                        DeleteObject(BACK_BUFFER_BITMAP);
                    }
                    DeleteDC(BACK_BUFFER_DC);
                    BACK_BUFFER_DC = null_mut();
                    BACK_BUFFER_BITMAP = null_mut();
                    BACK_BUFFER_OLD_BITMAP = null_mut();
                }
                
                RUNNING = false;
                PostQuitMessage(0);
                0
            }
            
            WM_CLOSE => {
                // 关闭窗口
                let _ = DestroyWindow(hwnd);
                0
            }
            
            WM_ERASEBKGND => {
                // 返回非零值表示已处理，阻止系统擦除背景
                // 这是防止闪烁的关键 - 我们在 WM_PAINT 中使用双缓冲自己处理
                1
            }
            
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = wparam as u16;
                
                // F11 切换全屏
                if vk == VK_F11 {
                    toggle_fullscreen(hwnd);
                    return 0;
                }
                
                // ESC 退出全屏（全屏模式下）
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
                // 处理重绘请求 - 使用双缓冲防止闪烁
                let mut ps: PAINTSTRUCT = zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                
                // 获取窗口客户区尺寸
                let mut client_rect: RECT = zeroed();
                GetClientRect(hwnd, &mut client_rect);
                let client_width = client_rect.right - client_rect.left;
                let client_height = client_rect.bottom - client_rect.top;
                
                // 检查是否需要重新创建后台缓冲区
                if BACK_BUFFER_DC.is_null() || 
                   BACK_BUFFER_WIDTH != client_width || 
                   BACK_BUFFER_HEIGHT != client_height {
                    // 清理旧的后台缓冲区
                    if !BACK_BUFFER_DC.is_null() {
                        if !BACK_BUFFER_OLD_BITMAP.is_null() {
                            SelectObject(BACK_BUFFER_DC, BACK_BUFFER_OLD_BITMAP);
                        }
                        if !BACK_BUFFER_BITMAP.is_null() {
                            DeleteObject(BACK_BUFFER_BITMAP);
                        }
                        DeleteDC(BACK_BUFFER_DC);
                    }
                    
                    // 创建新的后台缓冲区
                    BACK_BUFFER_DC = CreateCompatibleDC(hdc);
                    BACK_BUFFER_BITMAP = CreateCompatibleBitmap(hdc, client_width, client_height);
                    BACK_BUFFER_OLD_BITMAP = SelectObject(BACK_BUFFER_DC, BACK_BUFFER_BITMAP);
                    BACK_BUFFER_WIDTH = client_width;
                    BACK_BUFFER_HEIGHT = client_height;
                }
                
                // 在后台缓冲区绘制
                render_frame(hwnd, BACK_BUFFER_DC, client_width, client_height);
                
                // 一次性将后台缓冲区复制到前台（原子操作，无闪烁）
                BitBlt(
                    hdc,
                    0, 0,
                    client_width, client_height,
                    BACK_BUFFER_DC,
                    0, 0,
                    SRCCOPY,
                );
                
                EndPaint(hwnd, &ps);
                0
            }
            
            WM_SIZE => {
                // 窗口大小改变时更新当前尺寸
                let new_width = (lparam & 0xFFFF) as u32;
                let new_height = ((lparam >> 16) & 0xFFFF) as u32;
                if new_width > 0 && new_height > 0 {
                    CURRENT_WIDTH = new_width;
                    CURRENT_HEIGHT = new_height;
                }
                0
            }
            
            WM_SIZING => {
                // 强制保持等比例缩放
                let rect = &mut *(lparam as *mut RECT);
                let edge = wparam as u32;
                
                // 计算当前拖拽的矩形尺寸（不含边框）
                let mut style_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                AdjustWindowRect(&mut style_rect, style, 0);
                let border_width = (style_rect.right - style_rect.left) as i32;
                let border_height = (style_rect.bottom - style_rect.top) as i32;
                
                let width = (rect.right - rect.left - border_width) as f64;
                let height = (rect.bottom - rect.top - border_height) as f64;
                
                // 根据拖拽边缘决定以宽度还是高度为基准
                let (new_width, new_height) = match edge {
                    WMSZ_LEFT | WMSZ_RIGHT => {
                        // 水平拖拽，以宽度为基准
                        let h = (width / ASPECT_RATIO).round() as i32;
                        (width as i32, h)
                    }
                    WMSZ_TOP | WMSZ_BOTTOM => {
                        // 垂直拖拽，以高度为基准
                        let w = (height * ASPECT_RATIO).round() as i32;
                        (w, height as i32)
                    }
                    _ => {
                        // 对角拖拽，选择变化更大的维度
                        let aspect = width / height;
                        if aspect > ASPECT_RATIO {
                            // 宽度变化更大，以宽度为基准
                            let h = (width / ASPECT_RATIO).round() as i32;
                            (width as i32, h)
                        } else {
                            // 高度变化更大，以高度为基准
                            let w = (height * ASPECT_RATIO).round() as i32;
                            (w, height as i32)
                        }
                    }
                };
                
                // 应用新尺寸（加上边框）
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
                
                1 // 返回 TRUE 表示已处理
            }
            
            WM_GETMINMAXINFO => {
                // 限制窗口最小/最大尺寸
                let mmi = &mut *(lparam as *mut MINMAXINFO);
                
                // 计算边框尺寸
                let mut style_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                let style = WS_OVERLAPPEDWINDOW;
                AdjustWindowRect(&mut style_rect, style, 0);
                let border_width = (style_rect.right - style_rect.left) as i32;
                let border_height = (style_rect.bottom - style_rect.top) as i32;
                
                // 最小尺寸
                mmi.ptMinTrackSize.x = (GAME_WIDTH * MIN_SCALE) as i32 + border_width;
                mmi.ptMinTrackSize.y = (GAME_HEIGHT * MIN_SCALE) as i32 + border_height;
                
                // 最大尺寸
                mmi.ptMaxTrackSize.x = (GAME_WIDTH * MAX_SCALE) as i32 + border_width;
                mmi.ptMaxTrackSize.y = (GAME_HEIGHT * MAX_SCALE) as i32 + border_height;
                
                0
            }
            
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// 渲染一帧到后台缓冲区（支持动态缩放和全屏居中）
/// 使用双缓冲，所有绘制操作在内存 DC 中完成，避免闪烁
unsafe fn render_frame(_hwnd: HWND, hdc: HDC, buffer_width: i32, buffer_height: i32) {
    unsafe {
        // 先用黑色填充整个后台缓冲区（处理黑边和背景）
        // 因为是在后台缓冲区操作，用户看不到这个过程
        let brush = GetStockObject(BLACK_BRUSH);
        let rect = RECT {
            left: 0,
            top: 0,
            right: buffer_width,
            bottom: buffer_height,
        };
        FillRect(hdc, &rect, brush);
        
        if let Some(state) = GAME_STATE.as_ref() {
            // 确保缓冲区大小正确
            let buffer_size = (GAME_WIDTH * GAME_HEIGHT * 4) as usize;
            if RGBA_BUFFER.len() != buffer_size {
                RGBA_BUFFER.resize(buffer_size, 0);
            }
            
            // 渲染游戏到 RGBA 缓冲区
            state.render_to_rgba(&mut RGBA_BUFFER);
            
            // 创建 DIB（设备无关位图）信息
            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = GAME_WIDTH as i32;
            bmi.bmiHeader.biHeight = -(GAME_HEIGHT as i32); // 负值表示从上到下
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;
            
            // 将 RGBA 转换为 BGRA（Win32 GDI 格式）
            for i in (0..RGBA_BUFFER.len()).step_by(4) {
                RGBA_BUFFER.swap(i, i + 2); // R <-> B
            }
            
            // 设置拉伸模式（像素游戏使用 COLORONCOLOR 保持锐利）
            SetStretchBltMode(hdc, COLORONCOLOR as i32);
            
            // 使用 StretchDIBits 绘制并缩放到当前窗口尺寸
            // 全屏模式使用偏移居中显示
            StretchDIBits(
                hdc,
                RENDER_OFFSET_X, RENDER_OFFSET_Y,
                CURRENT_WIDTH as i32,
                CURRENT_HEIGHT as i32,
                0, 0,
                GAME_WIDTH as i32,
                GAME_HEIGHT as i32,
                RGBA_BUFFER.as_ptr() as *const _,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }
}

/// 运行游戏（平台入口函数）
pub fn run_game() -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // 打印启动信息（仅 debug 模式，release 无控制台）
        #[cfg(debug_assertions)]
        crate::game_runner::print_startup_info();
        
        // 初始化游戏状态和音频
        GAME_STATE = Some(GameState::new());
        // Construct the platform-appropriate audio backend
        AUDIO = Some(DesktopAudio::new());
        
        // 获取模块句柄
        let hinstance = GetModuleHandleW(null_mut());
        
        // 从资源加载图标（资源 ID = 1，在 mario.rc 中定义）
        // MAKEINTRESOURCEW(1) = 1 as *const u16
        let icon_id = 1 as *const u16;
        
        // 使用 LoadImageW 加载图标，指定具体尺寸
        // LR_DEFAULTCOLOR (0) 比 LR_SHARED 在 XP 上更兼容
        let hicon = LoadImageW(
            hinstance,
            icon_id,
            IMAGE_ICON,
            32, 32,  // 大图标尺寸
            0,       // LR_DEFAULTCOLOR
        ) as HICON;
        
        let hicon_sm = LoadImageW(
            hinstance,
            icon_id,
            IMAGE_ICON,
            16, 16,  // 小图标尺寸
            0,       // LR_DEFAULTCOLOR
        ) as HICON;
        
        // 如果从资源加载失败，使用系统默认图标
        let hicon = if hicon.is_null() {
            LoadIconW(null_mut(), IDI_APPLICATION)
        } else {
            hicon
        };
        let hicon_sm = if hicon_sm.is_null() { hicon } else { hicon_sm };
        
        // 注册窗口类
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
        
        // 计算窗口大小（包含边框）
        // WS_OVERLAPPEDWINDOW 包含 WS_THICKFRAME 允许调整大小
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
        
        // 创建窗口
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
        
        // 根据系统设置启用暗色标题栏
        // 只在 Windows 10 20H1+ 和 Windows 11 上执行（需要 dark-theme feature）
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
        
        // Disable IME for this window so input methods (e.g. Ctrl+Space) don't intercept keys
        // This disassociates the IME context from our window, giving us raw keyboard input.
        let _ = ImmAssociateContext(hwnd, 0);

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        
        // 主消息循环
        let mut msg: MSG = zeroed();
        let mut last_frame = std::time::Instant::now();
        let frame_duration = std::time::Duration::from_micros(16667); // ~60 FPS
        
        while RUNNING {
            // 处理 Windows 消息（非阻塞）
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
            
            // 帧更新
            if let Some(state) = GAME_STATE.as_mut() {
                let result = state.frame_update();
                
                if result == FrameResult::Exit {
                    state.shutdown();
                    RUNNING = false;
                    break;
                }
            }
            
            // 请求重绘
            InvalidateRect(hwnd, null_mut(), 0);
            UpdateWindow(hwnd);
            
            // 帧率控制
            let elapsed = last_frame.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
            last_frame = std::time::Instant::now();
        }
        
        Ok(())
    }
}

// ============================================================================
// 全局辅助函数（与 platform_desktop 兼容）
// ============================================================================

use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::cell::RefCell;

// Win32 RtlGenRandom (SystemFunction036) - 从 Windows XP 开始可用
// 比 BCryptGenRandom 更轻量，兼容性更好
#[link(name = "Advapi32")]
unsafe extern "system" {
    #[link_name = "SystemFunction036"]
    fn RtlGenRandom(buffer: *mut u8, length: u32) -> u8;
}

/// 使用 Win32 RtlGenRandom 生成随机种子
/// 返回 u64 种子，兼容所有架构
fn win32_random_seed() -> u64 {
    let mut seed = [0u8; 8];
    unsafe {
        // RtlGenRandom 返回非零表示成功
        if RtlGenRandom(seed.as_mut_ptr(), 8) == 0 {
            // 失败时使用时间戳作为后备
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            return time;
        }
    }
    u64::from_le_bytes(seed)
}

// 使用 SmallRng + Win32 随机种子（避免 getrandom crate，兼容 Win7）
thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(win32_random_seed()));
}

/// 生成 [0, max) 范围内的随机 i32
pub fn random_i32(max: i32) -> i32 {
    if max <= 0 { return 0; }
    RNG.with(|rng| rng.borrow_mut().gen_range(0..max))
}

/// 生成 [0, max) 范围内的随机 usize
pub fn random_usize(max: usize) -> usize {
    random_i32(max as i32) as usize
}

/// 生成 [0, max) 范围内的随机 u32
pub fn random_u32(max: u32) -> u32 {
    random_i32(max as i32) as u32
}

/// 生成 [0, max) 范围内的随机 u8
pub fn random_u8(max: u8) -> u8 {
    random_i32(max as i32) as u8
}

/// 生成 [0, max) 范围内的随机 f32
pub fn random_f32(max: f32) -> f32 {
    RNG.with(|rng| rng.borrow_mut().gen_range(0.0..max))
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// 日志函数（与 platform_desktop 兼容的函数签名）
pub fn log_debug(_msg: &str) {}
pub fn log_info(_msg: &str) {}
pub fn log_warn(_msg: &str) {}
pub fn log_error(msg: &str) { eprintln!("{}", msg); }

// ============================================================================
// 类型别名（与 platform_desktop 兼容）
// ============================================================================

use std::path::PathBuf;

/// 存储后端
pub struct DesktopStorage {
    base_path: PathBuf,
}

impl DesktopStorage {
    pub fn new() -> Self {
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
}

impl Default for DesktopStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for DesktopStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let full_path = self.base_path.join(key);
        std::fs::read(&full_path).ok()
    }

    fn save(&mut self, key: &str, data: &[u8]) -> std::result::Result<(), String> {
        let full_path = self.base_path.join(key);
        if let Some(parent) = full_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&full_path, data).map_err(|e| e.to_string())
    }

    fn remove(&mut self, key: &str) -> std::result::Result<(), String> {
        std::fs::remove_file(self.base_path.join(key)).map_err(|e| e.to_string())
    }

    fn exists(&self, key: &str) -> bool {
        self.base_path.join(key).exists()
    }
}

/// 显示后端（GDI 版本）
pub struct DesktopDisplay {
    width: u32,
    height: u32,
    framebuffer: Vec<u8>,
}

impl DesktopDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            framebuffer: vec![0u8; (width * height * 4) as usize],
        }
    }
}

impl DisplayBackend for DesktopDisplay {
    fn framebuffer_mut(&mut self) -> &mut [u8] { &mut self.framebuffer }
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn present(&mut self) -> std::result::Result<(), String> { Ok(()) }
    fn request_redraw(&self) {}
}

/// 输入后端
pub struct DesktopInput {
    close_requested: bool,
}

impl DesktopInput {
    pub fn new() -> Self {
        Self { close_requested: false }
    }
}

impl Default for DesktopInput {
    fn default() -> Self { Self::new() }
}

impl InputBackend for DesktopInput {
    fn poll_events(&mut self) -> Vec<crate::platform::KeyEvent> { Vec::new() }
    fn is_key_pressed(&self, _key: KeyCode) -> bool { false }
    fn should_close(&self) -> bool { self.close_requested }
    fn request_close(&mut self) { self.close_requested = true; }
}

/// 时间后端
pub struct DesktopTime {
    start: std::time::Instant,
}

impl DesktopTime {
    pub fn new() -> Self {
        Self { start: std::time::Instant::now() }
    }
}

impl Default for DesktopTime {
    fn default() -> Self { Self::new() }
}

impl TimeBackend for DesktopTime {
    fn now_ms(&self) -> f64 { now_ms() as f64 }
    fn elapsed_ms(&self) -> f64 { self.start.elapsed().as_millis() as f64 }
}

/// 随机数后端
pub struct DesktopRandom;
impl RandomBackend for DesktopRandom {
    fn random_range(&mut self, max: i32) -> i32 { random_i32(max) }
    fn random_range_f32(&mut self, max: f32) -> f32 { random_f32(max) }
    fn random_f32(&mut self) -> f32 { random_f32(1.0) }
}

/// 日志后端
pub struct DesktopLog;
impl LogBackend for DesktopLog {
    fn log(&self, level: crate::platform::LogLevel, message: &str) {
        match level {
            crate::platform::LogLevel::Error => eprintln!("[ERROR] {}", message),
            crate::platform::LogLevel::Warn => eprintln!("[WARN] {}", message),
            _ => {}
        }
    }
}
