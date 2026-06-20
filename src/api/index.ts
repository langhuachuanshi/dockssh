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
  CreateContainerOpts,
  DirListing,
  Host,
  Image,
  LogChunk,
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
export const pauseContainer = (hostId: string, id: string) =>
  invoke<void>('pause_container', { hostId, id })
export const unpauseContainer = (hostId: string, id: string) =>
  invoke<void>('unpause_container', { hostId, id })
export const renameContainer = (hostId: string, id: string, newName: string) =>
  invoke<void>('rename_container', { hostId, id, newName })
export const removeContainer = (hostId: string, id: string, force = false) =>
  invoke<void>('remove_container', { hostId, id, force })

/** 容器详情（完整 inspect，用于详情抽屉） */
export const inspectContainer = (hostId: string, id: string) =>
  invoke<ContainerInspect>('inspect_container', { hostId, id })

/** 创建并启动容器（docker run -d）。返回新容器 ID。 */
export const createContainer = (hostId: string, opts: CreateContainerOpts) =>
  invoke<string>('create_and_run_container', { hostId, opts })

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

export interface StartLogsOptions {
  /** 初始回看行数（默认 200） */
  tail?: string
  /** 时间过滤起点（RFC3339 或相对时间，透传 docker logs --since） */
  since?: string
  /** 时间过滤终点（透传 docker logs --until） */
  until?: string
  /** 每行加时间戳前缀 */
  timestamps?: boolean
  /** 是否持续跟踪，默认 true */
  follow?: boolean
}

export const startLogs = (
  hostId: string,
  container: string,
  opts: StartLogsOptions = {},
) =>
  invoke<void>('start_logs', {
    hostId,
    container,
    // 后端 tail 是 Option<String>，el-input-number 可能产生 number，统一强转
    tail: opts.tail != null ? String(opts.tail) : undefined,
    since: opts.since,
    until: opts.until,
    timestamps: opts.timestamps,
    follow: opts.follow,
  })

export const stopLogs = (hostId: string, container: string) =>
  invoke<void>('stop_logs', { hostId, container })

/** 监听某容器的日志数据块，返回取消监听函数 */
export const onLogChunk = (
  hostId: string,
  container: string,
  cb: (chunk: LogChunk) => void,
): Promise<UnlistenFn> =>
  listen<LogChunk>(`dockssh://log-chunk:${hostId}:${container}`, (e) =>
    cb(e.payload),
  )

/** 把文本写入本地文件（用于日志导出）。path 由 save() 对话框提供。 */
export const saveTextLocal = (path: string, content: string) =>
  invoke<void>('save_text_local', { path, content })

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

/** 监听 docker stats 的原始行（含无法解析的 stderr/表头），仅诊断用 */
export const onStatsRaw = (
  hostId: string,
  cb: (line: string) => void,
): Promise<UnlistenFn> =>
  listen<string>(`dockssh://stats-raw:${hostId}`, (e) => cb(e.payload))

// ===== 终端（docker exec -it 透传）=====
//
// 数据流：后端 emit base64 字符串，前端解码成 Uint8Array 喂给 xterm.write。
// 写入方向相反：xterm.onData → 字符串 → base64 → ptyWrite。
// 用 base64 是为了完整保留任意字节（ANSI 控制序列、非 UTF-8、CJK 截断等）。

/** 启动一个容器的交互式终端，返回 session_id */
export const ptyStart = (
  hostId: string,
  container: string,
  cols: number,
  rows: number,
  opts?: { user?: string; cwd?: string },
) =>
  invoke<string>('pty_start', {
    hostId,
    container,
    cols,
    rows,
    user: opts?.user,
    cwd: opts?.cwd,
  })

/** 写入按键（data 为 base64 编码的 bytes） */
export const ptyWrite = (sessionId: string, data: string) =>
  invoke<void>('pty_write', { sessionId, data })

/** 通知尺寸变化（cols/rows） */
export const ptyResize = (sessionId: string, cols: number, rows: number) =>
  invoke<void>('pty_resize', { sessionId, cols, rows })

/** 关闭并移除终端会话 */
export const ptyKill = (sessionId: string) =>
  invoke<void>('pty_kill', { sessionId })

/** 把某终端「投递」到独立终端窗口（单例，带 tabs）。
 *  主窗口应先 detach（保留后端 session）再调用。
 *  sessionId: 后端 PTY 会话 id；title: 容器名（tab 标题）。
 *  后端把终端推入待投递队列，并确保独立窗口存在。 */
export const attachTerminal = (sessionId: string, title: string) =>
  invoke<void>('attach_terminal', { sessionId, name: title })

/** 独立窗口加载完成后拉取所有待投递的终端（取出并清空队列）。 */
export const takePendingTerminals = () =>
  invoke<Array<{ session_id: string; name: string }>>('take_pending_terminals')

/** 监听「有新终端可拉取」通知（空 payload 信号），收到后调 takePendingTerminals。 */
export const onAttachTerminalNotify = (
  cb: () => void,
): Promise<UnlistenFn> =>
  listen('dockssh://attach-terminal', () => cb())

/** 监听终端输出（payload 为 base64 字符串），返回取消监听函数 */
export const onPtyData = (
  sessionId: string,
  cb: (b64: string) => void,
): Promise<UnlistenFn> =>
  listen<string>(`dockssh://pty-data:${sessionId}`, (e) => cb(e.payload))

/** 监听终端退出（payload 为退出码） */
export const onPtyExit = (
  sessionId: string,
  cb: (code: number) => void,
): Promise<UnlistenFn> =>
  listen<number>(`dockssh://pty-exit:${sessionId}`, (e) => cb(e.payload))

/** 监听终端错误 */
export const onPtyError = (
  sessionId: string,
  cb: (msg: string) => void,
): Promise<UnlistenFn> =>
  listen<string>(`dockssh://pty-error:${sessionId}`, (e) => cb(e.payload))
