<script setup lang="ts">
/**
 * 容器卡片组件（1Panel 行式卡片，一行一个，三行布局）。
 *
 *  左彩条 + 大号容器图标（与右侧 2 行信息垂直居中）
 *   第1行：名称 + ID(el-tag 可点复制) + 项目标签(无则隐藏)
 *   第2行：CPU / 内存 / 网络速率(差分) / 端口按钮(hover 弹菜单可点复制)
 *   第3行：操作按钮居右
 */
import { computed } from 'vue'
import { ElMessage } from 'element-plus'
import { FolderOpened } from '@element-plus/icons-vue'
import type { Container, StatsSample } from '@/api/types'
import ContainerIcon from './ContainerIcon.vue'

const props = defineProps<{
  container: Container
  stats: StatsSample | null
  /** 网络速率（字节/秒），由父组件差分计算后传入 */
  netRate?: { rx: number; tx: number } | null
}>()

const emit = defineEmits<{
  (e: 'action', kind: 'start' | 'stop' | 'restart', c: Container): void
  (e: 'terminal', c: Container): void
  (e: 'logs', c: Container): void
  (e: 'detail', c: Container): void
  (e: 'openDir', c: Container): void
}>()

const isRunning = computed(() => props.container.state === 'running')

// ===== 状态样式 =====
interface StateStyle {
  label: string
  color: string
  bar: string
}
const stateStyle = computed<StateStyle>(() => {
  const s = props.container.state
  if (s === 'running')
    return { label: '运行中', color: 'var(--el-color-success)', bar: '#67c23a' }
  if (s === 'exited')
    return { label: '已停止', color: 'var(--el-text-color-secondary)', bar: '#909399' }
  if (s === 'paused')
    return { label: '已暂停', color: 'var(--el-color-warning)', bar: '#e6a23c' }
  if (s === 'restarting')
    return { label: '重启中', color: 'var(--el-color-warning)', bar: '#e6a23c' }
  if (s === 'created')
    return { label: '已创建', color: 'var(--el-color-info)', bar: '#909399' }
  return { label: s, color: 'var(--el-text-color-secondary)', bar: '#909399' }
})

// ===== 指标 =====
const cpuPct = computed(() => props.stats?.cpu_percent ?? 0)
const memPct = computed(() => props.stats?.mem_percent ?? 0)
const memUsed = computed(() => {
  const u = props.stats?.mem_usage || ''
  return u.split('/')[0]?.trim() || '—'
})

// 网络速率格式化：字节/秒 → B/s KB/s MB/s
function formatRate(bps: number): string {
  if (!isFinite(bps) || bps < 0) bps = 0
  if (bps < 1000) return `${bps.toFixed(0)} B/s`
  const units = ['KB/s', 'MB/s', 'GB/s']
  let i = -1
  let v = bps
  do { v /= 1000; i++ } while (v >= 1000 && i < units.length - 1)
  return `${v.toFixed(2)} ${units[i]}`
}
const netRxRate = computed(() => props.netRate?.rx ?? 0)
const netTxRate = computed(() => props.netRate?.tx ?? 0)
const hasNetRate = computed(() => !!props.netRate)

function cpuColor(p: number): string {
  if (p >= 85) return 'var(--el-color-danger)'
  if (p >= 60) return 'var(--el-color-warning)'
  return 'var(--el-color-success)'
}

// ===== 点击复制 =====
async function copyText(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success(`已复制${label}`)
  } catch {
    ElMessage.info(text)
  }
}
const shortId = computed(() => props.container.id.replace('sha256:', '').slice(0, 12))
function copyName() { copyText(props.container.name, '名称') }
function copyId() { copyText(props.container.id, 'ID') }

