//! 单主机 SSH 会话封装（基于 russh 0.46）。
//!
//! 提供三种核心能力：
//! 1. `connect()`       —— 建立 SSH 连接并完成认证
//! 2. `exec()`          —— 执行一条命令，收集完整 stdout/stderr/退出码
//! 3. `exec_stream()`   —— 执行命令，逐块流式返回（用于 docker logs/stats tail）

use std::sync::Arc;
use std::time::Duration;

use russh::client::{Config, Handle, Handler};
use russh::{ChannelId, ChannelMsg, Pty};
use russh_keys::key;
use russh_sftp::client::SftpSession;

use crate::error::{AppError, AppResult};

/// 简单的 client::Handler 实现：我们不需要处理服务端主动推送的消息，
/// 所有数据都通过 channel 显式读取。
struct ClientHandler;

#[async_trait::async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 无条件信任服务端 host key。DockSSH 连接的是用户自己配置的主机，
        // 不做 known_hosts 指纹比对。
        Ok(true)
    }
}

/// 流式数据的来源：标准输出或标准错误。
/// SSH channel 把 stdout 作为 Data、stderr 作为 ExtendedData 推送。
/// docker logs 会把容器的 stderr 输出到 SSH 的 ExtendedData，借此可做分流。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

/// 一条命令的执行结果。
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// 单台主机的 SSH 会话。
pub struct SshClient {
    handle: Handle<ClientHandler>,
    /// 懒加载的 SFTP 会话（首次调用 sftp() 时建立，之后复用）。
    /// SFTP 协议在单个 channel 上用 request-id 多路复用，可长期持有。
    sftp: tokio::sync::Mutex<Option<Arc<SftpSession>>>,
}

