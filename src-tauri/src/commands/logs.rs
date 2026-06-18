//! 日志 commands：实时 tail 通过 Tauri event 推送到前端。
//!
//! 事件命名约定：
//!   - 数据块：  `dockssh://log-chunk:{host_id}:{container}`
//!
//! 前端调用 start_logs 后监听对应事件，调用 stop_logs 中断。

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::docker::logs as dlog;
use crate::error::AppResult;
use crate::state::SharedState;

/// 在线日志流的停止信号集合：key = "{host_id}:{container}"
pub struct LogHandles(pub Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>);

impl Default for LogHandles {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

fn key(host_id: &str, container: &str) -> String {
    format!("{host_id}:{container}")
}

/// 开始实时日志 tail。返回后流在后台持续推送，直到 stop_logs 或连接断开。
#[tauri::command]
pub async fn start_logs(
    state: State<'_, SharedState>,
    handles: State<'_, Arc<LogHandles>>,
    app: AppHandle,
    host_id: String,
    container: String,
    tail: Option<String>,
) -> AppResult<()> {
    let tail = tail.unwrap_or_else(|| "200".to_string());
    let cmd = dlog::build_logs_cmd(&container, true, &tail, None, false);

    let arc = state.pool.get(&host_id).await?;
    let event_data = format!("dockssh://log-chunk:{host_id}:{container}");
    let k = key(&host_id, &container);

    let mut client = arc.lock().await;
    // 先把同 key 旧流停掉，避免重复
    {
        let mut map = handles.0.lock().await;
        if let Some(old) = map.remove(&k) {
            let _ = old.send(());
        }
    }

    let (_chan_id, tx) = client
        .exec_stream(&cmd, move |chunk| {
            if let Ok(s) = std::str::from_utf8(&chunk) {
                let _ = app.emit(&event_data, s.to_string());
            }
        })
        .await?;

    handles.0.lock().await.insert(k, tx);
    Ok(())
}

#[tauri::command]
pub async fn stop_logs(
    handles: State<'_, Arc<LogHandles>>,
    host_id: String,
    container: String,
) -> AppResult<()> {
    let k = key(&host_id, &container);
    let tx = handles.0.lock().await.remove(&k);
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
    Ok(())
}
