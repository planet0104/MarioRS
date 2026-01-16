//! Intro 欢迎界面模块
//! 严格对应 Pascal MARIO.PAS 中的 Intro 过程

use crate::{
    backgr::BackGr,
    blocks::Blocks,
    buffers::{Buffers, GameData, H, MapBuffer, NH, NV, W, WorldOptions},
    context::GameContext,
    enemies::Enemies,
    figures::Figures,
    glitter::GlitterSystem,
    keyboard::Keyboard,
    music::MusicPlayer,
    play::Play,
    players::Players,
    renderer::{RenderContext, Renderer},
    sprites::SpriteDataManager,
    stars::Stars,
    status::Status,
    tmpobj::TmpObjManager,
    txt::{FontStyle, Txt},
    vga256::{MAX_PAGE, SCREEN_WIDTH, VGA},
};

// Intro阶段日志：默认禁用，需要调试时取消注释下面的println版本
macro_rules! intro_dbg {
    ($($t:tt)*) => { };
    // ($($t:tt)*) => { println!($($t)*); };  // 启用调试日志
}

// ============================================================================
// 介绍画面数据 - 对应 Pascal 的 Intro_0
// ============================================================================

/// 介绍画面地图数据
// 重要：Pascal WORLDS.PAS 的地图数据是"单字节字符"(0..255)。
// Rust源码若用UTF-8字符串，0xF0/0xF7等字节会被错误解码成多字节，导致列错位、地图渲染异常。
// 因此这里按原始字节声明，严格对齐 Pascal 的 db '...'。
pub const INTRO_0_MAP: &[&[u8]] = &[
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0       ",
    b"AA\xF7\xF0         ",
    b"AA\xF7          ",
    b"AA\xF0\xF0\xF0        ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA           ",
    b"AA\xF0\xF0\xF0\xF0\xF0      ",
    b"AA\xF0\xF0\xF0        ",
    b"AA\xF7          ",
    b"AA\xF7          ",
    b"AA           ",
    b"AA           ",
];

/// 介绍画面配置选项 - 对应 Pascal 的 Options_0
pub const INTRO_0_OPTIONS: WorldOptions = WorldOptions {
    init_x: (7 * W + 10) as u16,
    init_y: (9 * H) as u16,
    sky_type: 10,
    wall_type1: 0,
    wall_type2: 0,
    wall_type3: 0,
    pipe_color: 0x30,
    ground_color1: 0x4B,
    ground_color2: 0,
    horizon: 120,
    backgr_type: 10,
    backgr_color1: 0x36,
    backgr_color2: 0x30,
    stars: 0x0,
    clouds: 0x0,
    design: 2,
    c2r: 10,
    c2g: 23,
    c2b: 8,
    c3r: 22,
    c3g: 35,
    c3b: 20,
    brick_color: 0xB0,
    wood_color: 0x48,
    xblock_color: 0xA0,
    build_wall: false,
    x_size: 0,
};

// === Pascal常量移植 ===
pub const NUM_LEV: usize = 6;
pub const LAST_LEV: usize = 2 * NUM_LEV - 1;
pub const MAX_SAVE: usize = 3;
pub const WAIT_BEFORE_DEMO: i32 = 500;
pub const BG_SIZE: usize = (MAX_PAGE as usize) + 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntroPhase {
    NotStarted,         // 尚未开始
    InitData,           // 初始化游戏数据
    PlayIntroWorld,     // 播放Intro世界
    InitBackground,     // 初始化背景
    SetPalette,         // 设置调色板
    DrawIntro,          // 绘制Intro图像
    FadingUp,           // 淡入中（非阻塞）
    MenuLoop,           // 菜单循环
    FadingDownForDemo,  // Demo前淡出
    PlayDemo,           // 播放Demo
    FadingDownForExit,  // 退出前淡出
    Finished,           // 完成
}

pub struct Intro {
    p: i32,
    i: i32,
    j: i32,
    k: i32,
    l: i32,
    wd: i32,
    ht: i32,
    xp: i32,
    next_num_players: i16,
    selected: i32,
    intro_done: bool,
    test_vga_mode: bool,
    update: bool,
    counter: i32,
    macro_key: char,
    status: IntroStatus,
    old_status: IntroStatus,
    last_status: IntroStatus,
    menu: [String; 5],
    bg: [[u16; 5]; BG_SIZE],
    num_options: i32,
    page: usize,

    // 新增：状态机字段
    phase: IntroPhase,
    demo_runs: i32,
    
    // 用于frame_update的缓存数据
    intro_map: Option<MapBuffer>,
    intro_opt: WorldOptions,
    started: bool,
}

