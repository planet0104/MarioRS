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
};

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

// 游戏分辨率 (与 desktop.rs 保持一致)
const GAME_WIDTH: u32 = 320;
const GAME_HEIGHT: u32 = 200;

// ============================================================================
// 虚拟按钮渲染器 - 在 framebuffer 中绘制触摸控制UI
// ============================================================================

/// 虚拟按钮状态
#[derive(Default, Clone, Copy)]
pub struct ButtonStates {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
}

/// 虚拟按钮渲染器 - 在游戏画面上叠加半透明按钮
pub struct VirtualButtonsRenderer {
    // D-Pad 位置 (基于游戏分辨率 320x200)
    dpad_x: i32,
    dpad_y: i32,
    dpad_size: i32,
    // 跳跃按钮位置
    jump_x: i32,
    jump_y: i32,
    jump_radius: i32,
}

impl VirtualButtonsRenderer {
    pub fn new() -> Self {
        // 按钮位置基于游戏原始分辨率 (320x200)
        Self {
            // D-Pad 在左下角
            dpad_x: 20,
            dpad_y: 130,
            dpad_size: 60,
            // 跳跃按钮在右下角
            jump_x: 275,
            jump_y: 155,
            jump_radius: 25,
        }
    }

    /// 在 RGBA framebuffer 上绘制半透明虚拟按钮
    pub fn render_overlay(&self, framebuffer: &mut [u8], width: u32, _height: u32, states: &ButtonStates) {
        // 绘制 D-Pad (十字方向键)
        self.draw_dpad(framebuffer, width, states);
        
        // 绘制跳跃按钮 (圆形)
        self.draw_jump_button(framebuffer, width, states.jump);
    }

    fn draw_dpad(&self, fb: &mut [u8], width: u32, states: &ButtonStates) {
        let alpha = 0.35;
        let color_normal: (u8, u8, u8) = (120, 120, 120);
        let color_pressed: (u8, u8, u8) = (220, 80, 80);
        
        let btn_w = 18;
        let btn_h = 18;
        let gap = 2;
        
        // 中心点
        let cx = self.dpad_x + self.dpad_size / 2;
        let cy = self.dpad_y + self.dpad_size / 2;
        
        // 上
        self.draw_rect_alpha(fb, width,
            cx - btn_w / 2, cy - btn_h - gap - btn_h / 2, btn_w, btn_h,
            if states.up { color_pressed } else { color_normal },
            alpha);
        // 下
        self.draw_rect_alpha(fb, width,
            cx - btn_w / 2, cy + gap + btn_h / 2, btn_w, btn_h,
            if states.down { color_pressed } else { color_normal },
            alpha);
        // 左
        self.draw_rect_alpha(fb, width,
            cx - btn_w - gap - btn_w / 2, cy - btn_h / 2, btn_w, btn_h,
            if states.left { color_pressed } else { color_normal },
            alpha);
        // 右
        self.draw_rect_alpha(fb, width,
            cx + gap + btn_w / 2, cy - btn_h / 2, btn_w, btn_h,
            if states.right { color_pressed } else { color_normal },
            alpha);
        
        // 中心圆点
        self.draw_circle_alpha(fb, width, cx, cy, 6, (80, 80, 80), alpha * 0.8);
    }

    fn draw_jump_button(&self, fb: &mut [u8], width: u32, pressed: bool) {
        let color = if pressed { (100, 220, 100) } else { (100, 100, 100) };
        self.draw_circle_alpha(fb, width, self.jump_x, self.jump_y, self.jump_radius, color, 0.4);
        
        // 绘制 "A" 字母 (简单像素字体)
        let letter_color = if pressed { (255, 255, 255) } else { (200, 200, 200) };
        self.draw_letter_a(fb, width, self.jump_x - 4, self.jump_y - 5, letter_color, 0.8);
    }

