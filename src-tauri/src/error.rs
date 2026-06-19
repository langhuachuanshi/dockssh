//! 统一错误类型 —— 所有模块的错误都归一到这里，方便序列化回传前端。
//!
//! 设计要点：
//! - 每个错误带一个 `ErrorKind` 分类，前端据此给针对性提示（而非靠文案猜）。
//! - 序列化成 `{ kind, message }` JSON 对象，类型信息不丢失。

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use thiserror::Error;

/// 错误分类。前端按 kind 给针对性文案 / 引导（重试 / 编辑 / 检查网络…）。
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    /// 网络/连接层：TCP 不通、SSH 握手失败、端口拒绝
    Network,
    /// 认证：密码错、密钥被拒、私钥口令错
    Auth,
    /// 超时：TCP 连接超时、SSH 握手超时
    Timeout,
    /// 资源不存在：主机找不到
    NotFound,
    /// 本地凭据读不到（keyring 缺失/损坏）—— 与 Auth 区分：
    /// Auth = 服务端拒绝（用户填错），Credential = 本地丢失（要重配）
    Credential,
    /// 其它：IO / 解析 / 加密 / Docker 探测等，无专门分类
    Other,
}

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

impl AppError {
    /// 该错误归属的分类，供前端分流提示。
    pub fn kind(&self) -> ErrorKind {
        match self {
            AppError::HostNotFound(_) => ErrorKind::NotFound,
            // Credential 变体混了两种语义：靠文案区分
            // - 「认证被拒绝」→ 服务端拒绝（用户填错），归 Auth
            // - 其余（读不到/未配置/读取失败）→ 本地凭据问题，归 Credential
            AppError::Credential(msg) => {
                if msg.contains("被拒绝") || msg.contains("认证失败") {
                    ErrorKind::Auth
                } else {
                    ErrorKind::Credential
                }
            }
            // SSH 错误文案里若带「超时」归类为 Timeout，否则 Network。
            // （TCP 超时由 client.rs 在拆分时显式构造 Crypto/Other 之外的新错误，
            //   这里做兜底：从 SSH 文本里嗅探超时关键字。）
            AppError::Ssh(msg) => {
                if msg.contains("超时") || msg.contains("timed out") || msg.contains("timeout") {
                    ErrorKind::Timeout
                } else {
                    ErrorKind::Network
                }
            }
            AppError::NotConnected(_) => ErrorKind::Network,
            // IO 错误：连接拒绝/不可达归 Network，超时归 Timeout，其余 Other
            AppError::Io(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    ErrorKind::Timeout
                } else if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::AddrNotAvailable
                        | std::io::ErrorKind::NetworkUnreachable
                        | std::io::ErrorKind::HostUnreachable
                        | std::io::ErrorKind::NetworkDown
                ) {
                    ErrorKind::Network
                } else {
                    ErrorKind::Other
                }
            }
            // Docker / Crypto / Parse / Other 归类为 Other
            AppError::Docker(_)
            | AppError::Crypto(_)
            | AppError::Parse(_)
            | AppError::Other(_) => ErrorKind::Other,
        }
    }
}

impl From<russh::Error> for AppError {
    fn from(e: russh::Error) -> Self {
        AppError::Ssh(e.to_string())
    }
}

impl From<russh_keys::Error> for AppError {
    fn from(e: russh_keys::Error) -> Self {
        // 私钥读取/解析失败本质是凭据问题，归 Credential 让前端提示「检查密钥」
        AppError::Credential(e.to_string())
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

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

/// 序列化成 `{ kind, message }` 结构，前端按 kind 分流提示。
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", &self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// 便捷 Result 别名。
pub type AppResult<T> = Result<T, AppError>;
