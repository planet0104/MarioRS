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
    render_state::{MAX_PAGE, RenderState, SCREEN_WIDTH},
    renderer::{RenderContext, Renderer},
    sprites::SpriteDataManager,
    stars::Stars,
    status::Status,
    tmpobj::TmpObjManager,
    txt::{FontStyle, Txt},
};

// Intro阶段日志：默认禁用，需要调试时取消注释下面的println版本
macro_rules! intro_dbg {
    ($($t:tt)*) => {}; // ($($t:tt)*) => { println!($($t)*); };  // 启用调试日志
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
    NotStarted,        // 尚未开始
    InitData,          // 初始化游戏数据
    PlayIntroWorld,    // 播放Intro世界
    InitBackground,    // 初始化背景
    SetPalette,        // 设置调色板
    DrawIntro,         // 绘制Intro图像
    FadingUp,          // 淡入中（非阻塞）
    MenuLoop,          // 菜单循环
    FadingDownForDemo, // Demo前淡出
    PlayDemo,          // 播放Demo
    FadingDownForExit, // 退出前淡出
    Finished,          // 完成
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

    // 手柄菜单导航状态 (防抖动)
    js_last_up: bool,
    js_last_down: bool,
    js_last_button: bool,
    js_last_back: bool,

    // TV遥控器菜单导航状态 (防抖动)
    tv_last_up: bool,
    tv_last_down: bool,
    tv_last_ok: bool,
    tv_last_back: bool,
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
            // 手柄防抖状态
            js_last_up: false,
            js_last_down: false,
            js_last_button: false,
            js_last_back: false,
            // TV遥控器防抖状态
            tv_last_up: false,
            tv_last_down: false,
            tv_last_ok: false,
            tv_last_back: false,
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
        // 注意：palette 现在统一使用  render_state.palette，不再单独传递
        self.frame_update_inner(
            ctx.render_state,
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
    /// 注意：palette 现在统一使用  render_state.palette，不再作为独立参数
    #[allow(clippy::too_many_arguments)]
    fn frame_update_inner(
        &mut self,
        render_state: &mut RenderState,
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
        atlas: &mut crate::sprites::SpriteAtlas,
        keyboard: &mut Keyboard,
        joystick: &mut crate::joystick::JoystickState,
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
                intro_dbg!(
                    "[INTRO] palette[0]={:?}, lock={}",
                    render_state.palette.palette[0],
                    render_state.palette.lock_palette
                );

                // 准备地图数据
                let intro_map = self.intro_map.as_mut().unwrap();
                let intro_opt = self.intro_opt.clone();

                // 设置VGA（类似play_world初始化）
                render_state.set_y_offset(crate::render_state::YBASE);
                render_state.set_y_start(0x12);
                render_state.set_y_end(0x7D);
                render_state.clear_palette();
                intro_dbg!(
                    "[INTRO] after clear_palette: palette[0]={:?}",
                    render_state.palette.palette[0]
                );
                render_state.lock_pal();
                intro_dbg!(
                    "[INTRO] after lock_pal: lock={}",
                    render_state.palette.lock_palette
                );
                render_state.clear_vga_mem();

                // 初始化调色板
                render_state.palette_init(crate::mpal256::mpal256_palette());
                intro_dbg!(
                    "[INTRO] after palette_init: palette[0xA0]={:?}, palette[15]={:?}",
                    render_state.palette.palette[0xA0],
                    render_state.palette.palette[15]
                );

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
                render_state.set_view(buffers.x_view, buffers.y_view);

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
                // 关键：BuildWorld 会重着色/转换草地等，运行时精灵存入figures
                // GPU 图集必须同步更新，否则 Intro 地面/草/砖块颜色与 原版 不一致
                *atlas = sprites.build_atlas(figures);

                // 设置天空调色板和草地调色板
                {
                    let mut pal = std::mem::take(&mut render_state.palette);
                    figures.set_sky_palette(&mut pal, &current_opt);
                    render_state.palette = pal;
                }
                {
                    let mut pal = std::mem::take(&mut render_state.palette);
                    backgr.draw_pal_backgr(&mut pal, render_state, Some(&current_opt));
                    render_state.palette = pal;
                }
                render_state.palette_init_grass(&current_opt);

                // 关键修复：保存调色板颜色到source_palette，然后palette置黑
                // 在整个初始化过程中保持palette全黑，防止帧间闪烁
                render_state.palette.source_palette = render_state.palette.palette;
                render_state.palette.palette = [[0; 3]; 256];
                intro_dbg!(
                    "[INTRO] PlayIntroWorld: palette置黑，source_palette[0xA0]={:?}",
                    render_state.palette.source_palette[0xA0]
                );

                // 使用Renderer渲染初始帧（和play_world一致）
                for page in 0..=MAX_PAGE {
                    render_state.page = page;

                    let mut ctx = RenderContext {
                        render_state,
                        buffers,
                        backgr,
                        figures,
                        sprites,
                        atlas: &*atlas,
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
                    renderer.only_draw = true; // Intro模式
                    renderer.render_init_frame(&mut ctx, page);
                }

                // palette保持全黑，不恢复

                self.phase = IntroPhase::InitBackground;
                IntroResult::Continue
            }

            IntroPhase::InitBackground => {
                intro_dbg!("[INTRO] 初始化背景");
                // 对齐 原版/Pascal：Intro 背景使用 Options_0.BackGrType，不应在这里强行改成 3
                backgr.init_backgr(self.intro_opt.backgr_type, self.intro_opt.clouds);
                self.phase = IntroPhase::SetPalette;
                IntroResult::Continue
            }

            IntroPhase::SetPalette => {
                intro_dbg!("[INTRO] 设置Intro调色板到source_palette");
                // 直接修改 source_palette，不修改 palette（保持全黑）
                // 严格对齐 原版：只改 0xA0/0xA1 等少量索引，不覆盖 0xE0..0xEF 的天空渐变
                // 颜色对齐 原版 实机观感：
                // - 云：#E3E4F8 约等于 6bit [56,56,61]
                // - 山：#86B1C2 约等于 6bit [33,44,48]
                render_state.palette.source_palette[0xA0] = [56, 56, 61];
                render_state.palette.source_palette[0xA1] = [33, 44, 48];
                render_state.palette.source_palette[0x18] = [10, 15, 25];
                render_state.palette.source_palette[0x8D] = [28, 38, 50];
                render_state.palette.source_palette[0x8F] = [40, 50, 63];

                // blink 初始化也直接修改 source_palette
                for _ in 0..50 {
                    // 简化的 blink 初始化，只更新 source_palette 中的动画颜色
                    // 瀑布颜色索引 7-11
                    render_state.palette.source_palette[7] = render_state.palette.source_palette[7];
                    render_state.palette.source_palette[8] = render_state.palette.source_palette[8];
                    render_state.palette.source_palette[9] = render_state.palette.source_palette[9];
                    render_state.palette.source_palette[10] =
                        render_state.palette.source_palette[10];
                    render_state.palette.source_palette[11] =
                        render_state.palette.source_palette[11];
                }
                intro_dbg!(
                    "[INTRO] source_palette[0xA0]={:?}",
                    render_state.palette.source_palette[0xA0]
                );

                self.phase = IntroPhase::DrawIntro;
                IntroResult::Continue
            }

            IntroPhase::DrawIntro => {
                intro_dbg!("[INTRO] 绘制Intro元素");
                let intro_opt = self.intro_opt.clone();

                // palette 已经是全黑的（在 PlayIntroWorld 阶段置黑）
                // source_palette 已经设置好了（包含所有颜色）
                intro_dbg!(
                    "[INTRO] palette[0xA0]={:?}, source_palette[0xA0]={:?}",
                    render_state.palette.palette[0xA0],
                    render_state.palette.source_palette[0xA0]
                );

                for _i in 0..=MAX_PAGE {
                    intro_dbg!("[INTRO] drawing page {}", i);
                    self.draw_intro_screen(
                        render_state,
                        txt,
                        buffers,
                        players,
                        enemies,
                        backgr,
                        figures,
                        stars,
                        blocks,
                        status_mgr,
                        glitters,
                        tmpobj,
                        sprites,
                        atlas,
                        &intro_opt,
                    );
                    render_state.show_page();
                }

                // 解锁调色板并开始渐显
                // source_palette 已经在之前的阶段设置好了
                render_state.unlock_pal();
                // 手动设置渐显状态（palette 是黑的，source_palette 已设置）
                render_state.palette.fading_up = true;
                render_state.palette.fading_down = false;
                render_state.palette.fading_pos = 63;
                render_state.palette.fading_step = 1;
                render_state.palette.fading_done = false;
                intro_dbg!(
                    "[INTRO] 开始渐显，fading_pos={}, source_palette[0xA0]={:?}",
                    render_state.palette.fading_pos,
                    render_state.palette.source_palette[0xA0]
                );
                self.phase = IntroPhase::FadingUp;
                IntroResult::Continue
            }

            IntroPhase::FadingUp => {
                render_state.palette_fade_step();
                if render_state.palette.fading_done {
                    intro_dbg!("[INTRO] 淡入完成，进入菜单");
                    render_state.reset_stack();
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
                        buffers,
                        game_number,
                        play,
                        music,
                        config,
                        cur_player as usize,
                    );
                    self.old_status = self.status;
                    self.update = false;
                }

                self.macro_key = '\0';

                // 处理键盘输入
                if keyboard.kb_hit() {
                    if let Some(scan_code) = keyboard.get_current_scan_code() {
                        self.handle_keyboard_input(
                            scan_code,
                            buffers,
                            game_number,
                            &mut quit_game,
                            play,
                            music,
                            config,
                            cur_player as usize,
                        );
                    }
                    keyboard.clear_key();
                }

                // 处理手柄输入 (菜单导航)
                // 按钮映射 (与 Windows 手柄一致):
                // - A/START: 确认 (Enter)
                // - SELECT/B: 返回 (ESC) - B键在子菜单中作为返回
                // - 方向键: 菜单导航
                joystick.read();
                if joystick.detected {
                    // 方向上 (边沿触发，防止连续触发)
                    if joystick.up && !self.js_last_up {
                        self.up();
                        if self.macro_key == '\x1B' {
                            self.status = self.last_status;
                        }
                        self.macro_key = 'U';
                    }
                    // 方向下
                    if joystick.down && !self.js_last_down {
                        self.down();
                        if self.macro_key == '\x1B' {
                            self.status = self.last_status;
                        }
                        self.macro_key = 'D';
                    }
                    
                    // 确认按钮: A / START (不包括B，B用于返回)
                    let confirm_pressed = joystick.button_a || joystick.button_start;
                    if confirm_pressed && !self.js_last_button {
                        // 模拟 Enter 键 (scan_code = 28)
                        self.handle_keyboard_input(
                            28,
                            buffers,
                            game_number,
                            &mut quit_game,
                            play,
                            music,
                            config,
                            cur_player as usize,
                        );
                    }
                    
                    // 返回按钮: SELECT 或 B (在子菜单中)
                    // SELECT: 总是作为返回/ESC
                    // B: 在子菜单中作为返回，在主菜单中不退出游戏
                    let back_pressed = joystick.button_select || 
                        (joystick.button_b && self.status != IntroStatus::Menu);
                    if back_pressed && !self.js_last_back {
                        // 模拟 ESC 键 (scan_code = 1)
                        self.handle_keyboard_input(
                            1,
                            buffers,
                            game_number,
                            &mut quit_game,
                            play,
                            music,
                            config,
                            cur_player as usize,
                        );
                    }
                    
                    // 更新防抖状态
                    self.js_last_up = joystick.up;
                    self.js_last_down = joystick.down;
                    self.js_last_button = confirm_pressed;
                    self.js_last_back = back_pressed;
                }

                // 处理 TV 遥控器输入 (Android TV)
                #[cfg(target_os = "android")]
                {
                    let tv = crate::platform::joystick_android_tv::read_tv_remote_state();
                    if tv.detected {
                        // 方向上 (边沿触发)
                        if tv.up && !self.tv_last_up {
                            self.up();
                            if self.macro_key == '\x1B' {
                                self.status = self.last_status;
                            }
                            self.macro_key = 'U';
                        }
                        // 方向下
                        if tv.down && !self.tv_last_down {
                            self.down();
                            if self.macro_key == '\x1B' {
                                self.status = self.last_status;
                            }
                            self.macro_key = 'D';
                        }
                        
                        // OK键确认
                        if tv.ok && !self.tv_last_ok {
                            // 模拟 Enter 键 (scan_code = 28)
                            self.handle_keyboard_input(
                                28,
                                buffers,
                                game_number,
                                &mut quit_game,
                                play,
                                music,
                                config,
                                cur_player as usize,
                            );
                        }
                        
                        // 返回键
                        if tv.back && !self.tv_last_back {
                            // 模拟 ESC 键 (scan_code = 1)
                            self.handle_keyboard_input(
                                1,
                                buffers,
                                game_number,
                                &mut quit_game,
                                play,
                                music,
                                config,
                                cur_player as usize,
                            );
                        }
                        
                        // 更新防抖状态
                        self.tv_last_up = tv.up;
                        self.tv_last_down = tv.down;
                        self.tv_last_ok = tv.ok;
                        self.tv_last_back = tv.back;
                    }
                }

                if self.macro_key != '\0' {
                    self.counter = 0;
                    self.update = true;
                }

                // 渲染菜单
                let intro_opt = self.intro_opt.clone();
                self.render_menu_frame(
                    render_state,
                    txt,
                    buffers,
                    players,
                    enemies,
                    backgr,
                    figures,
                    stars,
                    blocks,
                    status_mgr,
                    glitters,
                    tmpobj,
                    sprites,
                    atlas,
                    &intro_opt,
                );

                self.counter += 1;

                // 检查退出条件
                if quit_game {
                    return IntroResult::Quit;
                }

                if self.intro_done {
                    render_state.palette.start_fade_down_steps(64);
                    self.phase = IntroPhase::FadingDownForExit;
                } else if self.counter >= WAIT_BEFORE_DEMO {
                    // Pascal: if not IntroDone then Demo;
                    // 超时后开始播放Demo
                    self.counter = 0;
                    render_state.palette.start_fade_down_steps(64);
                    self.phase = IntroPhase::FadingDownForDemo;
                }

                IntroResult::Continue
            }

            IntroPhase::FadingDownForDemo => {
                render_state.palette_fade_step();
                if render_state.palette.fading_done {
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
                render_state.palette_fade_step();
                if render_state.palette.fading_done {
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
                render_state.clear_palette();
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
                    let cmd = SpriteCommand::new(i * W, j * H, uv).with_palette(0, palette_index);
                    commands.push(cmd);
                }
            }
        }

        commands
    }

    fn draw_intro_screen(
        &mut self,
        render_state: &mut RenderState,
        txt: &mut Txt,
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
        _options: &WorldOptions,
    ) {
        // GPU模式：严格走 GPU 命令收集与提交（vga 的旧 CPU API 在 GPU 版是空实现）
        {
            let mut ctx = crate::renderer::RenderContext {
                render_state,
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
            let mut renderer = crate::renderer::Renderer::new();
            renderer.only_draw = true;
            renderer.show_status = false;
            renderer.show_objects = true;
            renderer.render_game_frame(&mut ctx);
        }

        // 叠加 INTRO 标题/边框（对齐 原版 DrawIntroScreen）
        let palette_index: u32 = 0;
        let sprite_cmds = self.collect_intro_sprites_gpu(atlas, palette_index);
        let batch = render_state.get_sprite_batch_mut();
        for s in sprite_cmds {
            batch.push_sprite(s);
        }
        // 原版 最后 DrawPlayer，GPU 这里再绘制一次以保证层级一致
        for p in players.collect_player_sprites_gpu(buffers, atlas, palette_index, enemies.star) {
            batch.push_sprite(p);
        }
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
            1 => {
                // ESC - 返回上一级菜单或退出
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
            75 | 203 => {
                // Left Arrow - 在子菜单中返回上一级，在主菜单中不退出游戏
                if self.status != IntroStatus::Menu {
                    self.status = self.last_status;
                    self.macro_key = '\x1B';
                }
                // 在主菜单中，Left Arrow 不触发任何操作，避免误触退出
            }
            77 | 205 => {
                // Right Arrow - 相当于 Enter，进入子菜单
                // 不做任何处理，让用户用 Enter/OK 键确认
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
        render_state: &mut RenderState,
        txt: &mut Txt,
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
        options: &WorldOptions,
    ) {
        // GPU模式下每帧完全重绘，避免批次累积导致内存和性能问题
        // 先绘制底图（天空、地形、玩家等），再叠加 INTRO 标题和菜单文字
        {
            let mut ctx = crate::renderer::RenderContext {
                render_state,
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
            let mut renderer = crate::renderer::Renderer::new();
            renderer.only_draw = true;
            renderer.show_status = false;
            renderer.show_objects = true;
            renderer.render_game_frame(&mut ctx);
        }

        // 叠加INTRO标题和边框
        {
            let palette_index: u32 = 0;
            let sprite_cmds = self.collect_intro_sprites_gpu(atlas, palette_index);
            let batch = render_state.get_sprite_batch_mut();
            for s in sprite_cmds {
                batch.push_sprite(s);
            }

            // 原版 最后 DrawPlayer，GPU 这里再绘制一次以保证层级一致
            for p in players.collect_player_sprites_gpu(buffers, atlas, palette_index, enemies.star)
            {
                batch.push_sprite(p);
            }
        }

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
        let _ht = 8;

        // 绘制菜单项
        for k in 0..5 {
            if !self.menu[k].is_empty() {
                let i = xp;
                // Pascal: j := 56 + 14 * k;  (k从1..5)
                let j = 56 + 14 * (k as i32 + 1);
                // 高频渲染不输出日志，避免刷屏
                if (k + 1) as i32 == self.selected {
                    // 红色(用于选择指示符)
                    render_state.palette_out(5, 63, 0, 0);
                    txt.write_text(render_state, i - 12, j, "\x10", 5);
                }

                let mut color = 15;
                if self.menu[k].len() > 19 && self.menu[k].chars().nth(18) == Some('*') {
                    color = 14 + (self.counter & 1) as u8;
                }
                // 黄色(用于闪烁菜单项) 和 白色(用于普通菜单文字)
                render_state.palette_out(14, 63, 61, 31);
                render_state.palette_out(15, 63, 63, 63);
                // 高频渲染不输出日志，避免刷屏
                txt.write_text(render_state, i + 8, j, &self.menu[k], color);
            }
        }

        render_state.show_page();
        render_state.palette_blink_wrapper(options);
        render_state.reset_stack();

        // 实际渲染到窗口由平台层（wgpu backend）负责
    }
}
