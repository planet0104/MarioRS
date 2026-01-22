//! 触摸控制面板公共模块
//!
//! 提供跨平台的虚拟按键渲染和触摸输入处理
//! 使用 tiny-skia 进行 2D 绘图，输出 RGBA buffer 供各平台渲染
//! 支持按钮布局编辑和持久化保存

use super::{KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent};

// ============================================================================
// 常量定义
// ============================================================================

// 布局保存的文件名
pub const LAYOUT_SAVE_KEY: &str = "touch_layout.dat";

// 按钮尺寸常量
const DEFAULT_DPAD_SIZE: f32 = 300.0;       // D-Pad 尺寸
const DEFAULT_BUTTON_SIZE: f32 = 200.0;     // 右侧按钮尺寸
const DEFAULT_SCREEN_WIDTH: f32 = 1080.0;   // 参考屏幕宽度
const DEFAULT_SCREEN_HEIGHT: f32 = 1920.0;  // 参考屏幕高度

// 编辑按钮常量
const EDIT_BUTTON_RADIUS: f32 = 75.0;       // 编辑按钮半径
const EDIT_BUTTON_MARGIN: f32 = 90.0;       // 编辑按钮距离屏幕边缘
const EDIT_BUTTON_Y: f32 = 90.0;            // 编辑按钮 Y 坐标
const EDIT_BUTTON_SPACING: f32 = 180.0;     // 编辑按钮和重置按钮间距

// 暂停按钮常量
const PAUSE_BUTTON_Y: f32 = 250.0;          // 暂停按钮 Y 坐标

// ============================================================================
// PNG 图片资源 (编译时内嵌) - 仅启用 touch-panel feature 时
// ============================================================================

#[cfg(feature = "touch-panel")]
mod assets {
    use tiny_skia::Pixmap;

    // 内嵌 PNG 图片数据
    static DPAD_PNG: &[u8] =
        include_bytes!("../../assets/onscreen_controls/pngs/transparency/d-pad_3.png");
    static BUTTON_A_PNG: &[u8] =
        include_bytes!("../../assets/onscreen_controls/pngs/transparency/a_button.png");
    static BUTTON_B_PNG: &[u8] =
        include_bytes!("../../assets/onscreen_controls/pngs/transparency/b_button.png");
    static BUTTON_X_PNG: &[u8] =
        include_bytes!("../../assets/onscreen_controls/pngs/transparency/x_button.png");
    static BUTTON_Y_PNG: &[u8] =
        include_bytes!("../../assets/onscreen_controls/pngs/transparency/y_button.png");

    /// 从 PNG 字节解码为 Pixmap
    fn decode_png(data: &[u8]) -> Option<Pixmap> {
        let img = image::load_from_memory(data).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        Pixmap::from_vec(rgba.into_raw(), tiny_skia::IntSize::from_wh(w, h)?)
    }

    /// 按钮图片资源
    pub struct ButtonAssets {
        pub dpad: Option<Pixmap>,
        pub button_a: Option<Pixmap>,
        pub button_b: Option<Pixmap>,
        pub button_x: Option<Pixmap>,
        pub button_y: Option<Pixmap>,
    }

    impl ButtonAssets {
        pub fn load() -> Self {
            Self {
                dpad: decode_png(DPAD_PNG),
                button_a: decode_png(BUTTON_A_PNG),
                button_b: decode_png(BUTTON_B_PNG),
                button_x: decode_png(BUTTON_X_PNG),
                button_y: decode_png(BUTTON_Y_PNG),
            }
        }

        pub fn get_dpad(&self) -> Option<&Pixmap> {
            self.dpad.as_ref()
        }
        pub fn get_button(&self, name: &str) -> Option<&Pixmap> {
            match name {
                "a" => self.button_a.as_ref(),
                "b" => self.button_b.as_ref(),
                "x" => self.button_x.as_ref(),
                "y" => self.button_y.as_ref(),
                _ => None,
            }
        }
    }

    impl Default for ButtonAssets {
        fn default() -> Self {
            Self::load()
        }
    }
}

#[cfg(feature = "touch-panel")]
use assets::ButtonAssets;

#[cfg(not(feature = "touch-panel"))]
pub struct ButtonAssets;

#[cfg(not(feature = "touch-panel"))]
impl ButtonAssets {
    pub fn load() -> Self {
        Self
    }
}

#[cfg(not(feature = "touch-panel"))]
impl Default for ButtonAssets {
    fn default() -> Self {
        Self::load()
    }
}

// ============================================================================
// 触摸动作枚举
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchAction {
    Down,
    Move,
    Up,
    Cancel,
}

// ============================================================================
// 按钮状态
// ============================================================================

#[derive(Default, Clone, Copy, Debug)]
pub struct ButtonStates {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub pause: bool,
}

// ============================================================================
// 按钮布局数据 (屏幕坐标系，比例值 0.0-1.0)
// ============================================================================

/// 按钮布局 - 使用屏幕比例坐标 (0.0-1.0)
#[derive(Clone, Copy, Debug)]
pub struct ButtonLayout {
    // D-Pad 位置 (左上角，比例)
    pub dpad_x: f32,
    pub dpad_y: f32,
    // 右侧按钮位置 (中心点，比例)
    pub button_a_x: f32,
    pub button_a_y: f32,
    pub button_b_x: f32,
    pub button_b_y: f32,
    pub button_x_x: f32,
    pub button_x_y: f32,
    pub button_y_x: f32,
    pub button_y_y: f32,
}