impl SshClient {
    /// 建立 TCP + SSH 连接（不做认证）。认证在 `authenticate_*` 方法中完成。
    ///
    /// 拆分 TCP 与 SSH 握手两阶段，便于给前端精准的错误分类：
    /// - TCP 阶段失败 → Network（拒绝/不可达）或 Timeout（超时）
    /// - SSH 握手失败 → Network（对端不是 SSH 服务 / 握手异常）
    ///
    /// TCP 连接带 10s 超时，避免目标不可达时长时间卡住。
    ///
    /// 注：当前无条件信任服务端 host key（见 ClientHandler::check_server_key）。
    /// DockSSH 的使用场景是连接用户自己配置的主机，不做 known_hosts 校验。
    pub async fn connect(addr: &str, port: u16) -> AppResult<Self> {
        let mut config = Config::default();
        config.inactivity_timeout = Some(Duration::from_secs(30));
        // 关闭空闲超时反而可能导致长连接（如 logs -f）被误断，这里设大一些。
        let config = Arc::new(config);

        // 阶段 1：TCP 连接（带 10s 超时）
        let target = (addr, port);
        let socket = match tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::TcpStream::connect(target),
        )
        .await
        {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(AppError::Ssh(format!("连接 {addr}:{port} 超时")))
            }
            // tokio TcpStream::connect 对端口未开返回 ConnectionRefused
            Ok(Err(e)) => {
                return Err(AppError::Ssh(format!(
                    "无法连接到 {addr}:{port}：{e}（请检查地址、端口、防火墙）"
                )))
            }
            // 整个 connect future 超时（网络不可达/对端无响应）
            Err(_) => {
                return Err(AppError::Ssh(format!(
                    "连接 {addr}:{port} 超时（请检查网络或主机是否在线）"
                )))
            }
        };

        // 阶段 2：SSH 握手
        let handle = russh::client::connect_stream(config, socket, ClientHandler)
            .await
            .map_err(|e| {
                AppError::Ssh(format!(
                    "SSH 握手失败：{e}（端口可能不是 SSH 服务）"
                ))
            })?;

        Ok(Self {
            handle,
            sftp: tokio::sync::Mutex::new(None),
        })
    }

    /// 密码认证。
    pub async fn auth_password(&mut self, user: &str, password: &str) -> AppResult<()> {
        let ok = self
            .handle
            .authenticate_password(user, password)
            .await
            .map_err(|e| AppError::Credential(format!("密码认证失败: {e}")))?;
        if ok {
            Ok(())
        } else {
            Err(AppError::Credential("密码认证被拒绝".into()))
        }
    }

    /// 公钥认证。可选传入私钥口令(passphrase)。
    pub async fn auth_publickey(
        &mut self,
        user: &str,
        key_path: &str,
        passphrase: Option<&str>,
    ) -> AppResult<()> {
        let key_pair = russh_keys::load_secret_key(key_path, passphrase)
            .map_err(|e| AppError::Credential(format!("读取私钥失败: {e}")))?;
        let ok = self
            .handle
            .authenticate_publickey(user, Arc::new(key_pair))
            .await
            .map_err(|e| AppError::Credential(format!("公钥认证失败: {e}")))?;
        if ok {
            Ok(())
        } else {
            Err(AppError::Credential("公钥认证被拒绝".into()))
        }
    }

    /// 执行一条命令，等待结束并收集完整输出。
    pub async fn exec(&mut self, command: &str) -> AppResult<ExecResult> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::Ssh(format!("打开 channel 失败: {e}")))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| AppError::Ssh(format!("执行命令失败: {e}")))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { ref data } => {
                    stdout.extend_from_slice(data);
                }
                ChannelMsg::ExtendedData { ref data, .. } => {
                    stderr.extend_from_slice(data);
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = exit_status as i32;
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code,
        })
    }

    /// 执行一条流式命令，对每个输出块调用回调（用于 logs -f / stats）。
    ///
    /// 回调携带 `StreamKind` 标签：stdout 块为 `Stdout`，stderr 块为 `Stderr`，
    /// 调用方可据此分流（如 docker logs 区分容器的 stdout/stderr）。
    ///
    /// 返回一个 oneshot Sender，调用方发送 () 即可中断后台读取。
    /// 注意：spawn 的任务只捕获 channel + 关闭信号，不持有 SshClient 的锁，
    /// 因此长流（如 logs -f）不会阻塞其他命令。
    pub async fn exec_stream<F>(
        &mut self,
        command: &str,
        mut on_chunk: F,
    ) -> AppResult<(ChannelId, tokio::sync::oneshot::Sender<()>)>
    where
        F: FnMut(StreamKind, Vec<u8>) + Send + 'static,
    {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::Ssh(format!("打开 channel 失败：{e}")))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| AppError::Ssh(format!("执行流式命令失败：{e}")))?;

        let id = channel.id();

        // 关闭信号：调用方发送 () 即可结束流。
        let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => {
                        // 收到停止信号，主动关闭 channel
                        let _ = channel.close().await;
                        break;
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { ref data }) => {
                                on_chunk(StreamKind::Stdout, data.to_vec());
                            }
                            Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                                on_chunk(StreamKind::Stderr, data.to_vec());
                            }
                            Some(ChannelMsg::ExitStatus { .. })
                            | Some(ChannelMsg::Eof)
                            | Some(ChannelMsg::Close)
                            | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok((id, tx))
    }

    /// 打开一条交互式 PTY channel（用于 `docker exec -it`）。
    ///
    /// 流程：开 session channel → request_pty → exec(shell)。
    /// 返回的 Channel 由调用方持有并 spawn 双向 task：
    /// - 读取：`channel.wait()` 拿 `ChannelMsg::Data` 转 emit
    /// - 写入：`channel.data(&buf[..])` 把前端按键送过去
    /// - resize：`channel.window_change(cols, rows, 0, 0)`
    ///
    /// 注意：channel 被 move 走后，本方法立即返回，**不持有 SshClient 的任何锁**。
    /// 这样 pty_write 拿 channel 句柄即可，无需再经 pool 的 Mutex。
    pub async fn open_pty_channel(
        &mut self,
        command: &str,
        cols: u32,
        rows: u32,
    ) -> AppResult<russh::Channel<russh::client::Msg>> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::Ssh(format!("打开 PTY channel 失败: {e}")))?;

        // 请求伪终端。TERM 固定 xterm-256color（与前端 xterm 配置一致）。
        // terminal_modes 用空切片（禁用所有终端模式）。
        let modes: &[(Pty, u32)] = &[];
        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, modes)
            .await
            .map_err(|e| AppError::Ssh(format!("请求 PTY 失败: {e}")))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| AppError::Ssh(format!("执行交互命令失败: {e}")))?;

        Ok(channel)
    }

    /// 获取（懒加载建立）SFTP 会话。首次调用时打开一个 channel、请求 sftp 子系统、
    /// 建立 SftpSession 并缓存；后续直接返回克隆的 Arc。
    ///
    /// SftpSession 的所有方法都是 `&self`，内部在单个 channel 上多路复用，
    /// 因此可以安全地用 Arc 共享给多个 command 并发使用。
    pub async fn sftp(&self) -> AppResult<Arc<SftpSession>> {
        let mut guard = self.sftp.lock().await;
        if let Some(s) = guard.as_ref() {
            return Ok(s.clone());
        }
        // 建立 SFTP channel：session channel → 请求 sftp subsystem → 转 stream → 建会话
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::Ssh(format!("打开 SFTP channel 失败: {e}")))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| AppError::Ssh(format!("请求 sftp 子系统失败: {e}")))?;
        let session = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| AppError::Ssh(format!("建立 SFTP 会话失败: {e}")))?;
        let arc = Arc::new(session);
        *guard = Some(arc.clone());
        Ok(arc)
    }

    /// 关闭会话。
    pub async fn disconnect(&self) -> AppResult<()> {
        let _ = self
            .handle
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
        Ok(())
    }
}
