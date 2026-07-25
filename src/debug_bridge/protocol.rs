//! AI 与 Mario 之间的通信协议
//!
//! 使用 JSON 行协议（newline-delimited JSON）：每条消息是一个 JSON 对象，以 \n 结尾。
//! 这样便于人类调试，也便于测试代码直接读写。

use serde::{Deserialize, Serialize};

/// AI -> Mario 的控制命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// 按下或释放某个按键
    SetKey { key: String, pressed: bool },
    /// 从指定关卡（0-5）重新开始新游戏
    StartLevel { level: i32 },
    /// 用当前关卡重新开始
    Restart,
    /// 结束游戏进程
    Quit,
    /// 切换运行速度
    SetSpeed { mode: SpeedMode },
    /// 设置随机种子（便于可复现训练）
    SetSeed { seed: u64 },
    /// 立即返回一次当前状态（普通模式下每帧已自动发送）
    GetState,
    /// 跳过 Intro 菜单，直接开始第一关（等同于 `StartLevel { level: 0 }`）。
    StartGame,
}

/// 运行速度模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedMode {
    /// 限制帧率（默认 60 FPS）
    Normal,
    /// 不限帧，CPU 能跑多快就跑多快
    Fast,
}

/// Mario -> AI 的观测数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Observation {
    pub frame: u64,
    pub main_phase: String,
    pub play_phase: String,
    pub level_index: i32,
    pub world_number: String,
    pub player: PlayerInfo,
    pub camera: CameraInfo,
    pub stats: GameStats,
    pub enemies: Vec<EnemyInfo>,
    pub world: WorldInfo,
    /// 游戏是否已经请求退出
    pub done: bool,
    /// 当前是否处于等待状态（如关卡开始/结束动画期间无法操作）
    pub waiting: bool,
    /// 是否处于 Demo 播放状态（0=否，>0=是）
    pub demo: i32,
    /// 当前键盘输入状态（供 AI/调试确认命令是否生效）
    pub key_right: bool,
    pub key_left: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_jump: bool,
    pub key_fire: bool,
    pub key_run: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    /// 玩家状态（ST_ON_THE_GROUND / ST_JUMPING / ST_FALLING）
    pub status: u8,
    /// 形态（0=small, 1=large, 2=fire）
    pub mode: u8,
    /// 朝向（0=left, 1=right）
    pub direction: i32,
    /// 是否在管道中
    pub in_pipe: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CameraInfo {
    pub x_view: i32,
    pub y_view: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameStats {
    pub lives: i16,
    pub coins: i16,
    pub score: i32,
    pub level_score: i32,
    pub progress: i16,
    pub game_done: bool,
    pub passed: bool,
    pub quit_game: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyInfo {
    pub tp: i32,
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub status: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldInfo {
    pub width: usize,
    pub height: usize,
    /// 地图有效数据的 x 起始偏移（常量 EX）
    pub x_offset: i32,
    /// 地图有效数据的 y 起始偏移（常量 EY1）
    pub y_offset: i32,
    /// 当前关卡实际宽度（格子数）
    pub x_size: u16,
    /// 完整世界地图，包含边界填充
    pub tiles: Vec<Vec<u8>>,
}
