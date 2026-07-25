// Mario游戏核心 - 独立于任何窗口库和平台
//
// 设计原则：
// 1. 不依赖任何窗口库（tao/winit/SDL等）
// 2. 每帧调用 frame_update() 驱动游戏状态机
// 3. 所有游戏阶段（intro/menu/play）都是状态机的不同状态
//
// 使用方式：
// ```
// let mut game = MarioGame::new();
// game.init_palette(vga);
// loop {
//     game.handle_key_event(key);
//     let result = game.frame_update(vga);
//     if result == FrameResult::Exit { break; }
// }
// ```

use crate::{
    backgr::BackGr,
    blocks::Blocks,
    buffers::{Buffers, GameData, H, MAX_WORLD_SIZE, NV, W},
    config,
    context::GameContext,
    enemies::Enemies,
    figures::Figures,
    glitter::{Glitter, GlitterSystem, MAX_GLITTER},
    joystick::JoystickState,
    keyboard::Keyboard,
    mpal256,
    music::MusicPlayer,
    platform::{FrameResult, GamePhase},
    play::Play,
    players::Players,
    render_state::{MAX_PAGE, RenderState},
    sprites::{SpriteAtlas, SpriteDataManager},
    stars::Stars,
    status::Status,
    tmpobj::TmpObjManager,
    txt::Txt,
    worlds::intro::Intro,
};

#[cfg(feature = "debug-bridge")]
use crate::debug_bridge::{CameraInfo, EnemyInfo, GameStats, Observation, PlayerInfo, WorldInfo};
#[cfg(feature = "debug-bridge")]
use crate::worlds::intro::{IntroPhase, IntroStatus};

/// Pascal 常量
pub const NUM_LEV: i32 = 6;
pub const LAST_LEV: i32 = 2 * NUM_LEV - 1;
pub const MAX_SAVE: usize = 3;

/// 配置数据（对应 Pascal MARIO.PAS line 40-46 ConfigData record）
#[derive(Clone, Default)]
pub struct ConfigData {
    /// 音效开关（Pascal: Sound）
    pub sound: bool,
    /// 状态栏开关（Pascal: SLine）
    pub sline: bool,
    /// 3个存档槽位（Pascal: Games: array[0..MAX_SAVE-1] of GameData）
    pub games: [GameData; MAX_SAVE],
    /// 使用手柄（Pascal: UseJS）
    pub use_js: bool,
    /// 手柄校准数据（Pascal: JSDat: JoyRec）
    pub jsdat: crate::joystick::JoyRec,
}

/// 主游戏阶段（顶层状态机）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPhase {
    /// 初始化中
    Initializing,
    /// Intro阶段（包含开场动画和菜单）
    Intro,
    /// 显示玩家名称闪屏（MARIO START / LUIGI START）
    ShowPlayerName,
    /// 游戏进行中
    Playing,
    /// 退出中
    Exiting,
}

/// plMario 和 plLuigi 常量（与 Pascal 一致）
const PL_MARIO: usize = 0;
const PL_LUIGI: usize = 1;

/// Mario游戏核心
pub struct MarioGame {
    // 主状态机
    pub main_phase: MainPhase,
    pub quit_requested: bool,

    // 游戏子系统
    pub buffers: Buffers,
    pub sprites: SpriteDataManager,
    pub atlas: SpriteAtlas,
    pub music: MusicPlayer,
    pub players: Players,
    pub enemies: Enemies,
    pub backgr: BackGr,
    pub figures: Figures,
    pub blocks: Blocks,
    pub stars: Stars,
    pub status: Status,
    pub glitters: GlitterSystem,
    pub tmpobj: TmpObjManager,
    pub txt: Txt,
    pub play: Play,
    pub keyboard: Keyboard,
    pub joystick: JoystickState,

    // 配置和存档
    pub config: ConfigData,
    pub cur_player: i32,
    // 注意：游戏数据统一存储在 buffers.data 中，不再使用独立的 game_data 字段
    pub game_number: i32,

    // Intro模块
    intro: Intro,

    // 当前关卡索引
    current_level: i32,

    // 帧计数
    frame_count: u64,

