//! 编排（compose）相关 commands。
//!
//! 暴露 docker/compose.rs 的能力：列出项目、读/写 compose 文件、down。
//! up 等写操作暂不开放。

use tauri::State;

use crate::docker::compose as dc;
use crate::error::AppResult;
use crate::models::ComposeProject;
use crate::state::SharedState;

#[tauri::command]
pub async fn list_compose_projects(
    state: State<'_, SharedState>,
    host_id: String,
) -> AppResult<Vec<ComposeProject>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::list_projects(&mut client).await
}

#[tauri::command]
pub async fn read_compose_file(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::read_file(&mut client, &path).await
}

#[tauri::command]
pub async fn save_compose_file(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
    content: String,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::write_file(&mut client, &path, &content).await
}

#[tauri::command]
pub async fn down_compose_project(
    state: State<'_, SharedState>,
    host_id: String,
    project_name: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::down(&mut client, &project_name).await
}

#[tauri::command]
pub async fn build_compose_project(
    state: State<'_, SharedState>,
    host_id: String,
    project_name: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::build(&mut client, &project_name).await
}

#[tauri::command]
pub async fn up_compose_project(
    state: State<'_, SharedState>,
    host_id: String,
    project_name: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::up(&mut client, &project_name).await
}

#[tauri::command]
pub async fn stop_compose_project(
    state: State<'_, SharedState>,
    host_id: String,
    project_name: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::stop(&mut client, &project_name).await
}

#[tauri::command]
pub async fn restart_compose_project(
    state: State<'_, SharedState>,
    host_id: String,
    project_name: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dc::restart(&mut client, &project_name).await
}
