<script setup lang="ts">
/**
 * 容器列表页（仅卡片视图）。
 *
 * 每容器一张横向卡片，展示状态/CPU/内存/网络/端口/操作。
 * 卡片订阅 stats 流事件，按 container_id 分发最新采样。
 * 详情为独立组件 ContainerDetail（真实 inspect + 监控曲线）。
 */
import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref } from 'vue'
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
let statsRawUnlisten: (() => void) | null = null
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

/** 把 docker 的 CreatedAt 字符串解析成可比较的毫秒时间戳。
 * docker 输出形如 "2024-01-01 12:00:00 +0800 CST"，去掉 CST 后用 Date 解析。 */
function parseCreated(s: string): number {
  if (!s) return 0
  const cleaned = s.replace(/\s*CST\s*$/i, '').trim()
  const t = Date.parse(cleaned)
  return isNaN(t) ? 0 : t
}

const filtered = computed(() => {
  const result = containers.value.filter((c) => {
    if (onlyRunning.value && c.state !== 'running') return false
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      c.name.toLowerCase().includes(kw) ||
      c.image.toLowerCase().includes(kw) ||
      c.id.toLowerCase().includes(kw)
    )
  })
  // 按创建时间排序：新创建的在前（倒序）
  result.sort((a, b) => parseCreated(b.created) - parseCreated(a.created))
  return result
})