    // ShowPlayerName 闪屏计数器（Pascal: for i := 1 to 100 do ShowPage）
    show_player_counter: i32,

    // Demo模式标志（用于自动播放第6关演示）
    demo_mode: bool,
}

impl MarioGame {
    /// 创建新的游戏实例
    pub fn new() -> Self {
        let mut buffers = Buffers::new();
        let sprites = SpriteDataManager::new();
        let mut music = MusicPlayer::new();
        let mut players = Players::new();
        let enemies = Enemies::new(&sprites);
        let backgr = BackGr::new(MAX_WORLD_SIZE as usize, W as usize, NV as usize, H as usize);
        let figures = Self::init_figures(&sprites);
        // 构建GPU精灵图集（需要figures来获取fig_list和运行时精灵）
        let atlas = sprites.build_atlas(&figures);
        let blocks = Blocks::new();
        let stars = Stars::new();
        let status = Status::new(); // GPU版Status包含FPS显示数据
        let glitters = GlitterSystem {
            count: vec![0u8; MAX_GLITTER + 1],
            glitter_list: vec![Glitter { attr: 0, pos: 0 }; MAX_GLITTER + 1],
        };
        let tmpobj = TmpObjManager::new();
        let txt = Txt::new();

        players.init_player_figures(&mut buffers, &sprites);

        let play = Play::new();
        let keyboard = Keyboard::new();
        let joystick = JoystickState::new();

        // 读取配置文件（对应 Pascal ReadConfig）
        let config = config::read_config();

        // 应用配置到游戏状态
        // Pascal line 136-138: Play.Stat := SLine; Buffers.BeeperSound := Sound
        let mut play = play;
        play.stat = config.sline;
        buffers.beeper_sound = config.sound;
        // 同步音效开关到 MusicPlayer
        if config.sound {
            music.beeper_on();
        } else {
            music.beeper_off();
        }

        // 应用手柄设置（Pascal line 628-629）
        let mut joystick = joystick;
        joystick.enabled = config.use_js;
        joystick.rec = config.jsdat;

        // 游戏数据初始化在 buffers.data 中
        let intro = Intro::new();

        Self {
            main_phase: MainPhase::Initializing,
            quit_requested: false,
            buffers,
            sprites,
            atlas,
            music,
            players,
            enemies,
            backgr,
            figures,
            blocks,
            stars,
            status,
            glitters,
            tmpobj,
            txt,
            play,
            keyboard,
            joystick,
            config,
            cur_player: 0,
            game_number: -1,
            intro,
            current_level: 0,
            frame_count: 0,
            show_player_counter: 0,
            demo_mode: false,
        }
    }

    /// 初始化调色板
    pub fn init_palette(&mut self, render_state: &mut RenderState) {
        render_state.palette_init(mpal256::mpal256_palette());
        self.main_phase = MainPhase::Intro;
        self.intro.start();
    }

    /// 处理键盘输入事件（平台无关版本）
    pub fn handle_key_event(&mut self, key_event: &crate::platform::KeyEvent) {
        self.keyboard.handle_keyboard_input(key_event);
    }

