//! 镜像相关 commands。

use tauri::State;

use crate::docker::image as di;
use crate::error::AppResult;
use crate::models::Image;
use crate::state::SharedState;

#[tauri::command]
pub async fn list_images(state: State<'_, SharedState>, host_id: String) -> AppResult<Vec<Image>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    di::list(&mut client).await
}

#[tauri::command]
pub async fn remove_image(
    state: State<'_, SharedState>,
    host_id: String,
    id: String,
    force: Option<bool>,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    di::remove(&mut client, &id, force.unwrap_or(false)).await
}

/// 批量删除镜像。一次 SSH 执行多个 id，比前端循环调用 remove_image 快得多。
#[tauri::command]
pub async fn remove_images(
    state: State<'_, SharedState>,
    host_id: String,
    ids: Vec<String>,
    force: Option<bool>,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    di::remove_many(&mut client, &ids, force.unwrap_or(false)).await
}
