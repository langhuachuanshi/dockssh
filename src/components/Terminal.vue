<script setup lang="ts">
/**
 * 单个终端实例：xterm.js 封装。
 *
 * 职责：
 * - 挂载时初始化 xterm + FitAddon，拿到初始 cols/rows
 * - 若 tab 尚无 sessionId：调 ptyStart 建立后端会话并回填
 * - 订阅 onPtyData/Exit/Error 事件，解码 base64 喂给 xterm
 * - onData（按键）→ base64 → ptyWrite
 * - ResizeObserver（防抖）→ fit → ptyResize
 * - 卸载时 dispose xterm + 注销监听（销毁后端 session 由 store.close 负责）
 *
 * 注意：本组件放在 keep-alive 之外，靠面板的 v-show 控制显隐，
 * 切换 tab 时组件不卸载（用 v-if 之外的显隐），保证 xterm 实例和 SSH 会话存活。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Terminal as XTerm } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import type { TerminalTab } from '@/store/terminals'
import { useTerminalsStore } from '@/store/terminals'
import { isMigrating } from '@/utils/terminal-migrate'

const props = defineProps<{ tab: TerminalTab }>()
const store = useTerminalsStore()

const containerEl = ref<HTMLDivElement | null>(null)
let term: XTerm | null = null
let fitAddon: FitAddon | null = null
let ro: ResizeObserver | null = null
let resizeTimer: number | null = null
const unlistens: Array<() => void> = []
// 事件是否已订阅的幂等标志：startSession 与 sessionId watcher 都可能触发
// attachEvents，靠它保证只订阅一次（否则 pty-data 被监听两份 → 字符重复显示）。
let attached = false

/** base64 字符串 → Uint8Array */
function decodeB64(b64: string): Uint8Array {
  const bin = atob(b64)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return bytes
}

/** Uint8Array → base64 字符串 */
function encodeB64(bytes: Uint8Array): string {
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin)
}

async function startSession() {
  if (!term || !fitAddon) return
  const { cols, rows } = fitAddon.proposeDimensions() ?? { cols: 80, rows: 24 }
  try {
    const sid = await api.ptyStart(
      props.tab.hostId,
      props.tab.containerId,
      cols,
      rows,
    )
    store.bindSession(props.tab.id, sid)
    await attachEvents(sid)
  } catch (e) {
    ElMessage.error(`终端启动失败：${e}`)
  }
}

async function attachEvents(sessionId: string) {
  if (!term) return
  // 幂等：startSession 路径与 sessionId watcher 路径都可能进入这里，
  // 关键是「标志位在任何 await 之前翻转」，避免 watcher 的微任务因
  // unlistens.push 还没执行而误判为未订阅，导致重复订阅 pty-data。
  if (attached) return
  attached = true
  unlistens.push(
    await api.onPtyData(sessionId, (b64) => {
      term?.write(decodeB64(b64))
    }),
  )
  unlistens.push(
    await api.onPtyExit(sessionId, (code) => {
      // 远端进程退出（exit / 容器停止 / 连接断开）→ 写提示后自动关闭该 tab。
      // 不再让用户手动关，避免遗留「已退出」的死终端占着面板。
      term?.write(`\r\n\x1b[33m[进程已退出，代码 ${code}]\x1b[0m\r\n`)
      store.close(props.tab.id)
    }),
  )
  unlistens.push(
    await api.onPtyError(sessionId, (msg) => {
      ElMessage.error(`终端错误：${msg}`)
      // 错误通常意味着会话已坏（SSH 断开等），同样自动关闭
      store.close(props.tab.id)
    }),
  )
}

function scheduleFit() {
  if (resizeTimer) window.clearTimeout(resizeTimer)
  resizeTimer = window.setTimeout(() => {
    if (!term || !fitAddon) return
    try {
      fitAddon.fit()
      const { cols, rows } = fitAddon.proposeDimensions() ?? { cols: 80, rows: 24 }
      const sid = props.tab.sessionId
      if (sid) api.ptyResize(sid, cols, rows).catch(() => {})
    } catch {
      /* 容器未渲染好时 fit 可能抛错，忽略 */
    }
  }, 150)
}

