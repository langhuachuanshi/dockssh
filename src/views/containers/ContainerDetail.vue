<script setup lang="ts">
/**
 * 容器详情抽屉。
 *
 * 内容分两段：
 *   1) 顶部：实时监控曲线（CPU + 内存 mini 折线图），复用 stats 流
 *   2) 下方：docker inspect 真实信息（基础 / Config / State / 网络 / 主机配置 / 挂载 / 环境变量）
 *
 * 抽屉打开时拉 inspect + 订阅 stats；关闭/切容器时清理。
 */
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import type { Container, ContainerInspect } from '@/api/types'

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer])

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    /** 主机 id；keep-alive 切换瞬间可能为 undefined，组件内部做了守卫 */
    hostId?: string
    container: Container | null
  }>(),
  { hostId: '' },
)

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'action', kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause', c: Container): void
  (e: 'rename', c: Container): void
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
})

const loading = ref(false)
const inspect = ref<ContainerInspect | null>(null)

// ===== 监控曲线 =====
const chartRef = ref<HTMLDivElement | null>(null)
let chart: echarts.ECharts | null = null
const cpuHistory = ref<number[]>([])
const memHistory = ref<number[]>([])
const MAX_POINTS = 60

let statsUnlisten: (() => void) | null = null

function ensureChart() {
  if (!chartRef.value) return
  if (chart) return
  chart = echarts.init(chartRef.value, 'dark')
  chart.setOption({
    grid: { left: 36, right: 36, top: 28, bottom: 24 },
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU %', '内存 %'], top: 0, textStyle: { color: '#ccc' } },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      show: false,
      data: [],
    },
    yAxis: [
      { type: 'value', max: 100, splitLine: { lineStyle: { color: '#333' } }, axisLabel: { color: '#888' } },
      { type: 'value', max: 100, splitLine: { show: false }, axisLabel: { color: '#888' } },
    ],
    series: [
      {
        name: 'CPU %',
        type: 'line',
        smooth: true,
        symbol: 'none',
        yAxisIndex: 0,
        data: [],
        lineStyle: { color: '#1d9bf0', width: 2 },
        areaStyle: { color: 'rgba(29,155,240,0.18)' },
      },
      {
        name: '内存 %',
        type: 'line',
        smooth: true,
        symbol: 'none',
        yAxisIndex: 1,
        data: [],
        lineStyle: { color: '#67c23a', width: 2 },
        areaStyle: { color: 'rgba(103,194,58,0.18)' },
      },
    ],
  })
}

function pushPoint(cpu: number, mem: number) {
  cpuHistory.value.push(cpu)
  memHistory.value.push(mem)
  if (cpuHistory.value.length > MAX_POINTS) cpuHistory.value.shift()
  if (memHistory.value.length > MAX_POINTS) memHistory.value.shift()
  chart?.setOption({
    xAxis: { data: cpuHistory.value.map((_, i) => i) },
    series: [{ data: cpuHistory.value }, { data: memHistory.value }],
  })
}

