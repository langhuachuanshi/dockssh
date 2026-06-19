/**
 * Tauri command 调用封装。
 * 每个函数对应 src-tauri 中的一个 #[tauri::command]。
 */
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ComposeProject,
  Container,
  ContainerInspect,
  ConnectResult,
  DirListing,
  Host,
  Image,
  Network,
  StatsSample,
  Volume,
  VolumeInspect,
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
  /** 私钥口令（密钥认证且密钥有口令时） */
  passphrase?: string
  verify_host_key: boolean
  group?: string | null
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

/** 容器详情（取挂载/工作目录，用于「打开目录」） */
export const inspectContainer = (hostId: string, id: string) =>
  invoke<ContainerInspect>('inspect_container', { hostId, id })

// ===== 镜像 =====

export const listImages = (hostId: string) =>
  invoke<Image[]>('list_images', { hostId })
export const removeImage = (hostId: string, id: string, force = false) =>
  invoke<void>('remove_image', { hostId, id, force })

// ===== 网络 =====

export const listNetworks = (hostId: string) =>
  invoke<Network[]>('list_networks', { hostId })

// ===== 存储卷 =====

export const listVolumes = (hostId: string) =>
  invoke<Volume[]>('list_volumes', { hostId })

/** 卷详情（取 Mountpoint，用于「打开目录」） */
export const inspectVolume = (hostId: string, name: string) =>
  invoke<VolumeInspect>('inspect_volume', { hostId, name })

/** 删除卷，force=true 时强制（即使有容器引用） */
export const removeVolume = (hostId: string, name: string, force = false) =>
  invoke<void>('remove_volume', { hostId, name, force })

// ===== 项目（compose） =====

export const listComposeProjects = (hostId: string) =>
  invoke<ComposeProject[]>('list_compose_projects', { hostId })
export const readComposeFile = (hostId: string, path: string) =>
  invoke<string>('read_compose_file', { hostId, path })
export const saveComposeFile = (hostId: string, path: string, content: string) =>
  invoke<void>('save_compose_file', { hostId, path, content })
export const downComposeProject = (hostId: string, projectName: string) =>
  invoke<string>('down_compose_project', { hostId, projectName })
export const buildComposeProject = (hostId: string, projectName: string) =>
  invoke<string>('build_compose_project', { hostId, projectName })
export const upComposeProject = (hostId: string, projectName: string) =>
  invoke<string>('up_compose_project', { hostId, projectName })
export const stopComposeProject = (hostId: string, projectName: string) =>
  invoke<string>('stop_compose_project', { hostId, projectName })
export const restartComposeProject = (hostId: string, projectName: string) =>
  invoke<string>('restart_compose_project', { hostId, projectName })

// ===== 仓库（registry） =====

export const listRegistries = (hostId: string) =>
  invoke<string[]>('list_registries', { hostId })

// ===== 镜像 logo 缓存 =====

/** 读本地缓存 logo，未命中返回 null */
export const getCachedLogo = (slug: string) =>
  invoke<string | null>('get_cached_logo', { slug })

/** 从 iconify 下载 logo 并写入缓存，返回 SVG 文本 */
export const fetchLogo = (slug: string, prefixes?: string[]) =>
  invoke<string>('fetch_logo', { slug, prefixes })

/** 删除单个 logo 缓存 */
export const deleteCachedLogo = (slug: string) =>
  invoke<void>('delete_cached_logo', { slug })

/** 清空所有 logo 缓存 */
export const clearLogoCache = () =>
  invoke<void>('clear_logo_cache')

// ===== 文件管理（SFTP） =====

/** 当前 SSH 用户主目录（首次进入「文件」时调用） */
export const fileHome = (hostId: string) =>
  invoke<string>('file_home', { hostId })

/** 列出目录 */
export const listDir = (hostId: string, path: string) =>
  invoke<DirListing>('list_dir', { hostId, path })

/** 读取小文本文件用于预览（后端限制 1MB） */
export const fileReadText = (hostId: string, path: string) =>
  invoke<string>('file_read_text', { hostId, path })

/** 新建目录 */
export const fileMkdir = (hostId: string, path: string) =>
  invoke<void>('file_mkdir', { hostId, path })

/** 删除文件或空目录 */
export const fileRemove = (hostId: string, path: string, isDir: boolean) =>
  invoke<void>('file_remove', { hostId, path, isDir })

/** 重命名/移动 */
export const fileRename = (hostId: string, from: string, to: string) =>
  invoke<void>('file_rename', { hostId, from, to })

/** 下载远程文件到本地 */
export const fileDownload = (hostId: string, remote: string, local: string) =>
  invoke<void>('file_download', { hostId, remote, local })

/** 上传本地文件到远程目录 */
export const fileUpload = (hostId: string, local: string, remoteDir: string) =>
  invoke<void>('file_upload', { hostId, local, remoteDir })

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