    /// 每帧更新（统一的状态机驱动）
    ///
    /// 这是游戏的核心更新函数，由main.rs的事件循环每帧调用一次
    pub fn frame_update(&mut self, render_state: &mut RenderState) -> FrameResult {
        self.frame_count += 1;

        // 处理调色板渐变
        if render_state.palette.is_fading() {
            render_state.palette_fade_step();
        }

        // 主状态机
        let result = match self.main_phase {
            MainPhase::Initializing => {
                // 等待init_palette被调用
                FrameResult::Continue
            }

            MainPhase::Intro => {
                // Intro状态机更新 - 使用 GameContext 简化参数传递
                let mut ctx = GameContext::new(
                    render_state,
                    &mut self.txt,
                    &mut self.buffers,
                    &mut self.players,
                    &mut self.enemies,
                    &mut self.backgr,
                    &mut self.figures,
                    &mut self.stars,
                    &mut self.blocks,
                    &mut self.glitters,
                    &mut self.tmpobj,
                    &mut self.status,
                    &mut self.sprites,
                    &mut self.atlas,
                    &mut self.music,
                    &mut self.keyboard,
                    &mut self.joystick,
                    self.cur_player as u8,
                );
                let result = self.intro.frame_update(
                    &mut ctx,
                    &mut self.play,
                    &mut self.config,
                    &mut self.game_number,
                );

                match result {
                    IntroResult::Continue => FrameResult::Continue,
                    IntroResult::StartGame => {
                        self.start_playing();
                        FrameResult::Continue
                    }
                    IntroResult::StartDemo => {
                        // 开始Demo模式：播放第6关的自动演示
                        // Pascal Demo过程已设置 Data.Progress[plMario] := 5
                        // 并调用了 PlayMacro
                        self.start_demo();
                        FrameResult::Continue
                    }
                    IntroResult::Quit => {
                        self.quit_requested = true;
                        self.main_phase = MainPhase::Exiting;
                        FrameResult::Exit
                    }
                }
            }

            MainPhase::ShowPlayerName => {
                // 显示玩家名称闪屏（Pascal: ShowPlayerName过程 line 588-620）
                // Pascal流程：
                //   1. ClearPalette; LockPal; ClearVGAMem; SetView(0,0)
                //   2. 循环 i=0 to MAX_PAGE：DrawImage + ShowPage
                //   3. NewPalette(P256^); UnLockPal; ReadPalette
                //   4. 循环 i=1 to 100：ShowPage
                //   5. ClearPalette; ClearVGAMem

                self.show_player_counter += 1;

                // 初始化阶段 (counter=1)
                if self.show_player_counter == 1 {
                    render_state.palette_clear();
                    render_state.lock_pal();
                    render_state.clear_vga_mem();
                    render_state.set_view(0, 0);
                }

                // 绘制图像阶段 (counter=2 to MAX_PAGE+2)
                if self.show_player_counter >= 2
                    && self.show_player_counter <= (MAX_PAGE as i32 + 2)
                {
                    // 显示 "MARIO START" 或 "LUIGI START"
                    // 对齐 原版 Pascal: DrawImage(160-iW/2, 85-iH/2, iW, iH, @Start000/001^)
                    use crate::sprites::SpriteId;
                    let (sprite_id, iw) = if self.cur_player == 0 {
                        (SpriteId::START_000, 116) // Mario: 116x13
                    } else {
                        (SpriteId::START_001, 108) // Luigi: 108x13
                    };
                    let ih = 13;
                    let x = 160 - iw / 2;
                    let y = 85 - ih / 2;
                    let uv = self.atlas.get(sprite_id);
                    // 使用屏幕坐标直接绘制（不经过世界坐标转换）
                    render_state.sprite_batch.add_sprite(x, y, uv);
                    render_state.show_page();

                    // 在绘制完成后设置调色板
                    if self.show_player_counter == MAX_PAGE as i32 + 2 {
                        render_state.unlock_pal();
                        render_state.palette_init(mpal256::mpal256_palette());
                    }
                }

                // 显示阶段 (counter=MAX_PAGE+3 to MAX_PAGE+102)
                // Pascal: for i := 1 to 100 do ShowPage
                if self.show_player_counter > (MAX_PAGE as i32 + 2)
                    && self.show_player_counter <= (MAX_PAGE as i32 + 102)
                {
                    render_state.show_page();
                }

                // 结束阶段 (counter > MAX_PAGE+102)
                if self.show_player_counter > MAX_PAGE as i32 + 102 {
                    // ClearPalette; ClearVGAMem
                    render_state.palette_clear();
                    render_state.clear_vga_mem();

                    // 闪屏结束，进入游戏
                    self.main_phase = MainPhase::Playing;
                    self.init_current_level();
                }

                FrameResult::Continue
            }

            MainPhase::Playing => {
                // 游戏状态机更新 - 使用 GameContext 简化参数传递
                let mut ctx = GameContext::new(
                    render_state,
                    &mut self.txt,
                    &mut self.buffers,
                    &mut self.players,
                    &mut self.enemies,
                    &mut self.backgr,
                    &mut self.figures,
                    &mut self.stars,
                    &mut self.blocks,
                    &mut self.glitters,
                    &mut self.tmpobj,
                    &mut self.status,
                    &mut self.sprites,
                    &mut self.atlas,
                    &mut self.music,
                    &mut self.keyboard,
                    &mut self.joystick,
                    self.cur_player as u8,
                );
                let result = self.play.frame_update(&mut ctx);

                match result {
                    PlayResult::Continue => FrameResult::Continue,
                    PlayResult::LevelComplete => {
                        // 渐隐已在play.rs中通过状态机完成
                        self.on_level_complete();
                        FrameResult::Continue
                    }
                    PlayResult::PlayerDeath => {
                        // 渐隐已在play.rs中通过状态机完成
                        self.on_player_death();
                        FrameResult::Continue
                    }
                    PlayResult::GameOver => {
                        // 渐隐已在play.rs中通过状态机完成
                        self.on_game_over();
                        FrameResult::Continue
                    }
                    PlayResult::Quit => {
                        // 渐隐已在play.rs中通过状态机完成

                        // 退出前保存存档（对应 Pascal MARIO.PAS line 740-741）
                        if self.game_number >= 0 && self.game_number < 3 {
                            self.config.games[self.game_number as usize] =
                                self.buffers.data.clone();
                        }

                        // 关键修复：重置quit_game状态，避免影响Intro
                        self.buffers.quit_game = false;

                        // 清除键盘状态，避免ESC键状态残留导致Demo计时器被重置
                        self.keyboard.clear_key();

                        // 返回 Intro 菜单而不是退出程序
                        self.main_phase = MainPhase::Intro;
                        self.intro.start();
                        FrameResult::Continue
                    }
                }
            }

            MainPhase::Exiting => FrameResult::Exit,
        };
        result
    }

