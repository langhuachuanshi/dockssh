/**
 * 主机 store：管理主机列表、当前选中主机、在线状态。
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as api from '@/api'
import type { ErrorKind, Host, HostProbe, StructuredError } from '@/api/types'

/** 连接阶段，供前端显示「正在连接/认证/探测」进度。 */
export type ConnectPhase = 'idle' | 'connecting' | 'auth' | 'probing' | 'done' | 'failed'

export const useHostsStore = defineStore('hosts', () => {
  const hosts = ref<Host[]>([])
  const currentId = ref<string | null>(null)
  const onlineMap = ref<Record<string, boolean>>({})
  const probeMap = ref<Record<string, HostProbe>>({})
  const loading = ref(false)
  /** 每台主机当前的连接阶段（key = host_id） */
  const phaseMap = ref<Record<string, ConnectPhase>>({})
  /** 最近一次连接失败的结构化错误（key = host_id），用于弹窗展示 */
  const lastErrorMap = ref<Record<string, StructuredError>>({})

  const current = () => hosts.value.find((h) => h.id === currentId.value) || null
  const isOnline = (id: string) => !!onlineMap.value[id]
  const phaseOf = (id: string) => phaseMap.value[id] || 'idle'

  async function refresh() {
    loading.value = true
    try {
      hosts.value = await api.listHosts()
      // 并发查询每台主机在线状态（之前是串行 await，主机一多列表打开明显卡顿）
      const results = await Promise.all(
        hosts.value.map(async (h) => {
          try {
            return [h.id, await api.isHostOnline(h.id)] as const
          } catch {
            return [h.id, false] as const
          }
        }),
      )
      const next: Record<string, boolean> = {}
      for (const [id, online] of results) next[id] = online
      onlineMap.value = next
    } finally {
      loading.value = false
    }
  }

  /**
   * 把 Tauri 抛出的错误归一成 StructuredError。
   * 后端 AppError 序列化成 {kind,message}，但 Tauri 透传时可能是对象/JSON 字符串/纯字符串，这里统一兜底。
   */
  function normalizeError(e: unknown): StructuredError {
    // 对象形态：直接是 {kind,message}
    if (e && typeof e === 'object') {
      const obj = e as Record<string, unknown>
      if (typeof obj.kind === 'string' && typeof obj.message === 'string') {
        return { kind: obj.kind as ErrorKind, message: obj.message }
      }
    }
    // 字符串形态：可能是 JSON 串或纯文本
    if (typeof e === 'string') {
      try {
        const parsed = JSON.parse(e)
        if (parsed && typeof parsed.kind === 'string' && typeof parsed.message === 'string') {
          return { kind: parsed.kind as ErrorKind, message: parsed.message }
        }
      } catch {
        // 纯文本
      }
      return { kind: 'other', message: e }
    }
    return { kind: 'other', message: String(e ?? '未知错误') }
  }

  async function connect(id: string) {
    // 阶段推进只是 UI 提示，实际后端是一次性 invoke；
    // 用定时器模拟阶段感，让用户看到「连接→认证→探测」的进度。
    phaseMap.value[id] = 'connecting'
    delete lastErrorMap.value[id]
    const timers: ReturnType<typeof setTimeout>[] = []
    timers.push(setTimeout(() => {
      if (phaseMap.value[id] === 'connecting') phaseMap.value[id] = 'auth'
    }, 600))
    timers.push(setTimeout(() => {
      if (phaseMap.value[id] === 'auth') phaseMap.value[id] = 'probing'
    }, 1800))
    try {
      const res = await api.connectHost(id)
      timers.forEach(clearTimeout)
      phaseMap.value[id] = 'done'
      onlineMap.value[id] = true
      probeMap.value[id] = res.probe
      return res
    } catch (e) {
      timers.forEach(clearTimeout)
      const err = normalizeError(e)
      phaseMap.value[id] = 'failed'
      lastErrorMap.value[id] = err
      throw err
    }
  }

  /** 确保主机已连接，已在线则跳过。各内容页 onMounted 调用。 */
  async function ensureConnected(id: string) {
    if (isOnline(id)) return
    await connect(id)
  }

  async function disconnect(id: string) {
    await api.disconnectHost(id)
    onlineMap.value[id] = false
    phaseMap.value[id] = 'idle'
  }

  function select(id: string) {
    currentId.value = id
  }

  return {
    hosts,
    currentId,
    onlineMap,
    probeMap,
    loading,
    phaseMap,
    lastErrorMap,
    current,
    isOnline,
    phaseOf,
    refresh,
    connect,
    ensureConnected,
    disconnect,
    select,
  }
})
