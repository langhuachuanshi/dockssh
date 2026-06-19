//! 终端 (PTY) 模块：docker exec -it 的双向透传。
//!
//! 架构（保留 channel，支持真实 resize）：
//!
//! `pty_start` 时：
//! 1. 从 pool 拿 SshClient，开 channel + request_pty + exec（短暂持锁，拿完即放）
//! 2. spawn 一个后台 task，**独占 channel**（russh 的 wait/data/window_change 都是
//!    &mut self，不能跨 task 共享，也不能用 Mutex——wait 长期持锁会阻塞 write），
//!    通过 mpsc 接收 write/resize 指令
//! 3. task 内 select! 三件事：
//!    - channel.wait()     → 远端 ChannelMsg::Data base64 emit 给前端
//!    - cmd_rx.recv()      → PtyCmd::Write => channel.data(bytes)
//!                           PtyCmd::Resize => channel.window_change(cols, rows, 0, 0)
//!    - stop_rx            → 主动停止
//! 4. 把 {cmd_tx, stop_tx, host_id} 存进 PtySessions 表
//!
//! 这样 `pty_write` / `pty_resize` 只往 cmd_tx 投指令，**完全不接触 SshClient 锁**，
//! resize 也能真正通知远端 PTY 尺寸变化（修复 vim/top 错位、终端变形等问题）。
//!
//! 事件命名约定：
//!   - 数据块：`dockssh://pty-data:{session_id}`    payload = base64 字符串
//!   - 退出：  `dockssh://pty-exit:{session_id}`     payload = 退出码 i32
//!   - 错误：  `dockssh://pty-error:{session_id}`   payload = 错误描述

use std::collections::HashMap;

use base64::Engine;
use russh::ChannelMsg;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::ssh::client::SshClient;

/// 后台 task 处理的指令：写入按键或调整 PTY 尺寸。
enum PtyCmd {
    Write(Vec<u8>),
    Resize(u32, u32),
}

/// 单个 PTY 会话的后台句柄。
/// 存在 PtySessions 表里，供 pty_write / pty_resize / pty_kill 取用。
pub struct PtyHandle {
    /// 前端指令（写入/resize）→ 后台 task（独占 channel）
    pub cmd_tx: mpsc::UnboundedSender<PtyCmd>,
    /// 主动停止信号
    pub stop_tx: oneshot::Sender<()>,
    /// 归属主机，用于断开/删除主机时批量清理
    pub host_id: String,
}

/// 全局 PTY 会话表：session_id -> PtyHandle。
/// 由 Tauri manage，注入到每个 pty_* command。
#[derive(Default)]
pub struct PtySessions(pub Mutex<HashMap<String, PtyHandle>>);