function statsOf(c: Container): StatsSample | null {
  // container_id 已归一化为 12 位短 ID，与 docker ps 的 c.id 一致，可直接精确匹配；
  // 兜底：name（去前导斜杠）+ 完整 ID 前缀匹配
  const nameNoSlash = c.name.replace(/^\//, '')
  const result =
    statsMap.value.get(c.id) ||
    statsMap.value.get(c.name) ||
    statsMap.value.get(nameNoSlash) ||
    findByShortId(statsMap.value, c.id)
  return result
}

function netRateOf(c: Container): { rx: number; tx: number } | null {
  return (
    netRateMap.value.get(c.id) ||
    netRateMap.value.get(c.name) ||
    netRateMap.value.get(c.name.replace(/^\//, '')) ||
    findByShortId(netRateMap.value, c.id)
  )
}

/** 兜底前缀匹配（key 已归一化为 12 位短 ID，正常无需走到这里）。 */
function findByShortId<V>(map: Map<string, V>, shortId: string): V | null {
  if (!shortId) return null
  for (const [k, v] of map) {
    if (k.length >= 12 && /^[0-9a-f]+$/i.test(k) && (k.startsWith(shortId) || shortId.startsWith(k))) {
      return v
    }
  }
  return null
}

async function refresh() {
  if (!hostId.value) return
  loading.value = true
  try {
    containers.value = await api.listContainers(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function action(kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause', c: Container) {
  const names: Record<string, string> = {
    start: '启动', stop: '停止', restart: '重启', pause: '暂停', unpause: '恢复',
  }
  const actionName = names[kind]
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
    else if (kind === 'restart') await api.restartContainer(hostId.value, c.id)
    else if (kind === 'pause') await api.pauseContainer(hostId.value, c.id)
    else if (kind === 'unpause') await api.unpauseContainer(hostId.value, c.id)
    ElMessage.success(`已${actionName}`)
    await refresh()
  } catch (e) {
    ElMessage.error(`操作失败：${e}`)
  }
}

/** 重命名容器：用 prompt 收集新名字 → rename_container → refresh */
async function renameContainer(c: Container) {
  let newName = ''
  try {
    const res = await ElMessageBox.prompt('请输入新的容器名称', `重命名「${c.name}」`, {
      inputValue: c.name,
      inputPlaceholder: '只能包含字母、数字、下划线、连字符、点',
      inputValidator: (v: string) => {
        const t = (v || '').trim()
        if (!t) return '名称不能为空'
        if (!/^[a-zA-Z0-9_.-]+$/.test(t)) return '只能包含字母、数字、下划线、连字符、点'
        return true
      },
    })
    newName = res.value.trim()
  } catch {
    return
  }
  try {
    await api.renameContainer(hostId.value, c.id, newName)
    ElMessage.success(`已重命名为「${newName}」`)
    await refresh()
  } catch (e) {
    ElMessage.error(`重命名失败：${e}`)
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

/** 处理一个 stats 采样：归一化短 ID、存入 map、计算网络速率差分。
 * 被 onStats（解析好的）和 onStatsRaw（原始行自行解析的）共用。 */
function handleSample(s: StatsSample) {
  // 归一化 key：后端可能返回 12 位短 ID（新版）或 64 位完整 ID（旧版），
  // 统一存短 ID（取前 12 位）+ name 两套 key，确保和 docker ps 的 c.id 精确匹配。
  const shortId = s.container_id.length > 12 ? s.container_id.slice(0, 12) : s.container_id
  const sample = { ...s, container_id: shortId }
  statsMap.value.set(shortId, sample)
  if (s.name) statsMap.value.set(s.name, sample)

  // 计算网络速率差分（统一用短 ID 作逻辑 key）
  const key = shortId
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
}

async function startStats() {
  // 先挂监听再启动流：docker stats 打开瞬间会突发推送第一批采样，
  // 若监听器还没注册，Tauri 事件会被丢弃 → 开头几秒无数据。
  statsUnlisten = await api.onStats(hostId.value, (s) => {
    handleSample(s)
  })

  // 兜底：监听原始行，自行解析。旧后端每批第一行带 \x1b[H 控制码导致 parse_line 失败，
  // 前端补解析，确保所有容器（尤其第一个）都能进 statsMap。
  statsRawUnlisten = await api.onStatsRaw(hostId.value, (line) => {
    const brace = line.indexOf('{')
    if (brace < 0) return
    const json = line.slice(brace)
    try {
      const row = JSON.parse(json)
      handleSample({
        container_id: row.ID || row.Container || '',
        name: row.Name || '',
        cpu_percent: parseFloat((row.CPUPerc || '0').replace('%', '')) || 0,
        mem_usage: row.MemUsage || '',
        mem_percent: parseFloat((row.MemPerc || '0').replace('%', '')) || 0,
        net_io: row.NetIO || '',
        block_io: row.BlockIO || '',
        pids: parseInt(row.PIDs || '0', 10) || 0,
      })
    } catch {
      /* 非 JSON 行（表头/空行），忽略 */
    }
  })

  try {
    await api.startStats(hostId.value, STATS_INTERVAL)
  } catch (e) {
    console.error('[stats] start_stats 失败:', e)
  }
}

function stopStats() {
  statsUnlisten?.()
  statsUnlisten = null
  statsRawUnlisten?.()
  statsRawUnlisten = null
  statsMap.value.clear()
  netPrevMap.value.clear()
  netRateMap.value.clear()
  if (hostId.value) api.stopStats(hostId.value).catch(() => {})
}

let inited = false
onMounted(async () => {
  // 守卫：keep-alive 场景下若路由 param 尚未就绪，跳过初始化
  if (!hostId.value) {
    console.warn('[containers] hostId 为空，跳过初始化')
    return
  }
  await store.ensureConnected(hostId.value)
  await refresh()
  await startStats()
  pollTimer = window.setInterval(refresh, 8000)
  inited = true
})

// keep-alive 激活/失活：恢复/暂停 stats 流和轮询，避免后台空跑 + hostId 漂移报错
onActivated(() => {
  if (!inited) return
  if (hostId.value) {
    startStats()
    if (!pollTimer) pollTimer = window.setInterval(refresh, 8000)
  }
})

onDeactivated(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  stopStats()
})

onBeforeUnmount(() => {
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
          @rename="renameContainer"
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
      @action="action"
      @rename="renameContainer"
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
