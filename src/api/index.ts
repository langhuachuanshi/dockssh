/**
 * Tauri command 调用封装。
 * 每个函数对应 src-tauri 中的一个 #[tauri::command]。
 */
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  Container,
  ConnectResult,
  Host,
  Image,
  StatsSample,
} from './types'

// ===== 主机 =====

export interface HostInput {
  id?: string
  name: string
  host: string
  port: number
  user: string
  auth_type: Host['auth_type']
  key_path?: string | null
  password?: string
  verify_host_key: boolean
}

export const listHosts = () => invoke<Host[]>('list_hosts')
export const saveHost = (input: HostInput) =>
  invoke<Host>('save_host', { input })
export const deleteHost = (hostId: string) =>
  invoke<void>('delete_host', { hostId })
export const connectHost = (hostId: string) =>
  invoke<ConnectResult>('connect_host', { hostId })
export const disconnectHost = (hostId: string) =>
  invoke<void>('disconnect_host', { hostId })
export const isHostOnline = (hostId: string) =>
  invoke<boolean>('is_host_online', { hostId })

// ===== 容器 =====

export const listContainers = (hostId: string) =>
  invoke<Container[]>('list_containers', { hostId })
export const startContainer = (hostId: string, id: string) =>
  invoke<void>('start_container', { hostId, id })
export const stopContainer = (hostId: string, id: string) =>
  invoke<void>('stop_container', { hostId, id })
export const restartContainer = (hostId: string, id: string) =>
  invoke<void>('restart_container', { hostId, id })
export const removeContainer = (hostId: string, id: string, force = false) =>
  invoke<void>('remove_container', { hostId, id, force })

// ===== 镜像 =====

export const listImages = (hostId: string) =>
  invoke<Image[]>('list_images', { hostId })
export const removeImage = (hostId: string, id: string, force = false) =>
  invoke<void>('remove_image', { hostId, id, force })

// ===== 日志（流式） =====

export const startLogs = (
  hostId: string,
  container: string,
  tail?: string,
) => invoke<void>('start_logs', { hostId, container, tail })
export const stopLogs = (hostId: string, container: string) =>
  invoke<void>('stop_logs', { hostId, container })

/** 监听某容器的日志数据块，返回取消监听函数 */
export const onLogChunk = (
  hostId: string,
  container: string,
  cb: (chunk: string) => void,
): Promise<UnlistenFn> =>
  listen<string>(`dockssh://log-chunk:${hostId}:${container}`, (e) =>
    cb(e.payload),
  )

// ===== stats（流式） =====

export const startStats = (hostId: string, intervalSecs = 2) =>
  invoke<void>('start_stats', { hostId, intervalSecs })
export const stopStats = (hostId: string) =>
  invoke<void>('stop_stats', { hostId })

export const onStats = (
  hostId: string,
  cb: (sample: StatsSample) => void,
): Promise<UnlistenFn> =>
  listen<StatsSample>(`dockssh://stats:${hostId}`, (e) => cb(e.payload))
