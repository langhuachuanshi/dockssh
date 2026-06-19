/**
 * 主机 store：管理主机列表、当前选中主机、在线状态。
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as api from '@/api'
import type { Host, HostProbe } from '@/api/types'

export const useHostsStore = defineStore('hosts', () => {
  const hosts = ref<Host[]>([])
  const currentId = ref<string | null>(null)
  const onlineMap = ref<Record<string, boolean>>({})
  const probeMap = ref<Record<string, HostProbe>>({})
  const loading = ref(false)

  const current = () => hosts.value.find((h) => h.id === currentId.value) || null
  const isOnline = (id: string) => !!onlineMap.value[id]

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

  async function connect(id: string) {
    const res = await api.connectHost(id)
    onlineMap.value[id] = true
    probeMap.value[id] = res.probe
    return res
  }

  /** 确保主机已连接，已在线则跳过。各内容页 onMounted 调用。 */
  async function ensureConnected(id: string) {
    if (isOnline(id)) return
    await connect(id)
  }

  async function disconnect(id: string) {
    await api.disconnectHost(id)
    onlineMap.value[id] = false
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
    current,
    isOnline,
    refresh,
    connect,
    ensureConnected,
    disconnect,
    select,
  }
})
