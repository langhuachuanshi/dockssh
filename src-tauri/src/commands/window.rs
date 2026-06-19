//! 窗口管理 commands。
//!
//! 独立终端窗口（单例，带 tabs，可承接多个终端）。
//!
//! 数据流（终端从主窗口「投递」到独立窗口）：
//! 1. 主窗口 detach 某终端（移除前端 tab，保留后端 PTY 会话）
//! 2. 主窗口调 attach_terminal(sessionId, name) → 本命令把它推入全局待投递队列，
//!    并确保独立窗口存在（不存在则创建）
//! 3. 独立窗口前端加载完成后，调 take_pending_terminals 拉取队列（取出并清空），
//!    逐个加为自己的 tab
//!
//! 用「队列 + 主动拉取」而非 emit 事件，彻底避免时序问题：
//! 窗口冷启动加载前端 bundle 可能要数秒，emit 会在 listen 注册前发出而丢失；
//! 拉取则保证无论多慢准备好，积压的终端都能被取走。

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppResult;

/// 独立终端窗口的固定 label（单例：所有投递的终端进同一个窗口）。
const TERMINAL_WINDOW_LABEL: &str = "terminal-window";

/// 通知独立窗口「有新终端可拉取」的事件名（空 payload，纯信号）。
const ATTACH_NOTIFY_EVENT: &str = "dockssh://attach-terminal";

/// 待投递到独立窗口的终端项。
#[derive(Clone, Serialize)]
pub struct PendingTerminal {
    pub session_id: String,
    pub name: String,
}

/// 全局待投递队列（由 Tauri manage 注入）。
/// attach_terminal 推入，独立窗口拉取并清空。
#[derive(Default)]
pub struct PendingTerminals(pub Mutex<Vec<PendingTerminal>>);

/// 把某终端「投递」到独立终端窗口。
///
/// 流程：
/// 1. 把 {sessionId, name} 推入待投递队列（唯一数据源）
/// 2. 确保独立窗口存在（不存在则创建）
/// 3. 向独立窗口 emit 通知信号（窗口已加载时，触发其拉取；首次创建时 emit 丢失，
///    但窗口加载后会主动拉取，队列兜底）
///
/// 主窗口应先 detach（保留后端 session）再调用本命令。
#[tauri::command]
pub async fn attach_terminal(
    app: AppHandle,
    pending: State<'_, std::sync::Arc<PendingTerminals>>,
    session_id: String,
    name: String,
) -> AppResult<()> {
    // 1. 推入待投递队列（唯一数据源）
    pending
        .0
        .lock()
        .unwrap()
        .push(PendingTerminal {
            session_id: session_id.clone(),
            name: name.clone(),
        });

    // 2. 确保独立窗口存在
    let existed = app.get_webview_window(TERMINAL_WINDOW_LABEL).is_some();
    match app.get_webview_window(TERMINAL_WINDOW_LABEL) {
        Some(win) => {
            // 已存在：聚焦并取消最小化
            win.show()?;
            win.set_focus()?;
            win.unminimize()?;
        }
        None => {
            // 不存在：创建。加载 /terminal-window 路由（独立窗口的 TerminalPanel）
            WebviewWindowBuilder::new(
                &app,
                TERMINAL_WINDOW_LABEL,
                WebviewUrl::App("/index.html#/terminal-window".into()),
            )
            .title("终端 · DockSSH")
            .inner_size(1000.0, 640.0)
            .min_inner_size(480.0, 300.0)
            .resizable(true)
            .center()
            .decorations(false)
            .build()?;

            if let Some(win) = app.get_webview_window(TERMINAL_WINDOW_LABEL) {
                let _ = win.eval(
                    "if(!location.hash||location.hash==='#')location.hash='#/terminal-window';",
                );
            }
            // 首次创建：窗口加载完成后会主动调 take_pending_terminals 拉走队列
        }
    }

    // 3. 向独立窗口发拉取通知（已存在窗口时立即触发拉取；首次创建时丢失但队列兜底）
    if existed {
        if let Some(win) = app.get_webview_window(TERMINAL_WINDOW_LABEL) {
            let _ = win.emit(ATTACH_NOTIFY_EVENT, ());
        }
    }

    Ok(())
}

/// 独立窗口拉取待投递队列：取出并清空。
///
/// 调用时机：
/// - 窗口加载完成时（onMounted）：拉走创建期间积压的终端
/// - 收到 attach-terminal 通知信号时：拉走新增的终端
///
/// 数据源唯一（队列），take 后即清空，绝不重复。
#[tauri::command]
pub fn take_pending_terminals(
    pending: State<'_, std::sync::Arc<PendingTerminals>>,
) -> AppResult<Vec<PendingTerminal>> {
    let mut guard = pending.0.lock().unwrap();
    Ok(guard.drain(..).collect())
}

