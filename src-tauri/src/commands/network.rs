//! 网络相关 commands（只读列表）。

use tauri::State;

use crate::docker::network as dn;
use crate::error::AppResult;
use crate::models::Network;
use crate::state::SharedState;

#[tauri::command]
pub async fn list_networks(state: State<'_, SharedState>, host_id: String) -> AppResult<Vec<Network>> {
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    dn::list(&mut client).await
}
