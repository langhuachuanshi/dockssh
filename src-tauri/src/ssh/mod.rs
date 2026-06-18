//! SSH 客户端模块。
//!
//! 基于 russh 封装，提供：
//! - `SshClient`: 单台主机的 SSH 会话
//! - `SshPool`:   多主机会话池（host_id -> SshClient）

pub mod client;
pub mod pool;

pub use client::SshClient;
pub use pool::SshPool;
