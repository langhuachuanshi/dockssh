<script setup lang="ts">
/**
 * 容器列表页（仅卡片视图）。
 *
 * 每容器一张横向卡片，展示状态/CPU/内存/网络/端口/操作。
 * 卡片订阅 stats 流事件，按 container_id 分发最新采样。
 * 详情为独立组件 ContainerDetail（真实 inspect + 监控曲线）。
 */
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import { useTerminalsStore } from '@/store/terminals'
import type { Container, StatsSample } from '@/api/types'
import ContainerCard from './ContainerCard.vue'
import ContainerDetail from './ContainerDetail.vue'

const route = useRoute()
const router = useRouter()
const store = useHostsStore()
const terminals = useTerminalsStore()

const hostId = computed(() => route.params.id as string)

const containers = ref<Container[]>([])
const loading = ref(false)
const search = ref('')
const onlyRunning = ref(false)

// 最新 stats 采样，按 container_id（若无则 name）索引
const statsMap = ref<Map<string, StatsSample>>(new Map())

// 网络速率差分：按容器保存上次累计字节 + 时间戳
interface NetPrev {
  rx: number
  tx: number
  ts: number
}
const netPrevMap = ref<Map<string, NetPrev>>(new Map())
// 每容器最近一次计算出的速率（字节/秒）
const netRateMap = ref<Map<string, { rx: number; tx: number }>>(new Map())

let pollTimer: number | null = null
let statsUnlisten: (() => void) | null = null
const STATS_INTERVAL = 2

/** 解析 docker stats 的网络字节数（如 "1.23kB" / "4.56MB" / "789B"）。
 * docker stats 用 IEC 二进制单位但写成 kB/MB 形式，按 1024 进制换算。 */
function parseSize(s: string): number {
  const m = (s || '').trim().match(/^([\d.]+)\s*(K|M|G|T)?i?B?/i)
  if (!m) return 0
  const val = parseFloat(m[1])
  if (isNaN(val)) return 0
  const unit = (m[2] || '').toUpperCase()
  const factor: Record<string, number> = {
    '': 1, K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4,
  }
  return val * (factor[unit] ?? 1)
}

const filtered = computed(() =>
  containers.value.filter((c) => {
    if (onlyRunning.value && c.state !== 'running') return false
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      c.name.toLowerCase().includes(kw) ||
      c.image.toLowerCase().includes(kw) ||
      c.id.toLowerCase().includes(kw)
    )
  }),
)