/** VS Code Dark+ 风格终端配色：背景纯黑 #1e1e1e，与 VS Code 内置终端一致。
 *  ANSI 16 色沿用 VS Code 默认终端色，辨识度高、视觉习惯友好。 */
const MOCHA_THEME = {
  background: '#1e1e1e', // VS Code 编辑器背景
  foreground: '#cccccc',
  cursor: '#ffffff',
  cursorAccent: '#1e1e1e',
  selectionBackground: '#264f78', // VS Code 选区蓝
  // 标准色（VS Code Dark+ 终端）
  black: '#000000',
  red: '#cd3131',
  green: '#0dbc79',
  yellow: '#e5e510',
  blue: '#2472c8',
  magenta: '#bc3fbc',
  cyan: '#11a8cd',
  white: '#e5e5e5',
  // 明亮色
  brightBlack: '#666666',
  brightRed: '#f14c4c',
  brightGreen: '#23d18b',
  brightYellow: '#f5f543',
  brightBlue: '#3b8eea',
  brightMagenta: '#d670d6',
  brightCyan: '#29b8db',
  brightWhite: '#e5e5e5',
}

onMounted(async () => {
  const el = containerEl.value
  if (!el) return

  term = new XTerm({
    fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
    fontSize: 13,
    cursorBlink: true,
    scrollback: 5000,
    theme: MOCHA_THEME,
  })
  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(el)
  // theme 在 new XTerm 时已传，这里再 setOption 兜底：热重载场景下
  // 旧 xterm 实例可能已被 open，显式重设确保配色即时刷新。
  term.options.theme = MOCHA_THEME

  // 关键顺序：open → fit（拿初始尺寸）→ start → 订阅
  try {
    fitAddon.fit()
  } catch {
    /* ignore */
  }

  // 按键 → 后端
  term.onData((data) => {
    const sid = props.tab.sessionId
    if (!sid) return
    const bytes = new TextEncoder().encode(data)
    api.ptyWrite(sid, encodeB64(bytes)).catch(() => {})
  })

  // 若 tab 已有 session（复用），直接订阅；否则启动新会话
  if (props.tab.sessionId) {
    await attachEvents(props.tab.sessionId)
  } else {
    await startSession()
  }

  // 尺寸监听（防抖）
  ro = new ResizeObserver(() => scheduleFit())
  ro.observe(el)
})

// session_id 异步回填后，确保事件已订阅（startSession 内已 attach，此处兜底）。
// 注意：startSession 里 bindSession 触发本 watcher 与内部 attachEvents 存在竞态，
// 故 attachEvents 自身做了幂等（attached 标志），这里无需额外判断。
watch(
  () => props.tab.sessionId,
  (sid) => {
    if (sid) attachEvents(sid)
  },
)

onBeforeUnmount(() => {
  if (resizeTimer) window.clearTimeout(resizeTimer)
  ro?.disconnect()
  unlistens.forEach((fn) => fn())
  unlistens.length = 0
  const sid = props.tab.sessionId
  // 组件销毁时是否需要主动 ptyKill 后端 session：
  // - 独立窗口关闭（tab 不在 store 且非迁移）：ptyKill 清理，防泄漏
  // - 迁移到独立窗口（isMigrating 为 true）：跳过，session 交给独立窗口接管
  // - 面板内 tab 关闭（在 store 中）：由 store.close 负责，这里不重复
  if (sid && !isMigrating(sid)) {
    const inStore = store.tabs.some((t) => t.id === props.tab.id)
    if (!inStore) {
      api.ptyKill(sid).catch(() => {})
    }
  }
  term?.dispose()
  term = null
  fitAddon = null
})

// 暴露给父组件：tab 重新激活时重新 fit（尺寸可能因面板收起/展开变化）
defineExpose({
  refit() {
    scheduleFit()
  },
})
</script>

<template>
  <div ref="containerEl" class="terminal-instance" />
</template>

<style scoped>
.terminal-instance {
  width: 100%;
  height: 100%;
  padding: 4px 8px;
  background: #1e1e1e; /* VS Code Dark+ 背景纯黑 */
  overflow: hidden;
}
.terminal-instance :deep(.xterm) {
  height: 100%;
}
.terminal-instance :deep(.xterm-viewport) {
  background-color: transparent;
}
</style>
