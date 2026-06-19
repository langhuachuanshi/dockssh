//! 主机相关 commands：CRUD、连接、探测、断开。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::crypto;
use crate::docker::detect;
use crate::error::AppResult;
use crate::models::{AuthType, Host, HostProbe};
use crate::ssh::client::SshClient;
use crate::state::SharedState;

/// 新增/更新主机时前端传的载荷（密码/口令可选，仅在用户填写时携带）。
#[derive(Debug, Deserialize)]
pub struct HostInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth_type: AuthType,
    pub key_path: Option<String>,
    pub password: Option<String>,
    /// 私钥口令（auth_type = Key 且密钥有口令时填写）
    pub passphrase: Option<String>,
    pub verify_host_key: bool,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// 列出所有主机（不含密码）。
#[tauri::command]
pub async fn list_hosts(state: State<'_, SharedState>) -> AppResult<Vec<Host>> {
    Ok(state.hosts.read().await.clone())
}

/// 新增/更新主机。
#[tauri::command]
pub async fn save_host(state: State<'_, SharedState>, input: HostInput) -> AppResult<Host> {
    let host = Host {
        id: input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: input.name,
        host: input.host,
        port: input.port,
        user: input.user,
        auth_type: input.auth_type,
        key_path: input.key_path,
        verify_host_key: input.verify_host_key,
        group: input.group,
        color: input.color,
    };
    state
        .upsert_host(host.clone(), input.password, input.passphrase)
        .await?;
    Ok(host)
}

/// 删除主机。
#[tauri::command]
pub async fn delete_host(state: State<'_, SharedState>, host_id: String) -> AppResult<()> {
    state.delete_host(&host_id).await
}

/// 测试连接 + 完成认证 + 探测环境。成功后会话进入池，可被后续命令复用。
#[derive(Debug, Serialize)]
pub struct ConnectResult {
    pub probe: HostProbe,
    pub online: bool,
}

#[tauri::command]
pub async fn connect_host(
    state: State<'_, SharedState>,
    host_id: String,
) -> AppResult<ConnectResult> {
    let host = state.get_host(&host_id).await?;
    let mut client = SshClient::connect(&host.host, host.port, host.verify_host_key).await?;

    match host.auth_type {
        AuthType::Password => {
            let pwd = crypto::load_password(&host.id)?;
            client.auth_password(&host.user, &pwd).await?;
        }
        AuthType::Key => {
            let path = host
                .key_path
                .ok_or_else(|| crate::error::AppError::Credential("未配置私钥路径".into()))?;
            // 口令可能未设置（无口令私钥），读取失败时按 None 处理
            let passphrase = crypto::load_passphrase(&host.id).ok();
            client
                .auth_publickey(&host.user, &path, passphrase.as_deref())
                .await?;
        }
        AuthType::Agent => {
            client.auth_agent(&host.user).await?;
        }
    }

    // 探测目标机
    let probe = detect::probe(&mut client).await?;
    state.probes.write().await.insert(host_id.clone(), probe.clone());
    state.pool.put(host_id, client).await;

    Ok(ConnectResult { probe, online: true })
}

/// 主动断开某主机。
#[tauri::command]
pub async fn disconnect_host(state: State<'_, SharedState>, host_id: String) -> AppResult<()> {
    state.pool.remove(&host_id).await
}

/// 某主机是否在线（有活动会话）。
#[tauri::command]
pub async fn is_host_online(state: State<'_, SharedState>, host_id: String) -> AppResult<bool> {
    Ok(state.pool.has(&host_id).await)
}