    /// 开始游戏
    /// 对应 Pascal 中主循环开始处的逻辑（MARIO.PAS line 673-689）
    fn start_playing(&mut self) {
        // Pascal: 同步两个玩家的进度（双人模式）
        // if NumPlayers = 2 then
        //   if Progress[plMario] > Progress[plLuigi] then Progress[plLuigi] := Progress[plMario]
        //   else Progress[plMario] := Progress[plLuigi]
        let data = &mut self.buffers.data;
        if data.num_players == 2 {
            if data.progress[PL_MARIO] > data.progress[PL_LUIGI] {
                data.progress[PL_LUIGI] = data.progress[PL_MARIO];
            } else {
                data.progress[PL_MARIO] = data.progress[PL_LUIGI];
            }
        }

        // Pascal line 681-688: 重置生命、金币、分数、模式
        data.lives[PL_MARIO] = 3;
        data.lives[PL_LUIGI] = 3;
        data.coins[PL_MARIO] = 0;
        data.coins[PL_LUIGI] = 0;
        data.score[PL_MARIO] = 0;
        data.score[PL_LUIGI] = 0;
        data.mode[PL_MARIO] = 0; // mdSmall
        data.mode[PL_LUIGI] = 0; // mdSmall

        // Pascal line 692-693: if Data.NumPlayers = 1 then Data.Lives[plLuigi] := 0
        if data.num_players == 1 {
            data.lives[PL_LUIGI] = 0;
        }

        // 从第一个玩家开始
        self.cur_player = 0;

        // 开始当前玩家的回合
        self.start_player_turn();
    }

    /// 开始Demo模式（自动播放第6关演示）
    /// 对应 Pascal Demo 过程（MARIO.PAS line 220-229）
    fn start_demo(&mut self) {
        // Pascal Demo过程:
        //   NewData;                        -- 已在Intro中完成
        //   Turbo := FALSE;                 -- 已在Intro中完成
        //   Data.Progress[plMario] := 5;    -- 已在Intro中完成
        //   PlayMacro;                      -- 已在Intro中完成
        //   PlayWorld(' ', ' ', Level_6a..., plMario);
        //   StopMacro;

        // 设置Demo模式标志
        self.demo_mode = true;
        self.play.demo_mode = true;
        // 游戏数据已在 buffers.data 中，无需同步

        // 从Mario开始
        self.cur_player = 0;

        // 设置关卡为第6关（progress=5, level_index=4 对应Level_6）
        // Pascal关卡顺序: 0=Level_1, 1=Level_2, 2=Level_3, 3=Level_5, 4=Level_6, 5=Level_4
        self.current_level = 4;

        // 直接进入Playing状态（跳过ShowPlayerName闪屏）
        // Pascal: PlayWorld(' ', ' ', ...)，其中' '表示不显示世界编号
        self.play.start(self.current_level);
        self.main_phase = MainPhase::Playing;
    }

