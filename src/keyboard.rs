use crate::persist as ps;
use crate::platform::{KeyCode, KeyEvent};

// Windows 使用 hashbrown 避免 BCryptGenRandom 依赖（兼容 Win7）
#[cfg(target_os = "windows")]
use hashbrown::HashMap;
#[cfg(not(target_os = "windows"))]
use std::collections::HashMap;

/// 键盘扫描码常量 (对应原Pascal代码中的常量)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScanCode {
    // 数字键
    Kb1 = 2,
    Kb2 = 3,
    Kb3 = 4,
    Kb4 = 5,
    Kb5 = 6,
    Kb6 = 7,
    Kb7 = 8,
    Kb8 = 9,
    Kb9 = 10,
    Kb0 = 11,

    // 字母键 - 第一行
    KbQ = 16,
    KbW = 17,
    KbE = 18,
    KbR = 19,
    KbT = 20,
    KbY = 21,
    KbU = 22,
    KbI = 23,
    KbO = 24,
    KbP = 25,

    // 字母键 - 第二行
    KbA = 30,
    KbS = 31,
    KbD = 32,
    KbF = 33,
    KbG = 34,
    KbH = 35,
    KbJ = 36,
    KbK = 37,
    KbL = 38,

    // 字母键 - 第三行
    KbZ = 44,
    KbX = 45,
    KbC = 46,
    KbV = 47,
    KbB = 48,
    KbN = 49,
    KbM = 50,

    // 特殊键
    KbEsc = 1,
    KbBS = 14,
    KbTab = 15,
    KbEnter = 28,
    KbSpace = 57,
    KbUpArrow = 72,
    KbLeftArrow = 75,
    KbRightArrow = 77,
    KbDownArrow = 80,

    // 功能键
    // Pascal/PC 扫描码集合：F1 = 59
    KbF1 = 59,
    // Pascal/PC 扫描码集合：F2 = 60
    KbF2 = 60,
}

/// 键盘状态跟踪 - 扩展版本，支持所有游戏需要的按键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyType {
    // 游戏控制键（0-8，用于宏录制）
    Left,       // 0
    Right,      // 1
    Up,         // 2
    Down,       // 3
    Alt,        // 4
    Ctrl,       // 5
    ShiftLeft,  // 6
    ShiftRight, // 7
    Space,      // 8
    // 其他功能键（不参与宏录制）
    Tab,
    Enter,
    Escape,
    F1,
    // 字母键 A-Z
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    // 数字键 0-9
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
}

impl KeyType {
    /// 获取用于宏录制的索引（0-8），其他键返回None
    fn macro_index(&self) -> Option<usize> {
        match self {
            KeyType::Left => Some(0),
            KeyType::Right => Some(1),
            KeyType::Up => Some(2),
            KeyType::Down => Some(3),
            KeyType::Alt => Some(4),
            KeyType::Ctrl => Some(5),
            KeyType::ShiftLeft => Some(6),
            KeyType::ShiftRight => Some(7),
            KeyType::Space => Some(8),
            _ => None,
        }
    }
}

const MAX_KEYS: usize = 9;
const MAX_SEQ_LEN: usize = 100;

