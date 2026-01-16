// 严格根据 PLAYERS.PAS 转换�?Rust 结构体框�?
// 仅实现结构体和字段声�?
// 变量、常量、类型、方法名均严格对�?Pascal 原文

use crate::{
    backgr::BackGr,
    blocks::Blocks,
    buffers::{
        Buffers, CAN_HOLD_YOU, CAN_STAND_ON, DIR_LEFT, DIR_RIGHT, DM_DEAD, DM_DOWN_INTO_PIPE,
        DM_DOWN_OUT_OF_PIPE, DM_NO_DEMO, DM_UP_INTO_PIPE, DM_UP_OUT_OF_PIPE, EY1, H, HIDDEN, MD_FIRE, MD_LARGE, MD_SMALL, NH, NV, PL_LUIGI, PL_MARIO, PictureBufferFill, W,
        WorldOptions,
    },
    enemies::{
        Enemies, TP_FIREBALL, TP_RISING_CHAMP, TP_RISING_FLOWER, TP_RISING_LIFE, TP_RISING_STAR,
    },
    figures::Figures,
    glitter::GlitterSystem,
    music::MusicPlayer,
    sprites::SpriteDataManager,
    tmpobj::{TP_NOTE, TmpObjManager},
    vga256::{MAX_PAGE, VGA},
    gpu::sprite_batch::SpriteCommand,
    gpu::texture_atlas::SpriteUV,
};

// 常量定义
pub const SAFE: i32 = EY1;
pub const H_SAFE: i32 = H * SAFE;
pub const ST_ON_THE_GROUND: u8 = 0;
pub const ST_JUMPING: u8 = 1;
pub const ST_FALLING: u8 = 2;
pub const SCROLL_AT: i32 = 112;
pub const JUMP_VEL: i32 = 4;
pub const JUMP_DELAY: i32 = 6;
pub const MAX_Y_VEL: i32 = JUMP_VEL * 2;
pub const SLIP: i32 = 6;
pub const BLINK_TIME: i32 = 125;
pub const STAR_TIME: i32 = 750;
pub const GROW_TIME: i32 = 24;
pub const MAX_SPEED: i32 = 2;

// 类型常量
const PLANE_H: usize = 2 * H as usize;
const W_DIV4: usize = W as usize / 4;

#[derive(Clone, Default)]
pub struct ScreenRec {
    pub visible: bool,
    pub xpos: i32,
    pub ypos: i32,
    // pub buffer: PicBuffer, // 如需实现可解注释
    // Pascal: BackGrAddr: Word; 0表示无效
    pub backgr_addr: i32,
}

// 结构体定�?
pub struct Players {
    // 状态变�?
    pub blinking: bool,
    pub growing: bool,
    pub in_pipe: bool,
    pub pipe_code: [u8; 2],
    pub map_x: i32,
    pub map_y: i32,
    pub earthquake: bool,
    pub earthquake_counter: i32,
    pub small: i32,
    // 键盘/手柄状�?
    pub key_left: bool,
    pub key_right: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_alt: bool,
    pub key_ctrl: bool,
    pub key_left_shift: bool,
    pub key_right_shift: bool,
    pub key_space: bool,
    pub save_screen: [ScreenRec; MAX_PAGE as usize + 1],
    // 玩家位置与状�?
    pub x: i32,
    pub y: i32,
    pub old_x: i32,
    pub old_y: i32,
    pub demo_x: i32,
    pub demo_y: i32,
    pub demo_counter1: i32,
    pub demo_counter2: i32,
    pub x_vel: i32,
    pub y_vel: i32,
    pub direction: i32,
    pub status: u8,
    pub walking_mode: usize,
    pub counter: u8,
    pub walk_count: u8,
    pub high_jump: bool,
    pub hit_enemy: bool,
    pub jumped: bool,
    pub fired: bool,
    pub fire_counter: i32,
    pub star_counter: i32,
    pub grow_counter: i32,
    pub blink_counter: i32,
    pub at_ch1: u8,
    pub at_ch2: u8,
    pub below1: u8,
    pub below2: u8,
    /// Alt 按下边沿锁存（避免快速点击发生在两帧之间被轮询错过）
    pub alt_pressed_once: bool,
    /// 跳跃按住锁：一旦在跳跃过程中松开跳跃键，则本次跳跃不再允许重新按住恢复大跳效�?
    pub jump_hold_cancelled: bool,
}

impl Players {
    // 框架方法声明
    pub fn new() -> Self {
        Self {
            blinking: false,
            growing: false,
            in_pipe: false,
            pipe_code: [b' ', b' '],
            map_x: 0,
            map_y: 0,
            earthquake: false,
            earthquake_counter: 0,
            small: 0,
            key_left: false,
            key_right: false,
            key_up: false,
            key_down: false,
            key_alt: false,
            key_ctrl: false,
            key_left_shift: false,
            key_right_shift: false,
            key_space: false,
            save_screen: std::array::from_fn(|_| ScreenRec::default()),
            x: 0,
            y: 0,
            old_x: 0,
            old_y: 0,
            demo_x: 0,
            demo_y: 0,
            demo_counter1: 0,
            demo_counter2: 0,
            x_vel: 0,
            y_vel: 0,
            direction: DIR_RIGHT,
            status: ST_ON_THE_GROUND,
            walking_mode: 0,
            counter: 0,
            walk_count: 0,
            high_jump: false,
            hit_enemy: false,
            jumped: false,
            fired: false,
            fire_counter: 0,
            star_counter: 0,
            grow_counter: 0,
            blink_counter: 0,
            at_ch1: 0,
            at_ch2: 0,
            below1: 0,
            below2: 0,
            alt_pressed_once: false,
            jump_hold_cancelled: false,
        }
    }

    /// 根据实际ImageBuffer定义的正确实�?
    pub fn high_mirror(&self, p1: &crate::buffers::PicBuffer, p2: &mut crate::buffers::PicBuffer) {
        // PicBuffer = [[u8; W as usize]; 2*H as usize]
        // 玩家精灵高为2*H，镜像必须覆盖全部行

        // 水平镜像：将每行的字节顺序反�?
        for y in 0..p1.len().min(p2.len()) {
            for x in 0..p1[y].len() {
                if x < p2[y].len() && (W as usize - 1 - x) < p1[y].len() {
                    p2[y][x] = p1[y][W as usize - 1 - x];
                }
            }
        }
    }

