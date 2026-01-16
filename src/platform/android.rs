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

// ============================================================================
// 显示后端 - 使用软件渲染 + ANativeWindow
// ============================================================================

/// 渲染参数（用于 touch_panel 绘制）
#[derive(Clone, Copy, Default)]
pub struct RenderParams {
    pub screen_width: u32,
    pub screen_height: u32,
    pub game_offset_x: u32,
    pub game_offset_y: u32,
    pub game_scaled_w: u32,
    pub game_scaled_h: u32,
    pub scale: f32,
}

pub struct AndroidDisplay {
    width: u32,
    height: u32,
    framebuffer: Vec<u8>,
    native_window: Option<NativeWindow>,
    render_params: RenderParams,
}

impl AndroidDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 4) as usize;
        Self {
            width,
            height,
            framebuffer: vec![0u8; buffer_size],
            native_window: None,
            render_params: RenderParams::default(),
        }
    }
    
    /// 获取渲染参数（用于 touch_panel 坐标转换）
    pub fn render_params(&self) -> RenderParams {
        self.render_params
    }

    pub fn set_native_window(&mut self, window: Option<NativeWindow>) {
        // 设置窗口格式为 RGBA_8888
        if let Some(ref w) = window {
            let win_width = w.width();
            let win_height = w.height();
            log_info("=== set_native_window ===");
            log_info(&format!("[Window] NativeWindow.width()={}, NativeWindow.height()={}", win_width, win_height));
            log_info(&format!("[Window] Window aspect ratio: {:.4}", win_width as f32 / win_height as f32));
            log_info(&format!("[Window] Game size: {}x{}", self.width, self.height));

            use ndk_sys::ANativeWindow_setBuffersGeometry;
            const WINDOW_FORMAT_RGBA_8888: i32 = 1;
            unsafe {
                // 使用 0,0 让系统自动选择 buffer 尺寸 (与窗口尺寸相同)
                let result = ANativeWindow_setBuffersGeometry(
                    w.ptr().as_ptr(),
                    0, 0,  // 0,0 表示使用窗口原始尺寸
                    WINDOW_FORMAT_RGBA_8888,
                );
                log_info(&format!(
                    "[Window] ANativeWindow_setBuffersGeometry(0, 0, RGBA8888) result={}",
                    result
                ));
            }
        } else {
            log_info("set_native_window: window=None");
        }
        self.native_window = window;
    }

    /// 渲染 framebuffer 到 ANativeWindow
    fn render_to_window(&mut self, window: &NativeWindow) -> Result<(), String> {
        use ndk_sys::{ANativeWindow_Buffer, ANativeWindow_lock, ANativeWindow_unlockAndPost};
        use std::ptr;

        let native_window_ptr = window.ptr().as_ptr();

        unsafe {
            let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();
            let lock_result = ANativeWindow_lock(native_window_ptr, &mut buffer, ptr::null_mut());
            if lock_result != 0 {
                log_error(&format!("ANativeWindow_lock failed: {}", lock_result));
                return Err(format!("ANativeWindow_lock failed: {}", lock_result));
            }

            // 记录 buffer 信息 (只在第一帧记录)
            static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
                LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
                log_info("=== First frame render info ===");
                log_info(&format!(
                    "[Buffer] ANativeWindow_Buffer: width={}, height={}, stride={}, format={}",
                    buffer.width, buffer.height, buffer.stride, buffer.format
                ));
                log_info(&format!(
                    "[Buffer] Buffer aspect ratio: {:.3}",
                    buffer.width as f32 / buffer.height as f32
                ));
                log_info(&format!(
                    "[Game] Game framebuffer: width={}, height={}, len={}",
                    self.width, self.height, self.framebuffer.len()
                ));
            }

            self.copy_framebuffer_to_window(&buffer);
            ANativeWindow_unlockAndPost(native_window_ptr);
        }

        Ok(())
    }

    /// 将游戏 framebuffer 缩放复制到窗口 buffer
    unsafe fn copy_framebuffer_to_window(&mut self, buffer: &ndk_sys::ANativeWindow_Buffer) {
        if buffer.bits.is_null() {
            log_error("copy_framebuffer: buffer.bits is null");
            return;
        }
        if buffer.width <= 0 || buffer.height <= 0 || buffer.stride <= 0 {
            log_error(&format!(
                "copy_framebuffer: invalid dimensions: width={}, height={}, stride={}",
                buffer.width, buffer.height, buffer.stride
            ));
            return;
        }

        // 支持 RGBA_8888 (format = 1) 和 RGBX_8888 (format = 2)
        if buffer.format != 1 && buffer.format != 2 {
            log_warn(&format!("Unsupported buffer format: {}", buffer.format));
            return;
        }

        let dst_ptr = buffer.bits as *mut u32;
        let dst_stride = buffer.stride as usize;
        let dst_width = buffer.width as usize;
        let dst_height = buffer.height as usize;
        let src = &self.framebuffer;

        // 游戏 framebuffer 尺寸 (已经是正确的 320x182)
        let src_width = self.width as usize;
        let src_height = self.height as usize;
        
        // 计算缩放比例 (保持宽高比，居中显示)
        let scale_x = dst_width as f32 / src_width as f32;
        let scale_y = dst_height as f32 / src_height as f32;
        let scale = scale_x.min(scale_y);
        let scaled_w = (src_width as f32 * scale) as usize;
        let scaled_h = (src_height as f32 * scale) as usize;
        // 水平居中，垂直居中
        let offset_x = dst_width.saturating_sub(scaled_w) / 2;
        let offset_y = dst_height.saturating_sub(scaled_h) / 2;

        // 更新渲染参数（用于 touch_panel 坐标转换）
        self.render_params = RenderParams {
            screen_width: dst_width as u32,
            screen_height: dst_height as u32,
            game_offset_x: offset_x as u32,
            game_offset_y: offset_y as u32,
            game_scaled_w: scaled_w as u32,
            game_scaled_h: scaled_h as u32,
            scale,
        };

        // 记录渲染参数 (只记录一次)
        static LOGGED_RENDER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED_RENDER.load(std::sync::atomic::Ordering::Relaxed) {
            LOGGED_RENDER.store(true, std::sync::atomic::Ordering::Relaxed);
            log_info("=== Render calculation details ===");
            log_info(&format!("[Render] Source (game framebuffer): {}x{}", src_width, src_height));
            log_info(&format!("[Render] Destination (window buffer): {}x{}", dst_width, dst_height));
            log_info(&format!("[Render] Scale factors: x={:.4}, y={:.4}", scale_x, scale_y));
            log_info(&format!("[Render] Chosen scale (min): {:.4}", scale));
            log_info(&format!("[Render] Scaled game size: {}x{}", scaled_w, scaled_h));
            log_info(&format!("[Render] Offset (centering): x={}, y={}", offset_x, offset_y));
            log_info(&format!("[Render] Margins: left={}, right={}, top={}, bottom={}",
                offset_x, dst_width.saturating_sub(scaled_w + offset_x),
                offset_y, dst_height.saturating_sub(scaled_h + offset_y)
            ));
        }

        // 清空整个 buffer 为黑色
        let black: u32 = 0xFF000000;
        for y in 0..dst_height {
            // SAFETY: dst_ptr 在函数开始已验证非空，y * dst_stride 在 buffer 范围内
            let row_start = unsafe { dst_ptr.add(y * dst_stride) };
            for x in 0..dst_width {
                // SAFETY: x 在 dst_width 范围内，row_start + x 在 buffer 范围内
                unsafe { *row_start.add(x) = black; }
            }
        }

        // 预计算缩放参数 (使用定点数)
        let scale_inv_x = (src_width << 16) / scaled_w.max(1);
        let scale_inv_y = (src_height << 16) / scaled_h.max(1);

        // 缩放复制 framebuffer
        for dst_y in 0..scaled_h {
            let src_y = ((dst_y * scale_inv_y) >> 16).min(src_height - 1);
            let src_row_offset = src_y * src_width * 4;
            // SAFETY: offset_y + dst_y 在 dst_height 范围内，指针计算在 buffer 范围内
            let dst_row_ptr = unsafe { dst_ptr.add((offset_y + dst_y) * dst_stride + offset_x) };

            for dst_x in 0..scaled_w {
                let src_x = ((dst_x * scale_inv_x) >> 16).min(src_width - 1);
                let src_idx = src_row_offset + src_x * 4;

                // RGBA -> ABGR (Android native window 格式)
                // SAFETY: src_idx 基于 min() 限制在源数据范围内
                let r = unsafe { *src.get_unchecked(src_idx) as u32 };
                let g = unsafe { *src.get_unchecked(src_idx + 1) as u32 };
                let b = unsafe { *src.get_unchecked(src_idx + 2) as u32 };
                let pixel = 0xFF000000 | (b << 16) | (g << 8) | r;

                // SAFETY: dst_x 在 scaled_w 范围内，dst_row_ptr + dst_x 在 buffer 范围内
                unsafe { *dst_row_ptr.add(dst_x) = pixel; }
            }
        }
    }
}