/// Demo按键序列数据 - 从Pascal DEMOKEYS.OBJ提取
/// 格式: 9个通道 x 100个u16值
/// 通道顺序: Left, Right, Up, Down, Alt, Ctrl, ShiftL, ShiftR, Space
pub const DEMO_KEY_SEQUENCES: [[u16; 100]; 9] = [
    // Channel 0: Left
    [
        508, 102, 1352, 12, 1018, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 1: Right
    [
        62, 298, 258, 176, 206, 314, 36, 12, 290, 74, 18, 22, 18, 60, 240, 8, 100, 18, 102, 26,
        184, 48, 42, 12, 44, 10, 314, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 2: Up
    [
        1496, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 3: Down
    [
        1477, 1, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 4: Alt
    [
        93, 45, 10, 18, 186, 27, 130, 8, 24, 27, 58, 29, 174, 33, 45, 9, 27, 28, 17, 33, 85, 24,
        25, 127, 214, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 5: Ctrl
    [
        56, 150, 122, 126, 38, 86, 7, 100, 120, 512, 179, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 6: ShiftL
    [
        1496, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 7: ShiftR
    [
        724, 50, 592, 53, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Channel 8: Space
    [
        1496, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
];

#[derive(Debug, Copy, Clone)]
struct KeySequence([u16; MAX_SEQ_LEN]);

/// 宏录制和播放系统
#[derive(Debug, Clone)]
pub struct MacroSystem {
    sequences: [KeySequence; MAX_KEYS],
    seq_positions: [usize; MAX_KEYS],
    recording: bool,
    playing: bool,
}

impl Default for MacroSystem {
    fn default() -> Self {
        Self {
            sequences: [KeySequence([0; MAX_SEQ_LEN]); MAX_KEYS],
            seq_positions: [0; MAX_KEYS],
            recording: false,
            playing: false,
        }
    }
}

impl MacroSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_recording(&mut self) {
        self.recording = true;
        self.playing = false;
        self.seq_positions = [0; MAX_KEYS];
        self.sequences = [KeySequence([0; MAX_SEQ_LEN]); MAX_KEYS];
    }

    pub fn start_playing(&mut self) {
        self.playing = true;
        self.recording = false;
        self.seq_positions = [0; MAX_KEYS];
    }

    /// 加载Demo按键序列（对应Pascal的PlayMacro中的Move操作）
    pub fn load_demo_sequences(&mut self) {
        for i in 0..MAX_KEYS {
            for j in 0..MAX_SEQ_LEN {
                self.sequences[i].0[j] = DEMO_KEY_SEQUENCES[i][j];
            }
        }
    }

    /// 开始播放Demo（加载预录制序列并开始播放）
    pub fn start_demo(&mut self) {
        self.load_demo_sequences();
        self.playing = true;
        self.recording = false;
        self.seq_positions = [0; MAX_KEYS];
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.recording = false;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn save_macro(&mut self) -> Result<(), std::io::Error> {
        use std::fs::File;

        let mut file = File::create("macro.dat")?;
        // 写入 9 * 100 u16 values in little-endian order
        for seq in self.sequences.iter() {
            for &v in seq.0.iter() {
                ps::write_u16_le(&mut file, v)?;
            }
        }
        self.recording = false;
        self.seq_positions = [0; MAX_KEYS];
        Ok(())
    }

    pub fn load_macro(&mut self) -> Result<(), std::io::Error> {
        use std::fs::File;

        let mut file = File::open("macro.dat")?;
        for i in 0..MAX_KEYS {
            for j in 0..MAX_SEQ_LEN {
                let v = ps::read_u16_le(&mut file)?;
                self.sequences[i].0[j] = v;
            }
        }
        Ok(())
    }

    fn check_key(&mut self, key_nr: usize, pressed: bool) -> bool {
        if !self.playing && !self.recording {
            return pressed;
        }

        let mut result = pressed;

        if self.recording {
            if pressed != (self.seq_positions[key_nr] % 2 == 1) {
                self.seq_positions[key_nr] += 1;
                if self.seq_positions[key_nr] >= MAX_SEQ_LEN {
                    self.seq_positions[key_nr] = MAX_SEQ_LEN - 1;
                }
            }
            if self.seq_positions[key_nr] < MAX_SEQ_LEN {
                self.sequences[key_nr].0[self.seq_positions[key_nr]] += 1;
            }
        }

        if self.playing {
            if self.sequences[key_nr].0[self.seq_positions[key_nr]] == 0 {
                self.playing = false;
            } else {
                self.sequences[key_nr].0[self.seq_positions[key_nr]] -= 1;
                if self.sequences[key_nr].0[self.seq_positions[key_nr]] == 0 {
                    self.seq_positions[key_nr] += 1;
                }
                result = self.seq_positions[key_nr] % 2 == 1;
            }
        }

        result
    }
}

/// 主键盘处理结构体
#[derive(Debug)]
pub struct Keyboard {
    key_states: HashMap<KeyType, bool>,
    current_key: Option<char>,
    current_scan_code: Option<u8>,
    key_hit: bool,
    macro_system: MacroSystem,
    handler_active: bool,
    // 事件锁存：用于避免"按下又立即松开"发生在两帧之间导致轮询时错过
    alt_pressed_once: bool,
    space_pressed_once: bool,
    f1_pressed_once: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    /// 获取当前按下的字符（如果有）
    pub fn get_current_char(&self) -> Option<char> {
        self.current_key
    }
    pub fn new() -> Self {
        Self {
            key_states: HashMap::new(),
            current_key: None,
            current_scan_code: None,
            key_hit: false,
            macro_system: MacroSystem::new(),
            handler_active: false,
            alt_pressed_once: false,
            space_pressed_once: false,
            f1_pressed_once: false,
        }
    }

    /// 初始化键盘处理
    pub fn init(&mut self) {
        self.handler_active = true;
        self.key_hit = false;
        self.reset();
    }

    /// 清理键盘处理
    pub fn done(&mut self) {
        if !self.handler_active {
            return;
        }
        self.handler_active = false;
        self.key_states.clear();
    }

    /// 重置键盘状态
    pub fn reset(&mut self) {
        self.macro_system.recording = false;
        self.macro_system.playing = false;
        self.key_states.clear();
        self.current_key = None;
        self.current_scan_code = None;
        self.alt_pressed_once = false;
        self.space_pressed_once = false;
        self.f1_pressed_once = false;
    }

    /// 处理平台键盘事件并更新内部状态
    /// 使用平台无关的 KeyEvent 结构
    pub fn handle_keyboard_input(&mut self, input: &KeyEvent) {
        let pressed = input.pressed;
        self.key_hit = pressed;

        // 获取扫描码
        if let Some(scan_code) = self.platform_keycode_to_scan_code(input.key) {
            self.current_scan_code = Some(scan_code);
            // 获取对应的ASCII字符
            self.current_key = self.scan_code_to_ascii(scan_code);
        }

        // 辅助函数：更新按键状态
        let mut update_key = |key_type: KeyType| {
            self.key_states.insert(key_type, pressed);
        };

        // 更新对应的按键状态
        match input.key {
            // 方向键
            KeyCode::Left => {
                update_key(KeyType::Left);
            }
            KeyCode::Right => {
                update_key(KeyType::Right);
            }
            KeyCode::Up => {
                update_key(KeyType::Up);
            }
            KeyCode::Down => {
                update_key(KeyType::Down);
            }

            // 修饰键
            KeyCode::AltLeft | KeyCode::AltRight => {
                let was_down = *self.key_states.get(&KeyType::Alt).unwrap_or(&false);
                if pressed && !was_down {
                    self.alt_pressed_once = true;
                }
                self.key_states.insert(KeyType::Alt, pressed);
            }
            KeyCode::ControlLeft | KeyCode::ControlRight => {
                update_key(KeyType::Ctrl);
            }
            KeyCode::ShiftLeft => {
                update_key(KeyType::ShiftLeft);
            }
            KeyCode::ShiftRight => {
                update_key(KeyType::ShiftRight);
            }

            // 功能键
            KeyCode::Space => {
                let was_down = *self.key_states.get(&KeyType::Space).unwrap_or(&false);
                if pressed && !was_down {
                    self.space_pressed_once = true;
                }
                self.key_states.insert(KeyType::Space, pressed);
            }
            KeyCode::Tab => {
                update_key(KeyType::Tab);
            }
            KeyCode::Enter => {
                update_key(KeyType::Enter);
            }
            KeyCode::Escape => {
                update_key(KeyType::Escape);
            }
            KeyCode::F1 => {
                self.key_states.insert(KeyType::F1, pressed);
                if pressed {
                    self.f1_pressed_once = true;
                }
            }

            // 字母键 A-Z
            KeyCode::KeyA => {
                update_key(KeyType::KeyA);
            }
            KeyCode::KeyB => {
                update_key(KeyType::KeyB);
            }
            KeyCode::KeyC => {
                update_key(KeyType::KeyC);
            }
            KeyCode::KeyD => {
                update_key(KeyType::KeyD);
            }
            KeyCode::KeyE => {
                update_key(KeyType::KeyE);
            }
            KeyCode::KeyF => {
                update_key(KeyType::KeyF);
            }
            KeyCode::KeyG => {
                update_key(KeyType::KeyG);
            }
            KeyCode::KeyH => {
                update_key(KeyType::KeyH);
            }
            KeyCode::KeyI => {
                update_key(KeyType::KeyI);
            }
            KeyCode::KeyJ => {
                update_key(KeyType::KeyJ);
            }
            KeyCode::KeyK => {
                update_key(KeyType::KeyK);
            }
            KeyCode::KeyL => {
                update_key(KeyType::KeyL);
            }
            KeyCode::KeyM => {
                update_key(KeyType::KeyM);
            }
            KeyCode::KeyN => {
                update_key(KeyType::KeyN);
            }
            KeyCode::KeyO => {
                update_key(KeyType::KeyO);
            }
            KeyCode::KeyP => {
                update_key(KeyType::KeyP);
            }
            KeyCode::KeyQ => {
                update_key(KeyType::KeyQ);
            }
            KeyCode::KeyR => {
                update_key(KeyType::KeyR);
            }
            KeyCode::KeyS => {
                update_key(KeyType::KeyS);
            }
            KeyCode::KeyT => {
                update_key(KeyType::KeyT);
            }
            KeyCode::KeyU => {
                update_key(KeyType::KeyU);
            }
            KeyCode::KeyV => {
                update_key(KeyType::KeyV);
            }
            KeyCode::KeyW => {
                update_key(KeyType::KeyW);
            }
            KeyCode::KeyX => {
                update_key(KeyType::KeyX);
            }
            KeyCode::KeyY => {
                update_key(KeyType::KeyY);
            }
            KeyCode::KeyZ => {
                update_key(KeyType::KeyZ);
            }

            // 数字键 0-9
            KeyCode::Digit0 => {
                update_key(KeyType::Digit0);
            }
            KeyCode::Digit1 => {
                update_key(KeyType::Digit1);
            }
            KeyCode::Digit2 => {
                update_key(KeyType::Digit2);
            }
            KeyCode::Digit3 => {
                update_key(KeyType::Digit3);
            }
            KeyCode::Digit4 => {
                update_key(KeyType::Digit4);
            }
            KeyCode::Digit5 => {
                update_key(KeyType::Digit5);
            }
            KeyCode::Digit6 => {
                update_key(KeyType::Digit6);
            }
            KeyCode::Digit7 => {
                update_key(KeyType::Digit7);
            }
            KeyCode::Digit8 => {
                update_key(KeyType::Digit8);
            }
            KeyCode::Digit9 => {
                update_key(KeyType::Digit9);
            }

            _ => {}
        }
    }

    /// 取出并清除一次性 Alt 按下事件（用于 jump trigger）
    pub fn take_alt_pressed_once(&mut self) -> bool {
        let v = self.alt_pressed_once;
        self.alt_pressed_once = false;
        v
    }

    pub fn take_space_pressed_once(&mut self) -> bool {
        let v = self.space_pressed_once;
        self.space_pressed_once = false;
        v
    }

    pub fn take_f1_pressed_once(&mut self) -> bool {
        let v = self.f1_pressed_once;
        self.f1_pressed_once = false;
        v
    }

    /// 平台轮询：纯tao事件驱动模式
    ///
    /// 说明：之前使用GetAsyncKeyState是为了解决Alt键触发系统菜单后方向键事件被吞掉的问题。
    /// 现在改为纯事件驱动，所有按键状态都由handle_keyboard_input更新。
    ///
    /// 如果遇到Alt键问题，应在窗口创建时禁用Alt菜单行为，而不是绕过事件系统。
    /// 例如：在vga256.rs创建窗口时使用with_skip_taskbar等选项。
    pub fn poll_os_keys(&mut self) {
        // 纯事件驱动模式：所有按键状态已由handle_keyboard_input更新
        // 此函数保留为兼容性接口，不再调用Windows API
        //
        // 注意：如果在Windows上按Alt键后出现方向键失效问题，
        // 请检查窗口是否正确处理了WM_SYSCOMMAND消息
    }

    /// 检测指定扫描码的按键是否按下
    /// 使用内部状态而非Windows API，实现跨平台兼容
    pub fn kb_key(&self, scan_code: u8) -> bool {
        // 扫描码到KeyType的映射
        let key_type = match scan_code {
            // 特殊键
            15 => Some(KeyType::Tab),
            1 => Some(KeyType::Escape),
            28 => Some(KeyType::Enter),
            57 => Some(KeyType::Space),

            // 字母键 A-Z (扫描码)
            30 => Some(KeyType::KeyA),
            48 => Some(KeyType::KeyB),
            46 => Some(KeyType::KeyC),
            32 => Some(KeyType::KeyD),
            18 => Some(KeyType::KeyE),
            33 => Some(KeyType::KeyF),
            34 => Some(KeyType::KeyG),
            35 => Some(KeyType::KeyH),
            23 => Some(KeyType::KeyI),
            36 => Some(KeyType::KeyJ),
            37 => Some(KeyType::KeyK),
            38 => Some(KeyType::KeyL),
            50 => Some(KeyType::KeyM),
            49 => Some(KeyType::KeyN),
            24 => Some(KeyType::KeyO),
            25 => Some(KeyType::KeyP),
            16 => Some(KeyType::KeyQ),
            19 => Some(KeyType::KeyR),
            31 => Some(KeyType::KeyS),
            20 => Some(KeyType::KeyT),
            22 => Some(KeyType::KeyU),
            47 => Some(KeyType::KeyV),
            17 => Some(KeyType::KeyW),
            45 => Some(KeyType::KeyX),
            21 => Some(KeyType::KeyY),
            44 => Some(KeyType::KeyZ),

            // 数字键 0-9 (扫描码)
            11 => Some(KeyType::Digit0),
            2 => Some(KeyType::Digit1),
            3 => Some(KeyType::Digit2),
            4 => Some(KeyType::Digit3),
            5 => Some(KeyType::Digit4),
            6 => Some(KeyType::Digit5),
            7 => Some(KeyType::Digit6),
            8 => Some(KeyType::Digit7),
            9 => Some(KeyType::Digit8),
            10 => Some(KeyType::Digit9),

            _ => None,
        };

        match key_type {
            Some(kt) => *self.key_states.get(&kt).unwrap_or(&false),
            None => false,
        }
    }

    // 原始按键状态读取
    // 说明1: 这些方法不走 MacroSystem，不会修改录制/回放状态
    // 说明2: 仅用于输入链路诊断或作为底层状态读取
    pub fn raw_left(&self) -> bool {
        *self.key_states.get(&KeyType::Left).unwrap_or(&false)
    }
    pub fn raw_right(&self) -> bool {
        *self.key_states.get(&KeyType::Right).unwrap_or(&false)
    }
    pub fn raw_up(&self) -> bool {
        *self.key_states.get(&KeyType::Up).unwrap_or(&false)
    }
    pub fn raw_down(&self) -> bool {
        *self.key_states.get(&KeyType::Down).unwrap_or(&false)
    }
    pub fn raw_alt(&self) -> bool {
        *self.key_states.get(&KeyType::Alt).unwrap_or(&false)
    }
    pub fn raw_ctrl(&self) -> bool {
        *self.key_states.get(&KeyType::Ctrl).unwrap_or(&false)
    }
    pub fn raw_space(&self) -> bool {
        *self.key_states.get(&KeyType::Space).unwrap_or(&false)
    }
    pub fn raw_left_shift(&self) -> bool {
        *self.key_states.get(&KeyType::ShiftLeft).unwrap_or(&false)
    }
    pub fn raw_right_shift(&self) -> bool {
        *self.key_states.get(&KeyType::ShiftRight).unwrap_or(&false)
    }

    /// 将平台 KeyCode 转换为扫描码
    fn platform_keycode_to_scan_code(&self, keycode: KeyCode) -> Option<u8> {
        match keycode {
            KeyCode::Digit1 => Some(ScanCode::Kb1 as u8),
            KeyCode::Digit2 => Some(ScanCode::Kb2 as u8),
            KeyCode::Digit3 => Some(ScanCode::Kb3 as u8),
            KeyCode::Digit4 => Some(ScanCode::Kb4 as u8),
            KeyCode::Digit5 => Some(ScanCode::Kb5 as u8),
            KeyCode::Digit6 => Some(ScanCode::Kb6 as u8),
            KeyCode::Digit7 => Some(ScanCode::Kb7 as u8),
            KeyCode::Digit8 => Some(ScanCode::Kb8 as u8),
            KeyCode::Digit9 => Some(ScanCode::Kb9 as u8),
            KeyCode::Digit0 => Some(ScanCode::Kb0 as u8),

            KeyCode::KeyQ => Some(ScanCode::KbQ as u8),
            KeyCode::KeyW => Some(ScanCode::KbW as u8),
            KeyCode::KeyE => Some(ScanCode::KbE as u8),
            KeyCode::KeyR => Some(ScanCode::KbR as u8),
            KeyCode::KeyT => Some(ScanCode::KbT as u8),
            KeyCode::KeyY => Some(ScanCode::KbY as u8),
            KeyCode::KeyU => Some(ScanCode::KbU as u8),
            KeyCode::KeyI => Some(ScanCode::KbI as u8),
            KeyCode::KeyO => Some(ScanCode::KbO as u8),
            KeyCode::KeyP => Some(ScanCode::KbP as u8),

            KeyCode::KeyA => Some(ScanCode::KbA as u8),
            KeyCode::KeyS => Some(ScanCode::KbS as u8),
            KeyCode::KeyD => Some(ScanCode::KbD as u8),
            KeyCode::KeyF => Some(ScanCode::KbF as u8),
            KeyCode::KeyG => Some(ScanCode::KbG as u8),
            KeyCode::KeyH => Some(ScanCode::KbH as u8),
            KeyCode::KeyJ => Some(ScanCode::KbJ as u8),
            KeyCode::KeyK => Some(ScanCode::KbK as u8),
            KeyCode::KeyL => Some(ScanCode::KbL as u8),

            KeyCode::KeyZ => Some(ScanCode::KbZ as u8),
            KeyCode::KeyX => Some(ScanCode::KbX as u8),
            KeyCode::KeyC => Some(ScanCode::KbC as u8),
            KeyCode::KeyV => Some(ScanCode::KbV as u8),
            KeyCode::KeyB => Some(ScanCode::KbB as u8),
            KeyCode::KeyN => Some(ScanCode::KbN as u8),
            KeyCode::KeyM => Some(ScanCode::KbM as u8),

            KeyCode::Escape => Some(ScanCode::KbEsc as u8),
            KeyCode::F1 => Some(ScanCode::KbF1 as u8),
            KeyCode::F2 => Some(ScanCode::KbF2 as u8),
            KeyCode::Backspace => Some(ScanCode::KbBS as u8),
            KeyCode::Tab => Some(ScanCode::KbTab as u8),
            KeyCode::Enter => Some(ScanCode::KbEnter as u8),
            KeyCode::Space => Some(ScanCode::KbSpace as u8),
            KeyCode::Up => Some(ScanCode::KbUpArrow as u8),
            KeyCode::Left => Some(ScanCode::KbLeftArrow as u8),
            KeyCode::Right => Some(ScanCode::KbRightArrow as u8),
            KeyCode::Down => Some(ScanCode::KbDownArrow as u8),

            // 对齐 Pascal KEYBOARD.PAS 的常用扫描码
            // Ctrl press = 29, Alt press = 56, ShiftL press = 42, ShiftR press = 54
            KeyCode::ControlLeft | KeyCode::ControlRight => Some(29),
            KeyCode::AltLeft | KeyCode::AltRight => Some(56),
            KeyCode::ShiftLeft => Some(42),
            KeyCode::ShiftRight => Some(54),

            _ => None,
        }
    }

    /// 将扫描码转换为ASCII字符 (对应原Pascal代码的GetAsciiCode函数)
    pub fn scan_code_to_ascii(&self, scan_code: u8) -> Option<char> {
        const KB_TABLE: [&str; 4] = [
            "1234567890", // 数字行 (扫描码 2-11)
            "QWERTYUIOP", // 第一字母行 (扫描码 16-25)
            "ASDFGHJKL",  // 第二字母行 (扫描码 30-38)
            "ZXCVBNM",    // 第三字母行 (扫描码 44-50)
        ];

        match scan_code {
            2..=11 => KB_TABLE[0].chars().nth((scan_code - 2) as usize),
            16..=25 => KB_TABLE[1].chars().nth((scan_code - 16) as usize),
            30..=38 => KB_TABLE[2].chars().nth((scan_code - 30) as usize),
            44..=50 => KB_TABLE[3].chars().nth((scan_code - 44) as usize),
            _ => None,
        }
    }

    // 公共接口函数 (对应原Pascal代码中的函数)

    pub fn kb_hit(&mut self) -> bool {
        let hit = self.key_hit;
        self.key_hit = false;
        hit
    }

    pub fn kb_left(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Left).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Left.macro_index().unwrap(), pressed)
    }

    pub fn kb_right(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Right).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Right.macro_index().unwrap(), pressed)
    }

    pub fn kb_up(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Up).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Up.macro_index().unwrap(), pressed)
    }

    pub fn kb_down(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Down).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Down.macro_index().unwrap(), pressed)
    }

    pub fn kb_alt(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Alt).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Alt.macro_index().unwrap(), pressed)
    }

    pub fn kb_ctrl(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Ctrl).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Ctrl.macro_index().unwrap(), pressed)
    }

    pub fn kb_left_shift(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::ShiftLeft).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::ShiftLeft.macro_index().unwrap(), pressed)
    }

    pub fn kb_right_shift(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::ShiftRight).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::ShiftRight.macro_index().unwrap(), pressed)
    }

    pub fn kb_space(&mut self) -> bool {
        let pressed = *self.key_states.get(&KeyType::Space).unwrap_or(&false);
        self.macro_system
            .check_key(KeyType::Space.macro_index().unwrap(), pressed)
    }

    /// 直接查询原始按键状态（不经过宏/连发系统），供 debug observation 使用
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_right(&self) -> bool {
        *self.key_states.get(&KeyType::Right).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_left(&self) -> bool {
        *self.key_states.get(&KeyType::Left).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_up(&self) -> bool {
        *self.key_states.get(&KeyType::Up).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_down(&self) -> bool {
        *self.key_states.get(&KeyType::Down).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_alt(&self) -> bool {
        *self.key_states.get(&KeyType::Alt).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_ctrl(&self) -> bool {
        *self.key_states.get(&KeyType::Ctrl).unwrap_or(&false)
    }
    #[cfg(feature = "debug-bridge")]
    pub fn raw_key_space(&self) -> bool {
        *self.key_states.get(&KeyType::Space).unwrap_or(&false)
    }

    // 宏操作接口
    pub fn record_macro(&mut self) {
        self.macro_system.start_recording();
    }

    pub fn play_macro(&mut self) {
        self.macro_system.start_playing();
    }

    /// 播放Demo（加载预录制的第6关演示按键序列）
    pub fn play_demo(&mut self) {
        self.macro_system.start_demo();
    }

    pub fn stop_macro(&mut self) {
        self.macro_system.stop();
    }

    pub fn save_macro(&mut self) -> Result<(), std::io::Error> {
        self.macro_system.save_macro()
    }

    pub fn playing_macro(&self) -> bool {
        self.macro_system.is_playing()
    }

    // 获取当前按键信息
    pub fn get_current_key(&self) -> Option<char> {
        self.current_key
    }

    pub fn get_current_scan_code(&self) -> Option<u8> {
        self.current_scan_code
    }

    pub fn get_ascii_code(&self, c: char) -> Option<char> {
        // 直接返回字符，游戏主要使用扫描码而非ASCII码
        Some(c)
    }

    /// 清除当前按键状态 (相当于Pascal中的 Key := #255)
    /// 清除当前按键状态 (相当于Pascal中的 Key := #255)
    pub fn clear_key(&mut self) {
        self.current_key = None;
        self.current_scan_code = None;
        self.key_hit = false;
    }
}
