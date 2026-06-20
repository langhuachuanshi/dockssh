//! 容器相关 commands。

use tauri::State;

use crate::docker::container as dc;
use crate::error::AppResult;
use crate::models::{Container, ContainerInspect, CreateContainerOpts};
use crate::state::SharedState;

#[tauri::command]
pub async fn list_containers(state: State<'_, SharedState>, host_id: String) -> AppResult<Vec<Container>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::list(&mut client).await
}

#[tauri::command]
pub async fn start_container(state: State<'_, SharedState>, host_id: String, id: String) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::start(&mut client, &id).await
}

#[tauri::command]
pub async fn stop_container(state: State<'_, SharedState>, host_id: String, id: String) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::stop(&mut client, &id).await
}

#[tauri::command]
pub async fn restart_container(state: State<'_, SharedState>, host_id: String, id: String) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::restart(&mut client, &id).await
}

#[tauri::command]
pub async fn pause_container(state: State<'_, SharedState>, host_id: String, id: String) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::pause(&mut client, &id).await
}

#[tauri::command]
pub async fn unpause_container(state: State<'_, SharedState>, host_id: String, id: String) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::unpause(&mut client, &id).await
}

/// 重命名容器。new_name 做校验：仅允许字母数字、`-`、`_`、`.`，避免 shell 注入。
#[tauri::command]
pub async fn rename_container(
    state: State<'_, SharedState>,
    host_id: String,
    id: String,
    new_name: String,
) -> AppResult<()> {
    // 容器名规则：[a-zA-Z0-9][a-zA-Z0-9_.-]*，这里用宽松校验防注入
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(crate::error::AppError::Other("容器名不能为空".into()));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(crate::error::AppError::Other(
            "容器名只能包含字母、数字、下划线、连字符和点".into(),
        ));
    }
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::rename(&mut client, &id, trimmed).await
}

#[tauri::command]
pub async fn remove_container(
    state: State<'_, SharedState>,
    host_id: String,
    id: String,
    force: Option<bool>,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::remove(&mut client, &id, force.unwrap_or(false)).await
}

/// 容器详情（用于「打开目录」：取 bind 挂载源路径或工作目录）。
#[tauri::command]
pub async fn inspect_container(
    state: State<'_, SharedState>,
    host_id: String,
    id: String,
) -> AppResult<ContainerInspect> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::inspect(&mut client, &id).await
}

/// 创建并启动容器（docker run -d）。返回新容器 ID。
#[tauri::command]
pub async fn create_and_run_container(
    state: State<'_, SharedState>,
    host_id: String,
    opts: CreateContainerOpts,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::create_and_run(&mut client, &opts).await
}
