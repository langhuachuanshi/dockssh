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
use commands::window::PendingTerminals;
use pty::PtySessions;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            handle.manage(Arc::new(PtySessions::default()));
            handle.manage(Arc::new(PendingTerminals::default()));
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
            commands::container::inspect_container,
            // 镜像
            commands::image::list_images,
            commands::image::remove_image,
            // 网络
            commands::network::list_networks,
            // 存储卷
            commands::volume::list_volumes,
            commands::volume::inspect_volume,
            commands::volume::remove_volume,
            // 编排（compose）
            commands::compose::list_compose_projects,
            commands::compose::read_compose_file,
            commands::compose::save_compose_file,
            commands::compose::down_compose_project,
            commands::compose::build_compose_project,
            commands::compose::up_compose_project,
            commands::compose::stop_compose_project,
            commands::compose::restart_compose_project,
            // 仓库（registry）
            commands::registry::list_registries,
            // 镜像 logo 缓存
            commands::logo::get_cached_logo,
            commands::logo::fetch_logo,
            commands::logo::delete_cached_logo,
            commands::logo::clear_logo_cache,
            // 文件管理（SFTP）
            commands::files::file_home,
            commands::files::list_dir,
            commands::files::file_read_text,
            commands::files::file_mkdir,
            commands::files::file_remove,
            commands::files::file_rename,
            commands::files::file_download,
            commands::files::file_upload,
            // 日志
            commands::logs::start_logs,
            commands::logs::stop_logs,
            commands::logs::save_text_local,
            // stats
            commands::stats::start_stats,
            commands::stats::stop_stats,
            // 终端（docker exec -it 透传）
            commands::pty::pty_start,
            commands::pty::pty_write,
            commands::pty::pty_resize,
            commands::pty::pty_kill,
            // 窗口
            commands::window::attach_terminal,
            commands::window::take_pending_terminals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
