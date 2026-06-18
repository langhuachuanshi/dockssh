//! 统一错误类型 —— 所有模块的错误都归一到这里，方便序列化回传前端。

use serde::{Serialize, Serializer};
use thiserror::Error;

/// 应用全局错误类型。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("SSH 错误: {0}")]
    Ssh(String),

    #[error("Docker 命令错误: {0}")]
    Docker(String),

    #[error("主机未连接或会话已失效: {0}")]
    NotConnected(String),

    #[error("主机不存在: {0}")]
    HostNotFound(String),

    #[error("凭据错误: {0}")]
    Credential(String),

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("解析错误: {0}")]
    Parse(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<russh::Error> for AppError {
    fn from(e: russh::Error) -> Self {
        AppError::Ssh(e.to_string())
    }
}

impl From<russh_keys::Error> for AppError {
    fn from(e: russh_keys::Error) -> Self {
        AppError::Ssh(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

// Tauri command 要求返回值实现 Serialize，这里把错误序列化成字符串。
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 便捷 Result 别名。
pub type AppResult<T> = Result<T, AppError>;