impl DisplayBackend for AndroidDisplay {
    fn framebuffer_mut(&mut self) -> &mut [u8] {
        &mut self.framebuffer
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        self.present_with_overlay(None, &[], None)
    }

    fn request_redraw(&self) {
        // Android 使用连续渲染模式
    }
}

impl AndroidDisplay {
    /// 渲染游戏画面并混合触摸面板 overlay (优化版: 只混合指定的边界框区域)
    /// fps_info: 可选的 (fps, frame_time_ms) 用于显示帧率信息
    pub fn present_with_overlay(
        &mut self, 
        overlay: Option<&[u8]>,
        blend_rects: &[(u32, u32, u32, u32)],
        fps_info: Option<(u32, f32)>
    ) -> Result<(), String> {
        let window = match &self.native_window {
            Some(w) => w.clone(),
            None => return Ok(()),
        };
        self.render_to_window_with_overlay(&window, overlay, blend_rects, fps_info)
    }
    
    /// 渲染到窗口并混合 overlay (优化版: 只混合指定的边界框区域)
    fn render_to_window_with_overlay(
        &mut self, 
        window: &NativeWindow, 
        overlay: Option<&[u8]>,
        blend_rects: &[(u32, u32, u32, u32)],
        fps_info: Option<(u32, f32)>
    ) -> Result<(), String> {
        use ndk_sys::{ANativeWindow_Buffer, ANativeWindow_lock, ANativeWindow_unlockAndPost};
        use std::ptr;

        let native_window_ptr = window.ptr().as_ptr();

        unsafe {
            let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();
            let lock_result = ANativeWindow_lock(native_window_ptr, &mut buffer, ptr::null_mut());
            if lock_result != 0 {
                return Err(format!("ANativeWindow_lock failed: {}", lock_result));
            }

            self.copy_framebuffer_to_window(&buffer);
            
            // 混合 overlay (touch_panel) - 只混合按钮区域
            if let Some(overlay_data) = overlay {
                self.blend_overlay_rects_to_window(&buffer, overlay_data, blend_rects);
            }
            
            // 绘制 FPS 信息
            if let Some((fps, frame_time_ms)) = fps_info {
                self.draw_fps_to_window(&buffer, fps, frame_time_ms);
            }
            
            ANativeWindow_unlockAndPost(native_window_ptr);
        }

        Ok(())
    }
    