impl PtySessions {
    /// 启动一个 PTY 会话，返回 session_id。
    ///
    /// command 形如 `docker exec -it <container> sh`（由调用方拼好）。
    pub async fn start(
        &self,
        client: &mut SshClient,
        app: AppHandle,
        host_id: String,
        command: String,
        cols: u32,
        rows: u32,
    ) -> AppResult<String> {
        // 开 PTY channel（短暂持 SshClient 锁，拿完即放）
        let mut channel = client.open_pty_channel(&command, cols, rows).await?;

        let session_id = Uuid::new_v4().to_string();
        let data_event = format!("dockssh://pty-data:{session_id}");
        let exit_event = format!("dockssh://pty-exit:{session_id}");
        let error_event = format!("dockssh://pty-error:{session_id}");

        // 指令通道（unbounded：按键/resize 是用户实时操作，背压无意义）
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<PtyCmd>();
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

        let sid_for_task = session_id.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // 远端 → 前端
                    msg = channel.wait() => {
                        match msg {
                            None => {
                                // 远端关闭
                                let _ = app.emit(&exit_event, 0);
                                break;
                            }
                            Some(ChannelMsg::Data { ref data }) => {
                                let b64 = base64::engine::general_purpose::STANDARD
                                    .encode(data);
                                let _ = app.emit(&data_event, b64);
                            }
                            Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                                // stderr 也当数据送给前端（终端里 stderr 同样要显示）
                                let b64 = base64::engine::general_purpose::STANDARD
                                    .encode(data);
                                let _ = app.emit(&data_event, b64);
                            }
                            Some(ChannelMsg::ExitStatus { exit_status }) => {
                                let _ = app.emit(&exit_event, exit_status as i32);
                            }
                            Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) => {
                                let _ = app.emit(&exit_event, 0);
                                break;
                            }
                            Some(_) => {}
                        }
                    }
                    // 前端 → 远端（写入 / resize）
                    maybe = cmd_rx.recv() => {
                        match maybe {
                            Some(PtyCmd::Write(bytes)) => {
                                if let Err(e) = channel.data(&bytes[..]).await {
                                    let _ = app.emit(&error_event, format!("写入失败: {e}"));
                                    break;
                                }
                            }
                            Some(PtyCmd::Resize(c, r)) => {
                                // 真正通知远端 PTY 尺寸变化（修复 vim/top 错位、变形）
                                if let Err(e) = channel.window_change(c, r, 0, 0).await {
                                    let _ = app.emit(
                                        &error_event,
                                        format!("resize 失败: {e}"),
                                    );
                                }
                            }
                            None => break, // cmd_tx 被丢弃，前端不再输入
                        }
                    }
                    // 主动停止
                    _ = &mut stop_rx => {
                        let _ = channel.close().await;
                        break;
                    }
                }
            }
            log::info!("[pty] session {sid_for_task} 结束");
        });

        self.0.lock().await.insert(
            session_id.clone(),
            PtyHandle {
                cmd_tx,
                stop_tx,
                host_id,
            },
        );
        Ok(session_id)
    }

    /// 前端按键写入。
    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> AppResult<()> {
        let map = self.0.lock().await;
        let h = map
            .get(session_id)
            .ok_or_else(|| AppError::Other(format!("终端会话不存在: {session_id}")))?;
        h.cmd_tx
            .send(PtyCmd::Write(data))
            .map_err(|_| AppError::Other("终端会话已关闭".into()))?;
        Ok(())
    }

    /// 通知远端 PTY 尺寸变化（window_change）。
    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> AppResult<()> {
        let map = self.0.lock().await;
        let h = map
            .get(session_id)
            .ok_or_else(|| AppError::Other(format!("终端会话不存在: {session_id}")))?;
        h.cmd_tx
            .send(PtyCmd::Resize(cols, rows))
            .map_err(|_| AppError::Other("终端会话已关闭".into()))?;
        Ok(())
    }

    /// 停止并移除一个会话。
    pub async fn kill(&self, session_id: &str) -> AppResult<()> {
        let h = self.0.lock().await.remove(session_id);
        if let Some(h) = h {
            let _ = h.stop_tx.send(());
        }
        Ok(())
    }

    /// 清理某主机的所有会话（断开/删除主机时调用）。
    pub async fn kill_by_host(&self, host_id: &str) {
        let mut map = self.0.lock().await;
        let ids: Vec<String> = map
            .iter()
            .filter(|(_, h)| h.host_id == host_id)
            .map(|(k, _)| k.clone())
            .collect();
        for id in ids {
            if let Some(h) = map.remove(&id) {
                let _ = h.stop_tx.send(());
            }
        }
    }
}

/// docker exec -it 命令构造。
/// shell 探测：优先 bash，缺失回退 sh。
pub fn build_exec_cmd(container: &str, user: Option<&str>, cwd: Option<&str>) -> String {
    // 用 sh -c 包装，让远端自己选 bash/sh，避免预先探测的往返。
    let user_arg = match user {
        Some(u) => format!(" --user {u}"),
        None => String::new(),
    };
    // 进入工作目录（若有）
    let cd = match cwd {
        Some(d) => format!("cd {d} 2>/dev/null; "),
        None => String::new(),
    };
    format!(
        "docker exec -it{user_arg} {container} sh -c \"{cd}command -v bash >/dev/null 2>&1 && exec bash || exec sh\""
    )
}
