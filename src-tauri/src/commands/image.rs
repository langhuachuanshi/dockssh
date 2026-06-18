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