    /// 将 RGBA overlay 混合到窗口 buffer (优化版: 只混合指定的边界框区域)
    unsafe fn blend_overlay_rects_to_window(
        &self, 
        buffer: &ndk_sys::ANativeWindow_Buffer, 
        overlay: &[u8],
        rects: &[(u32, u32, u32, u32)]  // (x, y, width, height)
    ) {
        let dst_ptr = buffer.bits as *mut u32;
        let dst_stride = buffer.stride as usize;
        let dst_width = buffer.width as u32;
        let dst_height = buffer.height as u32;
        let overlay_stride = dst_width as usize * 4;
        
        for &(rect_x, rect_y, rect_w, rect_h) in rects {
            // 边界检查
            let x_end = (rect_x + rect_w).min(dst_width) as usize;
            let y_end = (rect_y + rect_h).min(dst_height) as usize;
            let x_start = rect_x as usize;
            let y_start = rect_y as usize;
            
            for y in y_start..y_end {
                for x in x_start..x_end {
                    let src_idx = y * overlay_stride + x * 4;
                    if src_idx + 3 >= overlay.len() { continue; }
                    
                    let src_a = overlay[src_idx + 3];
                    if src_a == 0 { continue; }  // 完全透明，跳过
                    
                    let dst_idx = y * dst_stride + x;
                    // SAFETY: dst_idx 在 buffer 范围内
                    let dst_pixel = unsafe { *dst_ptr.add(dst_idx) };
                    
                    let pixel = if src_a == 255 {
                        // 完全不透明，直接覆盖
                        let r = overlay[src_idx] as u32;
                        let g = overlay[src_idx + 1] as u32;
                        let b = overlay[src_idx + 2] as u32;
                        0xFF000000 | (b << 16) | (g << 8) | r
                    } else {
                        // 半透明混合
                        let alpha = src_a as u32;
                        let inv_alpha = 255 - alpha;
                        
                        let src_r = overlay[src_idx] as u32;
                        let src_g = overlay[src_idx + 1] as u32;
                        let src_b = overlay[src_idx + 2] as u32;
                        
                        let dst_r = dst_pixel & 0xFF;
                        let dst_g = (dst_pixel >> 8) & 0xFF;
                        let dst_b = (dst_pixel >> 16) & 0xFF;
                        
                        let out_r = (src_r * alpha + dst_r * inv_alpha) / 255;
                        let out_g = (src_g * alpha + dst_g * inv_alpha) / 255;
                        let out_b = (src_b * alpha + dst_b * inv_alpha) / 255;
                        
                        0xFF000000 | (out_b << 16) | (out_g << 8) | out_r
                    };
                    
                    // SAFETY: dst_idx 在 buffer 范围内
                    unsafe { *dst_ptr.add(dst_idx) = pixel; }
                }
            }
        }
    }
    
