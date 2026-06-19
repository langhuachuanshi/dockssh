//! 应用全局状态。
//!
//! 持有：
//! - SSH 会话池
//! - 主机配置列表（内存镜像 + 持久化到 app data 目录的 hosts.json）
//! - 探测缓存（host_id -> HostProbe）

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::crypto;
use crate::error::{AppError, AppResult};
use crate::models::{Host, HostProbe};
use crate::ssh::SshPool;

/// 全局状态，被 Tauri 管理，注入到每个 command。
pub struct AppState {
    pub pool: SshPool,
    pub hosts: RwLock<Vec<Host>>,
    pub probes: RwLock<std::collections::HashMap<String, HostProbe>>,
    pub config_dir: RwLock<PathBuf>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pool: SshPool::new(),
            hosts: RwLock::new(Vec::new()),
            probes: RwLock::new(Default::default()),
            config_dir: RwLock::new(default_config_dir()),
        }
    }

    /// 配置目录 + hosts.json 路径。
    pub async fn hosts_file(&self) -> PathBuf {
        self.config_dir.read().await.join("hosts.json")
    }

    /// 从磁盘加载主机列表。
    pub async fn load(&self) -> AppResult<()> {
        let path = self.hosts_file().await;
        if !path.exists() {
            return Ok(());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let hosts: Vec<Host> = serde_json::from_str(&content)?;
        *self.hosts.write().await = hosts;
        Ok(())
    }

    /// 持久化主机列表到磁盘。
    pub async fn save(&self) -> AppResult<()> {
        let path = self.hosts_file().await;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let hosts = self.hosts.read().await.clone();
        let json = serde_json::to_string_pretty(&hosts)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }

    /// 新增/更新主机（按 id 匹配）。可选写入密码/私钥口令到密钥环。
    pub async fn upsert_host(
        &self,
        host: Host,
        password: Option<String>,
        passphrase: Option<String>,
    ) -> AppResult<()> {
        {
            let mut hosts = self.hosts.write().await;
            if let Some(existing) = hosts.iter_mut().find(|h| h.id == host.id) {
                *existing = host.clone();
            } else {
                hosts.push(host.clone());
            }
        }
        if let Some(pwd) = password {
            crypto::save_password(&host.id, &pwd)?;
        }
        if let Some(pass) = passphrase {
            crypto::save_passphrase(&host.id, &pass)?;
        }
        self.save().await
    }

    /// 删除主机，同时清理密钥环与活动会话。
    pub async fn delete_host(&self, host_id: &str) -> AppResult<()> {
        {
            let mut hosts = self.hosts.write().await;
            hosts.retain(|h| h.id != host_id);
        }
        self.probes.write().await.remove(host_id);
        let _ = crypto::delete_password(host_id);
        let _ = crypto::delete_passphrase(host_id);
        let _ = self.pool.remove(host_id).await;
        self.save().await
    }

    /// 按 id 取主机定义。
    pub async fn get_host(&self, host_id: &str) -> AppResult<Host> {
        self.hosts
            .read()
            .await
            .iter()
            .find(|h| h.id == host_id)
            .cloned()
            .ok_or_else(|| AppError::HostNotFound(host_id.to_string()))
    }
}

fn default_config_dir() -> PathBuf {
    if let Some(dir) = dirs() {
        dir.join("DockSSH")
    } else {
        PathBuf::from(".").join("DockSSH")
    }
}

/// 跨平台 app data 目录。
fn dirs() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
        })
}

/// 给 Tauri 用的状态别名。
pub type SharedState = Arc<AppState>;