// ===== 端口 =====
interface PortItem {
  host: string
  container: string
  hostExposed: boolean
}
const portItems = computed<PortItem[]>(() => {
  return (props.container.ports || []).map((raw) => {
    const s = String(raw).trim()
    // 形如 "0.0.0.0:8080->80/tcp" / "[::1]:8080->80/tcp" / "8080->80/tcp" / "80/tcp"
    const m = s.match(/^(?:(?:\[[^\]]+\]|[^:]+):)?(\d+)\s*->\s*(\d+).*$/)
    const m2 = s.match(/^(\d+)\/.*$/)
    if (m) {
      return { host: m[1], container: m[2], hostExposed: true }
    }
    if (m2) return { host: '', container: m2[1], hostExposed: false }
    return { host: '', container: s, hostExposed: false }
  })
})
const hasPorts = computed(() => portItems.value.length > 0)

function onAction(kind: 'start' | 'stop' | 'restart') {
  emit('action', kind, props.container)
}
</script>

<template>
  <div class="ctr-card" :class="{ running: isRunning }">
    <!-- 左侧状态彩条 -->
    <div class="state-bar" :style="{ background: stateStyle.bar }" />

    <div class="card-inner">
      <!-- 容器图标 + 信息(2行) -->
      <div class="head-block">
        <ContainerIcon :image="container.image" :size="40" class="ctr-icon" />
        <div class="info">
          <!-- 第1行：名称 + ID + 项目 -->
          <div class="row-info">
            <span class="name" @click="copyName" title="点击复制名称">{{ container.name }}</span>
            <el-tag
              size="small"
              type="info"
              effect="plain"
              class="id-tag mono"
              @click="copyId"
            >{{ shortId }}</el-tag>
            <el-tag
              v-if="container.compose_project"
              size="small"
              type="warning"
              effect="plain"
            >项目: {{ container.compose_project }}</el-tag>
            <span class="state-text" :style="{ color: stateStyle.color }">{{ stateStyle.label }}</span>
          </div>
          <!-- 第2行：监控 -->
          <div class="row-metrics">
            <div class="metric">
              <span class="metric-label">CPU</span>
              <span class="metric-val" :class="{ dim: !stats }">
                {{ stats ? `${cpuPct.toFixed(1)}%` : '—' }}
              </span>
              <div class="bar">
                <span
                  class="bar-fill"
                  v-if="stats"
                  :style="{ width: `${Math.min(cpuPct, 100)}%`, background: cpuColor(cpuPct) }"
                />
              </div>
            </div>
            <div class="metric">
              <span class="metric-label">内存</span>
              <span class="metric-val" :class="{ dim: !stats }">
                {{ stats ? `${memUsed} / ${memPct.toFixed(0)}%` : '—' }}
              </span>
              <div class="bar">
                <span
                  class="bar-fill"
                  v-if="stats"
                  :style="{ width: `${Math.min(memPct, 100)}%`, background: cpuColor(memPct) }"
                />
              </div>
            </div>
            <div class="metric">
              <span class="metric-label">网络</span>
              <span class="metric-val net-val" v-if="hasNetRate">
                <span class="tx">↑{{ formatRate(netTxRate) }}</span>
                <span class="rx">↓{{ formatRate(netRxRate) }}</span>
              </span>
              <span class="metric-val dim" v-else>—</span>
            </div>
            <!-- 端口按钮 -->
            <el-tooltip
              v-if="hasPorts"
              placement="top"
              effect="light"
              :show-after="120"
              :hide-after="0"
            >
              <template #content>
                <div class="port-pop">
                  <div
                    v-for="(p, i) in portItems"
                    :key="i"
                    class="port-pop-item mono"
                  >
                    <template v-if="p.hostExposed">
                      <span class="port-pop-host">{{ p.host }}</span>
                      <span class="port-pop-arrow">→</span>
                      <span class="port-pop-c">{{ p.container }}</span>
                    </template>
                    <template v-else>
                      <span class="port-pop-c">{{ p.container }}</span>
                    </template>
                  </div>
                </div>
              </template>
              <el-button size="small" text :icon="Connection" class="port-btn">
                {{ portItems.length }}
              </el-button>
            </el-tooltip>
          </div>
        </div>
      </div>

      <!-- 第3行：操作 -->
      <div class="row-actions">
        <template v-if="isRunning">
          <el-button size="small" :icon="VideoPause" @click="onAction('stop')">停止</el-button>
          <el-button size="small" :icon="RefreshRight" @click="onAction('restart')">重启</el-button>
          <el-button size="small" :icon="Monitor" @click="emit('terminal', container)">终端</el-button>
          <el-button size="small" :icon="Document" @click="emit('logs', container)">日志</el-button>
          <el-button size="small" :icon="FolderOpened" @click="emit('openDir', container)">目录</el-button>
        </template>
        <template v-else>
          <el-button size="small" type="primary" :icon="VideoPlay" @click="onAction('start')">启动</el-button>
          <el-button size="small" :icon="Document" @click="emit('logs', container)">日志</el-button>
          <el-button size="small" :icon="FolderOpened" @click="emit('openDir', container)">目录</el-button>
        </template>
        <el-button size="small" text :icon="MoreFilled" @click="emit('detail', container)" class="more-btn" />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  VideoPlay, VideoPause, RefreshRight, Monitor, Document, MoreFilled, Connection, Box,
} from '@element-plus/icons-vue'
export default {
  components: { VideoPlay, VideoPause, RefreshRight, Monitor, Document, MoreFilled, Connection, Box },
}
</script>