async function loadInspect() {
  if (!props.container || !props.hostId) return
  loading.value = true
  try {
    inspect.value = await api.inspectContainer(props.hostId, props.container.id)
  } catch (e) {
    ElMessage.error(`加载详情失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function startListen() {
  if (!props.hostId) return
  // 注意：不调 startStats —— 后端 stats 流是「每主机单实例」，
  // 由容器列表页统一管理生命周期。详情只监听已存在的流事件，
  // 避免关闭详情时 stopStats 把列表的流也停了。
  statsUnlisten = await api.onStats(props.hostId, (s) => {
    if (!props.container) return
    if (s.container_id !== props.container.id && s.name !== props.container.name) return
    pushPoint(s.cpu_percent, s.mem_percent)
  })
}

function stopListen() {
  statsUnlisten?.()
  statsUnlisten = null
  // 不调 stopStats，由列表页管理
}

function reset() {
  inspect.value = null
  cpuHistory.value = []
  memHistory.value = []
  chart?.setOption({ series: [{ data: [] }, { data: [] }] })
}

// 抽屉打开 / 切容器时重新加载
watch(
  () => [props.modelValue, props.container?.id],
  async ([open, _id]) => {
    if (!open) {
      stopListen()
      return
    }
    reset()
    await loadInspect()
    await nextTick()
    ensureChart()
    await startListen()
  },
  { immediate: true },
)

// 抽屉尺寸变化时 resize 图表
let resizeObs: ResizeObserver | null = null
watch(chartRef, (el) => {
  resizeObs?.disconnect()
  if (!el) return
  resizeObs = new ResizeObserver(() => chart?.resize())
  resizeObs.observe(el)
})

onBeforeUnmount(() => {
  resizeObs?.disconnect()
  stopListen()
  chart?.dispose()
  chart = null
})

// ===== 展示辅助 =====
const stateColor = computed(() => {
  const s = inspect.value?.state || props.container?.state || ''
  if (s === 'running') return 'var(--el-color-success)'
  if (s === 'exited') return 'var(--el-text-color-secondary)'
  if (s === 'paused') return 'var(--el-color-warning)'
  if (s === 'restarting') return 'var(--el-color-warning)'
  return 'var(--el-text-color-secondary)'
})

const portBindings = computed(() => inspect.value?.port_bindings || [])
const mounts = computed(() => inspect.value?.mounts || [])
const envList = computed(() => inspect.value?.env || [])
const entrypoint = computed(() => inspect.value?.entrypoint.filter(Boolean).join(' ') || '—')
const cmd = computed(() => inspect.value?.cmd.filter(Boolean).join(' ') || '—')

function onAction(kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause') {
  if (props.container) emit('action', kind, props.container)
}
function onRename() {
  if (props.container) emit('rename', props.container)
}

const isRunning = computed(() => props.container?.state === 'running')
const isPaused = computed(() => props.container?.state === 'paused')
</script>

<script lang="ts">
import { VideoPlay, VideoPause, RefreshRight, EditPen } from '@element-plus/icons-vue'
export default {
  components: { VideoPlay, VideoPause, RefreshRight, EditPen },
}
</script>

<template>
  <el-drawer v-model="visible" size="50%">
    <template #header>
      <div class="detail-header">
        <span class="detail-title">{{ container?.name || '容器详情' }}</span>
        <div class="detail-actions">
          <template v-if="isRunning">
            <el-button size="small" :icon="VideoPause" @click="onAction('pause')">暂停</el-button>
            <el-button size="small" :icon="RefreshRight" @click="onAction('restart')">重启</el-button>
            <el-button size="small" :icon="VideoPlay" @click="onAction('stop')">停止</el-button>
          </template>
          <template v-else-if="isPaused">
            <el-button size="small" type="primary" :icon="VideoPlay" @click="onAction('unpause')">恢复</el-button>
          </template>
          <template v-else>
            <el-button size="small" type="primary" :icon="VideoPlay" @click="onAction('start')">启动</el-button>
          </template>
          <el-button size="small" :icon="EditPen" @click="onRename">重命名</el-button>
        </div>
      </div>
    </template>
    <div class="detail-wrap" v-if="container" v-loading="loading">
      <!-- 实时监控曲线 -->
      <div class="section">
        <div class="section-title">实时监控</div>
        <div ref="chartRef" class="chart" />
        <div class="chart-hint" v-if="!cpuHistory.length">等待数据中…（容器需运行）</div>
      </div>

      <!-- 基础信息 -->
      <div class="section">
        <div class="section-title">基础信息</div>
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="名称">{{ container.name }}</el-descriptions-item>
          <el-descriptions-item label="ID">
            <span class="mono">{{ inspect?.id || container.id }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="镜像">
            <span class="mono">{{ container.image }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="状态">
            <span class="state-tag" :style="{ color: stateColor }">
              {{ inspect?.status || container.status || container.state }}
            </span>
            <span class="dim" v-if="inspect && inspect.state !== 'running' && inspect.exit_code !== 0">
              · 退出码 {{ inspect.exit_code }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="PID" v-if="inspect && inspect.pid">
            {{ inspect.pid }}
          </el-descriptions-item>
          <el-descriptions-item label="创建时间" v-if="inspect?.created">
            {{ inspect.created }}
          </el-descriptions-item>
          <el-descriptions-item label="启动时间" v-if="inspect?.started_at">
            {{ inspect.started_at }}
          </el-descriptions-item>
          <el-descriptions-item label="结束时间" v-if="inspect?.finished_at && inspect.state !== 'running'">
            {{ inspect.finished_at }}
          </el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- Config -->
      <div class="section">
        <div class="section-title">配置</div>
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="入口点">
            <span class="mono">{{ entrypoint }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="命令">
            <span class="mono">{{ cmd }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="工作目录">
            <span class="mono">{{ inspect?.working_dir || '—' }}</span>
          </el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 网络 -->
      <div class="section" v-if="inspect">
        <div class="section-title">网络</div>
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="所属网络">
            <el-tag v-for="n in inspect.networks" :key="n" size="small" type="info" effect="plain" class="tag-gap">
              {{ n }}
            </el-tag>
            <span v-if="!inspect.networks.length" class="dim">—</span>
          </el-descriptions-item>
          <el-descriptions-item label="IP 地址">
            <span class="mono">{{ inspect.ip_address || '—' }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="网关">
            <span class="mono">{{ inspect.gateway || '—' }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="MAC 地址">
            <span class="mono">{{ inspect.mac_address || '—' }}</span>
          </el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 端口映射 -->
      <div class="section" v-if="portBindings.length">
        <div class="section-title">端口映射</div>
        <div class="port-list">
          <div v-for="(p, i) in portBindings" :key="i" class="port-row mono">
            <span class="dim">{{ p.host_ip || '0.0.0.0' }}:{{ p.host_port }}</span>
            <span class="arrow">→</span>
            <span>{{ p.container_port }}</span>
          </div>
        </div>
      </div>

      <!-- 重启策略 -->
      <div class="section" v-if="inspect">
        <div class="section-title">主机配置</div>
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="重启策略">
            {{ inspect.restart_policy || '—' }}
            <span class="dim" v-if="inspect.restart_policy === 'on-failure' && inspect.restart_retries">
              · 最多 {{ inspect.restart_retries }} 次
            </span>
          </el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 挂载 -->
      <div class="section" v-if="mounts.length">
        <div class="section-title">挂载 ({{ mounts.length }})</div>
        <div class="mount-list">
          <div v-for="(m, i) in mounts" :key="i" class="mount-row">
            <el-tag size="small" :type="m.typ === 'bind' ? 'warning' : 'info'" effect="plain">
              {{ m.typ }}
            </el-tag>
            <span class="mono mount-path">
              <span class="dim">{{ m.source || '(anon)' }}</span>
              <span class="arrow">→</span>
              <span>{{ m.destination }}</span>
            </span>
          </div>
        </div>
      </div>

      <!-- 环境变量 -->
      <div class="section" v-if="envList.length">
        <div class="section-title">环境变量 ({{ envList.length }})</div>
        <div class="env-list">
          <div v-for="(e, i) in envList" :key="i" class="env-row mono">
            <span class="env-key">{{ e.split('=')[0] }}</span>
            <span class="env-val">={{ e.split('=').slice(1).join('=') }}</span>
          </div>
        </div>
      </div>
    </div>
  </el-drawer>
</template>

<style scoped>
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
}
.detail-title {
  font-weight: 600;
  font-size: 15px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.detail-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.detail-wrap {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 4px 0 24px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 8px;
  padding-left: 8px;
  border-left: 3px solid var(--el-color-primary);
}
.chart {
  width: 100%;
  height: 160px;
  background: #1e1e1e;
  border-radius: 6px;
  padding: 6px;
  box-sizing: border-box;
}
.chart-hint {
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin-top: 6px;
}

.mono {
  font-family: 'Cascadia Code', 'Fira Code', Consolas, monospace;
  font-size: 12px;
  word-break: break-all;
}
.dim { color: var(--el-text-color-secondary); }
.state-tag { font-weight: 600; }
.tag-gap { margin-right: 4px; }
.arrow { color: var(--el-text-color-placeholder); margin: 0 6px; }

.port-list,
.mount-list,
.env-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.port-row,
.mount-row,
.env-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  font-size: 12px;
  flex-wrap: wrap;
}
.mount-row { gap: 8px; }
.mount-path { flex: 1; min-width: 0; word-break: break-all; }
.env-key { color: var(--el-color-primary); font-weight: 600; }
.env-val { color: var(--el-text-color-regular); word-break: break-all; }
</style>