    /// 开始当前玩家的回合
    /// 对应 Pascal 中 for CurPlayer 循环内的逻辑
    fn start_player_turn(&mut self) {
        // Pascal: if Data.Lives[CurPlayer] >= 1 then begin ShowPlayerName(CurPlayer); PlayWorld... end
        let data = &self.buffers.data;
        if data.lives[self.cur_player as usize] >= 1 {
            self.current_level = data.progress[self.cur_player as usize] as i32;
            self.show_player_counter = 0;
            self.main_phase = MainPhase::ShowPlayerName;
        } else {
            // 当前玩家没有生命，尝试切换到下一个玩家
            self.try_next_player();
        }
    }

    /// 尝试切换到下一个玩家
    fn try_next_player(&mut self) {
        let data = &self.buffers.data;
        // Pascal: for CurPlayer := plMario to Data.NumPlayers - 1 do
        if self.cur_player < (data.num_players - 1) as i32 {
            self.cur_player += 1;
            self.start_player_turn();
        } else {
            // 所有玩家都遍历完了，检查是否还有玩家有生命
            // Pascal: until EndGame or QuitGame or (Data.Lives[plMario] + Data.Lives[plLuigi] = 0)
            if data.lives[PL_MARIO] + data.lives[PL_LUIGI] == 0 {
                // 游戏结束
                self.on_game_over();
            } else {
                // 重新开始循环
                self.cur_player = 0;
                self.start_player_turn();
            }
        }
    }

    /// 初始化当前关卡
    fn init_current_level(&mut self) {
        const NUM_LEV: i32 = 6;
        const LAST_LEV: i32 = 11;

        let mut progress = self.buffers.data.progress[self.cur_player as usize] as i32;
        self.buffers.data.turbo = progress >= NUM_LEV;

        if progress > LAST_LEV {
            progress = NUM_LEV;
            self.buffers.data.progress[self.cur_player as usize] = NUM_LEV as i16;
        }

        let level_index = progress % NUM_LEV;
        self.enemies.turbo = self.buffers.data.turbo;
        self.current_level = level_index;

        // 初始化Play模块的关卡数据
        self.play
            .init_level(level_index, &mut self.buffers, &mut self.backgr);
    }

    /// 关卡完成
    /// 对应 Pascal: if Passed then Inc(Data.Progress[CurPlayer])
    fn on_level_complete(&mut self) {
        // Demo模式下：关卡完成后停止宏播放并返回Intro
        if self.demo_mode {
            self.end_demo();
            return;
        }

        // 更新进度
        let idx = self.cur_player as usize;
        self.buffers.data.progress[idx] += 1;

        // 立即保存进度到配置文件（避免进度丢失）
        if self.game_number >= 0 && self.game_number < 3 {
            // 更新存档槽位数据
            self.config.games[self.game_number as usize] = self.buffers.data.clone();
            // 保存到文件
            crate::config::write_config(&self.config);
        }

        // 在双人模式中，通关后切换到下一个玩家
        // Pascal: for CurPlayer := plMario to Data.NumPlayers - 1 do
        self.try_next_player();
    }

    /// 玩家死亡（生命用完）
    /// 对应 Pascal 中玩家死亡后的处理
    fn on_player_death(&mut self) {
        // Demo模式下：玩家死亡后停止宏播放并返回Intro
        if self.demo_mode {
            self.end_demo();
            return;
        }

        // 尝试切换到另一个玩家
        self.try_next_player();
    }

    /// 游戏结束（所有玩家生命都为0）
    /// 对应 Pascal MARIO.PAS line 740-741:
    ///   if GameNumber <> -1 then Config.Games[GameNumber] := Data;
    fn on_game_over(&mut self) {
        // Demo模式下：游戏结束后停止宏播放并返回Intro
        if self.demo_mode {
            self.end_demo();
            return;
        }

        // 保存游戏数据到存档槽位
        if self.game_number >= 0 && self.game_number < 3 {
            self.config.games[self.game_number as usize] = self.buffers.data.clone();
        }

        // 关键修复：重置状态，避免影响Intro
        self.buffers.quit_game = false;
        self.keyboard.clear_key();

        // 返回Intro
        self.main_phase = MainPhase::Intro;
        self.intro.start();
    }