use crate::mario::{ConfigData, IntroResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntroStatus {
    None,
    Menu,
    Start,
    Load,
    Erase,
    Options,
    NumPlayers,
}

impl Intro {
    pub fn new() -> Self {
        // Pascal 原版在这里逐个字段初始化，Rust 直接使用下面的默认值
        Self {
            p: 0,
            i: 0,
            j: 0,
            k: 0,
            l: 0,
            wd: 0,
            ht: 0,
            xp: 0,
            next_num_players: 1,
            selected: 1,
            intro_done: false,
            test_vga_mode: false,
            update: true,
            counter: 1,
            macro_key: '\0',
            status: IntroStatus::None,
            old_status: IntroStatus::None,
            last_status: IntroStatus::None,
            menu: Default::default(),
            bg: [[0; 5]; BG_SIZE],
            num_options: 3,
            page: 0,
            phase: IntroPhase::NotStarted,
            demo_runs: 0,
            intro_map: None,
            intro_opt: INTRO_0_OPTIONS,
            started: false,
        }
    }
    
    /// 启动Intro（由mario.rs调用）
    pub fn start(&mut self) {
        self.phase = IntroPhase::NotStarted;
        self.intro_done = false;
        self.started = true;
        self.counter = 1;
        self.update = true;
        self.status = IntroStatus::None;
        self.old_status = IntroStatus::None;
        self.selected = 1;
    }
    
    /// 每帧更新（由mario.rs的统一事件循环调用）
    /// 使用 GameContext 封装大部分子系统引用，额外参数单独传递
    pub fn frame_update(
        &mut self,
        ctx: &mut GameContext,
        play: &mut Play,
        config: &mut ConfigData,
        game_number: &mut i32,
    ) -> IntroResult {
        // 解构 GameContext 获取各子系统引用
        // 注意：palette 现在统一使用 vga.palette，不再单独传递
        self.frame_update_inner(
            ctx.vga,
            ctx.txt,
            ctx.music,
            ctx.buffers,
            ctx.players,
            ctx.enemies,
            ctx.backgr,
            ctx.figures,
            ctx.stars,
            ctx.blocks,
            ctx.status,
            ctx.glitters,
            ctx.tmpobj,
            ctx.sprites,
            ctx.atlas,
            ctx.keyboard,
            ctx.joystick,
            play,
            config,
            game_number,
            ctx.cur_player,
        )
    }
    
    /// 内部实现：保持原有参数签名以最小化代码改动
    /// 注意：palette 现在统一使用 vga.palette，不再作为独立参数
    #[allow(clippy::too_many_arguments)]
    fn frame_update_inner(
        &mut self,
        vga: &mut VGA,
        txt: &mut Txt,
        music: &mut MusicPlayer,
        buffers: &mut Buffers,
        players: &mut Players,
        enemies: &mut Enemies,
        backgr: &mut BackGr,
        figures: &mut Figures,
        stars: &mut Stars,
        blocks: &mut Blocks,
        status_mgr: &mut Status,
        glitters: &mut GlitterSystem,
        tmpobj: &mut TmpObjManager,
        sprites: &mut SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
        keyboard: &mut Keyboard,
        _joystick: &mut crate::joystick::JoystickState,
        play: &mut Play,
        config: &mut ConfigData,
        game_number: &mut i32,
        cur_player: u8,
    ) -> IntroResult {
        if !self.started {
            return IntroResult::Continue;
        }
        
        // 轮询按键
        keyboard.poll_os_keys();
        
        // 状态机
        match self.phase {
            IntroPhase::NotStarted => {
                self.phase = IntroPhase::InitData;
                IntroResult::Continue
            }
            
            IntroPhase::InitData => {
                intro_dbg!("[INTRO] 初始化游戏数据");
                self.intro_done = false;

                // Pascal: Intro begin -> GameNumber := -1;
                // 关键：每次进入 Intro 都必须清空 game_number，避免上一次选择了存档槽位后，NO SAVE 仍然写回旧槽位。
                *game_number = -1;

                // Pascal: NextNumPlayers := Data.NumPlayers; 然后 NewData(不会修改 NumPlayers)
                // 这里等价处理：进入 Intro 时默认沿用上一局的 NumPlayers 作为默认选择
                self.next_num_players = buffers.data.num_players;
                buffers.data.lives[0] = 3;
                buffers.data.lives[1] = 3;
                buffers.data.coins[0] = 0;
                buffers.data.coins[1] = 0;
                buffers.data.score[0] = 0;
                buffers.data.score[1] = 0;
                buffers.data.progress[0] = 0;
                buffers.data.progress[1] = 0;
                buffers.data.mode[0] = 0;
                buffers.data.mode[1] = 0;
                buffers.data.turbo = false;
                
                // 准备地图数据
                self.intro_map = Some(self.convert_map_from_bytes(INTRO_0_MAP));
                self.intro_opt = INTRO_0_OPTIONS;
                
                self.phase = IntroPhase::PlayIntroWorld;
                IntroResult::Continue
            }
            
            IntroPhase::PlayIntroWorld => {
                intro_dbg!("[INTRO] 初始化Intro世界");
                intro_dbg!("[INTRO] palette[0]={:?}, lock={}", vga.palette.palette[0], vga.palette.lock_palette);
                
                // 准备地图数据
                let intro_map = self.intro_map.as_mut().unwrap();
                let intro_opt = self.intro_opt.clone();
                
                // 设置VGA（类似play_world初始化）
                vga.set_y_offset(crate::vga256::YBASE);
                vga.set_y_start(0x12);
                vga.set_y_end(0x7D);
                vga.clear_palette();
                intro_dbg!("[INTRO] after clear_palette: palette[0]={:?}", vga.palette.palette[0]);
                vga.lock_pal();
                intro_dbg!("[INTRO] after lock_pal: lock={}", vga.palette.lock_palette);
                vga.clear_vga_mem();
                
                // 初始化调色板
                vga.palette_init(crate::mpal256::mpal256_palette());
                intro_dbg!("[INTRO] after palette_init: palette[0xA0]={:?}, palette[15]={:?}", 
                    vga.palette.palette[0xA0], vga.palette.palette[15]);
                
                // 读取世界地图
                let mut tmp_world = std::mem::take(&mut buffers.world_map);
                buffers.read_world(intro_map, &mut tmp_world, &intro_opt);
                buffers.world_map = tmp_world;
                
                // 初始化玩家位置
                players.init_player(
                    intro_opt.init_x as i32,
                    intro_opt.init_y as i32,
                    cur_player,
                    buffers,
                    enemies,
                );
                players.map_x = (intro_opt.init_x as i32) / W;
                players.map_y = (intro_opt.init_y as i32) / H + 1;
                
                // 初始化视口
                buffers.x_view = 0;
                buffers.y_view = 0;
                buffers.last_x_view = [0; MAX_PAGE as usize + 1];
                vga.set_view(buffers.x_view, buffers.y_view);
                
                // 初始化世界元素
                let current_opt = buffers.options.clone();
                figures.init_sky(current_opt.sky_type);
                figures.init_walls(
                    current_opt.wall_type1,
                    current_opt.wall_type2,
                    current_opt.wall_type3,
                    sprites,
                    &current_opt,
                );
                figures.init_pipes(current_opt.pipe_color, sprites);
                
                // 关键修复：清理上一关残留的敌人数据（如管道花朵等）
                // 避免从游戏关卡ESC退出后，Intro界面显示残留敌人
                enemies.clear_enemies();
                
                enemies.init_enemy_figures(figures, sprites);
                backgr.init_backgr(current_opt.backgr_type, current_opt.clouds);
                
                if current_opt.stars != 0 {
                    stars.init_stars(buffers, &current_opt);
                }
                
                figures.build_world(&mut buffers.world_map, &current_opt, sprites);
                
                // 设置天空调色板和草地调色板
                {
                    let mut pal = std::mem::take(&mut vga.palette);
                    figures.set_sky_palette(&mut pal, &current_opt);
                    vga.palette = pal;
                }
                {
                    let mut pal = std::mem::take(&mut vga.palette);
                    backgr.draw_pal_backgr(&mut pal, vga, Some(&current_opt));
                    vga.palette = pal;
                }
                vga.palette_init_grass(&current_opt);
                
                // 关键修复：保存调色板颜色到source_palette，然后palette置黑
                // 在整个初始化过程中保持palette全黑，防止帧间闪烁
                vga.palette.source_palette = vga.palette.palette;
                vga.palette.palette = [[0; 3]; 256];
                intro_dbg!("[INTRO] PlayIntroWorld: palette置黑，source_palette[0xA0]={:?}", vga.palette.source_palette[0xA0]);
                
                // 使用Renderer渲染初始帧（和play_world一致）
                for page in 0..=MAX_PAGE {
                    vga.page = page;
                    
                    let mut ctx = RenderContext {
                        vga,
                        buffers,
                        backgr,
                        figures,
                        sprites,
                        atlas,
                        blocks,
                        enemies,
                        players,
                        tmpobj,
                        stars,
                        glitters,
                        status: status_mgr,
                        txt,
                    };
                    
                    let mut renderer = Renderer::new();
                    renderer.only_draw = true;  // Intro模式
                    renderer.render_init_frame(&mut ctx, page);
                }
                
                // palette保持全黑，不恢复
                
                self.phase = IntroPhase::InitBackground;
                IntroResult::Continue
            }
            
            IntroPhase::InitBackground => {
                intro_dbg!("[INTRO] 初始化背景");
                backgr.init_backgr(3, 0);
                self.phase = IntroPhase::SetPalette;
                IntroResult::Continue
            }
            
            IntroPhase::SetPalette => {
                intro_dbg!("[INTRO] 设置Intro调色板到source_palette");
                // 直接修改 source_palette，不修改 palette（保持全黑）
                vga.palette.source_palette[0xA0] = [35, 45, 50];
                vga.palette.source_palette[0xA1] = [45, 55, 60];
                vga.palette.source_palette[0xEF] = [30, 40, 30];
                vga.palette.source_palette[0x18] = [10, 15, 25];
                vga.palette.source_palette[0x8D] = [28, 38, 50];
                vga.palette.source_palette[0x8F] = [40, 50, 63];
                
                // blink 初始化也直接修改 source_palette
                for _ in 0..50 {
                    // 简化的 blink 初始化，只更新 source_palette 中的动画颜色
                    // 瀑布颜色索引 7-11
                    vga.palette.source_palette[7] = vga.palette.source_palette[7];
                    vga.palette.source_palette[8] = vga.palette.source_palette[8];
                    vga.palette.source_palette[9] = vga.palette.source_palette[9];
                    vga.palette.source_palette[10] = vga.palette.source_palette[10];
                    vga.palette.source_palette[11] = vga.palette.source_palette[11];
                }
                intro_dbg!("[INTRO] source_palette[0xA0]={:?}", vga.palette.source_palette[0xA0]);
                
                self.phase = IntroPhase::DrawIntro;
                IntroResult::Continue
            }
            
            IntroPhase::DrawIntro => {
                intro_dbg!("[INTRO] 绘制Intro元素");
                let intro_opt = self.intro_opt.clone();
                
                // palette 已经是全黑的（在 PlayIntroWorld 阶段置黑）
                // source_palette 已经设置好了（包含所有颜色）
                intro_dbg!("[INTRO] palette[0xA0]={:?}, source_palette[0xA0]={:?}", 
                    vga.palette.palette[0xA0], vga.palette.source_palette[0xA0]);
                
                for _i in 0..=MAX_PAGE {
                    intro_dbg!("[INTRO] drawing page {}", i);
                    self.draw_intro_screen(
                        vga, txt, sprites, atlas, buffers, players, figures, backgr, &intro_opt,
                    );
                    vga.show_page();
                }
                
                // 解锁调色板并开始渐显
                // source_palette 已经在之前的阶段设置好了
                vga.unlock_pal();
                // 手动设置渐显状态（palette 是黑的，source_palette 已设置）
                vga.palette.fading_up = true;
                vga.palette.fading_down = false;
                vga.palette.fading_pos = 63;
                vga.palette.fading_step = 1;
                vga.palette.fading_done = false;
                intro_dbg!("[INTRO] 开始渐显，fading_pos={}, source_palette[0xA0]={:?}", 
                    vga.palette.fading_pos, vga.palette.source_palette[0xA0]);
                self.phase = IntroPhase::FadingUp;
                IntroResult::Continue
            }
            
            IntroPhase::FadingUp => {
                vga.palette_fade_step();
                if vga.palette.fading_done {
                    intro_dbg!("[INTRO] 淡入完成，进入菜单");
                    vga.reset_stack();
                    self.bg = [[0; 5]; MAX_PAGE as usize + 1];
                    self.menu = Default::default();
                    txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);
                    
                    if self.status != IntroStatus::Options {
                        self.old_status = IntroStatus::None;
                        self.last_status = IntroStatus::None;
                        self.status = IntroStatus::Menu;
                        self.selected = 1;
                    }
                    self.update = true;
                    self.counter = 1;
                    self.phase = IntroPhase::MenuLoop;
                }
                IntroResult::Continue
            }
            
            IntroPhase::MenuLoop => {
                let mut quit_game = false;
                
                // 更新菜单内容
                if self.update || (self.status != self.old_status) {
                    self.update_menu_content(
                        buffers, game_number, play, music, config, cur_player as usize,
                    );
                    self.old_status = self.status;
                    self.update = false;
                }
                
                self.macro_key = '\0';
                
                // 处理键盘输入
                if keyboard.kb_hit() {
                    if let Some(scan_code) = keyboard.get_current_scan_code() {
                        self.handle_keyboard_input(
                            scan_code, buffers, game_number, &mut quit_game,
                            play, music, config, cur_player as usize,
                        );
                    }
                    keyboard.clear_key();
                }
                
                if self.macro_key != '\0' {
                    self.counter = 0;
                    self.update = true;
                }
                
                // 渲染菜单
                let intro_opt = self.intro_opt.clone();
                self.render_menu_frame(vga, txt, buffers, &intro_opt);
                
                self.counter += 1;
                
                // 检查退出条件
                if quit_game {
                    return IntroResult::Quit;
                }
                
                if self.intro_done {
                    vga.palette.start_fade_down_steps(64);
                    self.phase = IntroPhase::FadingDownForExit;
                } else if self.counter >= WAIT_BEFORE_DEMO {
                    // Pascal: if not IntroDone then Demo;
                    // 超时后开始播放Demo
                    self.counter = 0;
                    vga.palette.start_fade_down_steps(64);
                    self.phase = IntroPhase::FadingDownForDemo;
                }
                
                IntroResult::Continue
            }
            
            IntroPhase::FadingDownForDemo => {
                vga.palette_fade_step();
                if vga.palette.fading_done {
                    // 淡出完成后进入Demo模式
                    self.phase = IntroPhase::PlayDemo;
                }
                IntroResult::Continue
            }
            
            IntroPhase::PlayDemo => {
                // Pascal Demo过程:
                //   NewData;
                //   Turbo := FALSE;
                //   Data.Progress[plMario] := 5;
                //   PlayMacro;
                //   PlayWorld(' ', ' ', Level_6a..., plMario);
                //   StopMacro;
                
                // 重置游戏数据
                buffers.data.lives[0] = 3;
                buffers.data.lives[1] = 3;
                buffers.data.coins[0] = 0;
                buffers.data.coins[1] = 0;
                buffers.data.score[0] = 0;
                buffers.data.score[1] = 0;
                buffers.data.progress[0] = 5; // 第6关
                buffers.data.progress[1] = 0;
                buffers.data.mode[0] = 0;
                buffers.data.mode[1] = 0;
                buffers.data.turbo = false;
                
                // 开始播放Demo按键序列
                keyboard.play_demo();
                
                // 标记Demo模式开始，返回StartDemo让mario.rs处理
                self.demo_runs += 1;
                self.phase = IntroPhase::NotStarted; // Demo结束后重置到初始状态
                
                // 返回StartDemo结果让游戏核心启动第6关Demo播放
                IntroResult::StartDemo
            }
            
            IntroPhase::FadingDownForExit => {
                vga.palette_fade_step();
                if vga.palette.fading_done {
                    self.phase = IntroPhase::Finished;
                }
                IntroResult::Continue
            }
            
            IntroPhase::Finished => {
                // 设置游戏数据
                if *game_number != -1 {
                    let idx = *game_number as usize;
                    if idx < config.games.len() {
                        buffers.data = config.games[idx].clone();
                    }
                }
                buffers.data.num_players = self.next_num_players;
                
                // 完全清空VGA调色板，防止进入Play阶段时闪烁残留颜色
                vga.clear_palette();
                self.started = false;
                IntroResult::StartGame
            }
        }
    }

    fn up(&mut self) {
        if self.selected == 1 {
            if self.status == IntroStatus::Menu {
                self.selected = self.num_options;
            } else {
                self.macro_key = '\x1B'; // kbEsc
            }
        } else {
            self.selected -= 1;
        }
    }

    fn down(&mut self) {
        if self.selected == self.num_options {
            if self.status == IntroStatus::Menu {
                self.selected = 1;
            } else {
                self.macro_key = '\x1B'; // kbEsc
            }
        } else {
            self.selected += 1;
        }
    }

    // Old run and run_remaining_phases functions removed
    // Use new state-machine driven frame_update method

    // 辅助函数
    fn convert_map_from_bytes(&self, map_lines: &[&[u8]]) -> MapBuffer {
        // Pascal ReadWorld 逻辑是 M^[X+1, i]，因此MapBuffer的第0列是“占位列”，真实数据从列1开始。
        // 同时 ReadWorld 通过检查 M^[X+1,1] = #0 作为地图结束标记，因此未提供的列必须为#0（不是空格）。
        let mut map = [['\0'; NV as usize]; crate::buffers::MAX_WORLD_SIZE as usize + 1];
        for (col, line) in map_lines.iter().enumerate() {
            let i = col + 1; // 关键：列偏移，严格对齐Pascal的X+1访问
            if i > crate::buffers::MAX_WORLD_SIZE as usize {
                break;
            }
            for (j, &byte) in line.iter().enumerate() {
                if j >= NV as usize {
                    break;
                }
                // 关键：按原始单字节写入，保证0xF0/0xF7等与Pascal一致
                map[i][j] = byte as char;
            }
        }
        map
    }

    /// GPU模式：收集Intro画面的渲染命令
    pub fn collect_intro_sprites_gpu(
        &self,
        atlas: &crate::sprites::SpriteAtlas,
        palette_index: u32,
    ) -> Vec<crate::gpu::sprite_batch::SpriteCommand> {
        use crate::gpu::sprite_batch::SpriteCommand;
        use crate::sprites::SpriteId;
        
        let mut commands = Vec::new();
        
        // 1) 绘制三个INTRO图像（带阴影效果）
        for i in (0..=1).rev() {
            for j in (0..=1).rev() {
                for k in (0..=1).rev() {
                    // INTRO_000
                    let uv = atlas.get(SpriteId::INTRO_000);
                    let cmd = SpriteCommand::new(38 + i + j, 29 + i + k, uv)
                        .with_palette(0, palette_index);
                    commands.push(cmd);
                    
                    // INTRO_001
                    let uv = atlas.get(SpriteId::INTRO_001);
                    let cmd = SpriteCommand::new(159 + i + j, 29 + i + k, uv)
                        .with_palette(0, palette_index);
                    commands.push(cmd);
                    
                    // INTRO_002
                    let uv = atlas.get(SpriteId::INTRO_002);
                    let cmd = SpriteCommand::new(198 + i + j, 29 + i + k, uv)
                        .with_palette(0, palette_index);
                    commands.push(cmd);
                }
            }
        }
        
        // 2) 绘制边框砖块
        for i in 0..NH {
            for j in 0..NV {
                if i == 0 || i == NH - 1 || j == 0 || j == NV - 1 {
                    let uv = atlas.get(SpriteId::BLOCK_000);
                    let cmd = SpriteCommand::new(i * W, j * H, uv)
                        .with_palette(0, palette_index);
                    commands.push(cmd);
                }
            }
        }
        
        commands
    }

    fn draw_intro_screen(
        &mut self,
        vga: &mut VGA,
        _txt: &mut Txt,
        sprites: &mut SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
        buffers: &mut Buffers,
        players: &mut Players,
        figures: &Figures,
        backgr: &mut BackGr,
        options: &WorldOptions,
    ) {
        intro_dbg!("[INTRO] draw_intro_screen 开始");
        // 注意：不要清空屏幕。底图由PlayWorld(#0,#0,Intro_0...)绘制完成。

        // Pascal 顺序：先叠加 INTRO 标题，再绘制装饰块，再画边框砖块，最后画玩家。
        // 这里严格按原版顺序，避免覆盖关系差异导致像素不一致。

        // 1) 绘制三个INTRO图像（带阴影效果）
        intro_dbg!("[INTRO] 准备绘制INTRO图像");

        // Pascal: for i := 1 downto 0 do for j := 1 downto 0 do for k := 1 downto 0 do
        // 这会绘制8次（2x2x2），创建阴影效果
        for i in (0..=1).rev() {
            for j in (0..=1).rev() {
                for k in (0..=1).rev() {
                    // 第一个INTRO（108×28）
                    vga.draw_sprite(38 + i + j, 29 + i + k, &sprites.INTRO_000);
                    // 第二个INTRO（24×28）
                    vga.draw_sprite(159 + i + j, 29 + i + k, &sprites.INTRO_001);
                    // 第三个INTRO（84×28）
                    vga.draw_sprite(198 + i + j, 29 + i + k, &sprites.INTRO_002);
                }
            }
        }
        intro_dbg!("[INTRO] INTRO图像绘制完成");

        // 2) DrawBackGrMap - 绘制背景装饰块
        intro_dbg!("[INTRO] 绘制背景装饰块");
        backgr.draw_backgr_map(10 * H + 6, 11 * H - 1, 54, 0xA0, vga);
        backgr.draw_backgr_map(10 * H + 6, 11 * H - 1, 55, 0xA1, vga);
        backgr.draw_backgr_map(10 * H + 6, 11 * H - 1, 53, 0xA1, vga);

        // 3) 绘制边框砖块：必须使用与 Pascal include 文件一致的 BLOCK_000
        // 不能用全局 get_sprite("BLOCK_000")（硬编码数组可能与 Pascal 不一致）
        let mut border_count = 0;
        for i in 0..NH {
            for j in 0..NV {
                if i == 0 || i == NH - 1 || j == 0 || j == NV - 1 {
                    vga.draw_image_imagebuffer_world(i * W, j * H, &sprites.BLOCK_000);
                    border_count += 1;
                }
            }
        }
        intro_dbg!("[INTRO] 边框绘制完成，绘制了 {} 个方块", border_count);

        // 4) DrawPlayer
        intro_dbg!("[INTRO] 绘制玩家");
        players.draw_player(
            buffers,
            vga,
            sprites,
            figures,
            options,
            backgr,
            &mut crate::enemies::Enemies::new(sprites),
            atlas,
        );

        // 注意：不在这里调用present()，等fade_up之后再调用
        intro_dbg!("[INTRO] draw_intro_screen 完成");
    }

    fn update_menu_content(
        &mut self,
        buffers: &Buffers,
        _game_number: &i32,
        play: &Play,
        music: &MusicPlayer,
        config: &mut ConfigData,
        cur_player: usize,
    ) {
        intro_dbg!("[INTRO] update_menu_content: status={:?}", self.status);
        if self.status != self.old_status {
            self.selected = 1;
        }

        match self.status {
            IntroStatus::Menu => {
                self.menu[0] = "START".to_string();
                self.menu[1] = "OPTIONS".to_string();
                self.menu[2] = "END".to_string();
                self.menu[3] = String::new();
                self.menu[4] = String::new();
                self.num_options = 3;
                self.last_status = IntroStatus::Menu;
                intro_dbg!("[INTRO] Menu set: START, OPTIONS, END");
            }
            IntroStatus::Options => {
                self.menu[0] = if music.beeper_sound {
                    "SOUND ON "
                } else {
                    "SOUND OFF"
                }
                .to_string();
                self.menu[1] = if play.stat {
                    "STATUSLINE ON "
                } else {
                    "STATUSLINE OFF"
                }
                .to_string();
                self.menu[2] = String::new();
                self.menu[3] = String::new();
                self.menu[4] = String::new();
                self.num_options = 2;
                self.last_status = IntroStatus::Menu;
            }
            IntroStatus::Start => {
                self.menu[0] = "NO SAVE".to_string();
                self.menu[1] = "GAME SELECT".to_string();
                self.menu[2] = "ERASE".to_string();
                self.menu[3] = String::new();
                self.menu[4] = String::new();
                self.num_options = 3;
                self.last_status = IntroStatus::Menu;
            }
            IntroStatus::NumPlayers => {
                self.menu[0] = "ONE PLAYER".to_string();
                self.menu[1] = "TWO PLAYERS".to_string();
                self.menu[2] = String::new();
                self.menu[3] = String::new();
                self.menu[4] = String::new();
                if self.status != self.old_status {
                    self.selected = buffers.data.num_players as i32;
                }
                self.num_options = 2;
                self.last_status = IntroStatus::Start;
            }
            IntroStatus::Load | IntroStatus::Erase => {
                // Pascal:
                // Menu[i] := 'GAME #1 '#7' ';
                // if empty -> + 'EMPTY' else -> + 'LEVEL x ' + (#7 or '*') + ' ' + NumPlayers + 'P'
                for i in 0..3 {
                    let slot = &mut config.games[i];
                    let mut line = format!("GAME #{} \x07 ", i + 1);
                    if slot.progress[0] == 0 && slot.progress[1] == 0 {
                        line.push_str("EMPTY");
                    } else {
                        let mut j: i16 = slot.progress[0];
                        let mut k: i16 = 0;
                        if cur_player < 2 && slot.progress[cur_player] >= NUM_LEV as i16 {
                            k = 1;
                        }
                        if k > 0 {
                            j -= NUM_LEV as i16;
                        }
                        if slot.progress[1] > j {
                            j = slot.progress[1];
                            slot.progress[0] = j;
                        }
                        let level_char = (j + 1).clamp(1, 9) as u8 + b'0';
                        line.push_str("LEVEL ");
                        line.push(level_char as char);
                        line.push(' ');
                        if k == 0 {
                            line.push('\x07');
                            line.push(' ');
                        } else {
                            line.push('*');
                            line.push(' ');
                        }
                        let np = slot.num_players.clamp(1, 2);
                        line.push((np as u8 + b'0') as char);
                        line.push('P');
                    }
                    self.menu[i] = line;
                }
                self.menu[3] = String::new();
                self.menu[4] = String::new();
                self.num_options = 3;
                self.last_status = IntroStatus::Start;
            }
            _ => {}
        }
    }

    fn handle_keyboard_input(
        &mut self,
        scan_code: u8,
        buffers: &mut Buffers,
        game_number: &mut i32,
        quit_game: &mut bool,
        play: &mut Play,
        music: &mut MusicPlayer,
        config: &mut ConfigData,
        cur_player: usize,
    ) {
        // 调试：记录所有按键
        intro_dbg!("[INTRO] handle_keyboard_input: scan_code={}", scan_code);
        
        let _ = cur_player;
        match scan_code {
            1 | 75 => {
                // ESC 或 Left Arrow - 返回上一级菜单
                if self.status == IntroStatus::Menu {
                    self.intro_done = true;
                    *quit_game = true;
                } else {
                    self.status = self.last_status;
                }
                self.macro_key = '\x1B';
            }
            72 | 200 => {
                // Up Arrow
                self.up();
                // Pascal: 如果Up触发了MacroKey=Esc则相当于按Esc
                if self.macro_key == '\x1B' {
                    self.status = self.last_status;
                }
                self.macro_key = 'U';
            }
            80 | 208 => {
                // Down Arrow
                self.down();
                if self.macro_key == '\x1B' {
                    self.status = self.last_status;
                }
                self.macro_key = 'D';
            }
            28 | 56 | 57 => {
                // Enter, Alt 或 Space - 菜单确认
                match self.status {
                    IntroStatus::Menu => match self.selected {
                        1 => self.status = IntroStatus::Start,
                        2 => self.status = IntroStatus::Options,
                        3 => {
                            self.intro_done = true;
                            *quit_game = true;
                        }
                        _ => {}
                    },
                    IntroStatus::Start => match self.selected {
                        1 => self.status = IntroStatus::NumPlayers,
                        2 => self.status = IntroStatus::Load,
                        3 => self.status = IntroStatus::Erase,
                        _ => {}
                    },
                    IntroStatus::Options => match self.selected {
                        1 => {
                            // Pascal: if BeeperSound then BeeperOff else BeeperOn;
                            if music.beeper_sound {
                                music.beeper_off();
                                buffers.beeper_sound = false;
                                config.sound = false;
                            } else {
                                music.beeper_on();
                                buffers.beeper_sound = true;
                                config.sound = true;
                            }
                        }
                        2 => {
                            // Pascal: Play.Stat := not Play.Stat;
                            play.stat = !play.stat;
                            config.sline = play.stat;
                        }
                        _ => {}
                    },
                    IntroStatus::NumPlayers => match self.selected {
                        1 => {
                            self.next_num_players = 1;
                            self.intro_done = true;
                        }
                        2 => {
                            self.next_num_players = 2;
                            self.intro_done = true;
                        }
                        _ => {}
                    },
                    IntroStatus::Load => {
                        *game_number = self.selected - 1;
                        let idx = *game_number as usize;
                        if idx < config.games.len() {
                            // Pascal: Config.Games[GameNumber].NumPlayers := 1;
                            config.games[idx].num_players = 1;
                            if config.games[idx].progress[0] == 0
                                && config.games[idx].progress[1] == 0
                            {
                                self.status = IntroStatus::NumPlayers;
                            } else {
                                self.intro_done = true;
                                self.next_num_players = config.games[idx].num_players;
                            }
                        } else {
                            self.intro_done = true;
                        }
                    }
                    IntroStatus::Erase => {
                        // Pascal: NewData; Config.Games[Selected - 1] := Data; NumPlayers := 1; GameNumber := -1;
                        let idx = (self.selected - 1).max(0) as usize;
                        if idx < config.games.len() {
                            config.games[idx] = GameData::new();
                            config.games[idx].num_players = 1;
                        }
                        *game_number = -1;
                    }
                    _ => {}
                }
                self.macro_key = '\n';
            }
            _ => {
                // 未识别的按键，忽略，不触发菜单更新
                return;
            }
        }
        // Pascal：任意有效按键会重置Counter并触发Update
        if scan_code != 0 {
            self.counter = 0;
            self.update = true;
        }
    }

    fn render_menu_frame(
        &mut self,
        vga: &mut VGA,
        txt: &mut Txt,
        _buffers: &Buffers,
        options: &WorldOptions,
    ) {
        // 高频渲染不输出日志，避免刷屏
        // 计算菜单宽度和位置
        let mut wd = 0;
        let mut xp = 0;
        for i in 0..5 {
            if !self.menu[i].is_empty() {
                let width = txt.text_width(&self.menu[i]);
                if width > wd {
                    wd = width;
                    xp = txt.center_x(&self.menu[i], 0, SCREEN_WIDTH) / 4 * 4;
                }
            }
        }
        let ht = 8;

        // 弹出旧背景
        for k in 0..5 {
            if self.bg[self.page][k] != 0 {
                vga.pop_backgr_address(self.bg[self.page][k] as i32);
            }
        }

        // 绘制菜单项
        for k in 0..5 {
            if !self.menu[k].is_empty() {
                let i = xp;
                // Pascal: j := 56 + 14 * k;  (k从1..5)
                let j = 56 + 14 * (k as i32 + 1);
                self.bg[self.page][k] = vga.push_backgr_address(50, j, 220, ht) as u16;

                // 高频渲染不输出日志，避免刷屏
                if (k + 1) as i32 == self.selected {
                    // 红色(用于选择指示符)
                    vga.palette_out(5, 63, 0, 0);
                    txt.write_text(vga, i - 12, j, "\x10", 5);
                }

                let mut color = 15;
                if self.menu[k].len() > 19 && self.menu[k].chars().nth(18) == Some('*') {
                    color = 14 + (self.counter & 1) as u8;
                }
                // 黄色(用于闪烁菜单项) 和 白色(用于普通菜单文字)
                vga.palette_out(14, 63, 61, 31);
                vga.palette_out(15, 63, 63, 63);
                // 高频渲染不输出日志，避免刷屏
                txt.write_text(vga, i + 8, j, &self.menu[k], color);
            }
        }

        vga.show_page();
        vga.palette_blink_wrapper(options);
        vga.reset_stack();

        // 关键：实际渲染到窗口
        vga.present();
    }
}
