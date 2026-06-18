//! DockSSH 后端入口。

mod commands;
mod crypto;
mod docker;
mod error;
mod models;
mod pty;
mod ssh;
mod state;

use std::sync::Arc;

use state::AppState;
use tauri::Manager;

use commands::logs::LogHandles;
use commands::stats::StatsHandles;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // 初始化全局状态并加载已存主机配置
            let state = Arc::new(AppState::new());
            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state_clone.load().await {
                    log::warn!("加载主机配置失败: {e}");
                }
            });

            let handle = app.handle();
            handle.manage(state);
            handle.manage(Arc::new(LogHandles::default()));
            handle.manage(Arc::new(StatsHandles::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 主机
            commands::host::list_hosts,
            commands::host::save_host,
            commands::host::delete_host,
            commands::host::connect_host,
            commands::host::disconnect_host,
            commands::host::is_host_online,
            // 容器
            commands::container::list_containers,
            commands::container::start_container,
            commands::container::stop_container,
            commands::container::restart_container,
            commands::container::remove_container,
            // 镜像
            commands::image::list_images,
            commands::image::remove_image,
            // 日志
            commands::logs::start_logs,
            commands::logs::stop_logs,
            // stats
            commands::stats::start_stats,
            commands::stats::stop_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
