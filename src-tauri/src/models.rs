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
    /// 分组名（用于在主机列表里归类，可为空 = 未分组）
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// 密码
    Password,
    /// 私钥文件
    Key,
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
    /// 创建时间（docker 的 CreatedAt 原样，如 "2024-01-01 12:00:00 +0800 CST"）
    #[serde(default)]
    pub created: String,
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

/// 网络摘要（来自 docker network ls）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

/// 存储卷摘要（来自 docker volume ls + volume inspect）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub driver: String,
    pub name: String,
    /// 卷在宿主机上的挂载点绝对路径（volume inspect 的 Mountpoint），无则空串
    #[serde(default)]
    pub mountpoint: String,
    /// 创建时间（docker 输出的 CreatedAt 原样回传，如 "2024-01-01 12:00:00 +0800 CST"），无则空串
    #[serde(default)]
    pub created: String,
}

/// compose 项目摘要（来自扫描容器 labels）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeProject {
    pub name: String,
    /// 容器总数
    pub containers: usize,
    /// 运行中容器数量
    pub running: usize,
    /// 最早容器的创建时间（docker 输出的 CreatedAt 原样回传）
    pub created: String,
    /// compose 文件绝对路径（来自 label com.docker.compose.project.config_files）。
    /// 组内各容器该值通常一致；取第一条；缺失时为 None。
    pub config_files: Option<String>,
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

/// 单个文件/目录条目（来自 SFTP read_dir）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// 名称
    pub name: String,
    /// 是否目录
    pub is_dir: bool,
    /// 是否符号链接
    pub is_symlink: bool,
    /// 字节数（目录为 0）
    pub size: u64,
    /// 最后修改时间（unix 秒），无则 None
    pub modified: Option<i64>,
    /// 权限串，如 "rwxr-xr-x"，无则 None
    pub permissions: Option<String>,
}

/// 目录列表结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirListing {
    /// 规范化后的绝对路径
    pub path: String,
    /// 条目列表（已排序：目录在前，同类按名称）
    pub entries: Vec<FileEntry>,
}

/// 容器单个挂载（来自 docker inspect Mounts）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMount {
    /// 宿主机路径（bind 类型才有意义）
    pub source: String,
    /// 容器内路径
    pub destination: String,
    /// 挂载类型：bind / volume / tmpfs ...
    pub typ: String,
}

/// 容器端口映射（来自 docker inspect NetworkSettings.Ports 或 HostConfig.PortBindings）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPortBinding {
    /// 容器端口（如 "80/tcp"）
    pub container_port: String,
    /// 主机 IP（如 "0.0.0.0"）
    pub host_ip: String,
    /// 主机端口（如 "8080"）
    pub host_port: String,
}

/// 容器 inspect 结果。
///
/// 保留 working_dir / mounts（用于「打开目录」），新增 Config / State / 网络 / 资源限制
/// 等字段，供详情抽屉展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInspect {
    // ===== 基础信息 =====
    /// 容器 ID（完整）
    pub id: String,
    /// 容器名（去掉前导 /）
    pub name: String,
    /// 镜像
    pub image: String,
    /// 创建时间（RFC3339）
    pub created: String,

    // ===== Config =====
    /// 工作目录
    pub working_dir: String,
    /// 入口点（Entrypoint，数组）
    pub entrypoint: Vec<String>,
    /// 启动命令（Cmd，数组）
    pub cmd: Vec<String>,
    /// 环境变量（KEY=VALUE 数组）
    pub env: Vec<String>,
    /// 暴露端口（Config.ExposedPorts 的 key，如 "80/tcp"）
    pub exposed_ports: Vec<String>,

    // ===== State =====
    /// 状态（running / exited / paused ...）
    pub state: String,
    /// 状态详情（如 "exited" 时的 ExitCode）
    pub status: String,
    /// 退出码（非 running 时有意义）
    pub exit_code: i64,
    /// 启动时间（RFC3339）
    pub started_at: String,
    /// 结束时间（RFC3339）
    pub finished_at: String,
    /// PID（宿主机进程号，running 时有意义；无法获取为 0）
    pub pid: i64,

    // ===== 网络 =====
    /// 所属网络名（NetworkSettings.Networks 的 key 列表）
    pub networks: Vec<String>,
    /// IP 地址（取首个网络的 IPAddress）
    pub ip_address: String,
    /// 网关
    pub gateway: String,
    /// MAC 地址
    pub mac_address: String,

    // ===== 主机配置 =====
    /// 重启策略（no / always / unless-stopped / on-failure）
    pub restart_policy: String,
    /// 重启策略为 on-failure 时的最大重试次数
    pub restart_retries: i64,
    /// 端口绑定列表
    pub port_bindings: Vec<ContainerPortBinding>,

    // ===== 挂载（兼容旧字段：打开目录用） =====
    /// 所有挂载
    pub mounts: Vec<ContainerMount>,
}

/// 存储卷 inspect 结果（仅取文件跳转需要的字段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInspect {
    /// 卷在宿主机上的挂载点绝对路径
    pub mountpoint: String,
}
