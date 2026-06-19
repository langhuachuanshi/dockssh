/**
 * 终端面板状态：管理「已打开的终端会话」。
 *
 * 一个 tab = 一个容器的交互式终端（对应后端一个 PTY session）。
 *
 * 复用语义：以 `${hostId}:${containerId}` 为 key，
 *   - 点「终端」→ 若该容器已有 tab 则激活，不新建（避免重复 session）
 *   - 关 tab → 调 ptyKill 销毁后端 session，从列表移除
 *   - 全部关闭 → 面板收起（visible=false）
 *
 * 终端面板挂载在 App.vue 顶层，与 router-view 平级，
 * 因此路由切换/keep-alive 都不影响正在运行的终端。
 */
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as api from '@/api'

export interface TerminalTab {
  /** 面板内唯一 id：`${hostId}:${containerId}` */
  id: string
  hostId: string
  containerId: string
  /** 容器显示名（用于 tab 标题） */
  name: string
  /** 后端返回的 PTY session_id（ptyStart 后填充） */
  sessionId: string | null
}

/** 终端面板停靠方向（VS Code 风格：下/左/右，不做「上」）。 */
export type DockSide = 'bottom' | 'left' | 'right'

export const useTerminalsStore = defineStore('terminals', () => {
  const tabs = ref<TerminalTab[]>([])
  const activeId = ref<string | null>(null)
  /** 面板是否展开 */
  const visible = ref(false)
  /** 面板高度（px），底部停靠时生效，可拖拽调整 */
  const height = ref(300)
  /** 面板宽度（px），左右停靠时生效，可拖拽调整 */
  const width = ref(520)
  /** 停靠方向：bottom=底部抽屉，left/right=侧边竖条 */
  const dockSide = ref<DockSide>('bottom')

  const activeTab = computed(
    () => tabs.value.find((t) => t.id === activeId.value) ?? null,
  )

  function makeId(hostId: string, containerId: string) {
    return `${hostId}:${containerId}`
  }

  /**
   * 打开（或激活）某容器的终端 tab。
   * 返回该 tab —— 由调用方负责 ptyStart 并回填 sessionId（避免 store 直接依赖 invoke 的副作用）。
   */
  function open(hostId: string, containerId: string, name: string): TerminalTab {
    const id = makeId(hostId, containerId)
    let tab = tabs.value.find((t) => t.id === id)
    if (!tab) {
      tab = { id, hostId, containerId, name, sessionId: null }
      tabs.value.push(tab)
    } else {
      tab.name = name // 名称可能变了，刷新一下
    }
    activeId.value = id
    visible.value = true
    return tab
  }

  /** 回填后端 session_id（ptyStart 成功后调用） */
  function bindSession(tabId: string, sessionId: string) {
    const tab = tabs.value.find((t) => t.id === tabId)
    if (tab) tab.sessionId = sessionId
  }

  /**
   * 独立终端窗口专用：直接用已有 sessionId 加 tab（承接主窗口投递的终端）。
   * 与 open 的区别：open 的 tab 初始 sessionId=null（由 Terminal 组件 ptyStart 建立），
   * 本方法 sessionId 已存在（Terminal 组件跳过 ptyStart 直接订阅接管）。
   * 幂等：同一 sessionId 重复投递只激活已有 tab。
   */
  function openAttached(sessionId: string, name: string) {
    const id = `w-${sessionId}`
    let tab = tabs.value.find((t) => t.id === id)
    if (!tab) {
      tab = {
        id,
        hostId: 'window',
        containerId: sessionId,
        name,
        sessionId,
      }
      tabs.value.push(tab)
    } else {
      tab.name = name
    }
    activeId.value = id
    visible.value = true
  }

  function setActive(id: string) {
    if (tabs.value.some((t) => t.id === id)) {
      activeId.value = id
      visible.value = true
    }
  }

  /** 关闭单个 tab：先销毁后端 session，再移除。返回应激活的相邻 tab id */
  async function close(id: string): Promise<string | null> {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return activeId.value
    const tab = tabs.value[idx]
    // 销毁后端 PTY 会话（忽略错误：可能已随主机断开而消失）
    if (tab.sessionId) {
      await api.ptyKill(tab.sessionId).catch(() => {})
    }
    tabs.value.splice(idx, 1)

    if (activeId.value !== id) return activeId.value
    const next = tabs.value[idx] || tabs.value[idx - 1] || null
    activeId.value = next?.id ?? null
    if (tabs.value.length === 0) visible.value = false
    return next?.id ?? null
  }

  /** 关闭某主机的全部终端 tab（主机断开/删除时） */
  async function closeByHost(hostId: string) {
    const ids = tabs.value.filter((t) => t.hostId === hostId).map((t) => t.id)
    for (const id of ids) {
      await close(id)
    }
  }

  /**
   * 把某 tab 从面板「撕离」（迁移到独立窗口）：
   * - 仅移除前端 tab，**不销毁后端 PTY 会话**（由独立窗口接管同一个 sessionId）
   * - 返回该 tab 的完整信息（含 sessionId），供独立窗口重建 xterm 订阅
   * 与 close 的区别：close 会 ptyKill 后端，detach 保留后端。
   */
  function detach(id: string): TerminalTab | null {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return null
    const tab = tabs.value[idx]
    tabs.value.splice(idx, 1)
    if (activeId.value === id) {
      const next = tabs.value[idx] || tabs.value[idx - 1] || null
      activeId.value = next?.id ?? null
    }
    if (tabs.value.length === 0) visible.value = false
    return tab
  }

  function toggle() {
    if (tabs.value.length === 0) return
    visible.value = !visible.value
  }

  function setHeight(h: number) {
    height.value = Math.max(120, Math.min(h, 800))
  }

  function setWidth(w: number) {
    width.value = Math.max(240, Math.min(w, 900))
  }

  /** 切换停靠方向。切换后面板尺寸语义变化（高度↔宽度），由面板组件负责 refit。 */
  function setDockSide(s: DockSide) {
    dockSide.value = s
  }

  return {
    tabs,
    activeId,
    visible,
    height,
    width,
    dockSide,
    activeTab,
    open,
    openAttached,
    bindSession,
    setActive,
    close,
    detach,
    closeByHost,
    toggle,
    setHeight,
    setWidth,
    setDockSide,
  }
})
