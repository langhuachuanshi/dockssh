//! 存储卷相关 commands：列表 / 详情 / 删除。

use tauri::State;

use crate::docker::volume as dv;
use crate::error::AppResult;
use crate::models::{Volume, VolumeInspect};
use crate::state::SharedState;

#[tauri::command]
pub async fn list_volumes(state: State<'_, SharedState>, host_id: String) -> AppResult<Vec<Volume>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dv::list(&mut client).await
}

/// 卷详情（用于「打开目录」：取 Mountpoint 宿主机路径）。
#[tauri::command]
pub async fn inspect_volume(
    state: State<'_, SharedState>,
    host_id: String,
    name: String,
) -> AppResult<VolumeInspect> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dv::inspect(&mut client, &name).await
}

/// 删除卷。
#[tauri::command]
pub async fn remove_volume(
    state: State<'_, SharedState>,
    host_id: String,
    name: String,
    force: Option<bool>,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dv::remove(&mut client, &name, force.unwrap_or(false)).await
}
