/**
 * 与 Rust 后端共享的类型定义。
 * 命名与 src-tauri/src/models.rs 对齐。
 */

export type AuthType = 'password' | 'key'

export interface Host {
  id: string
  name: string
  host: string
  port: number
  user: string
  auth_type: AuthType
  key_path: string | null
  /** 分组名（用于列表归类，可为 null = 未分组） */
  group: string | null
}

export interface HostProbe {
  os: string
  is_wsl2: boolean
  is_windows_native: boolean
  docker_version: string
  docker_client_version: string
  has_compose: boolean
  hostname: string
}

export interface Container {
  id: string
  name: string
  image: string
  command: string
  state: string
  status: string
  ports: string[]
  compose_project: string | null
  /** 创建时间（docker 的 CreatedAt 原样，如 "2024-01-01 12:00:00 +0800 CST"） */
  created: string
}

export interface Image {
  id: string
  repository: string
  tag: string
  size: string
  created: string
}

export interface Network {
  id: string
  name: string
  driver: string
  scope: string
}

export interface Volume {
  driver: string
  name: string
  /** 卷在宿主机上的挂载点绝对路径，无则空串 */
  mountpoint: string
  /** 创建时间（docker 输出的 CreatedAt 原样），无则空串 */
  created: string
}

export interface ComposeProject {
  name: string
  /** 容器总数 */
  containers: number
  /** 运行中容器数量 */
  running: number
  /** 最早容器的创建时间（docker 输出的 CreatedAt 原样回传） */
  created: string
  /** compose 文件绝对路径；缺失时为 null */
  config_files: string | null
}

export interface StatsSample {
  container_id: string
  name: string
  cpu_percent: number
  mem_usage: string
  mem_percent: number
  net_io: string
  block_io: string
  pids: number
}

export interface ConnectResult {
  probe: HostProbe
  online: boolean
}

// ===== 文件管理（SFTP） =====

export interface FileEntry {
  name: string
  is_dir: boolean
  is_symlink: boolean
  size: number
  modified: number | null
  permissions: string | null
}

export interface DirListing {
  path: string
  entries: FileEntry[]
}

// ===== 容器/卷 inspect（用于「打开目录」跳转） =====

export interface ContainerMount {
  source: string
  destination: string
  typ: string
}

export interface ContainerPortBinding {
  container_port: string
  host_ip: string
  host_port: string
}

export interface ContainerInspect {
  // 基础
  id: string
  name: string
  image: string
  created: string
  // Config
  working_dir: string
  entrypoint: string[]
  cmd: string[]
  env: string[]
  exposed_ports: string[]
  // State
  state: string
  status: string
  exit_code: number
  started_at: string
  finished_at: string
  pid: number
  // 网络
  networks: string[]
  ip_address: string
  gateway: string
  mac_address: string
  // 主机配置
  restart_policy: string
  restart_retries: number
  port_bindings: ContainerPortBinding[]
  // 挂载
  mounts: ContainerMount[]
}

export interface VolumeInspect {
  mountpoint: string
}

// ===== 创建容器 =====

export interface PortMapping {
  host: string
  container: string
  protocol: string
}

export interface VolumeMount {
  host: string
  container: string
  readOnly: boolean
}

export interface CreateContainerOpts {
  image: string
  name: string
  command: string
  ports: PortMapping[]
  envs: string[]
  volumes: VolumeMount[]
  restartPolicy: string
  network: string
  cpuLimit: string
  memLimit: string
}

// ===== 日志 =====

/** 日志数据块，携带来源流标签（stdout/stderr）。 */
export interface LogChunk {
  /** 来源流：stdout 或 stderr */
  stream: 'stdout' | 'stderr'
  /** 本块文本（可能跨多行） */
  text: string
}

// ===== 错误（与 src-tauri/src/error.rs 的 ErrorKind 对齐） =====

/**
 * 错误分类。前端据此给针对性提示文案 / 引导（重试 / 编辑 / 检查网络…），
 * 而非靠错误字符串猜。与后端 ErrorKind 的 #[serde(rename_all="lowercase")] 对齐。
 */
export type ErrorKind =
  | 'network' // 网络/连接层：TCP 不通、SSH 握手失败、端口拒绝
  | 'auth' // 认证：密码错、密钥被拒、私钥口令错
  | 'timeout' // 超时：TCP 连接超时、SSH 握手超时
  | 'notfound' // 资源不存在：主机找不到
  | 'credential' // 本地凭据读不到（keyring 缺失/损坏）—— 与 auth 区分
  | 'other' // 其它：IO / 解析 / 加密 / Docker 探测等

/**
 * 后端 AppError 序列化后的结构 { kind, message }。
 * Tauri invoke 失败时，错误会以该对象形式抛给前端。
 */
export interface StructuredError {
  kind: ErrorKind
  message: string
}
