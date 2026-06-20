//! 资源占用 stats commands：流式 docker stats，推送采样到前端。
//!
//! 事件：
//!   - `dockssh://stats:{host_id}`  -> StatsSample JSON

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::docker::stats as dstats;
use crate::error::AppResult;
use crate::state::SharedState;

pub struct StatsHandles(pub Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>);

impl Default for StatsHandles {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

#[tauri::command]
pub async fn start_stats(
    state: State<'_, SharedState>,
    handles: State<'_, Arc<StatsHandles>>,
    app: AppHandle,
    host_id: String,
    interval_secs: Option<u64>,
) -> AppResult<()> {
    let interval = interval_secs.unwrap_or(2);
    let cmd = dstats::build_stream_cmd(interval);

    let arc = state.pool.get(&host_id).await?;
    let event = format!("dockssh://stats:{host_id}");
    let raw_event = format!("dockssh://stats-raw:{host_id}");
    let mut client = arc.lock().await;

    let (_id, tx) = client
        .exec_stream(&cmd, move |_kind, chunk| {
            let app = app.clone();
            let evt = event.clone();
            let raw_evt = raw_event.clone();
            if let Ok(s) = std::str::from_utf8(&chunk) {
                // 每行一个 JSON，可能一次拿到多行
                for line in s.lines() {
                    // 诊断：把原始行（含无法解析的 stderr/表头）转发到 raw 事件
                    if !line.trim().is_empty() {
                        let _ = app.emit(&raw_evt, line.to_string());
                    }
                    if let Some(sample) = dstats::parse_line(line) {
                        let _ = app.emit(&evt, &sample);
                    }
                }
            }
        })
        .await?;

    handles.0.lock().await.insert(host_id, tx);
    Ok(())
}

#[tauri::command]
pub async fn stop_stats(
    handles: State<'_, Arc<StatsHandles>>,
    host_id: String,
) -> AppResult<()> {
    let tx = handles.0.lock().await.remove(&host_id);
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
    Ok(())
}
