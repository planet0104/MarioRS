//! TCP 通信桥：把命令行/AI 发来的 JSON 命令转成 Command，
//! 并把每帧的 Observation 以 JSON 行形式发送回去。
//!
//! 为了便于 AI 训练时按帧同步，Observation 采用同步写入：
//! `send_observation` 会阻塞直到数据写入 TCP。这样 AI 每读一帧就对应
//! 游戏当前帧，避免 channel 缓冲导致观测滞后。

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use super::protocol::{Command, Observation};

/// AI 通信桥
pub struct Bridge {
    command_rx: Receiver<Command>,
    writer: Option<BufWriter<TcpStream>>,
    stream_rx: Receiver<TcpStream>,
}

impl Bridge {
    /// 在指定端口监听，等待 AI 连接。
    ///
    /// 返回 `(Bridge, 实际端口号)`。如果传入 `0`，系统自动分配端口。
    /// 函数会立即返回端口；实际的 TCP accept 在后台线程中进行，
    /// 这样调用方可以先在 stdout 打印 `MARIO_PORT <port>`，再由 AI 侧连接。
    pub fn listen(port: u16) -> Result<(Self, u16), String> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("bind 127.0.0.1:{} failed: {}", port, e))?;
        let actual_port = listener
            .local_addr()
            .map_err(|e| format!("get local_addr failed: {}", e))?
            .port();

        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (stream_tx, stream_rx) = channel::<TcpStream>();

        // accept / 读写都在后台线程，避免阻塞主循环输出端口
        thread::spawn(move || {
            let (stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[debug-bridge] accept failed: {}", e);
                    let _ = cmd_tx.send(Command::Quit);
                    return;
                }
            };
            // TCP 流使用阻塞模式，便于同步写入
            let _ = stream.set_nonblocking(false);

            let read_stream = match stream.try_clone() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[debug-bridge] clone stream failed: {}", e);
                    let _ = cmd_tx.send(Command::Quit);
                    return;
                }
            };

            // 读线程：把 AI 命令解包后传给主循环
            let cmd_tx2 = cmd_tx.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(read_stream);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => {
                            let _ = cmd_tx2.send(Command::Quit);
                            break;
                        }
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<Command>(trimmed) {
                                Ok(cmd) => {
                                    if cmd_tx2.send(cmd).is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[debug-bridge] invalid command '{}': {}", trimmed, e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[debug-bridge] read error: {}", e);
                            let _ = cmd_tx2.send(Command::Quit);
                            break;
                        }
                    }
                }
            });

            // 把写端交给主循环
            let _ = stream_tx.send(stream);
        });

        Ok((Bridge {
            command_rx: cmd_rx,
            writer: None,
            stream_rx,
        }, actual_port))
    }

    /// 一次性取出当前帧收到的所有命令
    pub fn poll_commands(&mut self) -> Vec<Command> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.command_rx.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }

    /// 非阻塞地尝试接收 AI 连接。可视化版本每帧调用，避免阻塞渲染线程。
    pub fn try_connect(&mut self) -> bool {
        if self.writer.is_some() {
            return true;
        }
        match self.stream_rx.try_recv() {
            Ok(stream) => {
                self.writer = Some(BufWriter::new(stream));
                true
            }
            Err(_) => false,
        }
    }

    /// 发送一帧观测数据（同步写入，保证 AI 读到的是当前帧）。
    /// 首次调用时会阻塞等待 AI 连接（headless 版本使用）。
    pub fn send_observation(&mut self, obs: Observation) -> Result<(), String> {
        if self.writer.is_none() {
            match self.stream_rx.recv() {
                Ok(stream) => self.writer = Some(BufWriter::new(stream)),
                Err(_) => return Err("failed to accept AI connection".to_string()),
            }
        }

        let line = serde_json::to_string(&obs)
            .map_err(|e| format!("serialize observation failed: {}", e))?;
        let writer = self.writer.as_mut().unwrap();
        writeln!(writer, "{}", line)
            .and_then(|_| writer.flush())
            .map_err(|e| format!("send observation failed: {}", e))
    }

    /// 返回是否已经与 AI 建立连接。
    /// 可视化版本可用此避免在没有 AI 连接时阻塞渲染线程。
    pub fn is_connected(&self) -> bool {
        self.writer.is_some()
    }
}
