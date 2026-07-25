//! Mario 命令行调试版本（无窗口、无 GPU）
//!
//! 通过 TCP 与 AI 训练代码通信：
//!   1. 启动时绑定端口（`--port 0` 由系统分配）
//!   2. 在 stdout 打印 `MARIO_PORT <port>`
//!   3. 等待 AI 连接后，每帧发送 JSON 行形式的 Observation
//!   4. 接收 AI 发来的 JSON 行命令（SetKey / StartLevel / Restart / Quit 等）

#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]

use std::io::Write;

use mario::debug_bridge::{Bridge, Command, SpeedMode};
use mario::game_runner::GameState;
use mario::platform::common::frame_timer::FrameTimer;
use mario::platform::common::random;
use mario::platform::FrameResult;

struct Args {
    port: u16,
    level: Option<i32>,
    fast: bool,
    fps: f64,
    seed: Option<u64>,
}

fn parse_args() -> Args {
    let mut args = Args {
        port: 0,
        level: None,
        fast: false,
        fps: 60.0,
        seed: None,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--port" => {
                args.port = iter.next().unwrap_or_default().parse().unwrap_or(0);
            }
            "--level" => {
                args.level = iter.next().and_then(|s| s.parse().ok());
            }
            "--fast" => {
                args.fast = true;
            }
            "--fps" => {
                args.fps = iter.next().and_then(|s| s.parse().ok()).unwrap_or(60.0);
            }
            "--seed" => {
                args.seed = iter.next().and_then(|s| s.parse().ok());
            }
            _ => {}
        }
    }
    args
}

fn main() -> Result<(), String> {
    eprintln!("[mario-debug-headless] starting...");
    let args = parse_args();

    let (mut bridge, port) = Bridge::listen(args.port)?;
    println!("MARIO_PORT {}", port);
    let _ = std::io::stdout().flush();

    if let Some(seed) = args.seed {
        random::reseed(seed);
    }

    let mut game = GameState::new();
    if let Some(level) = args.level {
        game.start_new_game_at_level(level);
    }

    let mut fast = args.fast;
    let fps = args.fps.max(1.0);
    let mut timer = FrameTimer::new(fps);

    loop {
        for cmd in bridge.poll_commands() {
            match cmd {
                Command::SetSpeed { mode } => {
                    fast = matches!(mode, SpeedMode::Fast);
                    if !fast {
                        timer = FrameTimer::new(fps);
                    }
                }
                Command::SetSeed { seed } => random::reseed(seed),
                Command::GetState => {}
                other => game.apply_command(other),
            }
        }

        if game.game.should_exit() {
            break;
        }

        let result = game.frame_update();
        let obs = game.observe();
        if bridge.send_observation(obs).is_err() {
            break;
        }

        if result == FrameResult::Exit || game.game.should_exit() {
            break;
        }

        if !fast {
            timer.wait_if_needed();
            timer.advance();
        }
    }

    game.shutdown();
    eprintln!("[mario-debug-headless] exiting.");
    Ok(())
}
