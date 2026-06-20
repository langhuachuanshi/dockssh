<script setup lang="ts">
/**
 * 容器日志页（流式 tail + 增强功能）。
 *
 * Bug 修复要点：必须先 await onLogChunk 再 await startLogs。
 * docker logs --tail 在流打开瞬间会把全部历史一次性突发推送，
 * Tauri 事件是无缓冲广播，监听器没注册就丢——所以注册顺序很关键。
 *
 * 增强功能：
 * A. 搜索 + 关键词高亮（正则 / 大小写可选）
 * B. 暂停 / 继续 / 下载导出
 * C. 时间过滤（since/until）+ 可选行时间戳（需重启流生效）
 * D. stdout/stderr 分流（即时过滤，无需重启流）
 *
 * 颜色处理：始终用 ansi_up 渲染——日志带 ANSI 码就显色，
 * 不带码（SSH 非交互模式容器多半如此）就显示浅灰纯文本。无需用户干预。
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { save } from '@tauri-apps/plugin-dialog'
import { AnsiUp } from 'ansi_up'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)
const cid = computed(() => route.params.cid as string)
const name = computed(() => (route.query.name as string) || cid.value)

const ansi = new AnsiUp()
// ansi_up 默认 use_classes=false（输出内联 style），适合深底日志区。
// escape_for_html 默认 true，HTML 实体已转义，v-html 安全。

const logEl = ref<HTMLDivElement>()

// ===== 行模型 =====
interface LogLine {
  /** 自增 id，作 v-for key */
  id: number
  /** 来源流标签 */
  stream: 'stdout' | 'stderr'
  /** 原始文本（保留 ANSI 码） */
  raw: string
  /** 去除 ANSI 的纯文本（缓存，供搜索/导出/纯文本渲染） */
  plain: string
}

let lineSeq = 0
const lines = ref<LogLine[]>([])
const MAX_LINES = 10000
const TRIM_TO = 8000

// 不完整行缓冲：跨 chunk 的半行暂存，等下一个 chunk 拼接
let pending = { text: '', stream: 'stdout' as 'stdout' | 'stderr' }

/** 把一块 chunk 追加成完整行（按 \n 切分，末尾不完整行暂存） */
function appendChunk(text: string, stream: 'stdout' | 'stderr') {
  const combined = pending.text + text
  const parts = combined.split('\n')
  // 最后一段可能是不完整行（没有结尾换行），暂存
  pending.text = parts.pop() ?? ''
  pending.stream = stream
  for (const p of parts) {
    pushLine(stream, p)
  }
}

function pushLine(stream: 'stdout' | 'stderr', raw: string) {
  lines.value.push({
    id: ++lineSeq,
    stream,
    raw,
    plain: stripAnsi(raw),
  })
  if (lines.value.length > MAX_LINES) {
    lines.value.splice(0, lines.value.length - TRIM_TO)
  }
}

function flushPending() {
  // 流结束时把残留的半行也输出
  if (pending.text !== '') {
    pushLine(pending.stream, pending.text)
    pending = { text: '', stream: 'stdout' }
  }
}