impl ButtonLayout {
    /// 默认布局 (基于比例坐标)
    pub fn default_layout() -> Self {
        // D-Pad 在左下角
        let dpad_x = 0.0; // 距离左边 2%
        let dpad_y = 0.68; // 距离顶部 68% (稍微上移给更大的 D-Pad 留空间)

        // 右侧按钮组在右下角
        let right_x = 0.93; // 距离左边 82%
        let right_y = 0.82; // 距离顶部 82%
        let spacing_h = 0.12; // 水平间距 12%
        let spacing_v = 0.22; // 垂直间距 22%

        Self {
            dpad_x,
            dpad_y,
            button_a_x: right_x,
            button_a_y: right_y,
            button_x_x: right_x - spacing_h,
            button_x_y: right_y,
            button_b_x: right_x,
            button_b_y: right_y - spacing_v,
            button_y_x: right_x - spacing_h,
            button_y_y: right_y - spacing_v,
        }
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(40);
        data.extend_from_slice(&self.dpad_x.to_le_bytes());
        data.extend_from_slice(&self.dpad_y.to_le_bytes());
        data.extend_from_slice(&self.button_a_x.to_le_bytes());
        data.extend_from_slice(&self.button_a_y.to_le_bytes());
        data.extend_from_slice(&self.button_b_x.to_le_bytes());
        data.extend_from_slice(&self.button_b_y.to_le_bytes());
        data.extend_from_slice(&self.button_x_x.to_le_bytes());
        data.extend_from_slice(&self.button_x_y.to_le_bytes());
        data.extend_from_slice(&self.button_y_x.to_le_bytes());
        data.extend_from_slice(&self.button_y_y.to_le_bytes());
        data
    }

    /// 从字节数组反序列化
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 40 {
            return None;
        }
        Some(Self {
            dpad_x: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            dpad_y: f32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            button_a_x: f32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            button_a_y: f32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            button_b_x: f32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            button_b_y: f32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            button_x_x: f32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            button_x_y: f32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            button_y_x: f32::from_le_bytes([data[32], data[33], data[34], data[35]]),
            button_y_y: f32::from_le_bytes([data[36], data[37], data[38], data[39]]),
        })
    }
}

impl Default for ButtonLayout {
    fn default() -> Self {
        Self::default_layout()
    }
}

// ============================================================================
// 虚拟按键渲染器 - 使用 tiny-skia
// ============================================================================

#[cfg(feature = "touch-panel")]
pub struct VirtualButtonsRenderer {
    // 屏幕尺寸
    screen_width: u32,
    screen_height: u32,

    // 渲染缓冲区
    pixmap: Option<tiny_skia::Pixmap>,

    // 布局 (比例坐标)
    layout: ButtonLayout,

    // 按钮尺寸 (像素)
    dpad_size: f32,
    button_size: f32,

    // 编辑模式
    edit_mode: bool,

    // 图片资源
    assets: ButtonAssets,

    // 渲染缓存标志 - 只在需要时重新渲染
    needs_redraw: bool,
}

#[cfg(feature = "touch-panel")]
impl VirtualButtonsRenderer {
    pub fn new() -> Self {
        Self {
            screen_width: 0,
            screen_height: 0,
            pixmap: None,
            layout: ButtonLayout::default_layout(),
            dpad_size: DEFAULT_DPAD_SIZE,
            button_size: DEFAULT_BUTTON_SIZE,
            edit_mode: false,
            assets: ButtonAssets::load(),
            needs_redraw: true,
        }
    }

    /// 设置屏幕尺寸并重新分配缓冲区
    pub fn set_screen_size(&mut self, width: u32, height: u32) {
        if self.screen_width != width || self.screen_height != height {
            self.screen_width = width;
            self.screen_height = height;

            // 根据屏幕尺寸调整按钮大小
            let scale =
                (width.min(height) as f32) / DEFAULT_SCREEN_WIDTH.min(DEFAULT_SCREEN_HEIGHT);
            self.dpad_size = DEFAULT_DPAD_SIZE * scale;
            self.button_size = DEFAULT_BUTTON_SIZE * scale;

            // 重新分配 Pixmap
            if let Some(size) = tiny_skia::IntSize::from_wh(width, height) {
                self.pixmap = tiny_skia::Pixmap::new(size.width(), size.height());
            }
            self.needs_redraw = true;
        }
    }

    /// 获取屏幕尺寸
    pub fn screen_size(&self) -> (u32, u32) {
        (self.screen_width, self.screen_height)
    }

    /// 应用布局
    pub fn apply_layout(&mut self, layout: &ButtonLayout) {
        self.layout = *layout;
        self.needs_redraw = true;
    }

    /// 获取当前布局
    pub fn get_layout(&self) -> ButtonLayout {
        self.layout
    }

    /// 重置为默认布局
    pub fn reset_layout(&mut self) {
        self.layout = ButtonLayout::default_layout();
        self.needs_redraw = true;
    }

    /// 设置编辑模式
    pub fn set_edit_mode(&mut self, edit: bool) {
        if self.edit_mode != edit {
            self.edit_mode = edit;
            self.needs_redraw = true;
        }
    }

