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

const props = defineProps<{ tab: TerminalTab }>()
const store = useTerminalsStore()

const containerEl = ref<HTMLDivElement | null>(null)
let term: XTerm | null = null
let fitAddon: FitAddon | null = null
let ro: ResizeObserver | null = null
let resizeTimer: number | null = null
const unlistens: Array<() => void> = []

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
  unlistens.push(
    await api.onPtyData(sessionId, (b64) => {
      term?.write(decodeB64(b64))
    }),
  )
  unlistens.push(
    await api.onPtyExit(sessionId, (code) => {
      term?.write(`\r\n\x1b[33m[进程已退出，代码 ${code}]\x1b[0m\r\n`)
    }),
  )
  unlistens.push(
    await api.onPtyError(sessionId, (msg) => {
      ElMessage.error(`终端错误：${msg}`)
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

onMounted(async () => {
  const el = containerEl.value
  if (!el) return

  term = new XTerm({
    fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
    fontSize: 13,
    cursorBlink: true,
    scrollback: 5000,
    theme: {
      background: '#1e1e2e',
      foreground: '#cdd6f4',
      cursor: '#f5e0dc',
    },
  })
  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(el)

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

// session_id 异步回填后，确保事件已订阅（startSession 内已 attach，此处兜底）
watch(
  () => props.tab.sessionId,
  (sid) => {
    if (sid && unlistens.length === 0) attachEvents(sid)
  },
)

onBeforeUnmount(() => {
  if (resizeTimer) window.clearTimeout(resizeTimer)
  ro?.disconnect()
  unlistens.forEach((fn) => fn())
  unlistens.length = 0
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
  background: #1e1e2e;
  overflow: hidden;
}
.terminal-instance :deep(.xterm) {
  height: 100%;
}
.terminal-instance :deep(.xterm-viewport) {
  background-color: transparent;
}
</style>
