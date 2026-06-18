//! 容器相关 commands。

use tauri::State;

use crate::docker::container as dc;
use crate::error::AppResult;
use crate::models::Container;
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