    pub fn is_edit_mode(&self) -> bool {
        self.edit_mode
    }

    /// 标记需要重新渲染
    pub fn mark_dirty(&mut self) {
        self.needs_redraw = true;
    }

    /// 渲染并返回 RGBA buffer (带缓存，只在需要时重新绘制)
    pub fn render(&mut self, _states: &ButtonStates) -> Option<&[u8]> {
        // 如果不需要重新渲染，直接返回缓存的数据
        if !self.needs_redraw {
            return self.pixmap.as_ref().map(|p| p.data());
        }

        // 标记已渲染
        self.needs_redraw = false;

        // 先获取需要的值，避免借用冲突
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        let dpad_x = (self.layout.dpad_x * w) as i32;
        let dpad_y = (self.layout.dpad_y * h) as i32;
        let dpad_size_u32 = self.dpad_size as u32;
        let btn_size = self.button_size as u32;
        let half = (btn_size / 2) as i32;
        let edit_mode = self.edit_mode;

        // 收集按钮布局信息
        let buttons = [
            ("a", self.layout.button_a_x, self.layout.button_a_y),
            ("b", self.layout.button_b_x, self.layout.button_b_y),
            ("x", self.layout.button_x_x, self.layout.button_x_y),
            ("y", self.layout.button_y_x, self.layout.button_y_y),
        ];

        // 编辑按钮参数
        let edit_btn_x = self.screen_width as f32 - EDIT_BUTTON_MARGIN;
        let edit_btn_y = EDIT_BUTTON_Y;
        let edit_btn_r = EDIT_BUTTON_RADIUS;
        let reset_btn_x = self.screen_width as f32 - EDIT_BUTTON_MARGIN - EDIT_BUTTON_SPACING;

        // 暂停按钮参数 (E按钮下方)
        let pause_btn_x = self.screen_width as f32 - EDIT_BUTTON_MARGIN;
        let pause_btn_y = PAUSE_BUTTON_Y;

        // 编辑高亮参数
        let dpad_cx = self.layout.dpad_x * w + self.dpad_size / 2.0;
        let dpad_cy = self.layout.dpad_y * h + self.dpad_size / 2.0;
        let dpad_highlight_r = self.dpad_size / 2.0 + 5.0;
        let btn_r = self.button_size / 2.0 + 5.0;

        let pixmap = self.pixmap.as_mut()?;

        // 清空为透明
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // 绘制 D-Pad
        if let Some(src) = self.assets.get_dpad() {
            Self::draw_scaled_image_static(
                pixmap,
                src,
                dpad_x,
                dpad_y,
                dpad_size_u32,
                dpad_size_u32,
            );
        }

        // 绘制右侧按钮
        for (name, bx, by) in buttons {
            if let Some(src) = self.assets.get_button(name) {
                let px = (bx * w) as i32 - half;
                let py = (by * h) as i32 - half;
                Self::draw_scaled_image_static(pixmap, src, px, py, btn_size, btn_size);
            }
        }

        // 绘制编辑按钮 (右上角)
        let edit_color = if edit_mode {
            tiny_skia::Color::from_rgba8(100, 200, 100, 180)
        } else {
            tiny_skia::Color::from_rgba8(150, 150, 150, 150)
        };
        Self::draw_circle(pixmap, edit_btn_x, edit_btn_y, edit_btn_r, edit_color);
        Self::draw_letter_static(pixmap, edit_btn_x, edit_btn_y, "E", tiny_skia::Color::WHITE);

        // 绘制暂停按钮 (编辑按钮下方)
        let pause_color = tiny_skia::Color::from_rgba8(100, 100, 200, 180);
        Self::draw_circle(pixmap, pause_btn_x, pause_btn_y, edit_btn_r, pause_color);
        Self::draw_letter_static(
            pixmap,
            pause_btn_x,
            pause_btn_y,
            "P",
            tiny_skia::Color::WHITE,
        );

        if edit_mode {
            // 绘制重置按钮
            let reset_color = tiny_skia::Color::from_rgba8(200, 100, 100, 180);
            Self::draw_circle(pixmap, reset_btn_x, edit_btn_y, edit_btn_r, reset_color);
            Self::draw_letter_static(
                pixmap,
                reset_btn_x,
                edit_btn_y,
                "R",
                tiny_skia::Color::WHITE,
            );

            // 绘制编辑高亮边框
            let highlight_color = tiny_skia::Color::from_rgba8(255, 255, 0, 200);
            Self::draw_circle_outline(pixmap, dpad_cx, dpad_cy, dpad_highlight_r, highlight_color);
            for (bx, by) in [
                (buttons[0].1, buttons[0].2),
                (buttons[1].1, buttons[1].2),
                (buttons[2].1, buttons[2].2),
                (buttons[3].1, buttons[3].2),
            ] {
                Self::draw_circle_outline(pixmap, bx * w, by * h, btn_r, highlight_color);
            }
        }

        Some(pixmap.data())
    }

