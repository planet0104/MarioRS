// 调色板管理 - 预烘焙调色板动画帧

// 调色板状态索引
pub const PALETTE_NORMAL: u32 = 0;
pub const PALETTE_FADE_START: u32 = 1;
pub const PALETTE_FADE_END: u32 = 16;
pub const PALETTE_BLINK_START: u32 = 17;
pub const PALETTE_BLINK_END: u32 = 32;

// 调色板生成器 - 预烘焙所有动画状态
// 注意：此组件为预留实现，用于未来的调色板动画优化
// 当前版本直接在CPU端修改调色板数据
#[allow(dead_code)]
pub struct PaletteGenerator {
    // 原始调色板颜色 (256色)
    base_palette: [[u8; 4]; 256],
}

impl PaletteGenerator {
    pub fn new(base_palette: [[u8; 4]; 256]) -> Self {
        Self { base_palette }
    }

    // 从sprites模块的PALETTE生成
    pub fn from_sprites_palette(palette: &[(u8, u8, u8, u8); 160]) -> Self {
        let mut base = [[0u8; 4]; 256];
        for (i, &(r, g, b, a)) in palette.iter().enumerate() {
            base[i] = [r, g, b, a];
        }
        Self::new(base)
    }

    // 获取基础调色板
    pub fn base(&self) -> &[[u8; 4]; 256] {
        &self.base_palette
    }

    // 生成淡入/淡出帧 (fade_level: 0-16, 0=全黑, 16=正常)
    pub fn generate_fade(&self, fade_level: u32) -> [[u8; 4]; 256] {
        let mut result = [[0u8; 4]; 256];
        let factor = fade_level as f32 / 16.0;
        
        for i in 0..256 {
            let [r, g, b, a] = self.base_palette[i];
            result[i] = [
                (r as f32 * factor) as u8,
                (g as f32 * factor) as u8,
                (b as f32 * factor) as u8,
                a,
            ];
        }
        result
    }

    // 生成闪烁帧 (blink_phase: 0-15)
    pub fn generate_blink(&self, blink_phase: u32) -> [[u8; 4]; 256] {
        let mut result = self.base_palette;
        
        // 闪烁效果: 特定颜色范围循环
        // 根据原版游戏的调色板动画逻辑
        let phase = blink_phase % 8;
        
        // 金币/问号块闪烁 (索引范围待确定)
        for i in 0..8 {
            let src_idx = 80 + ((i + phase as usize) % 8);
            let dst_idx = 80 + i;
            if src_idx < 256 && dst_idx < 256 {
                result[dst_idx] = self.base_palette[src_idx];
            }
        }
        
        result
    }

    // 生成所有预烘焙调色板帧
    pub fn generate_all_frames(&self) -> Vec<[[u8; 4]; 256]> {
        let mut frames = Vec::with_capacity(64);
        
        // 索引0: 正常调色板
        frames.push(self.base_palette);
        
        // 索引1-16: 淡入/淡出帧
        for level in 1..=16 {
            frames.push(self.generate_fade(level));
        }
        
        // 索引17-32: 闪烁帧
        for phase in 0..16 {
            frames.push(self.generate_blink(phase));
        }
        
        // 填充到64帧
        while frames.len() < 64 {
            frames.push(self.base_palette);
        }
        
        frames
    }
}

// 调色板动画控制器
// 注意：此组件为预留实现，用于未来的GPU端调色板动画
// 当前版本使用CPU端的fade_step等函数处理动画
#[allow(dead_code)]
pub struct PaletteAnimator {
    // 当前帧索引
    current_frame: u32,
    // 淡入淡出状态 (0-16)
    fade_level: u32,
    // 闪烁相位 (0-15)
    blink_phase: u32,
    // 是否启用闪烁
    blink_enabled: bool,
}

impl PaletteAnimator {
    pub fn new() -> Self {
        Self {
            current_frame: PALETTE_NORMAL,
            fade_level: 16,
            blink_phase: 0,
            blink_enabled: false,
        }
    }

    // 设置淡入淡出级别
    pub fn set_fade(&mut self, level: u32) {
        self.fade_level = level.min(16);
        if self.fade_level < 16 {
            self.current_frame = PALETTE_FADE_START + self.fade_level - 1;
        } else {
            self.update_frame();
        }
    }

    // 启用/禁用闪烁
    pub fn set_blink(&mut self, enabled: bool) {
        self.blink_enabled = enabled;
        self.update_frame();
    }

    // 更新闪烁相位 (每帧调用)
    pub fn tick(&mut self) {
        if self.blink_enabled {
            self.blink_phase = (self.blink_phase + 1) % 16;
            self.update_frame();
        }
    }

    // 获取当前调色板帧索引
    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    fn update_frame(&mut self) {
        if self.fade_level < 16 {
            self.current_frame = PALETTE_FADE_START + self.fade_level - 1;
        } else if self.blink_enabled {
            self.current_frame = PALETTE_BLINK_START + self.blink_phase;
        } else {
            self.current_frame = PALETTE_NORMAL;
        }
    }
}

impl Default for PaletteAnimator {
    fn default() -> Self {
        Self::new()
    }
}
