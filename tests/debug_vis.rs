//! Mario debug-vis 集成测试
//!
//! 自动化完整流程（通过 TCP 控制游戏窗口）：
//! 1. 启动 mario-debug-vis 并连接 TCP
//! 2. 发送 `start_game` 从菜单进入第一关
//! 3. 发送 `set_key right` 控制 Mario 向右走
//! 4. 发送 `set_key jump` 控制 Mario 跳跃
//! 5. 发送 `quit` 退出
//!
//! 运行方式（建议串行 + 显示日志，方便观察窗口中的 Mario 移动）：
//!   cargo test --features debug-vis debug_vis_normal -- --nocapture --test-threads=1
//!
//! `--fast` 版本会全速跑完，窗口一闪而过；想看清动作请跑 `debug_vis_normal`。
//!
//! 注意：本测试需要可用的 GPU/显示环境，CI 无头环境可能跳过或失败。

#![cfg(feature = "debug-vis")]

mod common;

#[test]
fn debug_vis_fast_can_start_and_be_controlled() {
    let mut client = common::spawn_and_connect("mario-debug-vis", true);
    common::run_full_flow(&mut client);
}

#[test]
fn debug_vis_normal_can_start_and_be_controlled() {
    let mut client = common::spawn_and_connect("mario-debug-vis", false);
    common::run_full_flow(&mut client);
}