// ANSI 去除正则（覆盖 CSI/OSC 等常见序列）
const ANSI_RE = /\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07?|\x1b[@-_]/g
function stripAnsi(s: string): string {
  return s.replace(ANSI_RE, '')
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

// ===== 流式控制 =====
let unlisten: (() => void) | null = null
const running = ref(false)

interface StreamOpts {
  tail?: string
  since?: string
  until?: string
  timestamps?: boolean
  follow?: boolean
}

/** 启动一条日志流。注意：先注册监听器再调 startLogs，避免历史首包被丢。 */
async function startStream(opts: StreamOpts = {}) {
  await stopStream()
  // 先挂监听，再启动流——关键修复
  unlisten = await api.onLogChunk(hostId.value, cid.value, (chunk) => {
    // 兼容新旧后端 payload：新版是 { stream, text }，旧版是裸字符串
    // （后端二进制若未重建，仍会发裸字符串）
    const text = typeof chunk === 'string' ? chunk : chunk?.text ?? ''
    const stream = typeof chunk === 'string' ? 'stdout' : chunk?.stream ?? 'stdout'
    console.debug('[log-chunk]', { stream, len: text.length, head: text.slice(0, 60) })
    appendChunk(text, stream)
    if (autoScroll.value) nextTick(() => scrollToBottom())
  })
  console.debug('[log] listener attached, starting stream', { hostId: hostId.value, cid: cid.value, opts })
  try {
    // el-input-number 可能把 tail 变成 number，后端要 String，统一强转
    const fixed: StreamOpts = {
      ...opts,
      tail: opts.tail != null ? String(opts.tail) : undefined,
    }
    await api.startLogs(hostId.value, cid.value, fixed)
    running.value = true
    console.debug('[log] startLogs resolved, running=true')
  } catch (e) {
    console.error('[log] startLogs failed:', e)
    ElMessage.error(`启动日志流失败：${e}`)
  }
}

async function stopStream() {
  unlisten?.()
  unlisten = null
  await api.stopLogs(hostId.value, cid.value).catch(() => {})
  running.value = false
}

function scrollToBottom() {
  if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
}

// ===== 工具栏状态 =====
const autoScroll = ref(true)
const tail = ref('500')
const streamFilter = ref<'all' | 'stdout' | 'stderr'>('all') // 功能 D：分流
const streamOptions = [
  { label: '全部', value: 'all' },
  { label: '输出', value: 'stdout' },
  { label: '错误', value: 'stderr' },
]
const showTimestamps = ref(false) // 功能 C：时间戳
const sinceDate = ref<string>('') // 功能 C：起始时间
const untilDate = ref<string>('') // 功能 C：结束时间

// 功能 A：搜索
const search = ref('')
const useRegex = ref(false)
const caseSensitive = ref(false)
const searchRegex = computed<RegExp | null>(() => {
  const kw = search.value
  if (!kw) return null
  try {
    if (useRegex.value) {
      return new RegExp(kw, caseSensitive.value ? '' : 'i')
    }
    // 普通字符串：转义元字符
    const escaped = kw.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    return new RegExp(escaped, caseSensitive.value ? '' : 'i')
  } catch {
    return null
  }
})

/** 当前搜索匹配到的行数（用于显示 "N/总数"） */
const matchCount = computed(() => {
  if (!searchRegex.value) return 0
  let n = 0
  for (const l of lines.value) {
    if (searchRegex.value.test(l.plain)) n++
  }
  return n
})

/** 过滤后的行（分流 + 搜索） */
const filteredLines = computed(() => {
  let arr = lines.value
  if (streamFilter.value !== 'all') {
    arr = arr.filter((l) => l.stream === streamFilter.value)
  }
  if (searchRegex.value) {
    arr = arr.filter((l) => searchRegex.value!.test(l.plain))
  }
  return arr
})

/** 渲染单行 HTML：搜索激活时降级为纯文本高亮，否则 ansi_up 渲染。
 * ansi_up 对无 ANSI 码的文本原样透传（已转义 HTML），无副作用，
 * 所以始终启用——日志带色就显示色，不带色就显示纯文本，无需用户干预。 */
function renderLine(l: LogLine): string {
  const re = searchRegex.value
  if (re) {
    // 搜索模式：纯文本 + 高亮 mark（global 标志确保 replace 替换所有）
    const flags = re.flags.includes('g') ? re.flags : re.flags + 'g'
    const global = new RegExp(re.source, flags)
    return escapeHtml(l.plain).replace(global, '<mark class="hl">$&</mark>')
  }
  // 搜索态以外：ansi_up 渲染（有色显色，无色透传）
  return ansi.ansi_to_html(l.raw)
}

// ===== 操作 =====
function clearLogs() {
  lines.value = []
  pending = { text: '', stream: 'stdout' }
  lineSeq = 0
}

// 功能 B：暂停 / 继续
async function togglePause() {
  if (running.value) {
    await stopStream()
    flushPending()
  } else {
    // 继续：只看新输出，避免重复历史
    await startStream({ tail: '0', follow: true })
  }
}

// 功能 C：应用时间过滤（重启流）
async function applyTimeFilter() {
  try {
    await startStream({
      tail: tail.value,
      since: sinceDate.value || undefined,
      until: untilDate.value || undefined,
      timestamps: showTimestamps.value,
      follow: true,
    })
  } catch (e) {
    ElMessage.error(`应用过滤失败：${e}`)
  }
}

// 功能 B：下载导出
async function downloadLogs() {
  flushPending()
  const target = filteredLines.value
  if (!target.length) {
    ElMessage.warning('当前没有可导出的日志')
    return
  }
  const dateStr = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')
  const defaultName = `logs-${name.value}-${dateStr}.log`
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: '日志', extensions: ['log', 'txt'] }],
  })
  if (!path) return
  const content = target.map((l) => l.plain).join('\n') + '\n'
  try {
    await api.saveTextLocal(path, content)
    ElMessage.success(`已导出到 ${path}`)
  } catch (e) {
    ElMessage.error(`导出失败：${e}`)
  }
}

