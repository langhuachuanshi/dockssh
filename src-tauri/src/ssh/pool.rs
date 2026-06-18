//! SSH 会话池：host_id -> 已认证的 SshClient。
//!
//! 管理所有已连接主机的会话，提供查找、加入、移除、断开能力。
//! 会话用 Mutex 保护，避免并发执行命令时 russh channel 状态错乱。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::ssh::client::SshClient;

/// 会话池。一个 App 共享一个实例。
#[derive(Default)]
pub struct SshPool {
    /// host_id -> 已认证会话
    sessions: Mutex<HashMap<String, Arc<Mutex<SshClient>>>>,
}

impl SshPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入/替换一个主机的会话。
    pub async fn put(&self, host_id: String, client: SshClient) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(host_id, Arc::new(Mutex::new(client)));
    }

    /// 获取某个主机会话的 Arc 句柄（用于在 command 里 lock 后使用）。
    pub async fn get(&self, host_id: &str) -> AppResult<Arc<Mutex<SshClient>>> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(host_id)
            .cloned()
            .ok_or_else(|| AppError::NotConnected(host_id.to_string()))
    }

    /// 判断某主机是否已连接。
    pub async fn has(&self, host_id: &str) -> bool {
        self.sessions.lock().await.contains_key(host_id)
    }

    /// 移除并断开某主机的会话。
    pub async fn remove(&self, host_id: &str) -> AppResult<()> {
        let client = self.sessions.lock().await.remove(host_id);
        if let Some(arc) = client {
            let client = arc.lock().await;
            let _ = client.disconnect().await;
        }
        Ok(())
    }

    /// 列出当前所有已连接的 host_id。
    #[allow(dead_code)]
    pub async fn list(&self) -> Vec<String> {
        self.sessions.lock().await.keys().cloned().collect()
    }
}