    /// 结束Demo模式
    /// 对应 Pascal: StopMacro; 然后返回Intro
    fn end_demo(&mut self) {
        // 停止宏播放
        self.keyboard.stop_macro();
        self.demo_mode = false;
        self.play.demo_mode = false;

        // 关键修复：重置状态，避免影响下一次Intro的Demo触发
        self.buffers.quit_game = false;
        self.keyboard.clear_key();

        // 返回Intro
        self.main_phase = MainPhase::Intro;
        self.intro.start();
    }

    /// 请求退出
    pub fn request_quit(&mut self) {
        self.quit_requested = true;
        self.main_phase = MainPhase::Exiting;
    }

    /// 检查是否应该退出
    pub fn should_exit(&self) -> bool {
        self.quit_requested || self.main_phase == MainPhase::Exiting
    }

    /// 设置FPS显示数据
    /// 由平台层调用，FPS仅在Intro界面显示
    pub fn set_fps_display(&mut self, fps: u32, frame_time_ms: f32) {
        self.intro.set_fps(fps, frame_time_ms);
    }

    /// 设置渲染模式显示（GPU/CPU）
    /// 由平台层调用，用于Intro界面显示当前渲染模式
    pub fn set_render_mode(&mut self, mode: crate::status::RenderMode) {
        self.intro.set_render_mode(mode);
    }

    /// 关闭游戏
    /// 对应 Pascal 程序结束时调用 WriteConfig
    pub fn shutdown(&mut self) {
        self.music.pause_music();

        // 保存配置到文件
        self.save_config();
    }

    /// 保存配置到文件
    /// 对应 Pascal WriteConfig 过程（MARIO.PAS line 150-168）
    pub fn save_config(&mut self) {
        // 同步当前游戏状态到配置
        // Pascal line 154-157: SLine := Play.Stat; Sound := Buffers.BeeperSound
        self.config.sline = self.play.stat;
        self.config.sound = self.music.beeper_sound;

        // 同步手柄设置
        self.config.use_js = self.joystick.enabled;
        self.config.jsdat = self.joystick.rec;

        // 保存到文件
        config::write_config(&self.config);
    }

    /// 获取当前阶段
    pub fn current_phase(&self) -> GamePhase {
        match self.main_phase {
            MainPhase::Initializing => GamePhase::Initializing,
            MainPhase::Intro => GamePhase::Intro,
            MainPhase::ShowPlayerName => GamePhase::ShowPlayerName,
            MainPhase::Playing => GamePhase::Playing,
            MainPhase::Exiting => GamePhase::Exiting,
        }
    }

    /// 初始化Figures
    fn init_figures(_sprites: &SpriteDataManager) -> Figures {
        use crate::buffers::ImageBuffer;
        use crate::figures::{N1, N2};

        let default_sprite: ImageBuffer = [[0; W as usize]; H as usize];
        let fig_list = [[default_sprite; N2]; N1];
        let bricks = [default_sprite; 4];

        Figures::new(fig_list, bricks, 0u8)
    }
}

impl Default for MarioGame {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 调试接口（仅在 debug-bridge feature 启用时编译）
// ============================================================================
#[cfg(feature = "debug-bridge")]
impl MarioGame {
    /// debug-bridge 模式下禁用 Intro 超时自动 Demo
    pub fn set_suppress_intro_demo(&mut self, suppress: bool) {
        self.intro.set_suppress_demo(suppress);
    }

    /// 重置 Intro Demo 倒计时
    pub fn reset_intro_demo_timer(&mut self) {
        self.intro.reset_demo_timer();
    }

    /// 获取当前主阶段
    pub fn main_phase(&self) -> MainPhase {
        self.main_phase
    }

    /// 获取当前菜单状态（仅在 Intro 阶段有效）
    pub fn intro_status(&self) -> Option<IntroStatus> {
        if self.main_phase == MainPhase::Intro {
            Some(self.intro.status())
        } else {
            None
        }
    }