    /// 绘制 FPS 信息到窗口 buffer
    /// fps: 当前帧率, frame_time_ms: 每帧渲染时间(毫秒)
    unsafe fn draw_fps_to_window(&self, buffer: &ndk_sys::ANativeWindow_Buffer, fps: u32, frame_time_ms: f32) {
        // 格式化 FPS 文本: "FPS:XX  MS:XX.X"
        let fps_str = format!("FPS:{} MS:{:.1}", fps, frame_time_ms);
        
        let dst_ptr = buffer.bits as *mut u32;
        let dst_stride = buffer.stride as usize;
        let dst_width = buffer.width as usize;
        let dst_height = buffer.height as usize;
        
        // 起始位置 (左上角，留一点边距)
        let mut x_pos = 10usize;
        let y_pos = 10usize;
        let scale = 1u32;  // 字体缩放 (1x = 原始大小)
        
        // 绘制颜色: 白色文字，黑色描边
        let text_color = 0xFFFFFFFFu32;  // 白色 ARGB
        let shadow_color = 0xFF000000u32;  // 黑色 ARGB
        
        for ch in fps_str.chars() {
            // 获取字符对应的字形索引 (SWISS_FONT 从 ASCII 32 开始)
            let ch_code = ch as usize;
            if ch_code < 32 || ch_code > 129 {
                x_pos += 8;  // 跳过不支持的字符
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
            
            // 绘制字形 (先绘制阴影再绘制文字)
            for pass in 0..2 {
                let (offset_x, offset_y, color) = if pass == 0 {
                    (1usize, 1usize, shadow_color)  // 阴影偏移
                } else {
                    (0usize, 0usize, text_color)
                };
                
                for row in 0..glyph_h {
                    for col in 0..glyph_w {
                        // 计算 bitmap 中的位索引
                        let bit_index = row * glyph_w + col;
                        let byte_index = bit_index / 8;
                        let bit_offset = bit_index % 8;
                        
                        if byte_index >= bitmap.len() { continue; }
                        
                        let byte = bitmap[byte_index];
                        let bit = (byte >> bit_offset) & 1;
                        
                        if bit == 1 {
                            // 计算屏幕位置 (考虑缩放)
                            for sy in 0..scale as usize {
                                for sx in 0..scale as usize {
                                    let px = x_pos + col * scale as usize + sx + offset_x;
                                    let py = y_pos + row * scale as usize + sy + offset_y;
                                    
                                    if px < dst_width && py < dst_height {
                                        let dst_idx = py * dst_stride + px;
                                        // SAFETY: 边界已检查
                                        unsafe { *dst_ptr.add(dst_idx) = color; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // 移动到下一个字符位置
            x_pos += glyph_w * scale as usize + 2;
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
                                
                                // 预计算缩放参数
                                let scale_x = win_width as f32 / GAME_WIDTH as f32;
                                let scale_y = win_height as f32 / GAME_HEIGHT as f32;
                                let scale = scale_x.min(scale_y);
                                let scaled_w = (GAME_WIDTH as f32 * scale) as i32;
                                let scaled_h = (GAME_HEIGHT as f32 * scale) as i32;
                                let offset_x = (win_width - scaled_w) / 2;
                                let offset_y = (win_height - scaled_h) / 2;
                                
                                log_info(&format!("[Scale] scale_x={:.3}, scale_y={:.3}, chosen_scale={:.3}", scale_x, scale_y, scale));
                                log_info(&format!("[Scale] scaled game size: {}x{}", scaled_w, scaled_h));
                                log_info(&format!("[Scale] offset: ({}, {})", offset_x, offset_y));
                                log_info(&format!("[Scale] margins: top={}, bottom={}", offset_y, win_height - scaled_h - offset_y));
                                
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
                            // 窗口尺寸或内容区域变化时更新 touch_panel
                            if let Some(window) = app.native_window() {
                                let width = window.width() as f32;
                                let height = window.height() as f32;
                                log_info(&format!("Window resized: {}x{}", width, height));
                                input.set_screen_size(width, height);
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

            // 渲染游戏画面
            let display_frame = display.framebuffer_mut();
            state.render_to_rgba(display_frame);
            
            // 渲染触摸面板 overlay 并提交显示 (边界框优化)
            {
                let (overlay, blend_rects) = if input.should_show_virtual_buttons() {
                    let button_states = input.touch_panel().button_states();
                    let rects = input.touch_panel().renderer().get_blend_rects();
                    let overlay_data = input.touch_panel_mut().renderer_mut().render(&button_states);
                    (overlay_data, rects)
                } else {
                    (None, Vec::new())
                };
                let fps_info = Some((fps_display, frame_time_display));
                let _ = display.present_with_overlay(overlay, &blend_rects, fps_info);
            }
            
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
