<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Container, Image } from '@/api/types'

echarts.use([LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const containers = ref<Container[]>([])
const images = ref<Image[]>([])
const loading = ref(false)

const probe = computed(() => store.probeMap[hostId.value])
const runningCount = computed(() => containers.value.filter((c) => c.state === 'running').length)

// stats 图表
const chartEl = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null
const cpuHistory: number[] = []
const memHistory: number[] = []
const labels: string[] = []
const MAX_POINTS = 30

async function loadAll() {
  loading.value = true
  try {
    const [c, i] = await Promise.all([
      api.listContainers(hostId.value),
      api.listImages(hostId.value),
    ])
    containers.value = c
    images.value = i
  } finally {
    loading.value = false
  }
}

function initChart() {
  if (!chartEl.value) return
  chart = echarts.init(chartEl.value, 'dark')
  chart.setOption({
    backgroundColor: 'transparent',
    grid: { left: 48, right: 48, top: 32, bottom: 28 },
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU %', '内存 %'], textStyle: { color: '#8b949e' } },
    xAxis: {
      type: 'category',
      data: [],
      axisLabel: { color: '#8b949e', fontSize: 10 },
      axisLine: { lineStyle: { color: '#30363d' } },
    },
    yAxis: [
      {
        type: 'value',
        name: 'CPU',
        axisLabel: { color: '#8b949e', fontSize: 10 },
        splitLine: { lineStyle: { color: '#30363d' } },
      },
      {
        type: 'value',
        name: '内存',
        axisLabel: { color: '#8b949e', fontSize: 10 },
        splitLine: { show: false },
      },
    ],
    series: [
      {
        name: 'CPU %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        data: [],
        itemStyle: { color: '#1d9bf0' },
        areaStyle: { opacity: 0.15 },
      },
      {
        name: '内存 %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        data: [],
        itemStyle: { color: '#39d0d8' },
        yAxisIndex: 1,
      },
    ],
  })
}

let unlisten: (() => void) | null = null
let pollTimer: number | null = null

async function startStats() {
  await api.startStats(hostId.value, 2)
  unlisten = await api.onStats(hostId.value, (s) => {
    // 汇总：取所有容器 CPU/内存百分比之和作为整机占用近似
    cpuHistory.push(Number((s.cpu_percent).toFixed(2)))
    memHistory.push(Number((s.mem_percent).toFixed(2)))
    labels.push(new Date().toLocaleTimeString())
    if (cpuHistory.length > MAX_POINTS) {
      cpuHistory.shift()
      memHistory.shift()
      labels.shift()
    }
    chart?.setOption({
      xAxis: { data: labels },
      series: [{ data: cpuHistory }, { data: memHistory }],
    })
  })
}

function stopStats() {
  unlisten?.()
  unlisten = null
  api.stopStats(hostId.value).catch(() => {})
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  await loadAll()
  initChart()
  await startStats()
  // 容器列表轮询刷新（反映启停）
  pollTimer = window.setInterval(loadAll, 8000)
})

onUnmounted(() => {
  stopStats()
  if (pollTimer) clearInterval(pollTimer)
  chart?.dispose()
})
</script>

<template>
  <div class="page" v-loading="loading">
    <!-- 主机信息卡 -->
    <el-card shadow="never" v-if="probe">
      <el-descriptions :column="1" border>
        <el-descriptions-item label="主机名">
          <span class="mono">{{ probe.hostname }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="操作系统">
          <span class="mono">{{ probe.os }}</span>
          <el-tag v-if="probe.is_wsl2" size="small" type="info" effect="dark" style="margin-left:8px">WSL2</el-tag>
          <el-tag v-if="probe.is_windows_native" size="small" type="warning" effect="dark" style="margin-left:8px">Windows 原生容器</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="Docker 版本">
          <span class="mono">{{ probe.docker_version || '未知' }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="Compose">
          <el-tag size="small" :type="probe.has_compose ? 'success' : 'danger'" effect="dark">
            {{ probe.has_compose ? '已安装 v2' : '未安装' }}
          </el-tag>
        </el-descriptions-item>
      </el-descriptions>
    </el-card>

    <!-- 数字概览 -->
    <el-row :gutter="16">
      <el-col :span="8">
        <el-card shadow="never" class="stat-card">
          <div class="stat-num">{{ containers.length }}</div>
          <div class="stat-label">容器总数</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="never" class="stat-card">
          <div class="stat-num success">{{ runningCount }}</div>
          <div class="stat-label">运行中</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="never" class="stat-card">
          <div class="stat-num">{{ images.length }}</div>
          <div class="stat-label">镜像</div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 实时图表 -->
    <el-card shadow="never" class="chart-card">
      <template #header>实时资源占用（所有容器汇总）</template>
      <div ref="chartEl" class="chart" />
    </el-card>
  </div>
</template>

<style scoped>
.page {
  padding: 20px 24px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1;
}
.stat-card {
  text-align: center;
}
.stat-num {
  font-size: 32px;
  font-weight: 600;
  color: var(--el-color-primary);
}
.stat-num.success {
  color: var(--el-color-success);
}
.stat-label {
  color: var(--el-text-color-secondary);
  font-size: 13px;
  margin-top: 4px;
}
.chart-card {
  flex: 1;
  min-height: 280px;
  display: flex;
  flex-direction: column;
}
.chart {
  height: 240px;
}
</style>
