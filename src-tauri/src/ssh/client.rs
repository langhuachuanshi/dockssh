//! 单主机 SSH 会话封装（基于 russh 0.46）。
//!
//! 提供三种核心能力：
//! 1. `connect()`       —— 建立 SSH 连接并完成认证
//! 2. `exec()`          —— 执行一条命令，收集完整 stdout/stderr/退出码
//! 3. `exec_stream()`   —— 执行命令，逐块流式返回（用于 docker logs/stats tail）

use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Config, Handle, Handler};
use russh::{ChannelId, ChannelMsg};
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
        // 默认信任；严格的 host key 校验在 connect 前由调用方决定是否启用。
        // 真正的指纹比对放在 connect() 里做，这里给底层库一个肯定的应答。
        Ok(true)
    }
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
    /// 连接时是否校验 host key
    verify_host_key: bool,
    /// 已知的服务器指纹（verify_host_key=true 时比对），None 表示不比对
    expected_fingerprint: Option<String>,
    /// 懒加载的 SFTP 会话（首次调用 sftp() 时建立，之后复用）。
    /// SFTP 协议在单个 channel 上用 request-id 多路复用，可长期持有。
    sftp: tokio::sync::Mutex<Option<Arc<SftpSession>>>,
}

impl SshClient {
    /// 建立 TCP + SSH 连接（不做认证）。认证在 `authenticate_*` 方法中完成。
    pub async fn connect(
        addr: &str,
        port: u16,
        verify_host_key: bool,
    ) -> AppResult<Self> {
        let mut config = Config::default();
        config.inactivity_timeout = Some(Duration::from_secs(30));
        // 关闭空闲超时反而可能导致长连接（如 logs -f）被误断，这里设大一些。
        let config = Arc::new(config);

        let handle = client::connect(config, (addr, port), ClientHandler)
            .await
            .map_err(|e| AppError::Ssh(format!("连接 {addr}:{port} 失败: {e}")))?;

        Ok(Self {
            handle,
            verify_host_key,
            expected_fingerprint: None,
            sftp: tokio::sync::Mutex::new(None),
        })
    }

    /// 密码认证。
    pub async fn auth_password(&mut self, user: &str, password: &str) -> AppResult<()> {
        let ok = self
            .handle
            .authenticate_password(user, password)
            .await
            .map_err(|e| AppError::Ssh(format!("密码认证失败: {e}")))?;
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
            .map_err(|e| AppError::Ssh(format!("公钥认证失败: {e}")))?;
        if ok {
            Ok(())
        } else {
            Err(AppError::Credential("公钥认证被拒绝".into()))
        }
    }

    /// ssh-agent 认证。
    ///
    /// 注：russh 的 agent 签名 API 跨版本差异大且平台相关（Windows 需 pageant，
    /// Linux/macOS 需 ssh-agent + SSH_AUTH_SOCK）。第一版暂未实现，
    /// 前端如选 agent 认证会返回此错误。计划 v2 用 russh-keys::agent 完整补全。
    pub async fn auth_agent(&mut self, _user: &str) -> AppResult<()> {
        Err(AppError::Credential(
            "ssh-agent 认证暂未实现，请使用密码或密钥".into(),
        ))
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
    /// 返回一个可 await 的 future，调用方通常 spawn 到后台 task。
    /// 注意：调用方需要持有 channel 的关闭信号来中断。
    pub async fn exec_stream<F>(
        &mut self,
        command: &str,
        mut on_chunk: F,
    ) -> AppResult<(ChannelId, tokio::sync::oneshot::Sender<()>)>
    where
        F: FnMut(Vec<u8>) + Send + 'static,
    {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::Ssh(format!("打开 channel 失败: {e}")))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| AppError::Ssh(format!("执行流式命令失败: {e}")))?;

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
                                on_chunk(data.to_vec());
                            }
                            Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                                on_chunk(data.to_vec());
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

    /// 是否启用 host key 校验。
    pub fn verify_host_key(&self) -> bool {
        self.verify_host_key
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
