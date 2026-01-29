// play.rs
// Rust 严格结构体骨架移植自 Pascal Play.pas

use crate::context::GameContext;
use crate::joystick::JoystickState;
use crate::{
    backgr::BackGr,
    blocks::Blocks,
    buffers::{Buffers, H, MapBuffer, NH, NV, W, WorldOptions},
    enemies::{Enemies, START_ENEMIES_AT, TP_CHAMP, TP_LIFE},
    figures::Figures,
    glitter::GlitterSystem,
    keyboard::Keyboard,
    mpal256,
    music::MusicPlayer,
    palettes::{PE_BLACK_WHITE, PE_EGA_MODE, PE_NO_EFFECT},
    players::Players,
    render_state::{MAX_PAGE, RenderState, SCREEN_WIDTH, YBASE},
    renderer::{RenderContext, Renderer},
    sprites::SpriteDataManager,
    stars::Stars,
    status::Status,
    tmpobj::TmpObjManager,
    txt::{FontStyle, Txt},
    utils::random_i32,
};

/// 游戏阶段状态机
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayPhase {
    /// 未初始化
    NotStarted,
    /// 初始化VGA和调色板
    InitRenderState,
    /// 构建关卡（BuildLevel）- 首次加载
    BuildLevel,
    /// 重建关卡 - 管道切换后使用当前buffers中的数据
    RebuildLevel,
    /// 重启循环（Restart）
    Restart,
    /// 渲染初始帧
    RenderInitFrames,
    /// 等待淡入完成
    FadingUp,
    /// 主游戏循环
    GameLoop,
    /// 暂停状态
    Paused,
    /// 从暂停恢复（重绘屏幕但不重置游戏状态）
    ResumeFromPause,
    /// 管道传送渐隐中（对应Pascal FadeDown(64)）
    FadingDownForPipe,
    /// 管道传送处理（渐隐完成后）
    PipeTransition,
    /// 关卡完成渐隐中
    FadingDownForComplete,
    /// 关卡完成
    LevelComplete,
    /// 玩家死亡渐隐中
    FadingDownForDeath,
    /// 当前玩家死亡（生命为0），切换到另一个玩家
    PlayerDeath,
    /// 游戏结束渐隐中
    FadingDownForGameOver,
    /// 游戏结束（所有玩家都没有生命了）
    GameOver,
    /// 退出游戏渐隐中
    FadingDownForQuit,
}

/// 暂停子状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PauseState {
    /// 初始化暂停（淡出、显示PAUSE文本）
    Init,
    /// 等待P键释放
    WaitingPRelease,
    /// 等待输入（可输入作弊码）
    WaitingInput,
    /// 退出暂停（淡入）
    Exiting,
}

pub struct Play {
    // Pascal const/var 全局状态
    pub stat: bool,         // Stat: Boolean = FALSE;
    pub show_retrace: bool, // ShowRetrace: Boolean = FALSE;
    pub cheats_used: i32,   // CheatsUsed: Integer = 0;
    // 其它全局变量
    pub waiting: bool,
    pub text_status: bool,
    pub show_score: bool,
    pub counting_score: bool,
    pub show_objects: bool,
    pub only_draw: bool,
    // 游戏阶段状态机
    pub phase: PlayPhase,
    pub render_page: i32,
    /// 当前关卡选项（克隆用于游戏循环）
    pub current_opt: Option<WorldOptions>,
    /// Renderer实例
    pub renderer: Renderer,
    /// 循环计数器
    pub loop_count: u64,

    // 暂停状态相关
    pub pause_state: PauseState,
    pub pause_cheat: String,
    pub pause_old_scan_code: u8,
    pub pause_text: String,
    pub pause_tab_mode: bool,
    pub p_key_was_pressed: bool,   // 防止P键重复触发暂停
    pub esc_key_was_pressed: bool, // 防止ESC键重复触发退出
    /// 管道传送后的重启标志（不重置玩家位置和x_view）
    pub pipe_restart: bool,
    /// 当前关卡索引（0-5对应Level 1-6）
    pub current_level_index: i32,
    /// Demo模式标志（自动播放时使用预录制按键）
    pub demo_mode: bool,
}

impl Play {
    pub fn new() -> Self {
        Play {
            stat: false,
            show_retrace: false,
            cheats_used: 0,
            waiting: false,
            text_status: false,
            show_score: false,
            counting_score: false,
            show_objects: true,
            only_draw: false,
            phase: PlayPhase::NotStarted,
            render_page: 0,
            current_opt: None,
            renderer: Renderer::new(),
            loop_count: 0,
            // 暂停相关初始化
            pause_state: PauseState::Init,
            pause_cheat: String::new(),
            pause_old_scan_code: 0,
            pause_text: String::from("PAUSE"),
            pause_tab_mode: false,
            p_key_was_pressed: false,
            esc_key_was_pressed: false,
            pipe_restart: false,
            current_level_index: 0,
            demo_mode: false,
        }
    }

    /// 获取当前关卡选项（复制一份以避免借用冲突）
    /// 优先使用缓存的 current_opt，否则使用 buffers.options
    /// 注意：由于 Rust 借用规则，返回引用会阻止后续对 buffers 的可变借用，
    /// 因此这里返回克隆值。WorldOptions 较小（约40字节），clone 开销可接受。
    #[inline]
    fn get_current_opt(&self, buffers: &Buffers) -> WorldOptions {
        self.current_opt
            .clone()
            .unwrap_or_else(|| buffers.options.clone())
    }

    /// 开始关卡（用于Demo模式）
    /// 这个方法仅设置关卡索引并启动状态机，不初始化buffers
    pub fn start(&mut self, level_index: i32) {
        self.current_level_index = level_index;
        self.phase = PlayPhase::NotStarted;
        self.render_page = 0;
        self.current_opt = None;
        self.loop_count = 0;
        self.waiting = false;
        self.show_score = false;
        self.counting_score = false;
        self.show_objects = true;
        self.only_draw = false;
        self.pipe_restart = false;
        self.renderer = Renderer::new();
    }

    /// 初始化关卡（由mario.rs调用）
    pub fn init_level(&mut self, level_index: i32, buffers: &mut Buffers, _backgr: &mut BackGr) {
        // 保存关卡索引
        self.current_level_index = level_index;

        // 重置状态
        self.phase = PlayPhase::NotStarted;
        self.render_page = 0;
        self.current_opt = None;
        self.loop_count = 0;
        self.waiting = false;
        self.show_score = false;
        self.counting_score = false;
        self.show_objects = true;
        self.only_draw = false;
        self.pipe_restart = false;

        // 重置renderer
        self.renderer = Renderer::new();

        // 初始化buffers
        buffers.x_view = 0;
        buffers.y_view = 0;
        buffers.last_x_view = [0; MAX_PAGE as usize + 1];
        buffers.text_counter = 0;
        buffers.game_done = false;
        buffers.passed = false;
        buffers.quit_game = false;

        // 注意：不要重置 self.stat，保持用户在 Options 菜单中的设置
        // Pascal: TextStatus := Stat and (not PlayingMacro);
        // 这里先设置为 false，后续在 FadingUp 阶段会根据 stat 设置
        self.text_status = false;

        // 设置初始阶段
        self.phase = PlayPhase::InitRenderState;
    }

