//! Mario debug-headless 集成测试
//!
//! 分别验证 `--fast` 和默认正常速度模式下的完整流程：
//! 1. 进程能正常启动并输出 `MARIO_PORT <port>`
//! 2. 可通过 `StartGame` 命令自动从菜单进入游戏
//! 3. 进入游戏后收到 `main_phase == "Playing"` 的 Observation
//! 4. 可发送 `SetKey` 命令控制 Mario 移动、跳跃
//! 5. 发送 Quit 命令后进程退出

#![cfg(feature = "debug-headless")]

mod common;

#[test]
fn debug_headless_fast_can_start_and_be_controlled() {
    let mut client = common::spawn_and_connect("mario-debug-headless", true);
    common::run_full_flow(&mut client);
}

#[test]
fn debug_headless_normal_can_start_and_be_controlled() {
    let mut client = common::spawn_and_connect("mario-debug-headless", false);
    common::run_full_flow(&mut client);
}
