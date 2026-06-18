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
