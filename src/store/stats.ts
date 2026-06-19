/**
 * 监控历史数据 store：按 hostId 持久化 stats 历史。
 *
 * DashboardView 组件切换离开时会卸载，局部数组随之丢失。
 * 把 cpu/内存/网络历史、以及上一次网络累计值放在这里，
 * 切回来时能恢复历史曲线，网络速率也能正确算出第一个点。
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface HostStats {
  /** 时间标签，与各 history 数组一一对应 */
  labels: string[]
  cpuHistory: number[]
  memHistory: number[]
  netRxRateHistory: number[]
  netTxRateHistory: number[]
  /** 上一 tick 的网络累计字节，用于算速率（null 表示尚未有基准） */
  prevRxTotal: number | null
  prevTxTotal: number | null
  prevTickTs: number | null
}

const MAX_POINTS = 30

function emptyStats(): HostStats {
  return {
    labels: [],
    cpuHistory: [],
    memHistory: [],
    netRxRateHistory: [],
    netTxRateHistory: [],
    prevRxTotal: null,
    prevTxTotal: null,
    prevTickTs: null,
  }
}

export const useStatsStore = defineStore('stats', () => {
  const map = ref<Record<string, HostStats>>({})

  function get(id: string): HostStats {
    if (!map.value[id]) map.value[id] = emptyStats()
    return map.value[id]
  }

  /** 超过 MAX_POINTS 时整体左移，保持各数组长度一致。 */
  function pushPoint(
    id: string,
    label: string,
    cpu: number,
    mem: number,
    rxRate: number,
    txRate: number,
  ) {
    const s = get(id)
    s.labels.push(label)
    s.cpuHistory.push(cpu)
    s.memHistory.push(mem)
    s.netRxRateHistory.push(rxRate)
    s.netTxRateHistory.push(txRate)
    if (s.labels.length > MAX_POINTS) {
      s.labels.shift()
      s.cpuHistory.shift()
      s.memHistory.shift()
      s.netRxRateHistory.shift()
      s.netTxRateHistory.shift()
    }
  }

  return { map, get, pushPoint }
})
