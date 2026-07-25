//! 调试版本集成测试公共 helper
//!
//! 提供 TCP 连接、发送命令、读取 observation 以及完整的
//! “启动游戏 -> 进入第一关 -> 控制 Mario 移动/跳跃 -> 退出”流程。

#![cfg(feature = "debug-bridge")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct TestClient {
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: TcpStream,
}

/// 启动指定的调试二进制，读取 `MARIO_PORT` 并建立 TCP 连接。
pub fn spawn_and_connect(bin_name: &str, fast: bool) -> TestClient {
    let default_path = format!("target/debug/{}.exe", bin_name);
    let bin_path = std::env::var(format!("CARGO_BIN_EXE_{}", bin_name))
        .unwrap_or_else(|_| default_path);

    let mut args = vec!["--port".to_string(), "0".to_string()];
    if fast {
        args.push("--fast".to_string());
    }

    let mut child = Command::new(&bin_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {}", bin_name, e));

    let stdout = child.stdout.take().expect("no stdout");
    let mut stdout_reader = BufReader::new(stdout);

    let mut port_line = String::new();
    stdout_reader
        .read_line(&mut port_line)
        .expect("failed to read MARIO_PORT line");
    let port: u16 = port_line
        .trim()
        .strip_prefix("MARIO_PORT ")
        .expect("unexpected MARIO_PORT line")
        .parse()
        .expect("invalid port number");
    assert!(port > 0, "port should be > 0");

    // 继续 drain stdout，避免子进程因写入关闭的管道而 panic（可视化版本会输出更多日志）
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match stdout_reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    let stream = TcpStream::connect(("127.0.0.1", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set_read_timeout failed");

    let reader_stream = stream
        .try_clone()
        .expect("failed to clone stream for reading");
    let writer = stream;

    TestClient {
        child,
        reader: BufReader::new(reader_stream),
        writer,
    }
}

/// 完整测试流程：连接 -> StartGame -> 进入 GameLoop -> 右移 -> 跳跃 -> Quit。
pub fn run_full_flow(client: &mut TestClient) {
    eprintln!("[debug_vis test] 读取首帧 observation...");
    let mut obs = read_obs(&mut client.reader);
    assert!(
        obs["frame"].as_u64().unwrap_or(0) > 0,
        "frame count should be > 0"
    );
    assert!(obs["player"].is_object(), "player field missing");
    assert!(obs["world"].is_object(), "world field missing");
    eprintln!(
        "[debug_vis test] 首帧: main_phase={}, frame={}",
        obs["main_phase"].as_str().unwrap_or("?"),
        obs["frame"].as_u64().unwrap_or(0)
    );

    // 自动从菜单进入第一关
    eprintln!("[debug_vis test] 发送 start_game，等待进入第一关...");
    send_cmd(&mut client.writer, &serde_json::json!({ "cmd": "start_game" }));

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut playing_obs = None;
    while Instant::now() < deadline {
        obs = read_obs(&mut client.reader);
        if obs["main_phase"].as_str() == Some("Playing")
            && obs["play_phase"].as_str() == Some("GameLoop")
        {
            playing_obs = Some(obs.clone());
            break;
        }
    }
    let playing_obs =
        playing_obs.expect("game did not enter Playing/GameLoop phase after StartGame");
    assert_eq!(
        playing_obs["level_index"].as_i64(),
        Some(0),
        "should start at level 0"
    );
    eprintln!(
        "[debug_vis test] 已进入第一关: x={}, y={}",
        playing_obs["player"]["x"].as_i64().unwrap_or(-1),
        playing_obs["player"]["y"].as_i64().unwrap_or(-1)
    );

    // 控制 Mario 向右移动
    eprintln!("[debug_vis test] 发送 right 键，等待 Mario 向右移动...");
    send_cmd(
        &mut client.writer,
        &serde_json::json!({"cmd": "set_key", "key": "right", "pressed": true }),
    );

    let start_x = playing_obs["player"]["x"]
        .as_i64()
        .expect("player x missing");
    let mut moved = false;
    for _ in 0..60 {
        obs = read_obs(&mut client.reader);
        let x = obs["player"]["x"].as_i64().expect("player x missing");
        if x > start_x {
            moved = true;
            eprintln!("[debug_vis test] Mario 已向右移动: {} -> {}", start_x, x);
            break;
        }
    }
    assert!(moved, "player should move right after pressing right");

    // 停止移动并跳跃
    eprintln!("[debug_vis test] 发送 jump 键，等待 Mario 跳跃...");
    send_cmd(
        &mut client.writer,
        &serde_json::json!({"cmd": "set_key", "key": "right", "pressed": false }),
    );
    send_cmd(
        &mut client.writer,
        &serde_json::json!({"cmd": "set_key", "key": "jump", "pressed": true }),
    );

    let jump_start_y = obs["player"]["y"]
        .as_i64()
        .expect("player y missing");
    let mut jumped = false;
    for _ in 0..60 {
        obs = read_obs(&mut client.reader);
        let y = obs["player"]["y"].as_i64().expect("player y missing");
        let vy = obs["player"]["vy"].as_i64().expect("player vy missing");
        let status = obs["player"]["status"]
            .as_i64()
            .expect("player status missing");
        if y < jump_start_y || vy < 0 || status != 0 {
            jumped = true;
            eprintln!(
                "[debug_vis test] Mario 已跳跃: y {} -> {}, vy={}",
                jump_start_y, y, vy
            );
            break;
        }
    }
    assert!(jumped, "player should jump after pressing jump");

    send_cmd(
        &mut client.writer,
        &serde_json::json!({"cmd": "set_key", "key": "jump", "pressed": false }),
    );

    // 发送 Quit 并等待进程退出
    eprintln!("[debug_vis test] 发送 quit，等待进程退出...");
    send_cmd(&mut client.writer, &serde_json::json!({ "cmd": "quit" }));

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match read_obs_opt(&mut client.reader) {
            Some(o) if o["done"].as_bool() == Some(true) => break,
            _ => continue,
        }
    }

    let status = client.child.wait().expect("child did not exit");
    assert!(status.success() || status.code().is_some());
}

pub fn send_cmd(writer: &mut TcpStream, cmd: &serde_json::Value) {
    let s = serde_json::to_string(cmd).expect("failed to serialize command");
    writeln!(writer, "{}", s).expect("failed to write command");
    writer.flush().expect("failed to flush command");
}

pub fn read_obs(reader: &mut BufReader<TcpStream>) -> serde_json::Value {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("failed to read observation");
    serde_json::from_str(&line).expect("observation is not valid JSON")
}

pub fn read_obs_opt(reader: &mut BufReader<TcpStream>) -> Option<serde_json::Value> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => serde_json::from_str(&line).ok(),
    }
}