// 时间戳开关变化时提示用户需要应用
watch([showTimestamps], () => {
  if (running.value) {
    ElMessage.info('时间戳设置将在下次「应用」或「继续」时生效')
  }
})

// 自动滚动：filteredLines 变化时跟随
watch(
  () => filteredLines.value.length,
  () => {
    if (autoScroll.value) nextTick(() => scrollToBottom())
  },
)

onMounted(async () => {
  if (!hostId.value || !cid.value) {
    console.warn('[logs] hostId/cid 为空，跳过初始化')
    return
  }
  await store.ensureConnected(hostId.value)
  await startStream({ tail: tail.value, follow: true })
})

onUnmounted(() => {
  stopStream()
})
</script>

<template>
  <div class="page">
    <!-- 工具栏（单行，全部原生 element 组件） -->
    <div class="toolbar">
      <div class="toolbar-left">
        <el-button :icon="ArrowLeft" text @click="$router.back()">返回</el-button>
        <span class="title">日志：{{ name }}</span>
        <el-tag v-if="running" size="small" type="success" effect="plain">实时</el-tag>
        <el-tag v-else size="small" type="info" effect="plain">已暂停</el-tag>
      </div>

      <div class="toolbar-right">
        <!-- 搜索（尾部 Aa 按钮切换区分大小写） -->
        <el-input
          v-model="search"
          placeholder="搜索"
          clearable
          size="small"
          :prefix-icon="Search"
          class="search-input"
        >
          <template #append>
            <el-tooltip content="区分大小写" placement="bottom">
              <el-button
                class="case-btn"
                :class="{ active: caseSensitive }"
                @click="caseSensitive = !caseSensitive"
              >Aa</el-button>
            </el-tooltip>
          </template>
        </el-input>

        <!-- 分流 -->
        <el-segmented v-model="streamFilter" :options="streamOptions" size="small" />

        <!-- 暂停 / 继续 -->
        <el-button
          size="small"
          :type="running ? 'warning' : 'success'"
          :icon="running ? VideoPause : VideoPlay"
          @click="togglePause"
        >
          {{ running ? '暂停' : '继续' }}
        </el-button>

        <!-- 导出 -->
        <el-button size="small" :icon="Download" @click="downloadLogs">导出</el-button>

        <!-- 设置（低频项：自动滚动 / 正则 / 时间戳 / 时间范围 / 初始行数） -->
        <el-popover placement="bottom-end" :width="280" trigger="click">
          <template #reference>
            <el-button size="small" text :icon="Setting">设置</el-button>
          </template>
          <div class="settings-panel">
            <div class="setting-row">
              <span class="setting-label">自动滚动</span>
              <el-switch v-model="autoScroll" size="small" />
            </div>
            <div class="setting-row">
              <span class="setting-label">正则搜索</span>
              <el-switch v-model="useRegex" size="small" />
            </div>
            <el-divider class="setting-divider" />
            <div class="setting-row">
              <span class="setting-label">显示时间戳</span>
              <el-switch v-model="showTimestamps" size="small" />
            </div>
            <div class="setting-row">
              <span class="setting-label">初始行数</span>
              <el-input-number v-model="tail" :min="0" :step="100" size="small" controls-position="right" class="tail-num" />
            </div>
            <el-divider class="setting-divider" />
            <div class="setting-time">
              <span class="setting-label">起始时间</span>
              <el-date-picker
                v-model="sinceDate"
                type="datetime"
                size="small"
                placeholder="不限"
                value-format="YYYY-MM-DDTHH:mm:ss"
                class="date-picker"
              />
            </div>
            <div class="setting-time">
              <span class="setting-label">结束时间</span>
              <el-date-picker
                v-model="untilDate"
                type="datetime"
                size="small"
                placeholder="不限"
                value-format="YYYY-MM-DDTHH:mm:ss"
                class="date-picker"
              />
            </div>
            <el-button size="small" type="primary" class="apply-btn" :icon="Check" @click="applyTimeFilter">
              应用时间过滤
            </el-button>
          </div>
        </el-popover>

        <!-- 清空 -->
        <el-button size="small" text :icon="Delete" @click="clearLogs" />
      </div>
    </div>

    <!-- 搜索命中提示 -->
    <div v-if="search" class="match-bar">
      匹配 <span class="mono">{{ matchCount }}</span> / {{ filteredLines.length }} 行
    </div>

    <!-- 日志区（始终用 ansi_up 渲染：带色显色，无色透传） -->
    <div
      ref="logEl"
      :class="['log-view', 'selectable', 'mono']"
    >
      <pre v-for="l in filteredLines" :key="l.id" :class="['log-line', `stream-${l.stream}`]" v-html="renderLine(l)" />
      <div v-if="!filteredLines.length" class="empty-wait">
        {{ search ? '无匹配日志' : '等待日志输出…' }}
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  ArrowLeft,
  Delete,
  Download,
  Search,
  Check,
  Setting,
  VideoPause,
  VideoPlay,
} from '@element-plus/icons-vue'
export default {
  components: { ArrowLeft, Delete, Download, Search, Check, Setting, VideoPause, VideoPlay },
}
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}
/* ===== 工具栏：单行，原生 element 组件 ===== */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--el-border-color);
  background: var(--el-bg-color);
  flex-wrap: nowrap;
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex-shrink: 0;
}
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: nowrap;
  justify-content: flex-end;
}
.title {
  font-weight: 600;
  color: var(--el-text-color-primary);
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 240px;
}
.search-input {
  width: 220px;
}
/* 让 append 里的按钮不撑高 */
.search-input :deep(.el-input-group__append) {
  padding: 0;
}
/* 区分大小写按钮：Aa 文字，激活时高亮。
 * el-input small 高度 24px，按钮要与之等高、等宽（正方形）才不违和。 */
