/**
 * 主机标签页 store：像浏览器标签那样管理「已打开的主机」。
 *
 * - tabs: 已打开的主机 id 列表（保持打开顺序）
 * - activeId: 当前激活的 tab（host id）
 *
 * 交互：
 * - 打开主机 → 若未在 tabs 则追加，并设为激活
 * - 关闭 tab → 从 tabs 移除；若关闭的是激活 tab，切到相邻 tab；全关则 activeId=null
 * - 切换 tab → 仅改激活 id，URL 由调用方处理
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useTabsStore = defineStore('tabs', () => {
  const tabs = ref<string[]>([])
  const activeId = ref<string | null>(null)

  /** 打开（或激活）某台主机的 tab */
  function open(id: string) {
    if (!tabs.value.includes(id)) {
      tabs.value.push(id)
    }
    activeId.value = id
  }

  /** 关闭某台主机的 tab，返回应激活的相邻 tab id（可能为 null） */
  function close(id: string): string | null {
    const idx = tabs.value.indexOf(id)
    if (idx === -1) return activeId.value
    tabs.value.splice(idx, 1)

    if (activeId.value !== id) return activeId.value

    // 关闭的是激活 tab：优先取右侧，否则左侧
    const next = tabs.value[idx] || tabs.value[idx - 1] || null
    activeId.value = next
    return next
  }

  /** 切换激活 tab */
  function setActive(id: string) {
    if (tabs.value.includes(id)) {
      activeId.value = id
    }
  }

  /** 拖拽排序：把 from 移到 to 的位置 */
  function move(from: number, to: number) {
    if (from === to || from < 0 || to < 0) return
    if (from >= tabs.value.length || to >= tabs.value.length) return
    const [id] = tabs.value.splice(from, 1)
    tabs.value.splice(to, 0, id)
  }

  /** 主机被删除时，从 tabs 移除 */
  function remove(id: string) {
    close(id)
  }

  return { tabs, activeId, open, close, setActive, move, remove }
})