function statsOf(c: Container): StatsSample | null {
  return (
    statsMap.value.get(c.id) ||
    statsMap.value.get(c.name) ||
    statsMap.value.get(c.name.replace(/^\//, '')) ||
    // 兜底：c.id 可能是 12 位短 ID，statsMap key 是 64 位完整 ID，用前缀匹配
    findByShortId(statsMap.value, c.id)
  )
}

function netRateOf(c: Container): { rx: number; tx: number } | null {
  return (
    netRateMap.value.get(c.id) ||
    netRateMap.value.get(c.name) ||
    netRateMap.value.get(c.name.replace(/^\//, '')) ||
    findByShortId(netRateMap.value, c.id)
  )
}

/** 用短 ID（12 位）前缀匹配 statsMap 里的完整 ID（64 位） */
function findByShortId<V>(map: Map<string, V>, shortId: string): V | null {
  if (!shortId || shortId.length >= 32) return null
  for (const [k, v] of map) {
    if (k.startsWith(shortId) || shortId.startsWith(k)) return v
  }
  return null
}

async function refresh() {
  loading.value = true
  try {
    containers.value = await api.listContainers(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function action(kind: 'start' | 'stop' | 'restart', c: Container) {
  const actionName = kind === 'start' ? '启动' : kind === 'stop' ? '停止' : '重启'
  try {
    await ElMessageBox.confirm(`确认${actionName}容器「${c.name}」？`, '操作确认', {
      type: 'warning',
    })
  } catch {
    return
  }
  try {
    if (kind === 'start') await api.startContainer(hostId.value, c.id)
    else if (kind === 'stop') await api.stopContainer(hostId.value, c.id)
    else await api.restartContainer(hostId.value, c.id)
    ElMessage.success(`已${actionName}`)
    await refresh()
  } catch (e) {
    ElMessage.error(`操作失败：${e}`)
  }
}

function viewLogs(c: Container) {
  router.push({
    name: 'container-logs',
    params: { id: hostId.value, cid: c.id },
    query: { name: c.name },
  })
}

// 打开容器终端：复用语义——已开则激活，未开则新建（后端会话由 Terminal.vue 建立）
function openTerminal(c: Container) {
  terminals.open(hostId.value, c.id, c.name)
}

// 详情抽屉
const detailVisible = ref(false)
const detailContainer = ref<Container | null>(null)
function openDetail(c: Container) {
  detailContainer.value = c
  detailVisible.value = true
}

// 删除容器：二次确认，可选强制（运行中容器需 -f）
async function removeContainer(c: Container) {
  try {
    await ElMessageBox.confirm(
      `确认删除容器「${c.name}」？该操作不可恢复。`,
      '删除容器',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  // 运行中容器需强制删除
  const force = c.state === 'running'
  try {
    await api.removeContainer(hostId.value, c.id, force)
    ElMessage.success(`已删除「${c.name}」`)
    await refresh()
  } catch (e) {
    ElMessage.error(`删除失败：${e}`)
  }
}

// 打开容器目录：跳转到文件管理器，路径取首个 bind 挂载源，否则工作目录
async function openDir(c: Container) {
  try {
    const info = await api.inspectContainer(hostId.value, c.id)
    const bind = info.mounts.find((m) => m.typ === 'bind' && m.source)
    const path = bind?.source || info.working_dir
    if (!path) {
      ElMessage.info(`「${c.name}」无可定位的宿主机目录（无 bind 挂载/工作目录）`)
      return
    }
    router.push({ name: 'files', params: { id: hostId.value }, query: { path } })
  } catch (e) {
    ElMessage.error(`打开目录失败：${e}`)
  }
}

// ===== stats 订阅 =====
async function startStats() {
  try {
    await api.startStats(hostId.value, STATS_INTERVAL)
  } catch (e) {
    console.error('[stats] start_stats 失败:', e)
  }
  statsUnlisten = await api.onStats(hostId.value, (s) => {
    // 同时用 container_id 和 name 作 key 存，方便卡片两种方式查找
    // （docker ps 的 id 是 12 位短 ID，docker stats 的 container_id 是 64 位完整 ID，
    //  精确查可能对不上，statsOf 另做前缀匹配兜底）
    statsMap.value.set(s.container_id, s)
    if (s.name) statsMap.value.set(s.name, s)

    // 计算网络速率差分（统一用 container_id 作逻辑 key）
    const key = s.container_id
    const parts = (s.net_io || '').split('/')
    const rxNow = parts[0] ? parseSize(parts[0]) : 0
    const txNow = parts[1] ? parseSize(parts[1]) : 0
    const now = Date.now()
    const prev = netPrevMap.value.get(key)
    if (prev && now > prev.ts) {
      const dt = (now - prev.ts) / 1000
      const rxRate = Math.max(0, (rxNow - prev.rx) / dt)
      const txRate = Math.max(0, (txNow - prev.tx) / dt)
      netRateMap.value.set(key, { rx: rxRate, tx: txRate })
      if (s.name) netRateMap.value.set(s.name, { rx: rxRate, tx: txRate })
    }
    netPrevMap.value.set(key, { rx: rxNow, tx: txNow, ts: now })

    // 触发响应式：重新赋值 Map
    statsMap.value = new Map(statsMap.value)
    netRateMap.value = new Map(netRateMap.value)
    netPrevMap.value = new Map(netPrevMap.value)
  })
}

function stopStats() {
  statsUnlisten?.()
  statsUnlisten = null
  statsMap.value.clear()
  netPrevMap.value.clear()
  netRateMap.value.clear()
  api.stopStats(hostId.value).catch(() => {})
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  await refresh()
  await startStats()
  pollTimer = window.setInterval(refresh, 8000)
})
onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
  stopStats()
})
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <div class="flex gap-12 flex-center">
        <el-input
          v-model="search"
          placeholder="搜索名称/镜像/ID"
          :prefix-icon="Search"
          clearable
          style="width: 260px"
        />
        <el-checkbox v-model="onlyRunning">仅运行中</el-checkbox>
      </div>
      <div class="flex gap-12 flex-center">
        <el-button :icon="Refresh" @click="refresh">刷新</el-button>
      </div>
    </div>

    <!-- 卡片视图 -->
    <div class="card-wrap" v-loading="loading">
      <div v-if="filtered.length" class="card-grid">
        <ContainerCard
          v-for="c in filtered"
          :key="c.id"
          :container="c"
          :stats="statsOf(c)"
          :net-rate="netRateOf(c)"
          @action="action"
          @remove="removeContainer"
          @terminal="openTerminal"
          @logs="viewLogs"
          @detail="openDetail"
          @open-dir="openDir"
        />
      </div>
      <el-empty v-else description="没有容器" />
    </div>

    <!-- 详情抽屉 -->
    <ContainerDetail
      v-model="detailVisible"
      :host-id="hostId"
      :container="detailContainer"
    />
  </div>
</template>

<script lang="ts">
import { Search, Refresh } from '@element-plus/icons-vue'
export default {
  name: 'ContainerList',
  components: { Search, Refresh },
}
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  border-bottom: 1px solid var(--el-border-color);
}

/* 卡片视图 */
.card-wrap {
  flex: 1;
  padding: 16px 24px;
  overflow: auto;
}
.card-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
