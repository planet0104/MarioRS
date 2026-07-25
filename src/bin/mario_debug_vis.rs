//! Mario 可视化调试版本
//!
//! 与 `mario-debug-headless` 类似，但保留 wgpu 渲染窗口，方便人工观察游戏状态。
//! 通过 TCP 与 AI 训练代码通信：
//!   1. 启动时绑定端口（`--port 0` 由系统分配）
//!   2. 在 stdout 打印 `MARIO_PORT <port>`
//!   3. 等待 AI 连接后，每帧发送 JSON 行形式的 Observation
//!   4. 接收 AI 发来的 JSON 行命令（SetKey / StartLevel / Restart / Quit / SetSpeed 等）
//!
//! 启动参数：
//!   --port <n>      指定监听端口（0 表示自动分配）
//!   --fast          启动即进入全速模式（不限制帧率）
//!   --fps <n>       设置正常模式帧率（默认 60）
//!   --level <n>     跳过 Intro 菜单，直接进入指定关卡（0-5）
//!   --auto-start    等同于 `--level 0`
//!
//! 用法说明：
//!   - 手动观察：加 `--auto-start` 直接进入第一关
//!   - AI/脚本控制：启动后连接 TCP，发送 `{"cmd":"start_game"}` 或 SetKey 等命令
//!   - 自动化集成测试：运行 `cargo test --features debug-vis debug_vis`
//!
//! 构建命令：
//!   cargo build --bin mario-debug-vis --features debug-vis

#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]

use std::io::Write;

use mario::debug_bridge::Bridge;
use mario::platform::{install_debug_bridge, set_debug_speed, set_debug_start_level};

struct Args {
    port: u16,
    level: Option<i32>,
    fast: bool,
    fps: f64,
}

fn parse_args() -> Args {
    let mut args = Args {
        port: 0,
        level: None,
        fast: false,
        fps: 60.0,
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
            "--auto-start" => {
                args.level = Some(0);
            }
            "--fast" => {
                args.fast = true;
            }
            "--fps" => {
                args.fps = iter.next().and_then(|s| s.parse().ok()).unwrap_or(60.0);
            }
            _ => {}
        }
    }
    args
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[mario-debug-vis] starting...");
    let args = parse_args();

    let (bridge, port) = Bridge::listen(args.port)?;
    println!("MARIO_PORT {}", port);
    let _ = std::io::stdout().flush();

    install_debug_bridge(bridge);
    set_debug_speed(args.fast, args.fps);
    set_debug_start_level(args.level);

    if args.level.is_none() {
        eprintln!(
            "[mario-debug-vis] waiting on Intro menu. Connect TCP and send \
             {{\"cmd\":\"start_game\"}}, or restart with --auto-start."
        );
    }

    mario::platform::run_game()
}