    /// Alpha 混合绘制矩形
    fn draw_rect_alpha(&self, fb: &mut [u8], width: u32,
        x: i32, y: i32, w: i32, h: i32,
        color: (u8, u8, u8), alpha: f32)
    {
        let height = GAME_HEIGHT as i32;
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && px < width as i32 && py < height {
                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                    if idx + 3 < fb.len() {
                        fb[idx]     = self.blend(color.0, fb[idx], alpha);
                        fb[idx + 1] = self.blend(color.1, fb[idx + 1], alpha);
                        fb[idx + 2] = self.blend(color.2, fb[idx + 2], alpha);
                    }
                }
            }
        }
    }

    /// Alpha 混合绘制圆形
    fn draw_circle_alpha(&self, fb: &mut [u8], width: u32,
        cx: i32, cy: i32, radius: i32,
        color: (u8, u8, u8), alpha: f32)
    {
        let height = GAME_HEIGHT as i32;
        let r2 = (radius * radius) as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= r2 {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && py >= 0 && px < width as i32 && py < height {
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        if idx + 3 < fb.len() {
                            fb[idx]     = self.blend(color.0, fb[idx], alpha);
                            fb[idx + 1] = self.blend(color.1, fb[idx + 1], alpha);
                            fb[idx + 2] = self.blend(color.2, fb[idx + 2], alpha);
                        }
                    }
                }
            }
        }
    }

    /// 绘制简单的 "A" 字母
    fn draw_letter_a(&self, fb: &mut [u8], width: u32, x: i32, y: i32, color: (u8, u8, u8), alpha: f32) {
        // 8x10 像素的 A 字母
        let pattern = [
            "  ##  ",
            " #  # ",
            "#    #",
            "#    #",
            "######",
            "#    #",
            "#    #",
            "#    #",
        ];
        
        let height = GAME_HEIGHT as i32;
        for (row, line) in pattern.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch == '#' {
                    let px = x + col as i32;
                    let py = y + row as i32;
                    if px >= 0 && py >= 0 && px < width as i32 && py < height {
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        if idx + 3 < fb.len() {
                            fb[idx]     = self.blend(color.0, fb[idx], alpha);
                            fb[idx + 1] = self.blend(color.1, fb[idx + 1], alpha);
                            fb[idx + 2] = self.blend(color.2, fb[idx + 2], alpha);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn blend(&self, src: u8, dst: u8, alpha: f32) -> u8 {
        ((src as f32 * alpha) + (dst as f32 * (1.0 - alpha))) as u8
    }

    // ========================================================================
    // 触摸区域检测
    // ========================================================================

    /// 获取 D-Pad 中心坐标 (游戏坐标系)
    pub fn dpad_center(&self) -> (i32, i32) {
        (self.dpad_x + self.dpad_size / 2, self.dpad_y + self.dpad_size / 2)
    }

    /// 获取 D-Pad 半径
    pub fn dpad_radius(&self) -> i32 {
        self.dpad_size / 2
    }

    /// 获取跳跃按钮中心和半径
    pub fn jump_button(&self) -> (i32, i32, i32) {
        (self.jump_x, self.jump_y, self.jump_radius)
    }
}

impl Default for VirtualButtonsRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 触摸输入处理
// ============================================================================

/// 触摸点状态
#[derive(Clone, Copy)]
struct TouchPoint {
    x: f32,
    y: f32,
    active: bool,
}

/// Android 输入后端 - 支持触摸和物理键盘
pub struct AndroidInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
    
    // 触摸相关
    touch_points: [Option<TouchPoint>; 10], // 最多支持10个触摸点
    button_states: ButtonStates,
    virtual_buttons: VirtualButtonsRenderer,
    
    // 屏幕尺寸 (用于坐标转换)
    screen_width: f32,
    screen_height: f32,
    
    // 是否有物理键盘
    has_physical_keyboard: bool,
}

impl AndroidInput {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
            touch_points: [None; 10],
            button_states: ButtonStates::default(),
            virtual_buttons: VirtualButtonsRenderer::new(),
            screen_width: 1920.0,
            screen_height: 1080.0,
            has_physical_keyboard: false,
        }
    }

    /// 更新屏幕尺寸
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// 设置是否有物理键盘
    pub fn set_has_physical_keyboard(&mut self, has: bool) {
        self.has_physical_keyboard = has;
    }

    /// 获取虚拟按钮渲染器
    pub fn virtual_buttons_renderer(&self) -> &VirtualButtonsRenderer {
        &self.virtual_buttons
    }

    /// 获取当前按钮状态
    pub fn get_button_states(&self) -> ButtonStates {
        self.button_states
    }

    /// 是否应该显示虚拟按钮
    pub fn should_show_virtual_buttons(&self) -> bool {
        !self.has_physical_keyboard
    }

    /// 处理触摸事件
    pub fn handle_touch(&mut self, pointer_id: usize, x: f32, y: f32, action: MotionAction) {
        if pointer_id >= self.touch_points.len() {
            return;
        }

        // 屏幕坐标转换为游戏坐标
        let game_x = self.screen_to_game_x(x);
        let game_y = self.screen_to_game_y(y);

        match action {
            MotionAction::Down | MotionAction::PointerDown => {
                self.touch_points[pointer_id] = Some(TouchPoint {
                    x: game_x,
                    y: game_y,
                    active: true,
                });
            }
            MotionAction::Move => {
                if let Some(ref mut point) = self.touch_points[pointer_id] {
                    point.x = game_x;
                    point.y = game_y;
                }
            }
            MotionAction::Up | MotionAction::PointerUp | MotionAction::Cancel => {
                self.touch_points[pointer_id] = None;
            }
            _ => {}
        }

        // 更新虚拟按钮状态
        self.update_button_states_from_touches();
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

    /// 屏幕X坐标转游戏坐标
    fn screen_to_game_x(&self, screen_x: f32) -> f32 {
        let game_w = GAME_WIDTH as f32;
        let game_h = GAME_HEIGHT as f32;
        let scale = (self.screen_width / game_w).min(self.screen_height / game_h);
        let offset_x = (self.screen_width - game_w * scale) / 2.0;
        (screen_x - offset_x) / scale
    }

    /// 屏幕Y坐标转游戏坐标
    fn screen_to_game_y(&self, screen_y: f32) -> f32 {
        let game_w = GAME_WIDTH as f32;
        let game_h = GAME_HEIGHT as f32;
        let scale = (self.screen_width / game_w).min(self.screen_height / game_h);
        let offset_y = (self.screen_height - game_h * scale) / 2.0;
        (screen_y - offset_y) / scale
    }

    /// 根据所有触摸点更新按钮状态
    fn update_button_states_from_touches(&mut self) {
        // 重置状态
        let mut new_states = ButtonStates::default();
        
        let (dpad_cx, dpad_cy) = self.virtual_buttons.dpad_center();
        let dpad_r = self.virtual_buttons.dpad_radius() as f32;
        let (jump_cx, jump_cy, jump_r) = self.virtual_buttons.jump_button();

        for point in self.touch_points.iter().flatten() {
            let x = point.x;
            let y = point.y;

            // 检测 D-Pad
            let dx = x - dpad_cx as f32;
            let dy = y - dpad_cy as f32;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < dpad_r * 1.8 {
                // 在 D-Pad 区域内，根据位置判断方向
                let threshold = dpad_r * 0.3;
                if dx < -threshold { new_states.left = true; }
                if dx > threshold { new_states.right = true; }
                if dy < -threshold { new_states.up = true; }
                if dy > threshold { new_states.down = true; }
            }

            // 检测跳跃按钮
            let jdx = x - jump_cx as f32;
            let jdy = y - jump_cy as f32;
            if (jdx * jdx + jdy * jdy).sqrt() < jump_r as f32 * 1.5 {
                new_states.jump = true;
            }
        }

        // 生成按键事件
        self.generate_key_events_from_states(&new_states);
        self.button_states = new_states;
    }

    /// 根据按钮状态变化生成按键事件
    fn generate_key_events_from_states(&mut self, new_states: &ButtonStates) {
        // 复制旧状态避免借用冲突
        let old = self.button_states;
        
        // 左
        if new_states.left != old.left {
            self.update_key_state(PlatformKeyCode::Left, new_states.left);
        }
        // 右
        if new_states.right != old.right {
            self.update_key_state(PlatformKeyCode::Right, new_states.right);
        }
        // 上
        if new_states.up != old.up {
            self.update_key_state(PlatformKeyCode::Up, new_states.up);
        }
        // 下
        if new_states.down != old.down {
            self.update_key_state(PlatformKeyCode::Down, new_states.down);
        }
        // 跳跃
        if new_states.jump != old.jump {
            self.update_key_state(PlatformKeyCode::Space, new_states.jump);
        }
    }

    fn update_key_state(&mut self, key: PlatformKeyCode, pressed: bool) {
        if pressed {
            self.key_states.insert(key);
        } else {
            self.key_states.remove(&key);
        }
        self.pending_events.push(PlatformKeyEvent { key, pressed });
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

// ============================================================================
// 显示后端 - 使用软件渲染 + ANativeWindow
// ============================================================================

pub struct AndroidDisplay {
    width: u32,
    height: u32,
    framebuffer: Vec<u8>,
    native_window: Option<NativeWindow>,
}

impl AndroidDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 4) as usize;
        Self {
            width,
            height,
            framebuffer: vec![0u8; buffer_size],
            native_window: None,
        }
    }

    pub fn set_native_window(&mut self, window: Option<NativeWindow>) {
        // 设置窗口格式为 RGBA_8888
        if let Some(ref w) = window {
            let win_width = w.width();
            let win_height = w.height();
            log_info(&format!(
                "set_native_window: window size={}x{}, game size={}x{}",
                win_width, win_height, self.width, self.height
            ));

            use ndk_sys::ANativeWindow_setBuffersGeometry;
            // WINDOW_FORMAT_RGBA_8888 = 1
            const WINDOW_FORMAT_RGBA_8888: i32 = 1;
            unsafe {
                let result = ANativeWindow_setBuffersGeometry(
                    w.ptr().as_ptr(),
                    0, 0, // 使用默认宽高
                    WINDOW_FORMAT_RGBA_8888,
                );
                log_info(&format!(
                    "ANativeWindow_setBuffersGeometry result={}, format={}",
                    result, WINDOW_FORMAT_RGBA_8888
                ));
            }
        } else {
            log_info("set_native_window: window=None");
        }
        self.native_window = window;
    }

    /// 渲染 framebuffer 到 ANativeWindow
    fn render_to_window(&self, window: &NativeWindow) -> Result<(), String> {
        use ndk_sys::{ANativeWindow_Buffer, ANativeWindow_lock, ANativeWindow_unlockAndPost};
        use std::ptr;

        let native_window_ptr = window.ptr().as_ptr();

        unsafe {
            // 准备 buffer 结构
            let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();

            // 锁定窗口 buffer
            let lock_result = ANativeWindow_lock(native_window_ptr, &mut buffer, ptr::null_mut());
            if lock_result != 0 {
                log_error(&format!("ANativeWindow_lock failed: {}", lock_result));
                return Err(format!("ANativeWindow_lock failed: {}", lock_result));
            }

            // 记录 buffer 信息 (只在第一帧记录，避免日志过多)
            static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
                LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
                log_info(&format!(
                    "ANativeWindow_Buffer: width={}, height={}, stride={}, format={}, bits={:?}",
                    buffer.width, buffer.height, buffer.stride, buffer.format, buffer.bits
                ));
                log_info(&format!(
                    "Game framebuffer: width={}, height={}, len={}",
                    self.width, self.height, self.framebuffer.len()
                ));
            }

            // 渲染到 buffer
            self.copy_framebuffer_to_window(&buffer);

            // 解锁并提交
            ANativeWindow_unlockAndPost(native_window_ptr);
        }

        Ok(())
    }

    /// 将游戏 framebuffer 缩放复制到窗口 buffer
    unsafe fn copy_framebuffer_to_window(&self, buffer: &ndk_sys::ANativeWindow_Buffer) {
        // 检查 buffer 是否有效
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

        // 检查格式 - 只支持 RGBA_8888 (format = 1)
        if buffer.format != 1 {
            log_warn(&format!("Unsupported buffer format: {}", buffer.format));
            return;
        }

        let dst_ptr = buffer.bits as *mut u32;
        let dst_stride = buffer.stride as usize;
        let dst_width = buffer.width as usize;
        let dst_height = buffer.height as usize;
        let src = &self.framebuffer;

        // 计算缩放比例 (保持宽高比) - 使用 buffer 尺寸
        let scale_x = dst_width as f32 / self.width as f32;
        let scale_y = dst_height as f32 / self.height as f32;
        let scale = scale_x.min(scale_y);

        // 计算居中偏移
        let scaled_w = (self.width as f32 * scale) as usize;
        let scaled_h = (self.height as f32 * scale) as usize;
        let offset_x = dst_width.saturating_sub(scaled_w) / 2;
        let offset_y = dst_height.saturating_sub(scaled_h) / 2;

        // 只记录一次渲染参数
        static LOGGED_RENDER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED_RENDER.load(std::sync::atomic::Ordering::Relaxed) {
            LOGGED_RENDER.store(true, std::sync::atomic::Ordering::Relaxed);
            log_info(&format!(
                "Render params: dst={}x{}, stride={}, scale={:.2}, scaled={}x{}, offset=({},{})",
                dst_width, dst_height, dst_stride, scale, scaled_w, scaled_h, offset_x, offset_y
            ));
            log_info(&format!(
                "Buffer memory: dst_ptr={:?}, max_offset={}",
                dst_ptr, (dst_height - 1) * dst_stride + (dst_width - 1)
            ));
        }

        // 先清空整个 buffer 为黑色
        for y in 0..dst_height {
            for x in 0..dst_width {
                *dst_ptr.add(y * dst_stride + x) = 0xFF000000; // ABGR 黑色
            }
        }

        // 缩放复制 framebuffer
        for dst_y in 0..scaled_h {
            for dst_x in 0..scaled_w {
                // 源坐标 (最近邻采样)
                let src_x = (dst_x as f32 / scale) as usize;
                let src_y = (dst_y as f32 / scale) as usize;
                let src_x = src_x.min(self.width as usize - 1);
                let src_y = src_y.min(self.height as usize - 1);
                let src_idx = (src_y * self.width as usize + src_x) * 4;

                // 检查源索引有效性
                if src_idx + 3 > src.len() {
                    continue;
                }

                // RGBA -> ABGR (Android native window 格式)
                let r = src[src_idx] as u32;
                let g = src[src_idx + 1] as u32;
                let b = src[src_idx + 2] as u32;
                let pixel = 0xFF000000 | (b << 16) | (g << 8) | r;

                // 目标位置 - 严格边界检查
                let dx = offset_x + dst_x;
                let dy = offset_y + dst_y;
                if dx < dst_width && dy < dst_height {
                    *dst_ptr.add(dy * dst_stride + dx) = pixel;
                }
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
        let window = match &self.native_window {
            Some(w) => w,
            None => return Ok(()), // 没有窗口时跳过
        };

        // 使用 unsafe 调用 ANativeWindow API
        self.render_to_window(window)
    }

    fn request_redraw(&self) {
        // Android 使用连续渲染模式，不需要显式请求重绘
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
    /// 创建存储后端 (无参数版本，用于 config.rs 等模块)
    /// 使用当前目录作为基础路径
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
        // 确保父目录存在
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
// 日志后端 - 使用 android_logger
// ============================================================================

pub struct AndroidLog;

impl AndroidLog {
    pub fn new() -> Self {
        Self
    }

    pub fn init() {
        // 初始化 Android 日志系统
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("MarioRS"),
        );
    }
}

impl Default for AndroidLog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBackend for AndroidLog {
    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => log::debug!("{}", message),
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warn => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
        }
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
    let _storage = AndroidStorage::with_app(&app);
    let mut game_state: Option<GameState> = None;
    
    // 帧率控制
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut next_frame = Instant::now();
    let mut running = true;

    // Android 事件循环
    while running {
        app.poll_events(Some(Duration::from_millis(16)), |event| {
            match event {
                PollEvent::Main(main_event) => {
                    match main_event {
                        MainEvent::InitWindow { .. } => {
                            // 窗口初始化
                            log_info("Native window initialized");
                            if let Some(window) = app.native_window() {
                                let width = window.width() as f32;
                                let height = window.height() as f32;
                                input.set_screen_size(width, height);
                                display.set_native_window(Some(window));
                                
                                // 初始化游戏状态
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
                            input.handle_key(key_event.key_code(), key_event.action());
                            input.set_has_physical_keyboard(true);
                        }
                        InputEvent::MotionEvent(motion_event) => {
                            let pointer_count = motion_event.pointer_count();
                            for i in 0..pointer_count {
                                let pointer = motion_event.pointer_at_index(i);
                                input.handle_touch(
                                    pointer.pointer_id() as usize,
                                    pointer.x(),
                                    pointer.y(),
                                    motion_event.action(),
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
        let now = Instant::now();
        if now < next_frame {
            continue;
        }
        next_frame = now + frame_duration;

        // 游戏帧更新
        if let Some(state) = &mut game_state {
            // 处理键盘事件
            for event in input.poll_events() {
                state.handle_key_event(&event);
            }

            // 更新游戏逻辑
            let result = state.frame_update();

            // 渲染
            let display_frame = display.framebuffer_mut();
            state.render_to_rgba(display_frame);

            // 叠加虚拟按钮 (如果需要显示)
            if input.should_show_virtual_buttons() {
                input.virtual_buttons_renderer().render_overlay(
                    display_frame,
                    GAME_WIDTH,
                    GAME_HEIGHT,
                    &input.get_button_states(),
                );
            }

            // 提交显示
            let _ = display.present();

            if result == FrameResult::Exit {
                state.shutdown();
                running = false;
            }
        }
    }

    log_info("MarioRS Android exiting...");
}

/// 游戏运行入口 (与其他平台保持一致的接口)
/// Android 平台不使用此函数，而是通过 lib.rs 中的 android_main 入口
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    Err("Android platform should use android_main entry point".into())
}