    /// Rust严格等价于Pascal InitPlayerFigures过程
    pub fn init_player_figures(&mut self, buffers: &mut Buffers, sprites: &SpriteDataManager) {
        // Fill Pictures array with 0xFF (equivalent to Pascal's #$FF)
        buffers.pictures.fill(0xFF);

        // Move sprite data for Mario
        buffers.pictures[PL_MARIO as usize][MD_SMALL as usize][0][DIR_LEFT as usize] =
            sprites.SWMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_SMALL as usize][1][DIR_LEFT as usize] =
            sprites.SWMAR_001;
        buffers.pictures[PL_MARIO as usize][MD_SMALL as usize][2][DIR_LEFT as usize] =
            sprites.SJMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_SMALL as usize][3][DIR_LEFT as usize] =
            sprites.SJMAR_001;

        buffers.pictures[PL_MARIO as usize][MD_LARGE as usize][0][DIR_LEFT as usize] =
            sprites.LWMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_LARGE as usize][1][DIR_LEFT as usize] =
            sprites.LWMAR_001;
        buffers.pictures[PL_MARIO as usize][MD_LARGE as usize][2][DIR_LEFT as usize] =
            sprites.LJMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_LARGE as usize][3][DIR_LEFT as usize] =
            sprites.LJMAR_001;

        buffers.pictures[PL_MARIO as usize][MD_FIRE as usize][0][DIR_LEFT as usize] =
            sprites.FWMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_FIRE as usize][1][DIR_LEFT as usize] =
            sprites.FWMAR_001;
        buffers.pictures[PL_MARIO as usize][MD_FIRE as usize][2][DIR_LEFT as usize] =
            sprites.FJMAR_000;
        buffers.pictures[PL_MARIO as usize][MD_FIRE as usize][3][DIR_LEFT as usize] =
            sprites.FJMAR_001;

        // Move sprite data for Luigi
        buffers.pictures[PL_LUIGI as usize][MD_SMALL as usize][0][DIR_LEFT as usize] =
            sprites.SWLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_SMALL as usize][1][DIR_LEFT as usize] =
            sprites.SWLUI_001;
        buffers.pictures[PL_LUIGI as usize][MD_SMALL as usize][2][DIR_LEFT as usize] =
            sprites.SJLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_SMALL as usize][3][DIR_LEFT as usize] =
            sprites.SJLUI_001;

        buffers.pictures[PL_LUIGI as usize][MD_LARGE as usize][0][DIR_LEFT as usize] =
            sprites.LWLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_LARGE as usize][1][DIR_LEFT as usize] =
            sprites.LWLUI_001;
        buffers.pictures[PL_LUIGI as usize][MD_LARGE as usize][2][DIR_LEFT as usize] =
            sprites.LJLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_LARGE as usize][3][DIR_LEFT as usize] =
            sprites.LJLUI_001;

        buffers.pictures[PL_LUIGI as usize][MD_FIRE as usize][0][DIR_LEFT as usize] =
            sprites.FWLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_FIRE as usize][1][DIR_LEFT as usize] =
            sprites.FWLUI_001;
        buffers.pictures[PL_LUIGI as usize][MD_FIRE as usize][2][DIR_LEFT as usize] =
            sprites.FJLUI_000;
        buffers.pictures[PL_LUIGI as usize][MD_FIRE as usize][3][DIR_LEFT as usize] =
            sprites.FJLUI_001;

        // Generate right-facing sprites by mirroring left-facing ones
        for pl in PL_MARIO as usize..=PL_LUIGI as usize {
            for md in MD_SMALL as usize..=MD_FIRE as usize {
                for n in 0..4 {
                    let src = buffers.pictures[pl][md][n][DIR_LEFT as usize].clone();
                    self.high_mirror(&src, &mut buffers.pictures[pl][md][n][DIR_RIGHT as usize]);
                }
            }
        }
    }

    pub fn init_player(
        &mut self,
        init_x: i32,
        init_y: i32,
        name: u8,
        buffers: &mut Buffers,
        enemies: &mut Enemies,
    ) {
        buffers.player = name as usize;
        self.x = init_x;
        self.y = init_y;
        self.old_x = self.x;
        self.old_y = self.y;
        self.x_vel = 0;
        self.y_vel = 0;
        self.direction = DIR_RIGHT;
        self.walking_mode = 0;
        self.status = ST_ON_THE_GROUND;
        self.jumped = false;
        self.fired = false;
        self.hit_enemy = false;

        // Initialize SaveScreen visibility
        for i in 0..=MAX_PAGE as usize {
            self.save_screen[i].visible = false;
        }

        // Calculate player boundaries
        enemies.player_x1 = self.x;
        enemies.player_x2 = self.x + W - 1;
        enemies.player_y1 = self.y + H;
        enemies.player_y2 = self.y + 2 * H - 1;

        // Copy velocities to player velocity fields
        enemies.player_x_vel = self.x_vel;
        enemies.player_y_vel = self.y_vel;

        // Initialize special states
        self.blinking = false;
        enemies.star = false;
        self.growing = false;
        self.earthquake = false;
    }

    /// GPU版draw_demo - Demo动画(进出管道/死亡)的GPU精灵渲染
    pub fn draw_demo(
        &mut self,
        buffers: &mut Buffers,
        _figures: &Figures,
        vga: &mut VGA,
        _options: &WorldOptions,
        _backgr: &mut BackGr,
        _sprites: &mut SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
    ) {
        // GPU模式下不需要保存背�?
        let sprite_id = self.get_player_sprite_id_enum(buffers);
        let uv = atlas.get(sprite_id);
        let flip_x = self.direction == DIR_LEFT;

        match buffers.demo {
            DM_DOWN_INTO_PIPE | DM_UP_OUT_OF_PIPE => {
                // 进入管道动画：玩家逐渐消失
                let draw_height = (2 * H - self.demo_y - 1) as f32;
                if draw_height > 0.0 {
                    vga.draw_sprite_partial_world_gpu(
                        self.x,
                        self.y + self.demo_y,
                        uv,
                        draw_height,
                    );
                }
            }
            DM_UP_INTO_PIPE | DM_DOWN_OUT_OF_PIPE => {
                // 从管道出来动画：玩家逐渐出现
                let visible_height = (2 * H + self.demo_y) as f32;
                if visible_height > 0.0 {
                    vga.draw_sprite_partial_world_gpu(
                        self.x,
                        self.y + self.demo_y,
                        uv,
                        visible_height.min(uv.height as f32),
                    );
                }
                // GPU模式下管道会在tilemap层自动渲染，不需要手动redraw
            }
            DM_DEAD => {
                // 死亡动画
                vga.draw_sprite_flipped_world_gpu(self.x, self.y, uv, flip_x, false);
            }
            _ => {}
        }

        // 鏇存柊鏃т綅缃负褰撳墠浣嶇疆
        self.old_x = self.x;
        self.old_y = self.y;
    }

    /// GPU版draw_player - 直接向vga.sprite_batch添加GPU精灵
    pub fn draw_player(
        &mut self,
        buffers: &mut Buffers,
        vga: &mut VGA,
        sprites: &mut SpriteDataManager,
        figures: &Figures,
        options: &WorldOptions,
        backgr: &mut BackGr,
        enemies: &mut Enemies,
        atlas: &crate::sprites::SpriteAtlas,
    ) {
        // Demo模式由draw_demo处理
        if buffers.demo != DM_NO_DEMO {
            self.draw_demo(buffers, figures, vga, options, backgr, sprites, atlas);
            return;
        }
        
        // 闪烁时隔帧不渲染
        if self.blinking && (self.blink_counter % 2 != 0) {
            return;
        }

        // GPU渲染：使用SpriteId从atlas获取UV
        let sprite_id = self.get_player_sprite_id_enum(buffers);
        let flip_x = self.direction == DIR_LEFT;
        let uv = atlas.get(sprite_id);
        
        // 计算调色板偏移（变身/无敌星闪烁效果）
        let palette_offset = if enemies.star || self.growing {
            (((self.grow_counter + self.star_counter) & 1) << 4) as i32
                - ((self.grow_counter + self.star_counter) & 0xF < 8) as i32
        } else {
            0
        };
        
        // 开火动画特殊处�?
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        if mode == MD_FIRE && self.key_space && self.fire_counter < 7 {
            self.fire_counter += 1;
        }
        
        // 添加精灵到GPU渲染队列
        vga.draw_sprite_flipped_world_gpu(self.x, self.y, uv, flip_x, false);
        if palette_offset != 0 {
            vga.draw_sprite_recolored_world_gpu(self.x, self.y, uv, palette_offset);
        }
        
        self.old_x = self.x;
        self.old_y = self.y;
    }

    /// 擦除玩家（GPU模式下为空操作，每帧完整重绘�?
    pub fn erase_player(&mut self, _vga: &mut VGA) {
        // GPU模式下不需要擦除，每帧完整重绘
    }

    // ========== GPU渲染支持方法 ==========

    /// GPU模式：收集玩家精灵渲染命�?
    pub fn collect_player_sprite(
        &self,
        buffers: &Buffers,
        sprite_uv: SpriteUV,
    ) -> Option<SpriteCommand> {
        // 闪烁时隔帧不渲染
        if self.blinking && (self.blink_counter % 2 != 0) {
            return None;
        }

        let player = buffers.player;
        let _mode = buffers.data.mode[player] as usize;
        let flip_x = self.direction == 0; // DIR_LEFT = 0

        let mut cmd = SpriteCommand::new(self.x, self.y, sprite_uv)
            .with_flip(flip_x, false);

        // 变身/无敌星闪烁效�?
        if self.growing || self.star_counter > 0 {
            let color_offset = (((self.grow_counter + self.star_counter) & 1) << 4) as i32;
            cmd = cmd.with_palette(color_offset, 0);
        }

        Some(cmd)
    }

    /// GPU模式：获取当前玩家精灵ID (使用SpriteId枚举)
    pub fn get_player_sprite_id_enum(&self, buffers: &Buffers) -> crate::sprites::SpriteId {
        use crate::sprites::SpriteId;
        
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        let is_mario = player == 0;
        let is_jumping = self.walking_mode != 0;
        
        match (mode, is_jumping, is_mario) {
            (0, false, true) => SpriteId::SWMAR_000,  // Small Walk Mario
            (0, true, true) => SpriteId::SJMAR_000,   // Small Jump Mario
            (1, false, true) => SpriteId::LWMAR_000,  // Large Walk Mario
            (1, true, true) => SpriteId::LJMAR_000,   // Large Jump Mario
            (2, false, true) => SpriteId::FWMAR_000,  // Fire Walk Mario
            (2, true, true) => SpriteId::FJMAR_000,   // Fire Jump Mario
            (0, false, false) => SpriteId::SWLUI_000, // Small Walk Luigi
            (0, true, false) => SpriteId::SJLUI_000,  // Small Jump Luigi
            (1, false, false) => SpriteId::LWLUI_000, // Large Walk Luigi
            (1, true, false) => SpriteId::LJLUI_000,  // Large Jump Luigi
            (2, false, false) => SpriteId::FWLUI_000, // Fire Walk Luigi
            (2, true, false) => SpriteId::FJLUI_000,  // Fire Jump Luigi
            _ => SpriteId::SWMAR_000,
        }
    }
    
    /// GPU模式：完整收集玩家精灵（使用SpriteAtlas自动获取UV�?
    pub fn collect_player_sprites_gpu(
        &self,
        buffers: &Buffers,
        atlas: &crate::sprites::SpriteAtlas,
        palette_index: u32,
    ) -> Vec<SpriteCommand> {
        let mut commands = Vec::new();
        
        // 闪烁时隔帧不渲染
        if self.blinking && (self.blink_counter % 2 != 0) {
            return commands;
        }
        
        // Demo模式检�?
        if buffers.demo != DM_NO_DEMO {
            return commands; // Demo模式由其他方法处�?
        }
        
        let sprite_id = self.get_player_sprite_id_enum(buffers);
        let uv = atlas.get(sprite_id);
        let flip_x = self.direction == 0; // DIR_LEFT = 0
        
        let mut cmd = SpriteCommand::new(self.x, self.y, uv)
            .with_flip(flip_x, false)
            .with_palette(0, palette_index);
        
        // 变身/无敌星闪烁效�?
        if self.growing || self.star_counter > 0 {
            let color_offset = (((self.grow_counter + self.star_counter) & 1) << 4) as i32;
            cmd = cmd.with_palette(color_offset, palette_index);
        }
        
        commands.push(cmd);
        commands
    }
    
    /// GPU模式：获取当前玩家精灵ID (字符串版本，用于调试)
    pub fn get_player_sprite_id(&self, buffers: &Buffers) -> &'static str {
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        
        let _prefix = if player == PL_MARIO as usize { "MAR" } else { "LUI" };
        let mode_char = match mode {
            0 => 'S', // MD_SMALL
            1 => 'L', // MD_LARGE
            2 => 'F', // MD_FIRE
            _ => 'S',
        };
        let action = if self.walking_mode == 0 { 'W' } else { 'J' };
        let _frame = if self.direction == 0 { "000" } else { "001" };
        
        // 返回精灵名称
        let is_mario = player == 0;
        match (mode_char, action, is_mario) {
            ('S', 'W', true) => "SWMAR_000",
            ('S', 'J', true) => "SJMAR_000",
            ('L', 'W', true) => "LWMAR_000",
            ('L', 'J', true) => "LJMAR_000",
            ('F', 'W', true) => "FWMAR_000",
            ('F', 'J', true) => "FJMAR_000",
            ('S', 'W', false) => "SWLUI_000",
            ('S', 'J', false) => "SJLUI_000",
            ('L', 'W', false) => "LWLUI_000",
            ('L', 'J', false) => "LJLUI_000",
            ('F', 'W', false) => "FWLUI_000",
            ('F', 'J', false) => "FJLUI_000",
            _ => "SWMAR_000",
        }
    }

    pub fn do_demo(&mut self, buffers: &mut Buffers) {
        // Small := 9 * Byte (Data.Mode [Player] in [mdSmall]);
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        let small = if mode == MD_SMALL { 9 } else { 0 };
        match buffers.demo {
            DM_DOWN_INTO_PIPE | DM_UP_OUT_OF_PIPE => {
                // Pascal: if PipeCode[1] = 'c' (0xE7) then Passed := TRUE
                // 0xE7 表示关卡完成出口
                if self.pipe_code[0] == 0xE7 {
                    if !buffers.passed {
                        buffers.passed = true;
                        buffers.text_counter = 0;
                    }
                }
                self.demo_counter1 += 1;
                if self.demo_counter1 % 3 == 0 {
                    if buffers.demo == DM_DOWN_INTO_PIPE {
                        // 先递增demo_y
                        self.demo_y += 1;
                        // 如果超过阈值，保持在阈值并增加计数�?
                        if self.demo_y > 2 * H - small {
                            self.demo_y = 2 * H - small; // 保持在阈值位置，不继续增�?
                            self.demo_counter2 += 1;
                            if self.demo_counter2 > 10 {
                                self.in_pipe = true;
                            }
                        }
                    } else {
                        self.demo_y -= 1;
                        if self.demo_y <= 0 {
                            self.demo_y = 0;
                            buffers.demo = DM_NO_DEMO;
                        }
                    }
                }
            }
            DM_UP_INTO_PIPE | DM_DOWN_OUT_OF_PIPE => {
                self.demo_counter1 += 1;
                if self.demo_counter1 % 3 == 0 {
                    if buffers.demo == DM_DOWN_OUT_OF_PIPE {
                        self.demo_y += 1;
                        // Pascal: if DemoY > -Small then Demo := dmNoDemo; Dec(DemoY);
                        // 注意：Pascal结束动画时不恢复Y坐标�?
                        // Y坐标在start_demo时已经调整过，动画结束后保持该位�?
                        let threshold = -small; // 对于大Mario�?，小Mario�?9
                        if self.demo_y > threshold {
                            self.demo_y -= 1;  // Pascal: Dec(DemoY)
                            buffers.demo = DM_NO_DEMO;
                            // �?不要恢复Y坐标！Pascal没有这个操作
                        }
                    } else {
                        // DM_UP_INTO_PIPE
                        self.demo_y -= 1;
                        if self.demo_y < -2 * H + small {
                            self.demo_y = -2 * H + small; // 保持在阈值位置，不继续减�?
                            self.demo_counter2 += 1;
                            if self.demo_counter2 > 10 {
                                self.in_pipe = true;
                            }
                        }
                    }
                }
            }
            DM_DEAD => {
                self.demo_counter1 += 1;
                if self.demo_counter1 % 7 == 0 {
                    self.y_vel += 1;
                }
                self.y += self.y_vel;
                if self.y > NV * H {
                    buffers.game_done = true;
                }
            }
            _ => {}
        }
    }

    pub fn start_demo(&mut self, dm: i32, buffers: &mut Buffers, music_player: &MusicPlayer) {
        buffers.demo = dm;
        self.demo_counter1 = 0;
        self.demo_counter2 = 0;
        self.demo_x = 0;
        self.demo_y = 0;
        self.below1 = b' ';
        self.below2 = b' ';
        self.at_ch1 = b' ';
        self.at_ch2 = b' ';
        if dm == DM_DOWN_INTO_PIPE
            || dm == DM_UP_INTO_PIPE
            || dm == DM_DOWN_OUT_OF_PIPE
            || dm == DM_UP_OUT_OF_PIPE
        {
            music_player.play_pipe();
        }
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        match dm {
            DM_UP_OUT_OF_PIPE => {
                self.demo_y = 2 * H - 9 * if mode == MD_SMALL { 1 } else { 0 };
            }
            DM_DOWN_OUT_OF_PIPE => {
                self.demo_y = -2 * H;
                // 关键修复：对于向下管道，Mario应该从管道底部开�?
                // (MapY-1)*H 给出的是管道tile的顶部Y坐标
                // 管道底部应该�?MapY*H
                // 
                // Pascal原始计算：Inc (Y, H - 7 * Byte (Data.Mode [Player] in [mdSmall]) - 2);
                // 小Mario: offset = 14 - 7 - 2 = 5
                // 大Mario: offset = 14 - 0 - 2 = 12
                //
                // 但这会导致Mario偏离管道底部�?
                // 
                // 正确的做法：调整Y到管道底�?= (MapY-1)*H + H = MapY*H
                // 即：Y += H，而不�?Y += (H - 7*small - 2)
                self.y += H; // 将Y从管道顶部调整到管道底部
            }
            DM_DEAD => {
                self.y_vel = -3;
                music_player.beep(220);
            }
            _ => {}
        }
        self.in_pipe = false;
    }

    pub fn check_pipe_below(&mut self, buffers: &mut Buffers, music_player: &MusicPlayer) {
        if self.x_vel != 0 || self.y_vel != 0 || self.y % H != 0 {
            return;
        }
        
        let mo = self.x % W;
        if mo < 4 || mo > W - 4 {
            return;
        }
        
        // Pascal: AtCh1 in $E0..$E7 AND AtCh2 in $E0..$EF
        // 其中 AtCh2($E8..$EF) 用于跨世界特殊出口编码（例如$E9/$EA/$EB�?
        let below1_ok = self.below1 == b'0';
        let below2_ok = self.below2 == b'1';
        let at_ch1_ok = (0xE0..=0xE7).contains(&self.at_ch1);
        let at_ch2_ok = (0xE0..=0xEF).contains(&self.at_ch2);
        
        if !below1_ok || !below2_ok || !at_ch1_ok || !at_ch2_ok {
            return;
        }
        
        self.pipe_code[0] = self.at_ch1 as u8;
        self.pipe_code[1] = self.at_ch2 as u8;
        self.start_demo(DM_DOWN_INTO_PIPE, buffers, music_player);
    }

    pub fn check_pipe_above(
        &mut self,
        c1: u8,
        c2: u8,
        buffers: &mut Buffers,
        music_player: &MusicPlayer,
    ) {
        let mo = self.x % W;
        if mo < 4 || mo > W - 4 {
            return;
        }
        if c1 != b'0' || c2 != b'1' {
            return;
        }
        self.map_x = self.x / W;
        self.map_y = self.y / H + 1;
        // 重要：WorldMap 访问必须包含 EX/EY1 偏移，否则会读到错误行列，导致管道判定失�?
        let ch1 = buffers.world_get(self.map_x, self.map_y);
        let ch2 = buffers.world_get(self.map_x + 1, self.map_y);
        if !(0xE0..=0xE7).contains(&ch1) || !(0xE0..=0xEF).contains(&ch2) {
            return;
        }
        self.pipe_code[0] = ch1;
        self.pipe_code[1] = ch2;
        self.start_demo(DM_UP_INTO_PIPE, buffers, music_player);
    }

    // CheckFall 宓屽鍑芥暟鐨勫唴瀹圭洿鎺ュ唴鑱斿埌瀵瑰簲浣嶇疆
    fn check_fall(
        &mut self,
        mo: &mut i32,
        hold1: bool,
        hold2: bool,
        new_ch1: u8,
        new_ch2: u8,
        new_x1: i32,
        new_x2: &mut i32,
        new_y: i32,
        ch: &mut u8,
        cd_hit: &mut i32,
        tmp_obj_manager: &mut TmpObjManager,
        blocks: &mut Blocks,
        buffers: &mut Buffers,
        vga: &mut VGA,
        glitter_sys: &mut GlitterSystem,
        music_player: &MusicPlayer,
    ) {
        if !(hold1 || hold2) {
            match new_ch1 {
                b'*' => tmp_obj_manager.hit_coin(
                    new_x1 * W,
                    new_y * H,
                    false,
                    glitter_sys,
                    buffers,
                    music_player,
                ),
                _ => {}
            }
            match new_ch2 {
                b'*' => tmp_obj_manager.hit_coin(
                    *new_x2 * W,
                    new_y * H,
                    false,
                    glitter_sys,
                    buffers,
                    music_player,
                ),
                _ => {}
            }
            if (self.counter as i32 % JUMP_DELAY) == 0 {
                self.y_vel += 1;
            }
            if self.y_vel > MAX_Y_VEL {
                self.y_vel = MAX_Y_VEL;
            }
        } else {
            if new_ch1 == b'=' || new_ch2 == b'=' {
                *cd_hit = 1;
            }

            *mo = (self.x + self.x_vel) % W;
            self.y = ((self.y + self.y_vel + 1 + H_SAFE) / H - SAFE) * H;
            self.y_vel = 0;
            self.status = ST_ON_THE_GROUND;
            self.jumped = true;

            if new_ch1 == b'K' || new_ch2 == b'K' {
                music_player.play_note();
                if new_ch1 == b'K' {
                    blocks.bump_block(new_x1 * W, new_y * H, crate::sprites::SpriteId::NOTE_000);
                    tmp_obj_manager.remove(new_x1 * W, new_y * H, W, H, TP_NOTE);
                    buffers.world_set(new_x1, new_y, b'K');
                }
                if new_ch2 == b'K' {
                    blocks.bump_block(*new_x2 * W, new_y * H, crate::sprites::SpriteId::NOTE_000);
                    tmp_obj_manager.remove(*new_x2 * W, new_y * H, W, H, TP_NOTE);
                    buffers.world_set(*new_x2, new_y, b'K');
                }
                self.counter = 0;
                self.status = ST_JUMPING;
                self.jumped = false;
                self.high_jump = true;
                self.y_vel = -5;
                self.hit_enemy = true;
            }

            if *mo >= 0 && *mo <= W / 2 - 1 {
                if hold1 {
                    *ch = new_ch1;
                    *new_x2 = new_x1;
                } else {
                    *ch = new_ch2;
                }
            // Pascal: mo in [W div 2 .. W - 1]
            } else if *mo >= W / 2 && *mo <= W - 1 {
                if hold2 {
                    *ch = new_ch2;
                } else {
                    *ch = new_ch1;
                    *new_x2 = new_x1;
                }
            }
        }
    }

    // CheckJump 宓屽鍑芥暟鐨勫唴瀹圭洿鎺ュ唴鑱斿埌瀵瑰簲浣嶇疆
    fn check_jump(&mut self, enemies: &mut Enemies) {
        if enemies.cd_enemy != 0 {
            self.hit_enemy = true;
            self.jumped = false;
        }
        if !self.jumped {
            // 触发跳跃�?按下边沿"，高度控制仍�?key_alt(按住)
            if self.alt_pressed_once || self.hit_enemy {
                self.counter = 0;
                self.status = ST_JUMPING;
                self.jump_hold_cancelled = false;
                self.high_jump = (self.x_vel.abs() == 2) || (self.hit_enemy && self.key_alt);
                self.y_vel = -JUMP_VEL
                    - 2 * if self.hit_enemy && self.key_alt { 1 } else { 0 }
                    - if enemies.turbo { 1 } else { 0 };
                // 只有在真正起跳时才消费按下边沿，避免"落地�?Jumped=true时按键被吃掉"导致看起来有冷却
                self.alt_pressed_once = false;
            }
        }
        enemies.cd_enemy = 0;
    }

    /// Rust严格等价于Pascal Check过程
    pub fn check(
        &mut self,
        buffers: &mut Buffers,
        enemies: &mut Enemies,
        tmp_obj_manager: &mut TmpObjManager,
        blocks: &mut Blocks,
        vga: &mut VGA,
        glitter_sys: &mut GlitterSystem,
        music_player: &MusicPlayer,
    ) {
        let mut new_x1: i32;
        let mut new_x2: i32;
        let mut mo: i32 = 0;
        let mut ch: u8 = 0;
        let mut hit: bool;

        let mut new_ch1 = b' ';
        let mut new_ch2 = b' ';
        let mut new_ch3 = b' ';

        let side = if self.x_vel > 0 { W - 1 } else { 0 };
        new_x1 = (self.x + side) / W;
        new_x2 = (self.x + side + self.x_vel) / W;
        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        let small = mode == MD_SMALL;

        if new_x1 != new_x2 {
            let y1 = (self.y + H_SAFE + 4) / H - SAFE;
            let y2 = (self.y + H_SAFE + H) / H - SAFE;
            let y3 = (self.y + H_SAFE + 2 * H - 1) / H - SAFE;
            new_ch1 = buffers.world_get(new_x2, y1);
            new_ch2 = buffers.world_get(new_x2, y2);
            new_ch3 = buffers.world_get(new_x2, y3);

            match new_ch3 {
                b'*' => tmp_obj_manager.hit_coin(
                    new_x2 * W,
                    y3 * H,
                    false,
                    glitter_sys,
                    buffers,
                    music_player,
                ),
                _ => {}
            }
            match new_ch2 {
                b'*' => tmp_obj_manager.hit_coin(
                    new_x2 * W,
                    y2 * H,
                    false,
                    glitter_sys,
                    buffers,
                    music_player,
                ),
                b'z' => enemies.turbo = true,
                _ => {}
            }
            if !small {
                match new_ch1 {
                    b'*' => tmp_obj_manager.hit_coin(
                        new_x2 * W,
                        y1 * H,
                        false,
                        glitter_sys,
                        buffers,
                        music_player,
                    ),
                    _ => {}
                }
            }

            let hold1 = CAN_HOLD_YOU.contains(&new_ch1) && !small;
            let hold2 = CAN_HOLD_YOU.contains(&new_ch2);
            let hold3 = CAN_HOLD_YOU.contains(&new_ch3);

            if hold1 || hold2 || hold3 {
                self.x_vel = 0;
                self.walking_mode = 0;
            }
        }

        new_x1 = (self.x + self.x_vel) / W;
        new_x2 = (self.x + self.x_vel + W - 1) / W;

        if enemies.cd_enemy != 0 {
            self.check_jump(enemies);
        }

        let new_y = if self.status == ST_JUMPING {
            (self.y + 1 + 4 + (H - 1 - 4) * if small { 1 } else { 0 } + self.y_vel + H_SAFE) / H
                - SAFE
        } else {
            (self.y + 1 + 2 * H + self.y_vel + H_SAFE) / H - SAFE
        };

        new_ch1 = buffers.world_get(new_x1, new_y);
        new_ch2 = buffers.world_get(new_x2, new_y);
        new_ch3 = buffers.world_get((self.x + self.x_vel + W / 2) / W, new_y);

        let hold1 = (CAN_HOLD_YOU.contains(&new_ch1)) || (CAN_STAND_ON.contains(&new_ch1));
        let hold2 = (CAN_HOLD_YOU.contains(&new_ch2)) || (CAN_STAND_ON.contains(&new_ch2));
        // let hold3 = (CAN_HOLD_YOU.contains(&new_ch3)) || (CAN_STAND_ON.contains(&new_ch3));

        match self.status {
            ST_FALLING => {
                self.check_fall(
                    &mut mo,
                    hold1,
                    hold2,
                    new_ch1,
                    new_ch2,
                    new_x1,
                    &mut new_x2,
                    new_y,
                    &mut ch,
                    &mut enemies.cd_hit,
                    tmp_obj_manager,
                    blocks,
                    buffers,
                    vga,
                    glitter_sys,
                    music_player,
                );
            }

            ST_ON_THE_GROUND => {
                if enemies.cd_lift == 0 {
                    if !(hold1 || hold2) {
                        self.status = ST_FALLING;
                        if self.x_vel.abs() < 2 {
                            self.y += 1;
                        }
                    } else if new_ch1 == b'K' || new_ch2 == b'K' {
                        self.check_fall(
                            &mut mo,
                            hold1,
                            hold2,
                            new_ch1,
                            new_ch2,
                            new_x1,
                            &mut new_x2,
                            new_y,
                            &mut ch,
                            &mut enemies.cd_hit,
                            tmp_obj_manager,
                            blocks,
                            buffers,
                            vga,
                            glitter_sys,
                            music_player,
                        );
                    } else {
                        if self.x_vel == 0 {
                            self.below1 = new_ch1;
                            self.below2 = new_ch2;
                            self.map_x = new_x1;
                            self.map_y = new_y - 1;
                            self.at_ch1 = buffers.world_get(self.map_x, self.map_y);
                            self.at_ch2 = buffers.world_get(self.map_x + 1, self.map_y);

                            mo = self.x % W;
                            if !hold1 && (1..=5).contains(&mo) {
                                self.x_vel -= 1;
                            }
                            if !hold2 && ((W - 5)..=(W - 1)).contains(&mo) {
                                self.x_vel += 1;
                            }
                        }

                        self.check_jump(enemies);
                    }
                } else {
                    enemies.player_y_vel = self.y_vel;
                    self.check_jump(enemies);
                }
            }

            ST_JUMPING => {
                // 一旦在本次跳跃过程中松开跳跃键，则不允许"重新按住"改变本次跳跃的重力节�?
                if !self.key_alt && !self.hit_enemy {
                    self.jump_hold_cancelled = true;
                }
                let key_hold = self.key_alt && !self.jump_hold_cancelled;

                let hold1 = CAN_HOLD_YOU.contains(&new_ch1) || HIDDEN.contains(&new_ch1);
                let hold2 = CAN_HOLD_YOU.contains(&new_ch2) || HIDDEN.contains(&new_ch2);
                let hold3 = CAN_HOLD_YOU.contains(&new_ch3) || HIDDEN.contains(&new_ch3);

                hit = hold1 || hold2;
                if hit {
                    mo = (self.x + self.x_vel) % W;
                    if ((1..=4).contains(&mo) || ((W - 4)..=W - 1).contains(&mo)) && !hold3 {
                        if !(HIDDEN.contains(&new_ch1) && HIDDEN.contains(&new_ch2)) {
                            hit = false;
                        }
                        if mo < W / 2 && !HIDDEN.contains(&new_ch2) {
                            self.x -= mo;
                        } else if mo >= W / 2 && !HIDDEN.contains(&new_ch1) {
                            self.x += W - mo;
                        }
                    }
                }

                if !hit {
                    match new_ch1 {
                        b'*' => tmp_obj_manager.hit_coin(
                            new_x1 * W,
                            new_y * H,
                            false,
                            glitter_sys,
                            buffers,
                            music_player,
                        ),
                        _ => {}
                    }
                    match new_ch2 {
                        b'*' => tmp_obj_manager.hit_coin(
                            new_x2 * W,
                            new_y * H,
                            false,
                            glitter_sys,
                            buffers,
                            music_player,
                        ),
                        _ => {}
                    }
                    if (self.counter as i32 % (JUMP_DELAY + if self.high_jump { 1 } else { 0 })
                        == 0)
                        || (!key_hold && !self.hit_enemy)
                    {
                        self.y_vel += 1;
                    }
                    if self.y_vel >= 0 {
                        self.y_vel = 0;
                        self.status = ST_FALLING;
                    }
                } else {
                    ch = 0;

                    if mo >= 0 && mo <= W / 2 - 1 {
                        if CAN_HOLD_YOU.contains(&new_ch1) || HIDDEN.contains(&new_ch1) {
                            ch = new_ch1;
                            new_x2 = new_x1;
                        } else {
                            ch = new_ch2;
                        }
                    } else if mo >= W / 2 && mo <= W - 1 {
                        ch = new_ch2;
                        if !(CAN_HOLD_YOU.contains(&ch) || HIDDEN.contains(&ch)) {
                            ch = new_ch1;
                            new_x2 = new_x1;
                        }
                    }

                    match ch {
                        b'=' => enemies.cd_hit = 1,
                        b'0' | b'1' => {
                            if self.key_up {
                                self.check_pipe_above(new_ch1, new_ch2, buffers, music_player);
                            }
                        }
                        b'?' | b'$' | b'J' | b'K' => {
                            mo = 0;
                            let above_ch = buffers.world_get(new_x2, new_y - 1);
                            match above_ch {
                                0xE0..=0xE2 => {
                                    buffers.world_set(new_x2, new_y, b'?');
                                    ch = b'?';
                                }
                                0xEF => {
                                    buffers.world_set(new_x2, new_y, b'K');
                                    ch = b'K';
                                }
                                _ => {
                                    if !small && ch == b'J' {
                                        tmp_obj_manager.break_block(
                                            new_x2,
                                            new_y,
                                            buffers,
                                            music_player,
                                        );
                                        buffers.add_score(10);
                                        mo = 1;
                                    }
                                }
                            }

                            // 只有mo==0时才bump+播放音效
                            if mo == 0 {
                                blocks.bump_block(new_x2 * W, new_y * H, crate::sprites::SpriteId::QUEST_000);

                                // 瀵归�?Pascal锛歅C Speaker 鍚庣画闊虫晥浼氱粓姝㈠綋�?Beep�?
                                // 如果顶出金币或道具，这里跳过 110Hz，直接交给后续音�?
                                let above_ch = buffers.world_get(new_x2, new_y - 1);
                                let coin_sound = match above_ch {
                                    b' ' | 0xE3..=0xEC => !matches!(ch, b'J' | b'K'),
                                    b'*' => true,
                                    _ => false,
                                };
                                let item_sound = matches!(above_ch, 0xE0..=0xE2 | 0xED);

                                if !(coin_sound || item_sound) {
                                    music_player.beep(110);
                                }
                            }

                            {
                                let above_ch = buffers.world_get(new_x2, new_y - 1);
                                match above_ch {
                                    b' ' | 0xE3..=0xEC => {
                                        if ![b'J', b'K'].contains(&ch) {
                                            tmp_obj_manager.hit_coin(
                                                new_x2 * W,
                                                new_y * H,
                                                true,
                                                glitter_sys,
                                                buffers,
                                                music_player,
                                            );
                                            if above_ch != b' ' {
                                                buffers.world_set(
                                                    new_x2,
                                                    new_y - 1,
                                                    above_ch.wrapping_add(1),
                                                );
                                                if ch == b'$' {
                                                    tmp_obj_manager.remove(
                                                        new_x2 * W,
                                                        new_y * H,
                                                        W,
                                                        H,
                                                        2,
                                                    );
                                                    buffers.world_set(new_x2, new_y, b'?');
                                                }
                                            }
                                        }
                                    }
                                    0xE0 => {
                                        if mode == MD_SMALL {
                                            enemies.new_enemy(
                                                TP_RISING_CHAMP,
                                                0,
                                                new_x2,
                                                new_y,
                                                0,
                                                -1,
                                                2,
                                                music_player,
                                            );
                                        } else {
                                            enemies.new_enemy(
                                                TP_RISING_FLOWER,
                                                0,
                                                new_x2,
                                                new_y,
                                                0,
                                                -1,
                                                2,
                                                music_player,
                                            );
                                        }
                                    }
                                    0xE1 => enemies.new_enemy(
                                        TP_RISING_LIFE,
                                        0,
                                        new_x2,
                                        new_y,
                                        0,
                                        -1,
                                        2,
                                        music_player,
                                    ),
                                    0xE2 => enemies.new_enemy(
                                        TP_RISING_STAR,
                                        0,
                                        new_x2,
                                        new_y,
                                        0,
                                        -1,
                                        1,
                                        music_player,
                                    ),
                                    b'*' => tmp_obj_manager.hit_coin(
                                        new_x2 * W,
                                        (new_y - 1) * H,
                                        false,
                                        glitter_sys,
                                        buffers,
                                        music_player,
                                    ),
                                    0xED => enemies.new_enemy(
                                        TP_RISING_CHAMP,
                                        1,
                                        new_x2,
                                        new_y,
                                        0,
                                        -1,
                                        2,
                                        music_player,
                                    ),
                                    _ => {}
                                }
                            }

                            enemies.hit_above(new_x2, new_y - 1, buffers);

                            if ch == b'K' {
                                tmp_obj_manager.remove(new_x2 * W, new_y * H, W, H, TP_NOTE);
                                buffers.world_set(new_x2, new_y, b'K');
                            } else if ch != b'J' {
                                let above_cell = buffers.world_get(new_x2, new_y - 1);
                                if !(0xE3..=0xEC).contains(&above_cell) {
                                    tmp_obj_manager.remove(new_x2 * W, new_y * H, W, H, 1);
                                    buffers.world_set(new_x2, new_y, b'@');
                                }
                            }
                        }
                        _ => {
                            music_player.beep(30);
                        }
                    }

                    if ch != b'J' || mode == MD_SMALL {
                        self.y_vel = 0;
                        self.status = ST_FALLING;
                    }
                    if ch == b'K' {
                        self.y_vel = 3;
                    }
                }
            }
            _ => {}
        }
    }

    /// Rust严格等价于Pascal MovePlayer过程
    pub fn move_player(
        &mut self,
        buffers: &mut Buffers,
        enemies: &mut Enemies,
        tmp_obj_manager: &mut TmpObjManager,
        blocks: &mut Blocks,
        vga: &mut VGA,
        glitter_sys: &mut GlitterSystem,
        music_player: &MusicPlayer,
        options: &WorldOptions,
    ) {
        if self.in_pipe {
            // 重要：WorldMap 访问必须包含 EX/EY1 偏移，否则会读到错误行列
            let cell_below = buffers.world_get(self.map_x, self.map_y + 1);
            if cell_below == b'0' {
                self.start_demo(DM_UP_OUT_OF_PIPE, buffers, music_player);
            } else {
                let cell_above = buffers.world_get(self.map_x, self.map_y - 1);
                if cell_above == b'0' {
                    self.start_demo(DM_DOWN_OUT_OF_PIPE, buffers, music_player);
                }
            }
            return;
        }

        if enemies.cd_champ != 0 {
            let player = buffers.player;
            if (buffers.data.mode[player] as usize) == MD_SMALL {
                buffers.data.mode[player] = MD_LARGE as u8;
                self.growing = true;
                self.grow_counter = 0;
            }
            music_player.play_grow();
            enemies.cd_champ = 0;
        }

        if enemies.cd_life != 0 {
            enemies.cd_life = 0;
            tmp_obj_manager.add_life(buffers, music_player);
        }

        if enemies.cd_flower != 0 {
            let player = buffers.player;
            buffers.data.mode[player] = MD_FIRE as u8;
            self.fired = true;
            self.fire_counter = 0;
            music_player.play_grow();
            self.growing = true;
            self.grow_counter = 0;
            enemies.cd_flower = 0;
        }

        if !self.blinking && !enemies.star && !self.growing {
            if enemies.cd_hit != 0 {
                let player = buffers.player;
                let mode = buffers.data.mode[player] as usize;
                match mode {
                    MD_SMALL => {
                        self.blink_counter = 0;
                        self.blinking = true;
                        self.start_demo(DM_DEAD, buffers, music_player);
                        music_player.play_dead();
                        return;
                    }
                    MD_LARGE | MD_FIRE => {
                        buffers.data.mode[player] = MD_SMALL as u8;
                        self.blink_counter = 0;
                        self.blinking = true;
                        music_player.play_hit();
                    }
                    _ => {}
                }
                enemies.cd_hit = 0;
            }
        } else {
            enemies.cd_hit = 0;
        }

        if self.blinking {
            self.blink_counter += 1;
            if self.blink_counter >= BLINK_TIME {
                self.blinking = false;
            }
        }

        if enemies.cd_star != 0 {
            music_player.play_star();
            self.star_counter = 0;
            enemies.star = true;
        }

        if enemies.star {
            self.star_counter += 1;
            if self.star_counter >= STAR_TIME {
                enemies.star = false;
            }
            if self.star_counter % 3 == 0 {
                let player = buffers.player;
                let mode = buffers.data.mode[player] as usize;
                let offset_y = if mode == MD_SMALL { 11 } else { 0 };
                let offset_h = if mode != MD_SMALL { 3 } else { 0 };
                glitter_sys.start_glitter(self.x, self.y + offset_y, W, H + 3 + offset_h, buffers);
            }
            enemies.cd_star = 0;
        }

        if self.growing {
            self.grow_counter += 1;
            if self.grow_counter > GROW_TIME {
                self.growing = false;
            }
        }

        self.counter = self.counter.wrapping_add(1);
        if self.x_vel == 0 && self.y_vel == 0 {
            self.counter = 0;
        }

        let check_x = (self.counter as i32 % SLIP) == 0;

        let mut old_dir = self.direction as u8;
        let mut old_x_vel = self.x_vel;

        // ReadJoystick() - 杩欓噷鍋囪鎵嬫焺杈撳叆宸茬粡澶勭悊
        // self.read_joystick();

        let last_key_left = self.key_left;
        let last_key_right = self.key_right;

        // 键盘/手柄输入合并 (这里需要根据实际的键盘和手柄状态更�?
        // self.key_left = kb_left || js_left;
        // self.key_right = kb_right || js_right;
        // self.key_up = kb_up || js_up;
        // self.key_down = kb_down || js_down;
        // self.key_alt = kb_alt || js_button1;
        // self.key_ctrl = kb_ctrl || js_button2;
        // self.key_space = kb_space || js_button2;

        if self.key_right && !last_key_right && self.direction == DIR_LEFT {
            old_dir = DIR_RIGHT as u8;
            old_x_vel = -self.x_vel;
        }
        if self.key_left && !last_key_left && self.direction == DIR_RIGHT {
            old_dir = DIR_LEFT as u8;
            old_x_vel = -self.x_vel;
        }

        if self.fired && !self.key_space {
            self.fired = false;
        }

        if self.key_space && !self.fired {
            let player = buffers.player;
            let mode = buffers.data.mode[player] as usize;
            if mode == MD_FIRE {
                self.fire_counter = 0;
                let vel_x = 10 * (-1 + 2 * self.direction);
                let vel_y =
                    3 + 3 * (if self.key_down { 1 } else { 0 } - if self.key_up { 1 } else { 0 });
                enemies.new_enemy(
                    TP_FIREBALL,
                    0,
                    self.x / W + self.direction,
                    (self.y + H) / H,
                    vel_x,
                    vel_y,
                    2,
                    music_player,
                );
                self.fired = true;
            }
        }

        if enemies.cd_lift != 0 {
            self.y = enemies.player_y1;
            self.x_vel = enemies.player_x_vel;
            self.y_vel = enemies.player_y_vel;
            self.status = ST_ON_THE_GROUND;
        }

        if enemies.cd_stop_jump != 0 {
            self.jumped = true;
            enemies.cd_stop_jump = 0;
        }

        if self.jumped && !self.key_alt {
            self.jumped = false;
        }

        let max_speed = MAX_SPEED - 1
            + if self.key_ctrl { 1 } else { 0 }
            + if enemies.turbo { 1 } else { 0 }
            + (enemies.cd_lift * enemies.player_x_vel).abs();
        let min_speed = -MAX_SPEED + 1
            - if self.key_ctrl { 1 } else { 0 }
            - if enemies.turbo { 1 } else { 0 }
            - (enemies.cd_lift * enemies.player_x_vel).abs();

        if self.key_left {
            if self.x_vel > min_speed {
                if check_x || enemies.cd_lift != 0 {
                    let dec_amount = 1 + if enemies.cd_lift != 0 && self.key_ctrl {
                        1
                    } else {
                        0
                    };
                    self.x_vel -= dec_amount;
                }
            } else {
                self.x_vel = min_speed;
            }
            self.direction = if self.x_vel > 0 { 1 } else { 0 };
            if self.x + self.x_vel < 0 {
                self.x_vel = -self.x;
            }
        } else if self.x_vel < 0 && check_x && enemies.cd_lift == 0 {
            self.x_vel += 1;
        }

        if self.key_right {
            if self.x_vel < max_speed {
                if check_x || enemies.cd_lift != 0 {
                    let inc_amount = 1 + if enemies.cd_lift != 0 && self.key_ctrl {
                        1
                    } else {
                        0
                    };
                    self.x_vel += inc_amount;
                }
            } else {
                self.x_vel = max_speed;
            }
            self.direction = if self.x_vel >= 0 { 1 } else { 0 };
        } else if self.x_vel > 0 && check_x && enemies.cd_lift == 0 {
            self.x_vel -= 1;
        }

        if self.key_left && self.key_right {
            self.direction = old_dir as i32;
            self.x_vel = old_x_vel;
        }

        if self.y + self.y_vel >= NV * H {
            buffers.game_done = true;
            music_player.play_dead();
        }

        if self.status == ST_ON_THE_GROUND {
            self.hit_enemy = false;
        }

        self.check(
            buffers,
            enemies,
            tmp_obj_manager,
            blocks,
            vga,
            glitter_sys,
            music_player,
        );

        if self.status == ST_ON_THE_GROUND && self.y_vel == 0 {
            if self.x_vel == 0 || (enemies.cd_lift != 0 && self.x_vel == enemies.player_x_vel) {
                self.walking_mode = 0;
                self.walk_count = 0;
            } else {
                self.walk_count = self.walk_count.wrapping_add(1);
                self.walk_count &= 0xF;
                self.walking_mode = if self.walk_count < 0x8 { 1 } else { 0 };
            }
        } else if self.y_vel < 0 {
            self.walking_mode = 2;
        } else {
            self.walking_mode = 3;
        }

        if self.key_down {
            self.check_pipe_below(buffers, music_player);
        }

        self.x += self.x_vel;
        self.y += self.y_vel;

        let old_x_view = buffers.x_view;
        
        // Pascal 原版滚动逻辑：所有场景（包括地下室）都正常滚�?
        buffers.x_view = buffers.x_view - if self.key_left_shift { 1 } else { 0 }
            + if self.key_right_shift { 1 } else { 0 };

        if self.x + W + SCROLL_AT > buffers.x_view + 320 {
            buffers.x_view = self.x + W + SCROLL_AT - 320;
        }
        if self.x < buffers.x_view + SCROLL_AT {
            buffers.x_view = self.x - SCROLL_AT;
        }
        if buffers.x_view - old_x_view > MAX_SPEED + if enemies.turbo { 1 } else { 0 } {
            buffers.x_view = old_x_view + MAX_SPEED + if enemies.turbo { 1 } else { 0 };
        }
        if buffers.x_view - old_x_view < -MAX_SPEED - if enemies.turbo { 1 } else { 0 } {
            buffers.x_view = old_x_view - MAX_SPEED - if enemies.turbo { 1 } else { 0 };
        }
        if buffers.x_view < 0 {
            buffers.x_view = 0;
            if self.x < 0 {
                self.x = 0;
            }
        }

        // 计算最大x_view，确保不小于0
        // 当地图宽度小于或等于屏幕宽度时，max_x_view应为0（地图不允许滚动�?
        let max_x_view = std::cmp::max(0, ((options.x_size as i32) - NH) * W);
        if buffers.x_view > max_x_view {
            buffers.x_view = max_x_view;
        }

        // 边界检查：左右边界墙检测（所有场景通用�?
        if buffers.x_view < old_x_view {
            let map_x = (buffers.x_view / W) as usize;
            let map_y = NV as usize;
            // 重要：WorldMap 访问必须包含 EX/EY1 偏移
            let cell = buffers.world_get(map_x as i32, map_y as i32);
            if cell == 0xFE {
                let check_map_x = map_x as i32;
                let check_map_y = ((enemies.player_y1 as f32) / (H as f32)).round() as i32;
                let check_cell = buffers.world_get(check_map_x, check_map_y);
                if check_cell != b' ' {
                    buffers.x_view = old_x_view;
                }
            }
        }

        if buffers.x_view > old_x_view {
            let map_x = ((buffers.x_view - 1) / W + NH) as usize;
            let map_y = NV as usize;
            // 重要：WorldMap 访问必须包含 EX/EY1 偏移
            let cell = buffers.world_get(map_x as i32, map_y as i32);
            if cell == 0xFF {
                let check_map_x = map_x as i32;
                let check_map_y = ((enemies.player_y1 as f32) / (H as f32)).round() as i32;
                let check_cell = buffers.world_get(check_map_x, check_map_y);
                if check_cell != b' ' {
                    buffers.x_view = old_x_view;
                }
            }
        }

        enemies.player_x1 = self.x + self.x_vel;
        enemies.player_x2 = enemies.player_x1 + W - 1;
        enemies.player_y1 = self.y;

        let player = buffers.player;
        let mode = buffers.data.mode[player] as usize;
        if mode == MD_SMALL {
            enemies.player_y1 = self.y + H;
        } else {
            enemies.player_y1 = self.y;
        }

        enemies.player_y2 = self.y + 2 * H - 1;
        enemies.player_x_vel = self.x_vel;
        enemies.player_y_vel = self.y_vel;

        if enemies.cd_lift != 0 {
            enemies.player_y_vel += 2 - self.y_vel;
            enemies.cd_lift = 0;
        }
    }
}