    /// 获取当前菜单选中项（仅在 Intro 阶段有效）
    pub fn intro_selected(&self) -> Option<i32> {
        if self.main_phase == MainPhase::Intro {
            Some(self.intro.selected())
        } else {
            None
        }
    }

    /// 获取当前 Intro 阶段
    pub fn intro_phase(&self) -> Option<IntroPhase> {
        if self.main_phase == MainPhase::Intro {
            Some(self.intro.phase())
        } else {
            None
        }
    }

    /// 获取当前关卡索引
    pub fn current_level(&self) -> i32 {
        self.current_level
    }

    /// 重新开始新游戏并直接进入指定关卡（跳过 Intro 和闪屏）
    pub fn start_new_game_at_level(&mut self, level_index: i32) {
        let level_index = level_index.clamp(0, NUM_LEV - 1);
        // 先走正常的新游戏流程（重置生命/分数/进度等）
        self.start_playing();
        // 覆盖为指定关卡
        self.current_level = level_index;
        self.buffers.data.progress[self.cur_player as usize] = level_index as i16;
        // 跳过 ShowPlayerName 闪屏，直接进入 Playing 并初始化关卡
        self.main_phase = MainPhase::Playing;
        self.init_current_level();
    }

    /// 打包当前游戏状态为观测数据
    pub fn observe(&self) -> Observation {
        let player = &self.players;
        let data = &self.buffers.data;
        let cur = self.cur_player as usize;

        let enemies: Vec<EnemyInfo> = self
            .enemies
            .enemies
            .iter()
            .filter(|e| e.tp > 0)
            .map(|e| EnemyInfo {
                tp: e.tp,
                x: e.x_pos,
                y: e.y_pos,
                vx: e.x_vel,
                vy: e.y_vel,
                status: e.status,
            })
            .collect();

        let world_width = self.buffers.world_map.len();
        let world_height = self.buffers.world_map.first().map(|r| r.len()).unwrap_or(0);

        Observation {
            frame: self.frame_count,
            main_phase: format!("{:?}", self.main_phase),
            play_phase: format!("{:?}", self.play.phase),
            level_index: self.current_level,
            world_number: self.buffers.world_number.clone(),
            player: PlayerInfo {
                x: player.x,
                y: player.y,
                vx: player.x_vel,
                vy: player.y_vel,
                status: player.status,
                mode: data.mode[cur],
                direction: player.direction,
                in_pipe: player.in_pipe,
            },
            camera: CameraInfo {
                x_view: self.buffers.x_view,
                y_view: self.buffers.y_view,
            },
            stats: GameStats {
                lives: data.lives[cur],
                coins: data.coins[cur],
                score: data.score[cur],
                level_score: self.buffers.level_score,
                progress: data.progress[cur],
                game_done: self.buffers.game_done,
                passed: self.buffers.passed,
                quit_game: self.buffers.quit_game,
            },
            enemies,
            world: WorldInfo {
                width: world_width,
                height: world_height,
                x_offset: crate::buffers::EX,
                y_offset: crate::buffers::EY1,
                x_size: self.buffers.options.x_size,
                tiles: self.buffers.world_map.clone(),
            },
            done: self.should_exit(),
            waiting: self.play.waiting,
            demo: self.buffers.demo,
            key_right: self.keyboard.raw_key_right(),
            key_left: self.keyboard.raw_key_left(),
            key_up: self.keyboard.raw_key_up(),
            key_down: self.keyboard.raw_key_down(),
            key_jump: self.keyboard.raw_key_alt(),
            key_fire: self.keyboard.raw_key_space(),
            key_run: self.keyboard.raw_key_ctrl(),
        }
    }
}

/// Intro模块返回结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntroResult {
    Continue,
    StartGame,
    /// 开始Demo演示（自动播放第6关）
    StartDemo,
    Quit,
}

/// Play模块返回结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayResult {
    Continue,
    LevelComplete,
    /// 玩家死亡（生命-1），可能切换到另一个玩家
    PlayerDeath,
    /// 游戏结束（返回Intro）
    GameOver,
    Quit,
}