.case-btn {
  --el-button-size: 24px;
  width: 24px;
  height: 24px;
  min-height: 24px;
  padding: 0;
  margin: 0;
  font-weight: 700;
  font-size: 11px;
  letter-spacing: -0.5px;
  border: none;
  border-radius: 0;
  background: transparent;
  color: var(--el-text-color-secondary);
}
.case-btn:hover {
  color: var(--el-color-primary);
  background: var(--el-fill-color);
}
.case-btn.active {
  color: var(--el-color-primary);
  background: var(--el-color-primary-light-8);
}
.match-bar {
  padding: 4px 16px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.match-bar .mono {
  color: var(--el-color-primary);
  font-weight: 600;
}

/* ===== 设置面板 ===== */
.settings-panel {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.setting-row,
.setting-time {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.setting-time {
  flex-direction: column;
  align-items: stretch;
  gap: 4px;
}
.setting-label {
  font-size: 13px;
  color: var(--el-text-color-regular);
  white-space: nowrap;
}
.setting-divider {
  margin: 6px 0;
}
.tail-num {
  width: 130px;
}
.date-picker {
  width: 100% !important;
}
.apply-btn {
  margin-top: 4px;
  width: 100%;
}

/* ===== 日志区 ===== */
.log-view {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  font-size: 12.5px;
  line-height: 1.55;
  /* 深色终端风背景，让带色日志更醒目 */
  background: #1e1e1e;
}
.log-line {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  /* 深底用浅色文字，无色日志也清晰可读；带色日志由 ansi_up 自行着色 */
  color: #d4d4d4;
  font-family: inherit;
}
.empty-wait {
  color: var(--el-text-color-secondary);
  text-align: center;
  margin-top: 40px;
}
:deep(.hl) {
  background: var(--el-color-warning-light-7);
  color: var(--el-color-warning-dark-2);
  border-radius: 2px;
  padding: 0 1px;
}
</style>
