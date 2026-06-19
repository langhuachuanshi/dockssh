/**
 * 与 Rust 后端共享的类型定义。
 * 命名与 src-tauri/src/models.rs 对齐。
 */

export type AuthType = 'password' | 'key' | 'agent'

export interface Host {
  id: string
  name: string
  host: string
  port: number
  user: string
  auth_type: AuthType
  key_path: string | null
  verify_host_key: boolean
  /** 分组名（用于列表归类，可为 null = 未分组） */
  group: string | null
  /** 颜色标识 hex（用户手选，可为 null） */
  color: string | null
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

export interface ContainerInspect {
  working_dir: string
  mounts: ContainerMount[]
}

export interface VolumeInspect {
  mountpoint: string
}