    /// 缩放绘制图片 (静态方法)
    fn draw_scaled_image_static(
        dst: &mut tiny_skia::Pixmap,
        src: &tiny_skia::Pixmap,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) {
        let scale_x = w as f32 / src.width() as f32;
        let scale_y = h as f32 / src.height() as f32;

        let transform =
            tiny_skia::Transform::from_translate(x as f32, y as f32).pre_scale(scale_x, scale_y);

        let pattern = tiny_skia::Pattern::new(
            src.as_ref(),
            tiny_skia::SpreadMode::Pad,
            tiny_skia::FilterQuality::Bilinear,
            1.0,
            tiny_skia::Transform::identity(),
        );

        let paint = tiny_skia::Paint {
            shader: pattern,
            blend_mode: tiny_skia::BlendMode::SourceOver,
            anti_alias: true,
            force_hq_pipeline: false,
        };

        let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, src.width() as f32, src.height() as f32);
        if let Some(rect) = rect {
            let path = tiny_skia::PathBuilder::from_rect(rect);
            dst.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        }
    }

    /// 绘制实心圆
    fn draw_circle(
        pixmap: &mut tiny_skia::Pixmap,
        cx: f32,
        cy: f32,
        r: f32,
        color: tiny_skia::Color,
    ) {
        if let Some(path) = tiny_skia::PathBuilder::from_circle(cx, cy, r) {
            let paint = tiny_skia::Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                blend_mode: tiny_skia::BlendMode::SourceOver,
                anti_alias: true,
                force_hq_pipeline: false,
            };
            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    /// 绘制圆形边框
    fn draw_circle_outline(
        pixmap: &mut tiny_skia::Pixmap,
        cx: f32,
        cy: f32,
        r: f32,
        color: tiny_skia::Color,
    ) {
        if let Some(path) = tiny_skia::PathBuilder::from_circle(cx, cy, r) {
            let stroke = tiny_skia::Stroke {
                width: 3.0,
                ..Default::default()
            };
            let paint = tiny_skia::Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                blend_mode: tiny_skia::BlendMode::SourceOver,
                anti_alias: true,
                force_hq_pipeline: false,
            };
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    /// 绘制简单字母 (静态方法)
    fn draw_letter_static(
        pixmap: &mut tiny_skia::Pixmap,
        cx: f32,
        cy: f32,
        letter: &str,
        color: tiny_skia::Color,
    ) {
        let patterns: &[(&str, &[&str])] = &[
            ("E", &["#####", "#    ", "#####", "#    ", "#####"]),
            ("P", &["#### ", "#   #", "#### ", "#    ", "#    "]),
            ("R", &["#### ", "#   #", "#### ", "#  # ", "#   #"]),
        ];

        if let Some((_, pattern)) = patterns.iter().find(|(l, _)| *l == letter) {
            let scale = 6.0; // 字母缩放 (与编辑按钮放大比例匹配)
            let h = pattern.len() as f32 * scale;
            let w = pattern[0].len() as f32 * scale;
            let start_x = cx - w / 2.0;
            let start_y = cy - h / 2.0;

            let paint = tiny_skia::Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                blend_mode: tiny_skia::BlendMode::SourceOver,
                anti_alias: false,
                force_hq_pipeline: false,
            };

            for (row, line) in pattern.iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    if ch == '#' {
                        let rect = tiny_skia::Rect::from_xywh(
                            start_x + col as f32 * scale,
                            start_y + row as f32 * scale,
                            scale,
                            scale,
                        );
                        if let Some(rect) = rect {
                            pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
    }

    // ========== 碰撞检测方法 ==========

    /// D-Pad 中心点 (屏幕像素)
    pub fn dpad_center(&self) -> (f32, f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        (
            self.layout.dpad_x * w + self.dpad_size / 2.0,
            self.layout.dpad_y * h + self.dpad_size / 2.0,
        )
    }

    /// D-Pad 尺寸
    pub fn dpad_size(&self) -> f32 {
        self.dpad_size
    }

    /// 按钮位置和半径 (屏幕像素)
    pub fn button_a(&self) -> (f32, f32, f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        (
            self.layout.button_a_x * w,
            self.layout.button_a_y * h,
            self.button_size / 2.0,
        )
    }

    pub fn button_b(&self) -> (f32, f32, f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        (
            self.layout.button_b_x * w,
            self.layout.button_b_y * h,
            self.button_size / 2.0,
        )
    }

    pub fn button_x(&self) -> (f32, f32, f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        (
            self.layout.button_x_x * w,
            self.layout.button_x_y * h,
            self.button_size / 2.0,
        )
    }

    pub fn button_y(&self) -> (f32, f32, f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        (
            self.layout.button_y_x * w,
            self.layout.button_y_y * h,
            self.button_size / 2.0,
        )
    }

    /// 编辑按钮位置和半径
    pub fn edit_button(&self) -> (f32, f32, f32) {
        (
            self.screen_width as f32 - EDIT_BUTTON_MARGIN,
            EDIT_BUTTON_Y,
            EDIT_BUTTON_RADIUS,
        )
    }

    /// 重置按钮位置和半径
    pub fn reset_button(&self) -> (f32, f32, f32) {
        (
            self.screen_width as f32 - EDIT_BUTTON_MARGIN - EDIT_BUTTON_SPACING,
            EDIT_BUTTON_Y,
            EDIT_BUTTON_RADIUS,
        )
    }

    /// 暂停按钮位置和半径 (在编辑按钮下方)
    pub fn pause_button(&self) -> (f32, f32, f32) {
        (
            self.screen_width as f32 - EDIT_BUTTON_MARGIN,
            PAUSE_BUTTON_Y,
            EDIT_BUTTON_RADIUS,
        )
    }

    /// 检测点是否在编辑按钮上
    pub fn is_on_edit_button(&self, x: f32, y: f32) -> bool {
        let (ex, ey, r) = self.edit_button();
        let dx = x - ex;
        let dy = y - ey;
        dx * dx + dy * dy <= r * r * 2.25 // 1.5x 触摸区域
    }

    /// 检测点是否在重置按钮上
    pub fn is_on_reset_button(&self, x: f32, y: f32) -> bool {
        let (rx, ry, r) = self.reset_button();
        let dx = x - rx;
        let dy = y - ry;
        dx * dx + dy * dy <= r * r * 2.25
    }

    /// 检测点是否在暂停按钮上
    pub fn is_on_pause_button(&self, x: f32, y: f32) -> bool {
        let (px, py, r) = self.pause_button();
        let dx = x - px;
        let dy = y - py;
        dx * dx + dy * dy <= r * r * 2.25
    }

    /// 获取所有需要混合的边界框 (x, y, width, height)
    /// 用于优化 overlay 混合 - 只混合按钮区域而不是整个屏幕
    pub fn get_blend_rects(&self) -> Vec<(u32, u32, u32, u32)> {
        let mut rects = Vec::with_capacity(8);
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;

        // D-Pad 边界框
        let dpad_x = (self.layout.dpad_x * w) as i32;
        let dpad_y = (self.layout.dpad_y * h) as i32;
        let dpad_size = self.dpad_size as u32;
        rects.push((
            dpad_x.max(0) as u32,
            dpad_y.max(0) as u32,
            dpad_size.min(self.screen_width.saturating_sub(dpad_x.max(0) as u32)),
            dpad_size.min(self.screen_height.saturating_sub(dpad_y.max(0) as u32)),
        ));

        // 右侧 4 个按钮边界框
        let btn_size = self.button_size as u32;
        let half = btn_size / 2;
        for (bx, by) in [
            (self.layout.button_a_x, self.layout.button_a_y),
            (self.layout.button_b_x, self.layout.button_b_y),
            (self.layout.button_x_x, self.layout.button_x_y),
            (self.layout.button_y_x, self.layout.button_y_y),
        ] {
            let cx = (bx * w) as i32;
            let cy = (by * h) as i32;
            let x = (cx - half as i32).max(0) as u32;
            let y = (cy - half as i32).max(0) as u32;
            rects.push((
                x,
                y,
                btn_size.min(self.screen_width.saturating_sub(x)),
                btn_size.min(self.screen_height.saturating_sub(y)),
            ));
        }

        // 编辑按钮边界框
        let (ex, ey, er) = self.edit_button();
        let edit_size = (er * 2.0) as u32;
        let edit_x = (ex - er).max(0.0) as u32;
        let edit_y = (ey - er).max(0.0) as u32;
        rects.push((
            edit_x,
            edit_y,
            edit_size.min(self.screen_width.saturating_sub(edit_x)),
            edit_size.min(self.screen_height.saturating_sub(edit_y)),
        ));

        // 暂停按钮边界框 (始终显示)
        let (px, py, pr) = self.pause_button();
        let pause_size = (pr * 2.0) as u32;
        let pause_x = (px - pr).max(0.0) as u32;
        let pause_y = (py - pr).max(0.0) as u32;
        rects.push((
            pause_x,
            pause_y,
            pause_size.min(self.screen_width.saturating_sub(pause_x)),
            pause_size.min(self.screen_height.saturating_sub(pause_y)),
        ));

        // 如果在编辑模式，添加重置按钮边界框
        if self.edit_mode {
            let (rx, ry, rr) = self.reset_button();
            let reset_size = (rr * 2.0) as u32;
            let reset_x = (rx - rr).max(0.0) as u32;
            let reset_y = (ry - rr).max(0.0) as u32;
            rects.push((
                reset_x,
                reset_y,
                reset_size.min(self.screen_width.saturating_sub(reset_x)),
                reset_size.min(self.screen_height.saturating_sub(reset_y)),
            ));
        }

        rects
    }

    // ========== 编辑模式位置更新 ==========

    pub fn set_dpad_position(&mut self, x: f32, y: f32) {
        let w = self.screen_width as f32;
        let h = self.screen_height as f32;
        self.layout.dpad_x = ((x - self.dpad_size / 2.0) / w).clamp(0.0, 1.0 - self.dpad_size / w);
        self.layout.dpad_y = ((y - self.dpad_size / 2.0) / h).clamp(0.0, 1.0 - self.dpad_size / h);
        self.needs_redraw = true;
    }

    pub fn set_button_a_position(&mut self, x: f32, y: f32) {
        self.layout.button_a_x = (x / self.screen_width as f32).clamp(0.05, 0.95);
        self.layout.button_a_y = (y / self.screen_height as f32).clamp(0.05, 0.95);
        self.needs_redraw = true;
    }

    pub fn set_button_b_position(&mut self, x: f32, y: f32) {
        self.layout.button_b_x = (x / self.screen_width as f32).clamp(0.05, 0.95);
        self.layout.button_b_y = (y / self.screen_height as f32).clamp(0.05, 0.95);
        self.needs_redraw = true;
    }

    pub fn set_button_x_position(&mut self, x: f32, y: f32) {
        self.layout.button_x_x = (x / self.screen_width as f32).clamp(0.05, 0.95);
        self.layout.button_x_y = (y / self.screen_height as f32).clamp(0.05, 0.95);
        self.needs_redraw = true;
    }

    pub fn set_button_y_position(&mut self, x: f32, y: f32) {
        self.layout.button_y_x = (x / self.screen_width as f32).clamp(0.05, 0.95);
        self.layout.button_y_y = (y / self.screen_height as f32).clamp(0.05, 0.95);
        self.needs_redraw = true;
    }
}

#[cfg(feature = "touch-panel")]
impl Default for VirtualButtonsRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 非 touch-panel 的空实现
// ============================================================================

#[cfg(not(feature = "touch-panel"))]
pub struct VirtualButtonsRenderer;

#[cfg(not(feature = "touch-panel"))]
impl VirtualButtonsRenderer {
    pub fn new() -> Self {
        Self
    }
    pub fn set_screen_size(&mut self, _: u32, _: u32) {}
    pub fn screen_size(&self) -> (u32, u32) {
        (0, 0)
    }
    pub fn apply_layout(&mut self, _: &ButtonLayout) {}
    pub fn get_layout(&self) -> ButtonLayout {
        ButtonLayout::default()
    }
    pub fn reset_layout(&mut self) {}
    pub fn set_edit_mode(&mut self, _: bool) {}
    pub fn is_edit_mode(&self) -> bool {
        false
    }
    pub fn mark_dirty(&mut self) {}
    pub fn render(&mut self, _: &ButtonStates) -> Option<&[u8]> {
        None
    }
    pub fn dpad_center(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
    pub fn dpad_size(&self) -> f32 {
        0.0
    }
    pub fn button_a(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn button_b(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn button_x(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn button_y(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn edit_button(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn reset_button(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn pause_button(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub fn is_on_edit_button(&self, _: f32, _: f32) -> bool {
        false
    }
    pub fn is_on_reset_button(&self, _: f32, _: f32) -> bool {
        false
    }
    pub fn is_on_pause_button(&self, _: f32, _: f32) -> bool {
        false
    }
    pub fn get_blend_rects(&self) -> Vec<(u32, u32, u32, u32)> {
        Vec::new()
    }
    pub fn set_dpad_position(&mut self, _: f32, _: f32) {}
    pub fn set_button_a_position(&mut self, _: f32, _: f32) {}
    pub fn set_button_b_position(&mut self, _: f32, _: f32) {}
    pub fn set_button_x_position(&mut self, _: f32, _: f32) {}
    pub fn set_button_y_position(&mut self, _: f32, _: f32) {}
}

#[cfg(not(feature = "touch-panel"))]
impl Default for VirtualButtonsRenderer {
    fn default() -> Self {
        Self
    }
}

// ============================================================================
// 触摸面板输入处理
// ============================================================================

/// 拖拽目标
#[derive(Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    None,
    Dpad,
    ButtonA,
    ButtonB,
    ButtonX,
    ButtonY,
}

/// 触摸控制类型
#[derive(Clone, Copy, PartialEq, Eq)]
enum TouchControl {
    Dpad,
    ButtonA,
    ButtonB,
    ButtonX,
    ButtonY,
}

/// 单个触摸点的状态
struct TouchPointer {
    control: Option<TouchControl>,
}

/// 触摸面板输入处理器
pub struct TouchPanelInput {
    renderer: VirtualButtonsRenderer,
    button_states: ButtonStates,
    pending_events: Vec<PlatformKeyEvent>,
    layout_changed: bool,

    // 多点触摸支持
    pointers: std::collections::HashMap<usize, TouchPointer>,

    // 编辑模式
    drag_target: DragTarget,
}

impl TouchPanelInput {
    pub fn new() -> Self {
        Self {
            renderer: VirtualButtonsRenderer::new(),
            button_states: ButtonStates::default(),
            pending_events: Vec::new(),
            layout_changed: false,
            pointers: std::collections::HashMap::new(),
            drag_target: DragTarget::None,
        }
    }

    /// 设置屏幕尺寸
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.renderer.set_screen_size(width as u32, height as u32);
    }

    /// 获取渲染器引用
    pub fn renderer(&self) -> &VirtualButtonsRenderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut VirtualButtonsRenderer {
        &mut self.renderer
    }

    /// 获取按钮状态
    pub fn button_states(&self) -> ButtonStates {
        self.button_states
    }

    /// 应用布局
    pub fn apply_layout(&mut self, layout: &ButtonLayout) {
        self.renderer.apply_layout(layout);
    }

    /// 获取布局
    pub fn get_layout(&self) -> ButtonLayout {
        self.renderer.get_layout()
    }

    /// 重置布局
    pub fn reset_layout(&mut self) {
        self.renderer.reset_layout();
        self.layout_changed = true;
    }

    /// 切换编辑模式
    pub fn toggle_edit_mode(&mut self) {
        let new_mode = !self.renderer.is_edit_mode();
        self.renderer.set_edit_mode(new_mode);
        if !new_mode {
            self.layout_changed = true;
        }
    }

    /// 检查布局是否改变
    pub fn take_layout_changed(&mut self) -> bool {
        let changed = self.layout_changed;
        self.layout_changed = false;
        changed
    }

    /// 获取待处理的键盘事件
    pub fn take_pending_events(&mut self) -> Vec<PlatformKeyEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// 处理触摸事件 (屏幕坐标)
    pub fn handle_touch(&mut self, pointer_id: usize, x: f32, y: f32, action: TouchAction) {
        if self.renderer.is_edit_mode() {
            self.handle_edit_touch(pointer_id, x, y, action);
        } else {
            self.handle_game_touch(pointer_id, x, y, action);
        }
    }

    /// 游戏模式触摸处理
    fn handle_game_touch(&mut self, pointer_id: usize, x: f32, y: f32, action: TouchAction) {
        match action {
            TouchAction::Down => {
                // 检查编辑按钮
                if self.renderer.is_on_edit_button(x, y) {
                    self.toggle_edit_mode();
                    return;
                }

                // 检查暂停按钮 - 发送 P 键按下事件
                if self.renderer.is_on_pause_button(x, y) {
                    // 只发送按下事件，按钮状态保持一帧让游戏检测到
                    self.button_states.pause = true;
                    self.emit_key(PlatformKeyCode::KeyP, true);
                    self.renderer.mark_dirty(); // 标记需要重绘
                    return;
                }

                // 检测触摸了哪个控件
                if let Some(control) = self.detect_control(x, y) {
                    self.pointers.insert(
                        pointer_id,
                        TouchPointer {
                            control: Some(control),
                        },
                    );
                    self.update_button_state(control, true, x, y);
                }
            }
            TouchAction::Move => {
                if let Some(pointer) = self.pointers.get(&pointer_id) {
                    if let Some(control) = pointer.control {
                        self.update_button_state(control, true, x, y);
                    }
                }
            }
            TouchAction::Up | TouchAction::Cancel => {
                // 释放暂停按钮 (如果之前按下)
                if self.button_states.pause {
                    self.button_states.pause = false;
                    self.emit_key(PlatformKeyCode::KeyP, false);
                    self.renderer.mark_dirty();
                }

                if let Some(pointer) = self.pointers.remove(&pointer_id) {
                    if let Some(control) = pointer.control {
                        self.release_control(control);
                    }
                }
            }
        }
    }

    /// 编辑模式触摸处理
    fn handle_edit_touch(&mut self, _pointer_id: usize, x: f32, y: f32, action: TouchAction) {
        match action {
            TouchAction::Down => {
                // 检查编辑按钮
                if self.renderer.is_on_edit_button(x, y) {
                    self.toggle_edit_mode();
                    return;
                }

                // 检查重置按钮
                if self.renderer.is_on_reset_button(x, y) {
                    self.reset_layout();
                    return;
                }

                // 检测拖拽目标
                self.drag_target = self.detect_drag_target(x, y);
                if self.drag_target != DragTarget::None {
                    self.move_drag_target(x, y);
                }
            }
            TouchAction::Move => {
                if self.drag_target != DragTarget::None {
                    self.move_drag_target(x, y);
                }
            }
            TouchAction::Up | TouchAction::Cancel => {
                if self.drag_target != DragTarget::None {
                    self.layout_changed = true;
                    self.drag_target = DragTarget::None;
                }
            }
        }
    }

    /// 检测触摸的控件
    fn detect_control(&self, x: f32, y: f32) -> Option<TouchControl> {
        // 检测 D-Pad
        let (dpad_cx, dpad_cy) = self.renderer.dpad_center();
        let dpad_r = self.renderer.dpad_size() / 2.0;
        if Self::in_circle(x, y, dpad_cx, dpad_cy, dpad_r * 1.2) {
            return Some(TouchControl::Dpad);
        }

        // 检测按钮
        let touch_scale = 1.3;
        let (ax, ay, ar) = self.renderer.button_a();
        if Self::in_circle(x, y, ax, ay, ar * touch_scale) {
            return Some(TouchControl::ButtonA);
        }

        let (bx, by, br) = self.renderer.button_b();
        if Self::in_circle(x, y, bx, by, br * touch_scale) {
            return Some(TouchControl::ButtonB);
        }

        let (xx, xy, xr) = self.renderer.button_x();
        if Self::in_circle(x, y, xx, xy, xr * touch_scale) {
            return Some(TouchControl::ButtonX);
        }

        let (yx, yy, yr) = self.renderer.button_y();
        if Self::in_circle(x, y, yx, yy, yr * touch_scale) {
            return Some(TouchControl::ButtonY);
        }

        None
    }

    /// 检测拖拽目标
    fn detect_drag_target(&self, x: f32, y: f32) -> DragTarget {
        let (dpad_cx, dpad_cy) = self.renderer.dpad_center();
        let dpad_r = self.renderer.dpad_size() / 2.0;
        if Self::in_circle(x, y, dpad_cx, dpad_cy, dpad_r * 1.2) {
            return DragTarget::Dpad;
        }

        let (ax, ay, ar) = self.renderer.button_a();
        if Self::in_circle(x, y, ax, ay, ar * 1.5) {
            return DragTarget::ButtonA;
        }

        let (bx, by, br) = self.renderer.button_b();
        if Self::in_circle(x, y, bx, by, br * 1.5) {
            return DragTarget::ButtonB;
        }

        let (xx, xy, xr) = self.renderer.button_x();
        if Self::in_circle(x, y, xx, xy, xr * 1.5) {
            return DragTarget::ButtonX;
        }

        let (yx, yy, yr) = self.renderer.button_y();
        if Self::in_circle(x, y, yx, yy, yr * 1.5) {
            return DragTarget::ButtonY;
        }

        DragTarget::None
    }

    /// 移动拖拽目标
    fn move_drag_target(&mut self, x: f32, y: f32) {
        match self.drag_target {
            DragTarget::Dpad => self.renderer.set_dpad_position(x, y),
            DragTarget::ButtonA => self.renderer.set_button_a_position(x, y),
            DragTarget::ButtonB => self.renderer.set_button_b_position(x, y),
            DragTarget::ButtonX => self.renderer.set_button_x_position(x, y),
            DragTarget::ButtonY => self.renderer.set_button_y_position(x, y),
            DragTarget::None => {}
        }
    }

    fn in_circle(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> bool {
        let dx = x - cx;
        let dy = y - cy;
        dx * dx + dy * dy <= r * r
    }

    /// 更新按钮状态
    fn update_button_state(&mut self, control: TouchControl, pressed: bool, x: f32, y: f32) {
        match control {
            TouchControl::Dpad => {
                let (cx, cy) = self.renderer.dpad_center();
                let dx = x - cx;
                let dy = y - cy;
                let threshold = self.renderer.dpad_size() * 0.15;

                let new_left = dx < -threshold;
                let new_right = dx > threshold;
                let new_up = dy < -threshold;
                let new_down = dy > threshold;

                // 内联更新方向键状态，避免借用冲突
                if new_left != self.button_states.left {
                    self.button_states.left = new_left;
                    self.pending_events.push(PlatformKeyEvent {
                        key: PlatformKeyCode::Left,
                        pressed: new_left,
                    });
                }
                if new_right != self.button_states.right {
                    self.button_states.right = new_right;
                    self.pending_events.push(PlatformKeyEvent {
                        key: PlatformKeyCode::Right,
                        pressed: new_right,
                    });
                }
                if new_up != self.button_states.up {
                    self.button_states.up = new_up;
                    self.pending_events.push(PlatformKeyEvent {
                        key: PlatformKeyCode::Up,
                        pressed: new_up,
                    });
                }
                if new_down != self.button_states.down {
                    self.button_states.down = new_down;
                    self.pending_events.push(PlatformKeyEvent {
                        key: PlatformKeyCode::Down,
                        pressed: new_down,
                    });
                }
            }
            TouchControl::ButtonA => {
                // A = 跳跃 (Alt)
                if pressed != self.button_states.a {
                    self.button_states.a = pressed;
                    self.emit_key(PlatformKeyCode::AltLeft, pressed);
                }
            }
            TouchControl::ButtonB => {
                // B = 加速/跑步 (Ctrl)
                if pressed != self.button_states.b {
                    self.button_states.b = pressed;
                    self.emit_key(PlatformKeyCode::ControlLeft, pressed);
                }
            }
            TouchControl::ButtonX => {
                // X = 发射火球 (Space)
                if pressed != self.button_states.x {
                    self.button_states.x = pressed;
                    self.emit_key(PlatformKeyCode::Space, pressed);
                }
            }
            TouchControl::ButtonY => {
                // Y = 备用 (Shift，暂未使用)
                if pressed != self.button_states.y {
                    self.button_states.y = pressed;
                    self.emit_key(PlatformKeyCode::ShiftLeft, pressed);
                }
            }
        }
    }

    /// 释放控件
    fn release_control(&mut self, control: TouchControl) {
        match control {
            TouchControl::Dpad => {
                if self.button_states.left {
                    self.emit_key(PlatformKeyCode::Left, false);
                }
                if self.button_states.right {
                    self.emit_key(PlatformKeyCode::Right, false);
                }
                if self.button_states.up {
                    self.emit_key(PlatformKeyCode::Up, false);
                }
                if self.button_states.down {
                    self.emit_key(PlatformKeyCode::Down, false);
                }
                self.button_states.left = false;
                self.button_states.right = false;
                self.button_states.up = false;
                self.button_states.down = false;
            }
            TouchControl::ButtonA => {
                // A = 跳跃 (Alt)
                if self.button_states.a {
                    self.button_states.a = false;
                    self.emit_key(PlatformKeyCode::AltLeft, false);
                }
            }
            TouchControl::ButtonB => {
                // B = 加速/跑步 (Ctrl)
                if self.button_states.b {
                    self.button_states.b = false;
                    self.emit_key(PlatformKeyCode::ControlLeft, false);
                }
            }
            TouchControl::ButtonX => {
                // X = 发射火球 (Space)
                if self.button_states.x {
                    self.button_states.x = false;
                    self.emit_key(PlatformKeyCode::Space, false);
                }
            }
            TouchControl::ButtonY => {
                // Y = 备用 (Shift)
                if self.button_states.y {
                    self.button_states.y = false;
                    self.emit_key(PlatformKeyCode::ShiftLeft, false);
                }
            }
        }
    }

    fn emit_key(&mut self, key: PlatformKeyCode, pressed: bool) {
        self.pending_events.push(PlatformKeyEvent { key, pressed });
    }
}

impl Default for TouchPanelInput {
    fn default() -> Self {
        Self::new()
    }
}
