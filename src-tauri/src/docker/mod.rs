//! Docker 命令封装与输出解析模块。
//!
//! 所有 docker 子命令都在 SSH 会话上执行原始 `docker ...` 命令，
//! 再把文本输出解析成结构化数据返回前端。

pub mod compose;
pub mod container;
pub mod detect;
pub mod image;
pub mod logs;
pub mod network;
pub mod parse;
pub mod registry;
pub mod stats;
pub mod volume;