    /// 每帧更新（由mario.rs的统一事件循环调用）
    /// 使用 GameContext 封装所有子系统引用，简化调用
    pub fn frame_update(&mut self, ctx: &mut GameContext) -> crate::mario::PlayResult {
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
        joystick: &mut JoystickState,
        cur_player: u8,
    ) -> crate::mario::PlayResult {
        use crate::mario::PlayResult;

        // 轮询按键
        keyboard.poll_os_keys();

        // 检查Demo模式：当宏播放结束时自动结束关卡
        // Pascal: PlayMacro播放完成后自动停止，此时Check函数中 Playing := FALSE
        if self.demo_mode && !keyboard.playing_macro() {
            // 宏播放已停止，返回GameOver让mario.rs处理Demo结束
            return PlayResult::GameOver;
        }

        // Pascal: Demo模式下按任意键或手柄按钮退出
        // PLAY.PAS line 513-517: if Key <> #0 then begin GameDone := TRUE; Passed := TRUE; end;
        // 手柄: 任意按钮 (A/B/X/Y/LB/RB/START/SELECT) 也可以退出Demo
        joystick.read();
        let gamepad_any_button = joystick.detected && 
            (joystick.button_a || joystick.button_b || joystick.button_x || joystick.button_y ||
             joystick.button_lb || joystick.button_rb || joystick.button_start || joystick.button_select);
        if self.demo_mode && (keyboard.kb_hit() || gamepad_any_button) {
            // 按任意键或手柄按钮退出Demo
            return PlayResult::GameOver;
        }

        // 检查ESC键或手柄SELECT键（暂停状态下不退出游戏，ESC用于退出暂停）
        // 使用kb_key检测ESC键状态，添加去抖动逻辑防止重复触发
        // 手柄: SELECT键 = 返回/退出 (与Windows手柄一致)
        // 注意：渐隐过程中不再检测ESC，让状态机处理
        if self.phase != PlayPhase::Paused && self.phase != PlayPhase::FadingDownForQuit {
            // 手柄状态已在上面读取
            let esc_key_held = keyboard.kb_key(1) || joystick.button_select; // ESC键或手柄SELECT

            if esc_key_held && !self.esc_key_was_pressed {
                // ESC键或SELECT键刚被按下，启动渐隐后退出
                self.esc_key_was_pressed = true;
                buffers.quit_game = true;
                // 启动非阻塞渐隐（ render_state.palette已经是当前显示的调色板）
                render_state.palette.start_fade_down_steps(64);
                self.phase = PlayPhase::FadingDownForQuit;
                // 不要return，让状态机处理
            }

            // ESC键/SELECT键释放时重置标志
            if !esc_key_held {
                self.esc_key_was_pressed = false;
            }
        }

        // 注意：调色板渐变在FadingUp阶段处理，不要在这里重复调用fade()

        // 状态机
        match self.phase {
            PlayPhase::NotStarted => {
                self.phase = PlayPhase::InitRenderState;
                PlayResult::Continue
            }

            PlayPhase::InitRenderState => {
                // 对应Pascal play_world初始化：重置管道状态
                // 确保新关卡开始时不会误触发管道处理
                players.in_pipe = false;
                players.pipe_code = [b' ', b' '];

                // 初始化VGA
                render_state.set_y_offset(YBASE);
                render_state.set_y_start(0x12);
                render_state.set_y_end(0x7D);
                render_state.clear_palette();
                render_state.lock_pal();
                render_state.clear_vga_mem();

                // 锁定调色板，防止渲染初始帧时写入VGA导致闪烁
                render_state.lock_pal();

                // 初始化调色板
                render_state.palette_init(mpal256::mpal256_palette());

                // 根据关卡索引加载对应的关卡数据
                // Pascal关卡顺序（严格对应MARIO.PAS line 708-720）:
                //   0: Level_1 (显示为x-1)
                //   1: Level_2 (显示为x-2)
                //   2: Level_3 (显示为x-3)
                //   3: Level_5 (显示为x-4) - 注意不是Level_4！
                //   4: Level_6 (显示为x-5) - 注意不是Level_5！
                //   5: Level_4 (显示为x-6) - 注意不是Level_6！
                //
                // Turbo模式（progress >= 6）使用不同的Options配置（Opt_Xa而非Options_Xa）
                // Pascal: if not Turbo then ReadWorld(Map1, WorldMap, Opt1) else ReadWorld(Map1, WorldMap, Opt1b)
                let turbo = buffers.data.turbo;
                let (map1_data, opt1, map2_data, opt2, world_number) = match self
                    .current_level_index
                {
                    0 => {
                        use crate::worlds::level_1::{
                            LEVEL_1A_MAP, LEVEL_1B_MAP, OPT_1A, OPTIONS_1A, OPTIONS_1B,
                        };
                        if turbo {
                            (
                                LEVEL_1A_MAP,
                                OPT_1A.clone(),
                                LEVEL_1B_MAP,
                                OPTIONS_1B.clone(),
                                "x-1",
                            )
                        } else {
                            (
                                LEVEL_1A_MAP,
                                OPTIONS_1A.clone(),
                                LEVEL_1B_MAP,
                                OPTIONS_1B.clone(),
                                "x-1",
                            )
                        }
                    }
                    1 => {
                        use crate::worlds::level_2::{LEVEL_2A_MAP, LEVEL_2B_MAP, Level2Options};
                        if turbo {
                            (
                                LEVEL_2A_MAP,
                                Level2Options::opt_2a(),
                                LEVEL_2B_MAP,
                                Level2Options::options_2b(),
                                "x-2",
                            )
                        } else {
                            (
                                LEVEL_2A_MAP,
                                Level2Options::options_2a(),
                                LEVEL_2B_MAP,
                                Level2Options::options_2b(),
                                "x-2",
                            )
                        }
                    }
                    2 => {
                        use crate::worlds::level_3::{LEVEL_3A_MAP, LEVEL_3B_MAP, Level3Options};
                        if turbo {
                            (
                                LEVEL_3A_MAP,
                                Level3Options::opt_3a(),
                                LEVEL_3B_MAP,
                                Level3Options::options_3b(),
                                "x-3",
                            )
                        } else {
                            (
                                LEVEL_3A_MAP,
                                Level3Options::options_3a(),
                                LEVEL_3B_MAP,
                                Level3Options::options_3b(),
                                "x-3",
                            )
                        }
                    }
                    3 => {
                        // Pascal: case 3 使用 Level_5 数据，显示为 x-4
                        use crate::worlds::level_5::{LEVEL_5A_MAP, LEVEL_5B_MAP, Level5Options};
                        if turbo {
                            (
                                LEVEL_5A_MAP,
                                Level5Options::opt_5a(),
                                LEVEL_5B_MAP,
                                Level5Options::options_5b(),
                                "x-4",
                            )
                        } else {
                            (
                                LEVEL_5A_MAP,
                                Level5Options::options_5a(),
                                LEVEL_5B_MAP,
                                Level5Options::options_5b(),
                                "x-4",
                            )
                        }
                    }
                    4 => {
                        // Pascal: case 4 使用 Level_6 数据，显示为 x-5
                        use crate::worlds::level_6::{LEVEL_6A_MAP, LEVEL_6B_MAP, Level6Options};
                        if turbo {
                            (
                                LEVEL_6A_MAP,
                                Level6Options::opt_6a(),
                                LEVEL_6B_MAP,
                                Level6Options::options_6b(),
                                "x-5",
                            )
                        } else {
                            (
                                LEVEL_6A_MAP,
                                Level6Options::options_6a(),
                                LEVEL_6B_MAP,
                                Level6Options::options_6b(),
                                "x-5",
                            )
                        }
                    }
                    5 | _ => {
                        // Pascal: case 5 使用 Level_4 数据，显示为 x-6
                        use crate::worlds::level_4::{LEVEL_4A_MAP, LEVEL_4B_MAP, Level4Options};
                        if turbo {
                            (
                                LEVEL_4A_MAP,
                                Level4Options::opt_4a(),
                                LEVEL_4B_MAP,
                                Level4Options::options_4b(),
                                "x-6",
                            )
                        } else {
                            (
                                LEVEL_4A_MAP,
                                Level4Options::options_4a(),
                                LEVEL_4B_MAP,
                                Level4Options::options_4b(),
                                "x-6",
                            )
                        }
                    }
                };

                // 读取地下室地图
                let mut map2 = Self::convert_map_from_bytes(map2_data);
                let mut tmp_world = std::mem::take(&mut buffers.world_map);
                buffers.read_world(&mut map2, &mut tmp_world, &opt2);
                buffers.world_map = tmp_world;

                // 交换到备份
                buffers.swap();

                // 读取主关卡地图
                let mut map1 = Self::convert_map_from_bytes(map1_data);
                let mut tmp_world = std::mem::take(&mut buffers.world_map);
                buffers.read_world(&mut map1, &mut tmp_world, &opt1);
                buffers.world_map = tmp_world;

                // 初始化玩家
                players.init_player(
                    opt1.init_x as i32,
                    opt1.init_y as i32,
                    cur_player,
                    buffers,
                    enemies,
                );
                players.map_x = (opt1.init_x as i32) / W;
                players.map_y = (opt1.init_y as i32) / H + 1;

                // 设置世界编号
                buffers.world_number = world_number.to_string();
                buffers.init_level_score();
                // GPU渲染每帧完全重绘，不需要背景保存/恢复

                // 设置视口
                buffers.x_view = 0;
                buffers.y_view = 0;
                buffers.last_x_view = [0; MAX_PAGE as usize + 1];
                render_state.set_view(buffers.x_view, buffers.y_view);

                self.phase = PlayPhase::BuildLevel;
                PlayResult::Continue
            }

            PlayPhase::BuildLevel => {
                // 保存选项快照到 current_opt，后续使用引用避免重复 clone
                self.current_opt = Some(buffers.options.clone());
                let current_opt = self.current_opt.as_ref().unwrap();

                // 初始化世界元素
                figures.init_sky(current_opt.sky_type);
                figures.init_walls(
                    current_opt.wall_type1,
                    current_opt.wall_type2,
                    current_opt.wall_type3,
                    sprites,
                    current_opt,
                );
                figures.init_pipes(current_opt.pipe_color, sprites);
                enemies.init_enemy_figures(figures, sprites);
                backgr.init_backgr(current_opt.backgr_type, current_opt.clouds);

                if current_opt.stars != 0 {
                    stars.init_stars(buffers, current_opt);
                }

                figures.build_world(&mut buffers.world_map, current_opt, sprites);
                // BuildWorld 会重着色/转换草地等，运行时精灵存入figures
                // GPU 图集必须同步更新，否则地面/砖块/草颜色与 原版 不一致
                *atlas = sprites.build_atlas(figures);

                self.phase = PlayPhase::Restart;
                PlayResult::Continue
            }

            PlayPhase::RebuildLevel => {
                // 管道切换后使用当前buffers中的数据重建关卡

                // 锁定调色板防止渲染初始帧时闪烁
                render_state.clear_palette();
                render_state.lock_pal();

                // 使用当前buffers.options（已被swap交换），保存快照后使用引用
                self.current_opt = Some(buffers.options.clone());
                let current_opt = self.current_opt.as_ref().unwrap();

                // 重新初始化所有世界元素（与Pascal BuildLevel循环一致）
                figures.init_sky(current_opt.sky_type);
                figures.init_walls(
                    current_opt.wall_type1,
                    current_opt.wall_type2,
                    current_opt.wall_type3,
                    sprites,
                    current_opt,
                );
                figures.init_pipes(current_opt.pipe_color, sprites);
                enemies.init_enemy_figures(figures, sprites);
                backgr.init_backgr(current_opt.backgr_type, current_opt.clouds);

                if current_opt.stars != 0 {
                    stars.init_stars(buffers, current_opt);
                }

                figures.build_world(&mut buffers.world_map, current_opt, sprites);
                *atlas = sprites.build_atlas(figures);

                self.phase = PlayPhase::Restart;
                PlayResult::Continue
            }

            PlayPhase::Restart => {
                // 关键修复：重置show_objects标志
                // 死亡动画期间会设置show_objects=false来隐藏敌人
                // 进入Restart阶段时必须重置为true，否则敌人不可见但仍有碰撞判定
                self.show_objects = true;

                // 关键修复：清除残留的跳跃边沿状态
                // 防止从菜单进入游戏时触发意外跳跃
                // 问题原因：intro菜单阶段未消费keyboard.alt_pressed_once，
                // 或者TV遥控器/虚拟按钮在菜单淡出期间产生了新的边沿事件
                let _ = keyboard.take_alt_pressed_once();
                players.alt_pressed_once = false;
                #[cfg(target_os = "android")]
                {
                    // 消费残留的TV遥控器边沿状态
                    let _ = crate::platform::joystick_android_tv::read_tv_remote_state();
                }

                let current_opt = self.get_current_opt(buffers);

                // 关键修复：只有死亡重启才重置玩家位置和x_view
                // 管道传送后不重置（保持find_pipe_exit设置的位置）
                // 对应Pascal：restart循环不重置玩家位置，只在PlayWorld入口处初始化一次
                if !self.pipe_restart {
                    players.init_player(
                        current_opt.init_x as i32,
                        current_opt.init_y as i32,
                        cur_player,
                        buffers,
                        enemies,
                    );
                    players.map_x = (current_opt.init_x as i32) / W;
                    players.map_y = (current_opt.init_y as i32) / H + 1;

                    // 重置视图位置
                    buffers.x_view = 0;
                    buffers.y_view = 0;
                    buffers.last_x_view = [0; MAX_PAGE as usize + 1];
                } else {
                    // 管道传送：只同步last_x_view到当前x_view
                    for i in 0..=MAX_PAGE as usize {
                        buffers.last_x_view[i] = buffers.x_view;
                    }
                }
                // 管道重启时，立即触发出管道动画（在渲染初始帧之前）
                // 对应Pascal：move_player中的in_pipe检测逻辑
                let is_pipe_restart = self.pipe_restart;
                if is_pipe_restart && players.in_pipe {
                    // 检测管道出口方向并触发相应的出管道动画
                    use crate::buffers::{DM_DOWN_OUT_OF_PIPE, DM_UP_OUT_OF_PIPE};
                    let cell_below = buffers.world_get(players.map_x, players.map_y + 1);
                    if cell_below == b'0' {
                        players.start_demo(DM_UP_OUT_OF_PIPE, buffers, music);
                    } else {
                        let cell_above = buffers.world_get(players.map_x, players.map_y - 1);
                        if cell_above == b'0' {
                            players.start_demo(DM_DOWN_OUT_OF_PIPE, buffers, music);
                        }
                    }
                }

                // 重置管道重启标志
                self.pipe_restart = false;
                render_state.set_view(buffers.x_view, buffers.y_view);

                // 对应 Pascal PLAY.PAS Restart 循环开始部分
                render_state.reset_stack();
                self.text_status = false;
                status_mgr.init_status();

                blocks.init_blocks();
                tmpobj.init_temp_obj(&current_opt, sprites, figures);
                glitters.clear_glitter(render_state, buffers);
                enemies.clear_enemies();

                // 锁定调色板，防止渲染初始帧时闪烁
                render_state.clear_palette();
                render_state.lock_pal();

                buffers.game_done = false;
                buffers.passed = false;
                // 关键修复：只有非管道重启才重置demo状态
                // 管道重启时，出管道动画已在上面触发，保持demo状态
                if !is_pipe_restart {
                    buffers.demo = 0;
                }
                self.waiting = false;

                // 启动敌人
                for i in -START_ENEMIES_AT..=NH + START_ENEMIES_AT {
                    let j = (buffers.x_view / W) + i;
                    let direction = if j > players.map_x { -1 } else { 1 };
                    enemies.start_enemies(j, direction, buffers, music, &current_opt);
                }

                render_state.set_y_offset(YBASE);

                // 初始化renderer
                self.renderer = Renderer::new();
                self.renderer.only_draw = self.only_draw;
                self.renderer.show_objects = self.show_objects;
                self.renderer.show_score = self.show_score;
                // 对齐 原版：游戏过程中状态栏一直显示（地下室也一样）
                self.renderer.show_status = true;
                self.renderer.show_retrace = self.show_retrace;

                self.phase = PlayPhase::RenderInitFrames;
                PlayResult::Continue
            }

            PlayPhase::RenderInitFrames => {
                // 关键：在同一阶段内渲染所有页面，避免闪烁
                // 对应 Pascal PLAY.PAS 中的 for page := 0 to MaxPage do
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
                    self.renderer.render_init_frame(&mut ctx, page);

                    // 关键：同步 last_x_view，避免后续 GameLoop 计算错误的 scroll
                    buffers.last_x_view[page as usize] = buffers.x_view;
                }

                // 对应 Pascal PLAY.PAS 第716-717行：渲染初始帧后重置waiting
                // 注意：demo的重置已在Restart阶段处理，管道重启时不重置demo以保持出管道动画
                self.waiting = false;

                // 对应 Pascal PLAY.PAS 第719行：重置调色板到默认状态
                render_state.palette.new_palette(mpal256::mpal256_palette());

                // 获取当前选项用于后续操作
                let current_opt = self.get_current_opt(buffers);

                // 对应 Pascal PLAY.PAS 第720-723行：瀑布动画初始化（100次blink）
                for _ in 1..=100 {
                    render_state.palette_blink_wrapper(&current_opt);
                }

                // 设置调色板（天空/远景/草地）
                // 使用临时变量避免双重可变借用
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

                // 开始淡入
                render_state.unlock_pal();
                render_state.palette.start_fade_up_steps(64);

                self.phase = PlayPhase::FadingUp;
                PlayResult::Continue
            }

            PlayPhase::FadingUp => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    // 关键修复：淡入期间继续更新出管道动画
                    // 对应Pascal：fade_up是同步的，但Rust是异步的，需要在淡入期间更新动画
                    if buffers.demo != 0 {
                        players.do_demo(buffers);
                    }
                    PlayResult::Continue
                } else {
                    // 地下室砖块调色板初始化（必须在fade_up之后）
                    let current_opt = self.get_current_opt(buffers);
                    if current_opt.sky_type == 8 && current_opt.backgr_type == 4 {
                        // Pascal FIGURES.PAS InitBackGr case 8 (地下室深棕色砖块):
                        render_state.palette_out(0xFD, 17, 10, 10); // 深红棕色
                        render_state.palette_out(0xFE, 11, 5, 5); // 暗红棕色
                        render_state.palette_out(0xFF, 20, 14, 14); // 中红棕色
                        {
                            let mut pal = std::mem::take(&mut render_state.palette);
                            backgr.brick_palette(0, &mut pal, render_state);
                            render_state.palette = pal;
                        }
                    }

                    // 不要重置 self.stat，保持用户在 Options 菜单中的设置
                    // Pascal: TextStatus := Stat and (not PlayingMacro);
                    self.text_status = self.stat && !keyboard.playing_macro();
                    self.phase = PlayPhase::GameLoop;
                    PlayResult::Continue
                }
            }

            // 从暂停恢复：重绘所有页面但不重置游戏状态
            PlayPhase::ResumeFromPause => {
                // 对所有页面重绘（类似RenderInitFrames但不重置游戏状态）
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
                    self.renderer.render_init_frame(&mut ctx, page);

                    // 同步last_x_view
                    buffers.last_x_view[page as usize] = buffers.x_view;
                }

                self.phase = PlayPhase::GameLoop;
                PlayResult::Continue
            }

            PlayPhase::GameLoop => self.game_loop_tick(
                render_state,
                txt,
                music,
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
                &*atlas,
                keyboard,
                joystick,
                cur_player,
            ),

            PlayPhase::Paused => self.pause_tick(
                render_state,
                txt,
                music,
                buffers,
                players,
                enemies,
                tmpobj,
                keyboard,
                joystick,
            ),

            // 管道传送渐隐中
            PlayPhase::FadingDownForPipe => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    PlayResult::Continue
                } else {
                    // 渐隐完成，执行管道传送逻辑
                    self.execute_pipe_transition(
                        render_state,
                        players,
                        buffers,
                        enemies,
                        cur_player,
                    );
                    PlayResult::Continue
                }
            }

            PlayPhase::PipeTransition => {
                // 管道传送后重新进入相应阶段
                // 此状态不直接使用，由game_loop_tick中的管道逻辑直接转换到Restart或BuildLevel
                self.phase = PlayPhase::Restart;
                PlayResult::Continue
            }

            // 关卡完成渐隐中
            PlayPhase::FadingDownForComplete => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    PlayResult::Continue
                } else {
                    // 渐隐完成，返回关卡完成
                    self.phase = PlayPhase::LevelComplete;
                    PlayResult::LevelComplete
                }
            }

            PlayPhase::LevelComplete => PlayResult::LevelComplete,

            // 玩家死亡渐隐中
            PlayPhase::FadingDownForDeath => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    PlayResult::Continue
                } else {
                    // 渐隐完成，返回玩家死亡
                    self.phase = PlayPhase::PlayerDeath;
                    PlayResult::PlayerDeath
                }
            }

            PlayPhase::PlayerDeath => PlayResult::PlayerDeath,

            // 游戏结束渐隐中
            PlayPhase::FadingDownForGameOver => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    PlayResult::Continue
                } else {
                    // 渐隐完成，返回游戏结束
                    self.phase = PlayPhase::GameOver;
                    PlayResult::GameOver
                }
            }

            PlayPhase::GameOver => PlayResult::GameOver,

            // 退出游戏渐隐中
            PlayPhase::FadingDownForQuit => {
                if render_state.palette.is_fading() {
                    render_state.palette_fade_step();
                    PlayResult::Continue
                } else {
                    // 渐隐完成，返回退出
                    PlayResult::Quit
                }
            }
        }
    }

    /// 游戏主循环单帧处理
    #[allow(clippy::too_many_arguments)]
    fn game_loop_tick(
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
        atlas: &crate::sprites::SpriteAtlas,
        keyboard: &mut Keyboard,
        joystick: &mut JoystickState,
        cur_player: u8,
    ) -> crate::mario::PlayResult {
        use crate::mario::PlayResult;

        // 获取当前关卡选项（值类型，避免持久借用导致后续无法操作 buffers）
        let current_opt = self.get_current_opt(buffers);
        let current_scan_code = keyboard.get_current_scan_code().unwrap_or(0);

        self.loop_count += 1;

        // S - Status on/off (扫描码 31)
        if current_scan_code == 31 {
            self.stat = !self.stat;
            self.text_status = self.stat;
            keyboard.reset();
        }

        // Q - quiet/sound (扫描码 16)
        if current_scan_code == 16 {
            if music.beeper_sound {
                music.beeper_off();
                buffers.beeper_sound = false;
            } else {
                music.beeper_on();
                buffers.beeper_sound = true;
                music.beep(80);
            }
            keyboard.reset();
        }

        if buffers.text_counter >= 40 && buffers.text_counter <= 40 + MAX_PAGE {
            self.show_objects = false;
        }

        // GPU模式下不需要隐藏操作，每帧完全重绘

        buffers.lava_counter = buffers.lava_counter.wrapping_add(1);

        if !self.waiting {
            if buffers.demo == 0 {
                // 读取手柄
                joystick.read();
                players.key_left = keyboard.kb_left() || joystick.left;
                players.key_right = keyboard.kb_right() || joystick.right;
                players.key_up = keyboard.kb_up() || joystick.up;
                players.key_down = keyboard.kb_down() || joystick.down;
                // 跳跃: Alt 或 手柄 A/B 按钮
                // Android TV 遥控器按键映射:
                // - OK键: 跳跃
                // - 上键: 发射子弹
                // - 左右键: 行走
                // - 下键: 蹲下/钻管道
                let mut tv_jump_hold = false;
                let mut tv_fire = false;
                #[cfg(target_os = "android")]
                {
                    let tv = crate::platform::joystick_android_tv::read_tv_remote_state();
                    // OK键用于跳跃
                    tv_jump_hold = tv.ok;
                    // 上键用于发射子弹
                    tv_fire = tv.up;
                    // TV遥控器模式：检测到遥控器输入时启用空中慢动作
                    players.tv_remote_mode = tv.detected;
                    // TV遥控器左右键控制行走
                    players.key_left = players.key_left || tv.left;
                    players.key_right = players.key_right || tv.right;
                    players.key_down = players.key_down || tv.down;
                    if players.status == crate::players::ST_ON_THE_GROUND {
                        // 只有OK键触发跳跃
                        if tv.ok_pressed_once {
                            players.alt_pressed_once = true;
                        }
                    }
                }

                players.key_alt = keyboard.kb_alt() || joystick.button1 || tv_jump_hold;

                if players.status == crate::players::ST_ON_THE_GROUND {
                    players.alt_pressed_once |= keyboard.take_alt_pressed_once();
                    // 手柄跳跃按钮也触发 alt_pressed_once
                    if joystick.button1 {
                        players.alt_pressed_once = true;
                    }
                } else {
                    let _ = keyboard.take_alt_pressed_once();
                }

                // 加速: Ctrl 或 手柄 LB/RB (肩键)
                // 注意: 加速和发射必须分开映射，否则按加速时会触发发射动画
                // - key_ctrl: 加速功能，使用 LB/RB 肩键，TV遥控器自动开启
                // - key_space: 发射功能，使用 X/Y 按钮，TV遥控器上键
                // TV遥控器模式下自动开启加速（因为遥控器按键有限）
                players.key_ctrl = keyboard.kb_ctrl() || joystick.button2 || players.tv_remote_mode;
                players.key_space = keyboard.kb_space() || joystick.button_x || joystick.button_y || tv_fire;
                players.key_left_shift = keyboard.kb_left_shift();
                players.key_right_shift = keyboard.kb_right_shift();

                enemies.move_enemies(render_state, music, buffers, glitters, tmpobj);
                players.move_player(
                    buffers,
                    enemies,
                    tmpobj,
                    blocks,
                    render_state,
                    glitters,
                    music,
                    &current_opt,
                );
            } else {
                players.do_demo(buffers);
            }
        }

        // 检查游戏状态
        if !self.waiting {
            if buffers.passed {
                if buffers.demo == 0 || players.in_pipe {
                    self.waiting = true;
                    buffers.text_counter = 0;
                }
                buffers.text_counter += 1;
                // 注意：show_score 的设置已移到 if self.waiting 分支中
            } else if buffers.game_done {
                // Pascal一致性: lives不能为负数, 0表示本玩家GameOver
                let idx = cur_player as usize;
                if buffers.data.lives[idx] > 0 {
                    buffers.data.lives[idx] -= 1;
                } else {
                    buffers.data.lives[idx] = 0;
                }
                buffers.data.mode[cur_player as usize] = 0;
                buffers.text_counter = 0;
                buffers.data.score[cur_player as usize] += buffers.level_score;
                self.waiting = true;
                buffers.game_done = false;
            }
        }

        if self.waiting {
            buffers.text_counter += 1;

            if buffers.passed {
                // 当 text_counter >= 50 时，设置 show_score = true
                // 对应 Pascal 截图: "STAGE CLEAR!" 和 "TOTAL SCORE" 同时显示
                if !self.show_score && buffers.text_counter >= 50 {
                    self.show_score = true;
                }
                if buffers.text_counter > 250 {
                    self.waiting = false;
                    // 启动非阻塞渐隐（对应Pascal FadeDown(64)）
                    render_state.palette.start_fade_down_steps(64);
                    self.phase = PlayPhase::FadingDownForComplete;
                    return PlayResult::Continue;
                }
            } else if buffers.data.lives[cur_player as usize] == 0 {
                // 当前玩家生命为0，显示 GAME OVER 后切换到另一个玩家
                // 注意：文字渲染已移到 render_game_frame 之后，避免被 begin_gpu_frame 清除
                if buffers.text_counter >= 100 && buffers.text_counter <= 100 + MAX_PAGE {
                    self.show_score = true;
                }
                if buffers.text_counter > 350 {
                    // 启动非阻塞渐隐（对应Pascal FadeDown(64)）
                    render_state.palette.start_fade_down_steps(64);
                    self.phase = PlayPhase::FadingDownForDeath;
                    return PlayResult::Continue;
                }
            } else if buffers.text_counter > 100 {
                self.phase = PlayPhase::Restart;
                return PlayResult::Continue;
            }
        }

        // P 或 手柄START - pause (P键扫描码 25)
        // 使用kb_key检测P键状态，添加去抖动逻辑防止重复触发
        // 手柄: START键 = 暂停 (与Windows/Android手柄一致)
        let p_key_held = keyboard.kb_key(25) || joystick.button_start;
        if p_key_held && !self.p_key_was_pressed {
            // P键或START键刚被按下，进入暂停状态
            self.p_key_was_pressed = true;
            self.start_pause(
                render_state,
                txt,
                music,
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
                status_mgr,
            );
            return PlayResult::Continue;
        }
        if !p_key_held {
            // P键/START键已释放，重置标志
            self.p_key_was_pressed = false;
        }

        // 分数统计逻辑
        if self.show_score && buffers.text_counter == 120 && buffers.level_score > 0 {
            let i = if buffers.level_score > 50 {
                buffers.level_score - 50
            } else {
                0
            };
            buffers.data.score[cur_player as usize] += buffers.level_score - i;
            buffers.level_score = i;
            buffers.text_counter = 119;
            self.counting_score = true;
        } else {
            self.counting_score = false;
        }

        tmpobj.move_temp_obj(glitters, buffers);
        blocks.move_blocks();

        // 渲染（current_opt 已是值类型，可直接使用可变引用）
        let mut current_opt_mut = current_opt;
        self.move_screen(
            backgr,
            players,
            enemies,
            render_state,
            buffers,
            music,
            &mut current_opt_mut,
            figures,
            sprites,
        );

        {
            let mut ctx = RenderContext {
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
            // 对齐 原版：游戏过程中状态栏一直显示（地下室也一样）
            self.renderer.show_status = true;
            self.renderer.render_game_frame(&mut ctx);
        }

        // 关卡结束文字渲染（必须在 render_game_frame 之后，避免被 begin_gpu_frame 清除）
        // 对应Pascal截图: "STAGE CLEAR!" 和 "TOTAL SCORE" 同时显示直到关卡结束
        // 条件: passed && waiting && text_counter >= 50
        // 必须检查 waiting，避免在管道动画期间提前显示（此时 text_counter 可能已增加但还未重置）
        if buffers.passed && self.waiting && buffers.text_counter >= 50 {
            // 显示 "STAGE CLEAR!" 文字
            txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);
            txt.center_text_ui(
                render_state,
                20,
                &buffers.player_name[cur_player as usize],
                0x1E,
                buffers.x_view,
                SCREEN_WIDTH,
            );
            txt.set_font(1, FontStyle::BOLD | FontStyle::SHADOW);
            txt.center_text_ui(
                render_state,
                40,
                "STAGE CLEAR!",
                31,
                buffers.x_view,
                SCREEN_WIDTH,
            );
        }

        // 玩家死亡后显示 "GAME OVER" 文字
        if self.waiting
            && buffers.data.lives[cur_player as usize] == 0
            && buffers.text_counter >= 100
            && buffers.text_counter <= 100 + MAX_PAGE
        {
            txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);
            txt.center_text_ui(
                render_state,
                20,
                &buffers.player_name[cur_player as usize],
                0x1E,
                buffers.x_view,
                SCREEN_WIDTH,
            );
            txt.set_font(1, FontStyle::BOLD | FontStyle::SHADOW);
            txt.center_text_ui(
                render_state,
                40,
                "GAME OVER",
                31,
                buffers.x_view,
                SCREEN_WIDTH,
            );
        }

        if self.show_score {
            self.show_total_back(buffers, render_state, txt, music);
        }

        if self.show_retrace {
            render_state.set_palette(0, 0, 0, 0);
        }
        render_state.show_page();
        if self.show_retrace {
            render_state.set_palette(0, 63, 63, 63);
        }

        {
            let mut pal = std::mem::take(&mut render_state.palette);
            backgr.draw_pal_backgr(&mut pal, render_state, Some(&current_opt_mut));
            render_state.palette = pal;
        }
        render_state.palette_blink_wrapper(&current_opt_mut);
        music.tick();
        music.play_music();

        // 管道处理逻辑
        // Pascal逻辑：当 in_pipe=true 且不是游戏结束/等待状态时触发
        // 注意：进入管道动画完成时 in_pipe=true，但 demo 仍是 DM_DOWN_INTO_PIPE
        // 不能检查 demo==0，否则会阻止管道处理！
        if players.in_pipe && !buffers.game_done && !self.waiting {
            enemies.stop_enemies(buffers);
            glitters.clear_glitter(render_state, buffers);

            // 启动非阻塞渐隐（对应Pascal FadeDown(64)）
            render_state.palette.start_fade_down_steps(64);
            self.phase = PlayPhase::FadingDownForPipe;
            return PlayResult::Continue;
        }

        PlayResult::Continue
    }

    /// 辅助函数：从字节数组转换地图
    fn convert_map_from_bytes(map_bytes: &[&[u8]]) -> MapBuffer {
        use crate::buffers::MAX_WORLD_SIZE;
        let mut map: MapBuffer = [['\0'; NV as usize]; MAX_WORLD_SIZE as usize + 1];

        for (row_idx, row) in map_bytes.iter().enumerate() {
            if row_idx >= map.len() {
                break;
            }
            for (col_idx, &ch) in row.iter().enumerate() {
                if col_idx < NV as usize {
                    map[row_idx][col_idx] = ch as char;
                }
            }
        }
        map
    }

    /// move_screen 拆分 - 逻辑部分（P0-2 修复）
    ///
    /// 只负责：设置视口、启动敌人、调整地平线
    /// 不负责：渲染（由 Renderer::render_scroll 处理）
    pub fn move_screen_logic(
        &mut self,
        players: &mut Players,
        enemies: &mut Enemies,
        render_state: &mut RenderState,
        buffers: &mut Buffers,
        music: &mut MusicPlayer,
        options: &mut WorldOptions,
    ) -> i32 {
        let page = render_state.current_page() as usize;
        let scroll = buffers.x_view - buffers.last_x_view[page];

        // 1. 设置视口（带地震效果）
        if !players.earthquake {
            render_state.set_view(buffers.x_view, buffers.y_view);
        } else {
            players.earthquake_counter += 1;
            if players.earthquake_counter > 0 {
                players.earthquake = false;
            }
            render_state.set_view(
                buffers.x_view,
                buffers.y_view + random_i32(2) - random_i32(2),
            );
        }

        // 2. 启动敌人
        if scroll < 0 {
            enemies.start_enemies(
                (buffers.x_view / W) - START_ENEMIES_AT,
                1,
                buffers,
                music,
                options,
            );
        } else if scroll > 0 {
            enemies.start_enemies(
                (buffers.x_view / W) + NH + START_ENEMIES_AT,
                -1,
                buffers,
                music,
                options,
            );
        }
        // Pascal MoveScreen 会临时修改 Options.Horizon 只用于 DrawBackGr(FALSE)，然后立刻还原。
        // Rust 渲染已收敛到 renderer.rs，因此这里不再“永久写入”options.horizon，避免污染后续逻辑/绘制。

        scroll // 返回 scroll 值供渲染使用
    }

    /// move_screen：逻辑+渲染分离版本（P0-2 已修复）
    ///
    /// 现在只执行逻辑部分（视口更新、敌人启动等），渲染由 renderer.render_game_frame 统一处理
    /// 保留此函数仅为向后兼容，未使用的参数标记为 _
    pub fn move_screen(
        &mut self,
        _backgr: &mut BackGr,
        players: &mut Players,
        enemies: &mut Enemies,
        render_state: &mut RenderState,
        buffers: &mut Buffers,
        music: &mut MusicPlayer,
        options: &mut WorldOptions,
        _figures: &Figures,
        _sprites: &mut SpriteDataManager,
    ) {
        // 只执行逻辑部分，渲染由主循环中的 renderer.render_game_frame 统一处理
        let _scroll =
            self.move_screen_logic(players, enemies, render_state, buffers, music, options);
    }

    /// 执行管道传送逻辑（渐隐完成后调用）
    /// 对应Pascal管道处理中FadeDown(64)之后的代码
    fn execute_pipe_transition(
        &mut self,
        render_state: &mut RenderState,
        players: &mut Players,
        buffers: &mut Buffers,
        enemies: &mut Enemies,
        cur_player: u8,
    ) {
        // 对应Pascal: ClearPalette; LockPal; ClearVGAMem;
        render_state.clear_palette();
        render_state.lock_pal();
        render_state.clear_vga_mem();

        match players.pipe_code[0] {
            // 0xE0: 同世界传送
            0xE0 => {
                self.find_pipe_exit(players, buffers);
            }
            // 0xE1: 切换世界（进入/退出地下室）
            0xE1 => {
                buffers.swap();
                self.find_pipe_exit(players, buffers);
            }
            // 0xE7: 关卡完成（通过管道退出）
            0xE7 => {
                // 关卡完成后必须重置in_pipe状态
                players.in_pipe = false;
                players.pipe_code = [b' ', b' '];
                buffers.game_done = true;
                // 渐隐已在FadingDownForPipe阶段完成，直接进入LevelComplete
                self.phase = PlayPhase::LevelComplete;
                return;
            }
            _ => {}
        }

        // 重新初始化玩家位置
        let init_x = players.map_x * W + W / 2;
        let init_y = (players.map_y - 1) * H;
        players.init_player(init_x, init_y, cur_player, buffers, enemies);

        render_state.set_view(buffers.x_view, buffers.y_view);
        render_state.set_y_offset(YBASE);

        for i in 0..=MAX_PAGE {
            buffers.last_x_view[i as usize] = buffers.x_view;
        }

        // 根据管道类型决定下一阶段
        // 关键：设置pipe_restart标志，防止Restart阶段重置玩家位置
        self.pipe_restart = true;
        if players.pipe_code[0] == 0xE0 {
            // 同世界传送：GoTo Restart
            self.phase = PlayPhase::Restart;
        } else if players.pipe_code[0] == 0xE1 {
            // 切换世界：GoTo RebuildLevel（重建天空/背景/调色板等）
            self.phase = PlayPhase::RebuildLevel;
        }
    }

    /// 查找管道出口位置（直接使用 buffers.options，避免借用冲突导致的 clone）
    pub fn find_pipe_exit(&mut self, players: &mut Players, buffers: &mut Buffers) {
        // Pascal PLAY.PAS::FindPipeExit 匹配逻辑：
        //   for i := 0 to Options.XSize - 1 - 1 do
        //     for j := 0 to NH - 1 do  // 注意：Pascal 用 NH，不是 NV
        //       if (WorldMap^[i,j] in [$E0..$E7]) and (WorldMap^[i+1,j] = PipeCode[2]) then ...
        //
        // 注意：Pascal 中 j 的范围是 0..NH-1 = 0..15，这是因为 WorldBuffer 的 Y 范围
        // 是 -EY1..NV-1+EY2 = -8..15，所以 j=0..15 是有效的。
        // 但实际上管道标记通常在 Y=0..NV-1 = 0..12 范围内。

        // 直接使用 buffers.options，避免额外的 clone
        let x_size = buffers.options.x_size;
        let init_x = buffers.options.init_x;
        let init_y = buffers.options.init_y;

        // 防御：x_size 过小会导致下溢
        if x_size <= 1 {
            players.map_x = (init_x as i32) / W;
            players.map_y = (init_y as i32) / H + 1;
            buffers.x_view = 0;
            return;
        }

        let mut found = false;

        // Pascal 范围：i: 0..XSize-2, j: 0..NH-1
        // 使用 world_get 来正确处理坐标偏移
        for i in 0..((x_size as i32) - 1) {
            for j in 0..NH {
                if (i != players.map_x) || (j != players.map_y) {
                    let cell = buffers.world_get(i, j);
                    // Pascal: in ['à' .. 'ï'] = 0xE0..0xEF (not 0xE0..0xE7!)
                    if (0xE0..=0xEF).contains(&cell) {
                        let next_cell = buffers.world_get(i + 1, j);
                        if next_cell == players.pipe_code[1] {
                            players.map_x = i;
                            players.map_y = j;
                            buffers.x_view = (i - NH / 2 + 1) * W;
                            if buffers.x_view > ((x_size as i32) - NH) * W {
                                buffers.x_view = ((x_size as i32) - NH) * W;
                            }
                            if buffers.x_view < 0 {
                                buffers.x_view = 0;
                            }
                            found = true;
                            break;
                        }
                    }
                }
            }
            if found {
                break;
            }
        }

        if !found {
            // 找不到出口时，使用当前世界的默认初始位置
            players.map_x = (init_x as i32) / W;
            players.map_y = (init_y as i32) / H + 1;
            buffers.x_view = 0;
        }
    }

    pub fn write_total_score(
        &self,
        buffers: &Buffers,
        render_state: &mut RenderState,
        txt: &mut Txt,
    ) {
        txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW); // Bold + Shadow
        let score_str = format!("{:11}", buffers.data.score[buffers.player]);
        let mut s = String::new();
        for (i, ch) in score_str.chars().enumerate() {
            if i >= 3 && ch == ' ' {
                s.push('0');
            } else {
                s.push(ch);
            }
        }
        let text = format!("TOTAL SCORE:{}", s);
        // 使用UI层渲染，确保分数文字在所有精灵之上显示
        // 对应Pascal版本ShowTotalBack在DrawPlayer之后、ShowStatus之前的渲染顺序
        txt.center_text_ui(render_state, 120, &text, 31, buffers.x_view, SCREEN_WIDTH);
    }

    pub fn show_total_back(
        &mut self,
        buffers: &Buffers,
        render_state: &mut RenderState,
        txt: &mut Txt,
        music_player: &MusicPlayer,
    ) {
        if buffers.passed && self.counting_score {
            music_player.beep(4 * 880);
        }
        if buffers.passed && self.counting_score {
            music_player.beep(2 * 880);
        }
        self.write_total_score(buffers, render_state, txt);
        if buffers.passed && self.counting_score {
            music_player.beep(0);
        }
    }

    /// 开始暂停（初始化暂停状态）
    fn start_pause(
        &mut self,
        render_state: &mut RenderState,
        txt: &mut Txt,
        music: &mut MusicPlayer,
        buffers: &mut Buffers,
        backgr: &mut BackGr,
        figures: &mut Figures,
        sprites: &mut SpriteDataManager,
        atlas: &crate::sprites::SpriteAtlas,
        blocks: &mut Blocks,
        enemies: &mut Enemies,
        players: &mut Players,
        tmpobj: &mut TmpObjManager,
        stars: &mut Stars,
        glitters: &mut GlitterSystem,
        status_mgr: &mut Status,
    ) {
        // 初始化暂停状态
        self.pause_text = String::from("PAUSE");
        self.pause_cheat.clear();
        self.pause_tab_mode = false;
        self.pause_old_scan_code = 25; // P键的扫描码，初始化为P键按下状态

        // 暂停音乐
        music.pause_music();

        // 淡出 - 使用 render_state.palette_fade_down_wrapper确保使用当前vga调色板
        // 注意：不能用palette.fade_down，因为mario.palettes和 render_state.palette可能不同步
        // （blink_palette通过 render_state.palette_blink_wrapper修改 render_state.palette，不更新mario.palettes）
        render_state.palette_fade_down_wrapper(8);

        // 交换页面 - 关键：在另一页上绘制暂停界面，原始游戏页面保持不变
        render_state.swap_pages();

        // GPU 全量重绘：对齐 原版 的 hide_* 行为
        // 在暂停页先渲染一次“无玩家/无状态栏/无对象”的世界帧，避免暂停画面残留 Mario/状态栏
        let prev_show_objects = self.renderer.show_objects;
        let prev_show_status = self.renderer.show_status;
        let prev_show_players = self.renderer.show_players;
        let prev_only_draw = self.renderer.only_draw;

        self.renderer.show_objects = false;
        self.renderer.show_status = false;
        self.renderer.show_players = false;
        self.renderer.only_draw = false;

        {
            let mut ctx = RenderContext {
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
            self.renderer.render_game_frame(&mut ctx);
        }

        self.renderer.show_objects = prev_show_objects;
        self.renderer.show_status = prev_show_status;
        self.renderer.show_players = prev_show_players;
        self.renderer.only_draw = prev_only_draw;

        // 设置调色板 - 只设置0x0F为白色，与Pascal一致
        // 注意：不要设置0x07-0x0B等，因为它们被河流/瀑布动画使用（颜色索引7-11）
        // 直接调用render_state.set_palette，因为fade_down已经使用 render_state.palette完成
        render_state.set_palette(0x0F, 63, 63, 63); // 白色

        // 使用8x8字体，带阴影
        txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);

        // 显示PAUSE标题 - 使用UI层渲染确保在所有精灵之上
        txt.center_text_ui(render_state, 8, "PAUSE", 0x0F, buffers.x_view, SCREEN_WIDTH);

        // 显示作弊码提示 - 使用阴影效果，只使用0x0F（白色）
        txt.set_font(0, FontStyle::SHADOW);

        // 左列 - 游戏效果类作弊码（使用UI层渲染）
        let left_x = buffers.x_view + 20;
        let mut y = 24;

        txt.write_text_world_ui(render_state, left_x, y, "Press TAB for cheats:", 0x0F);
        y += 12;

        txt.write_text_world_ui(render_state, left_x, y, "03E8 = +1 Life", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "B172 = 10000 Lives", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "9C32 = Star", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "F1F2 = Mushroom", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "FFB5 = Fire Flower", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "2305 = Complete Level", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "D235 = Turbo Mode", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, left_x, y, "1UP  = 1UP Mushroom", 0x0F);

        // 右列 - 演示和调试类作弊码（使用UI层渲染）
        let right_x = buffers.x_view + 175;
        y = 36;

        txt.write_text_world_ui(render_state, right_x, y, "76DD = Record Demo", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, right_x, y, "C7B4 = Play Demo", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, right_x, y, "208D = Save Demo", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, right_x, y, "TEST = Debug Retrace", 0x0F);
        y += 10;
        txt.write_text_world_ui(render_state, right_x, y, "CREDITS = Author", 0x0F);
        y += 14;

        // 底部提示
        txt.write_text_world_ui(render_state, right_x, y, "Any key = Resume", 0x0F);

        // 进入暂停状态
        self.pause_state = PauseState::WaitingPRelease;
        self.phase = PlayPhase::Paused;
    }

    /// 暂停状态每帧处理
    #[allow(clippy::too_many_arguments)]
    fn pause_tick(
        &mut self,
        render_state: &mut RenderState,
        txt: &mut Txt,
        music: &mut MusicPlayer,
        buffers: &mut Buffers,
        players: &mut Players,
        enemies: &mut Enemies,
        tmpobj: &mut TmpObjManager,
        keyboard: &mut Keyboard,
        joystick: &mut JoystickState,
    ) -> crate::mario::PlayResult {
        use crate::mario::PlayResult;

        // 轮询按键和手柄
        keyboard.poll_os_keys();
        joystick.read();

        // 使用kb_hit检测是否有新按键事件
        let has_key_event = keyboard.kb_hit();
        let current_scan_code = keyboard.get_current_scan_code().unwrap_or(0);

        // 使用kb_key检测P键或手柄START键是否被按住
        let p_key_held = keyboard.kb_key(25) || joystick.button_start; // P键或START键

        match self.pause_state {
            PauseState::Init => {
                // 不应该到达这里，start_pause会直接设置为WaitingPRelease
                self.pause_state = PauseState::WaitingPRelease;
                PlayResult::Continue
            }

            PauseState::WaitingPRelease => {
                // 等待P键/START键释放
                if !p_key_held {
                    // P键/START键已释放，进入等待输入状态
                    self.pause_old_scan_code = 0;
                    keyboard.clear_key(); // 清除按键状态
                    self.pause_state = PauseState::WaitingInput;
                }
                PlayResult::Continue
            }

            PauseState::WaitingInput => {
                let mut end_pause = false;

                // 检测手柄按钮退出暂停 (A/B/START/SELECT)
                let gamepad_resume = joystick.detected && 
                    (joystick.button_a || joystick.button_b || 
                     joystick.button_start || joystick.button_select);
                if gamepad_resume {
                    end_pause = true;
                }

                // 检测是否有新按键事件
                if !end_pause && has_key_event && current_scan_code != 0 && current_scan_code < 0x80 {
                    // 有新的按键按下事件

                    // 检查是否按下Tab进入作弊模式
                    if current_scan_code == 15 && !self.pause_tab_mode {
                        self.pause_tab_mode = true;
                        // 在底部显示输入提示（使用白色0x0F，因为其他颜色被fade_down变暗了）
                        // 使用UI层确保文字不被游戏精灵遮挡
                        txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);
                        txt.center_text_ui(
                            render_state,
                            130,
                            "CHEAT MODE - Enter code:",
                            0x0F,
                            buffers.x_view,
                            SCREEN_WIDTH,
                        );
                    } else if self.pause_tab_mode {
                        // 作弊模式：处理按键输入
                        let ascii_char = keyboard.scan_code_to_ascii(current_scan_code);

                        if let Some(ch) = ascii_char {
                            // 有效的字母/数字键，添加到作弊码
                            self.pause_cheat.push(ch);

                            // 清除旧的回显区域（用黑色矩形覆盖）- 使用UI层确保在精灵之上
                            let clear_x = buffers.x_view + 100;
                            let clear_y = 145;
                            let clear_w = 120;
                            let clear_h = 10;
                            render_state.fill_ui_world_gpu(clear_x, clear_y, clear_w, clear_h, 0);

                            // 在底部显示当前输入的作弊码 - 使用UI层确保在精灵之上
                            txt.set_font(0, FontStyle::BOLD | FontStyle::SHADOW);
                            let display_text = format!("CODE: {}", self.pause_cheat);
                            txt.center_text_ui(
                                render_state,
                                145,
                                &display_text,
                                0x0F,
                                buffers.x_view,
                                SCREEN_WIDTH,
                            );

                            // 检查作弊码
                            end_pause = self.check_cheat_codes(
                                render_state,
                                txt,
                                buffers,
                                players,
                                enemies,
                                tmpobj,
                                keyboard,
                                music,
                            );
                        } else {
                            // 非字母数字键（如ESC、Enter、空格等），退出作弊模式
                            end_pause = true;
                        }
                    } else {
                        // 非作弊模式：任意非Tab键退出暂停
                        end_pause = true;
                    }
                }

                if end_pause {
                    self.pause_state = PauseState::Exiting;
                }
                PlayResult::Continue
            }

            PauseState::Exiting => {
                // 切换回原始游戏页面
                render_state.swap_pages();
                // GPU渲染每帧完全重绘，不需要背景恢复

                // 淡入恢复调色板（对应Pascal的FadeUp(8)）
                render_state.palette_fade_up_wrapper(8);

                keyboard.reset();
                keyboard.clear_key();

                // 标记ESC键为"已按下"状态，防止退出暂停后立即触发游戏退出
                self.esc_key_was_pressed = true;

                // 进入ResumeFromPause阶段重绘屏幕
                self.phase = PlayPhase::ResumeFromPause;
                PlayResult::Continue
            }
        }
    }

    /// 检查作弊码
    #[allow(clippy::too_many_arguments)]
    fn check_cheat_codes(
        &mut self,
        render_state: &mut RenderState,
        txt: &mut Txt,
        buffers: &mut Buffers,
        players: &mut Players,
        enemies: &mut Enemies,
        tmpobj: &mut TmpObjManager,
        keyboard: &mut Keyboard,
        music: &mut MusicPlayer,
    ) -> bool {
        const CRED_LEN: usize = 26;
        const CREDIT: [u8; 27] = [
            CRED_LEN as u8,
            b'P' + 1 + 0x10,
            b'R' + 2 + 0x20,
            b'O' + 3 + 0x30,
            b'G' + 4 + 0x40,
            b'R' + 5 + 0x50,
            b'A' + 6 + 0x60,
            b'M' + 7 + 0x70,
            b'M' + 8 + 0x80,
            b'E' + 9 + 0x10,
            b'D' + 10 + 0x20,
            b' ' + 11 + 0x30,
            b'B' + 12 + 0x40,
            b'Y' + 13 + 0x50,
            b' ' + 14 + 0x60,
            b'M' + 15 + 0x70,
            b'I' + 16 + 0x80,
            b'K' + 17 + 0x10,
            b'E' + 18 + 0x20,
            b' ' + 19 + 0x30,
            b'W' + 20 + 0x40,
            b'I' + 21 + 0x50,
            b'E' + 22 + 0x60,
            b'R' + 23 + 0x70,
            b'I' + 24 + 0x80,
            b'N' + 25 + 0x10,
            b'G' + 26 + 0x20,
        ];

        let cheat = &self.pause_cheat;

        // TEST 或 0044 - 切换显示刷新调试
        if cheat == "TEST" || cheat == "0044" {
            self.show_retrace = !self.show_retrace;
            return true;
        }

        // 03E8 - 增加一条生命
        if cheat == "03E8" {
            tmpobj.add_life(buffers, music);
            return true;
        }

        // B172 - 生命数设为10000
        if cheat == "B172" {
            buffers.data.lives[buffers.player] = 10000;
            return true;
        }

        // 9C32 - 获得无敌星星
        if cheat == "9C32" {
            enemies.cd_star = 1;
            return true;
        }

        // F1F2 - 获得蘑菇
        if cheat == "F1F2" {
            enemies.cd_champ = 1;
            return true;
        }

        // FFB5 - 获得火焰花
        if cheat == "FFB5" {
            enemies.cd_flower = 1;
            return true;
        }

        // D235 - 切换Turbo模式
        if cheat == "D235" {
            enemies.turbo = !enemies.turbo;
            return true;
        }

        // 76DD - 开始录制演示
        if cheat == "76DD" {
            keyboard.record_macro();
            return true;
        }

        // C7B4 - 播放演示
        if cheat == "C7B4" {
            keyboard.play_macro();
            return true;
        }

        // 208D - 保存演示到文件
        if cheat == "208D" {
            let _ = keyboard.save_macro();
            return true;
        }

        // 1UP - 生成1UP蘑菇
        if cheat == "1UP" {
            if self.cheats_used & 1 == 0 {
                enemies.new_enemy(TP_LIFE, 0, buffers.x_view / W, -1, 2, 0, 2, music);
                self.cheats_used |= 1;
            } else {
                enemies.new_enemy(
                    TP_CHAMP,
                    1,
                    (buffers.x_view + random_i32(255) % 100) / W,
                    -1,
                    2 - random_i32(255) % 2,
                    0,
                    2,
                    music,
                );
                if random_i32(255) % 10 == 0 {
                    self.cheats_used &= !1;
                }
            }
            return true;
        }

        // 2305 - 直接通关当前关卡
        if cheat == "2305" {
            buffers.passed = true;
            self.waiting = true;
            buffers.text_counter = 200;
            players.pipe_code[0] = 0xE7;
            players.in_pipe = true;
            return true;
        }

        // MONO - 黑白模式
        if cheat == "MONO" {
            render_state.palette.palette_effect = PE_BLACK_WHITE;
            {
                let mut pal = std::mem::take(&mut render_state.palette);
                let p = pal.palette.clone();
                pal.refresh_palette(&p, render_state);
                render_state.palette = pal;
            }
            return true;
        }

        // EGAMODE - EGA 16色模式
        if cheat == "EGAMODE" {
            render_state.palette.palette_effect = PE_EGA_MODE;
            {
                let mut pal = std::mem::take(&mut render_state.palette);
                let p = pal.palette.clone();
                pal.refresh_palette(&p, render_state);
                render_state.palette = pal;
            }
            return true;
        }

        // VGAMODE 或 COLOR - 恢复正常颜色模式
        if cheat == "VGAMODE" || cheat == "COLOR" {
            render_state.palette.palette_effect = PE_NO_EFFECT;
            {
                let mut pal = std::mem::take(&mut render_state.palette);
                let p = pal.palette.clone();
                pal.refresh_palette(&p, render_state);
                render_state.palette = pal;
            }
            return true;
        }

        // CREDITS - 显示开发者信息
        if cheat == "CREDITS" {
            self.pause_text.clear();
            for i in 1..=CRED_LEN {
                let encoded = CREDIT[i];
                let decoded = (encoded.wrapping_sub(i as u8))
                    .wrapping_sub(0x10)
                    .wrapping_sub(((i - 1) % 8) as u8 * 0x10);
                self.pause_text.push(decoded as char);
            }
            // GPU渲染每帧完全重绘，不需要背景保存/恢复
            // 使用UI层渲染确保在精灵之上显示
            txt.center_text_ui(
                render_state,
                85,
                &self.pause_text,
                0x0F,
                buffers.x_view,
                SCREEN_WIDTH,
            );
            // 不立即退出，让用户看到信息
            return false;
        }

        false
    }
}
