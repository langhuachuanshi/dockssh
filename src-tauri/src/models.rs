//! 数据模型 —— 与前端共享的数据结构。字段命名用 snake_case，前端 TS 类型对齐。

use serde::{Deserialize, Serialize};

/// 主机（远程服务器）定义。凭据（密码/密钥）不在这里，单独走加密存储。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    /// 唯一 id（uuid）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 主机地址
    pub host: String,
    /// SSH 端口
    pub port: u16,
    /// 登录用户名
    pub user: String,
    /// 认证方式
    pub auth_type: AuthType,
    /// 私钥路径（auth_type = Key 时有效）
    pub key_path: Option<String>,
    /// 是否在连接时校验 host key（生产建议 true，首次可用 false）
    pub verify_host_key: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// 密码
    Password,
    /// 私钥文件
    Key,
    /// ssh-agent
    Agent,
}

/// 目标机 Docker 运行环境探测结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostProbe {
    /// 操作系统内核类型：linux / windows
    pub os: String,
    /// 是否 WSL2 上的 Docker
    pub is_wsl2: bool,
    /// 是否 Windows 原生容器
    pub is_windows_native: bool,
    /// Docker Server 版本
    pub docker_version: String,
    /// Docker Client 版本
    pub docker_client_version: String,
    /// 是否安装了 docker compose（v2 插件）
    pub has_compose: bool,
    /// 主机名
    pub hostname: String,
}

/// 容器摘要（来自 docker ps）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub command: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<String>,
    /// compose 项目名（如有）
    pub compose_project: Option<String>,
}

/// 镜像摘要（来自 docker images）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
}

/// 单次 docker stats 采样。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsSample {
    pub container_id: String,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_usage: String,
    pub mem_percent: f64,
    pub net_io: String,
    pub block_io: String,
    pub pids: u64,
}

/// 终端尺寸（用于 exec 终端 resize）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}