<style scoped>
.ctr-card {
  position: relative;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  transition: transform 0.18s ease, box-shadow 0.18s ease;
}
/* hover 交互：彩条加宽变亮 + 整卡轻微抬升 + 操作区填充，不动边框色 */
.ctr-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
}
.ctr-card:hover .state-bar {
  width: 5px;
  filter: saturate(1.3) brightness(1.1);
}
.ctr-card:hover .row-actions {
  background: var(--el-fill-color-light);
}

/* 左侧状态彩条 */
.state-bar {
  width: 3px;
  flex-shrink: 0;
  transition: width 0.18s ease, filter 0.18s ease;
}

.card-inner {
  flex: 1;
  min-width: 0;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* ===== 头部：图标 + 2行信息 垂直居中 ===== */
.head-block {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}
.ctr-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

/* 第1行 信息 */
.row-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
}
.name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 280px;
}
.name:hover { color: var(--el-color-primary); }
.id-tag {
  cursor: pointer;
  font-size: 11px;
}
.id-tag:hover {
  color: var(--el-color-primary);
  border-color: var(--el-color-primary);
}
.state-text {
  font-size: 12px;
  font-weight: 600;
  margin-left: auto;
  flex-shrink: 0;
}

/* 第2行 监控 */
.row-metrics {
  display: flex;
  align-items: center;
  gap: 18px;
  flex-wrap: wrap;
  min-width: 0;
}
.metric {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.metric-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}
.metric-val {
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  white-space: nowrap;
}
.metric-val.dim {
  color: var(--el-text-color-secondary);
  font-weight: 400;
}
.bar {
  width: 60px;
  height: 4px;
  background: var(--el-fill-color-dark);
  border-radius: 2px;
  overflow: hidden;
  flex-shrink: 0;
}
.bar-fill {
  display: block;
  height: 100%;
  transition: width 0.4s ease;
}
.net-val {
  display: inline-flex;
  gap: 8px;
}
.net-val .rx { color: #1d9bf0; }
.net-val .tx { color: #39d0d8; }

/* 端口按钮 */
.port-btn {
  margin-left: auto;
  color: var(--el-text-color-secondary);
  font-weight: 600;
  padding: 4px 8px;
}
.port-btn:hover { color: var(--el-color-primary); }

/* 第3行 操作 */
.row-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  /* 左右各延伸 padding，让 hover 填充贴合卡片边缘 */
  margin: 4px -16px 0;
  padding: 10px 16px;
  border-top: 1px solid var(--el-border-color-lighter);
  transition: background 0.18s ease;
}
.more-btn { color: var(--el-text-color-secondary); }

/* 端口弹出层 */
.port-pop { min-width: 160px; }
.port-pop-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  font-size: 12px;
}
.port-pop-host { color: var(--el-text-color-primary); font-weight: 600; }
.port-pop-arrow { color: var(--el-text-color-placeholder); }
.port-pop-c { color: var(--el-color-primary); }
</style>
