//! 仓库（registry）相关 commands（只读）。

use tauri::State;

use crate::docker::registry as dr;
use crate::error::AppResult;
use crate::state::SharedState;

#[tauri::command]
pub async fn list_registries(
    state: State<'_, SharedState>,
    host_id: String,
) -> AppResult<Vec<String>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dr::list_logged_in(&mut client).await
}
