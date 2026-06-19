//! 所有 Tauri commands 的聚合模块。
//!
//! 注册入口在 lib.rs，用 `tauri::generate_handler![...]` 宏完成，
//! 各子模块只暴露带 `#[tauri::command]` 的函数。

pub mod compose;
pub mod container;
pub mod files;
pub mod host;
pub mod image;
pub mod logo;
pub mod logs;
pub mod network;
pub mod pty;
pub mod registry;
pub mod stats;
pub mod volume;
