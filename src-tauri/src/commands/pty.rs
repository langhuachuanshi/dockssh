//! 终端 commands：docker exec -it 的双向透传入口。
//!
//! 前端调用 pty_start 后监听对应事件，通过 pty_write 投递按键，
//! pty_resize 调整尺寸（当前 no-op，见 pty 模块说明），pty_kill 关闭。
//!
//! 事件（payload 见 pty 模块）：
//!   - dockssh://pty-data:{session_id}
//!   - dockssh://pty-exit:{session_id}
//!   - dockssh://pty-error:{session_id}

use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::pty::{build_exec_cmd, PtySessions};
use crate::state::SharedState;

/// 启动一个容器的交互式终端，返回 session_id。
///
/// 前端拿到 session_id 后订阅对应事件，并开始 pty_write。
#[tauri::command]
pub async fn pty_start(
    state: State<'_, SharedState>,
    sessions: State<'_, std::sync::Arc<PtySessions>>,
    app: AppHandle,
    host_id: String,
    container: String,
    cols: Option<u32>,
    rows: Option<u32>,
    user: Option<String>,
    cwd: Option<String>,
) -> AppResult<String> {
    let cols = cols.unwrap_or(80).max(1);
    let rows = rows.unwrap_or(24).max(1);

    let cmd = build_exec_cmd(&container, user.as_deref(), cwd.as_deref());

    // 短暂持 SshClient 锁开 PTY channel，拿完即放
    let arc = state.pool.get(&host_id).await?;
    let mut client = arc.lock().await;
    sessions
        .start(&mut client, app, host_id, cmd, cols, rows)
        .await
}

/// 前端按键写入（payload 为 base64 编码的 bytes，保证任意字节流）。
#[tauri::command]
pub async fn pty_write(
    sessions: State<'_, std::sync::Arc<PtySessions>>,
    session_id: String,
    data: String, // base64
) -> AppResult<()> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| crate::error::AppError::Other(format!("base64 解码失败: {e}")))?;
    sessions.write(&session_id, bytes).await
}

/// 通知远端 PTY 尺寸变化（当前 no-op，见 pty 模块说明）。
#[tauri::command]
pub async fn pty_resize(
    sessions: State<'_, std::sync::Arc<PtySessions>>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<()> {
    sessions.resize(&session_id, cols, rows).await
}

/// 关闭并移除一个终端会话。
#[tauri::command]
pub async fn pty_kill(
    sessions: State<'_, std::sync::Arc<PtySessions>>,
    session_id: String,
) -> AppResult<()> {
    sessions.kill(&session_id).await
}
