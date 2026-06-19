//! 终端 (PTY) 模块：docker exec -it 的双向透传。
//!
//! 架构（解决「写入不碰 SshClient 锁」的核心问题）：
//!
//! `pty_start` 时：
//! 1. 从 pool 拿 SshClient，开 channel + request_pty + exec（短暂持锁，拿完即放）
//! 2. channel `into_stream()` → `tokio::io::split()` 得到 reader / writer 两半
//! 3. spawn 一个后台 task，select! 三件事：
//!    - reader.read()       → 远端输出 base64 编码后 emit 给前端
//!    - write_rx.recv()     → 前端按键转 writer.write_all() 送远端
//!    - stop_rx             → 主动停止，关 stream
//! 4. 把 {write_tx, stop_tx, host_id} 存进 PtySessions 表
//!
//! 这样 `pty_write` 只往 write_tx 投数据，**完全不接触 SshClient 的 Mutex**，
//! 也不会和后台读取 task 争抢 channel。channel 被 into_stream 消费后，
//! 其读写两半分属 select 的不同分支，无借用冲突。
//!
//! ⚠️ 已知限制：russh 0.46 的 `Channel` 在 `into_stream()` 消费后无法再调
//! `window_change()`（库的固有设计）。因此当前 resize 仅在前端 FitAddon 层面
//! 调整 xterm 显示，不会通知远端 PTY 尺寸。对 vim/top 等全屏 TUI 程序会有
//! 画面错位。后续可通过改用「保留 channel + 多 task 分发」架构解决，但需要
//! 包一层 Arc<Mutex<Channel>> 且接受更复杂的生命周期管理。
//!
//! 事件命名约定：
//!   - 数据块：`dockssh://pty-data:{session_id}`    payload = base64 字符串
//!   - 退出：  `dockssh://pty-exit:{session_id}`     payload = 退出码 i32
//!   - 错误：  `dockssh://pty-error:{session_id}`   payload = 错误描述

use std::collections::HashMap;

use base64::Engine;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::ssh::client::SshClient;

/// 单个 PTY 会话的后台句柄。
/// 存在 PtySessions 表里，供 pty_write / pty_resize / pty_kill 取用。
pub struct PtyHandle {
    /// 前端按键 → 后台 task（转 writer.write_all）
    pub write_tx: mpsc::UnboundedSender<Vec<u8>>,
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
        let channel = client.open_pty_channel(&command, cols, rows).await?;

        let session_id = Uuid::new_v4().to_string();
        let data_event = format!("dockssh://pty-data:{session_id}");
        let exit_event = format!("dockssh://pty-exit:{session_id}");
        let error_event = format!("dockssh://pty-error:{session_id}");

        // 消费 channel → 双向流 → split 成读写两半（见模块顶部「已知限制」说明）
        let stream = channel.into_stream();
        let (mut reader, mut writer) = tokio::io::split(stream);

        // 前端按键写入通道（unbounded：按键是用户实时输入，背压无意义）
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let sid_for_task = session_id.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            let mut stop_rx = stop_rx;
            loop {
                tokio::select! {
                    // 远端 → 前端
                    n = reader.read(&mut buf) => {
                        match n {
                            Ok(0) => {
                                // 远端关闭（EOF）
                                let _ = app.emit(&exit_event, 0);
                                break;
                            }
                            Ok(n) => {
                                let b64 = base64::engine::general_purpose::STANDARD
                                    .encode(&buf[..n]);
                                let _ = app.emit(&data_event, b64);
                            }
                            Err(e) => {
                                let _ = app.emit(&error_event, format!("读取失败: {e}"));
                                break;
                            }
                        }
                    }
                    // 前端 → 远端
                    maybe = write_rx.recv() => {
                        match maybe {
                            Some(bytes) => {
                                if let Err(e) = writer.write_all(&bytes).await {
                                    let _ = app.emit(&error_event, format!("写入失败: {e}"));
                                    break;
                                }
                            }
                            None => break, // write_tx 被丢弃，前端不再输入
                        }
                    }
                    // 主动停止
                    _ = &mut stop_rx => {
                        let _ = writer.shutdown().await;
                        break;
                    }
                }
            }
            log::info!("[pty] session {sid_for_task} 结束");
        });

        self.0.lock().await.insert(
            session_id.clone(),
            PtyHandle {
                write_tx,
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
        h.write_tx
            .send(data)
            .map_err(|_| AppError::Other("终端会话已关闭".into()))?;
        Ok(())
    }

    /// 通知远端 PTY 尺寸变化。
    ///
    /// ⚠️ 当前架构（into_stream + split）下，channel 已被消费，
    /// 无法再调 `window_change()`，故此方法为 no-op，仅记录日志。
    /// 前端 FitAddon 仍会正确调整 xterm 自身显示。
    /// 完整支持需改用「保留 channel + Arc<Mutex>」架构（见模块顶部说明）。
    pub async fn resize(&self, session_id: &str, _cols: u32, _rows: u32) -> AppResult<()> {
        let map = self.0.lock().await;
        if !map.contains_key(session_id) {
            return Err(AppError::Other(format!("终端会话不存在: {session_id}")));
        }
        // no-op：stream 模式下无法 window_change
        log::debug!("[pty] resize {session_id} -> {_cols}x{_rows} (no-op, stream 模式)");
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
