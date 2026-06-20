<script setup lang="ts">
/**
 * 容器卡片组件（紧凑两行布局）。
 *
 *  左彩条 + 容器图标（仅第1行高度）
 *   第1行：图标 + 名称 + ID + 项目 + 端口按钮
 *   第2行：监控(CPU/内存/网络 或 状态文案) 左对齐 + 操作按钮 右对齐（两端对齐）
 *
 * 运行中容器显示 CPU/内存/网络；非运行中容器在监控区显示状态文案（如"已创建"）。
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
  (e: 'action', kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause', c: Container): void
  (e: 'remove', c: Container): void
  (e: 'terminal', c: Container): void
  (e: 'logs', c: Container): void
  (e: 'detail', c: Container): void
  (e: 'openDir', c: Container): void
  (e: 'rename', c: Container): void
}>()

const isRunning = computed(() => props.container.state === 'running')
const isPaused = computed(() => props.container.state === 'paused')

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
/** 直接从 docker stats 的 "19.48MiB / 31.24GiB" 取已用值（字节数）。
 * 用 1024 进制（docker stats 是 IEC 二进制单位）。 */
function parseMemUsed(s: string): number {
  const used = (s || '').split('/')[0]?.trim() || ''
  const m = used.match(/^([\d.]+)\s*(K|M|G|T)?i?B?/i)
  if (!m) return 0
  const val = parseFloat(m[1])
  if (isNaN(val)) return 0
  const unit = (m[2] || '').toUpperCase()
  const factor: Record<string, number> = {
    '': 1, K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4,
  }
  return val * (factor[unit] ?? 1)
}

/** 字节数 → 自适应单位字符串（B/KiB/MiB/GiB/TiB），保留 2 位小数，不补前导 0。
 * 与 docker stats 原始单位一致，无换算误差。 */
function formatBytesAuto(bytes: number): string {
  if (!isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  let i = 0
  let v = bytes
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++ }
  return v.toFixed(2) + ' ' + units[i]
}

const cpuPct = computed(() => props.stats?.cpu_percent ?? 0)
/** 只显示已用内存（不显示总量），自适应单位 */
const memUsedText = computed(() => {
  const used = parseMemUsed(props.stats?.mem_usage || '')
  if (used <= 0) return '0 B'
  return formatBytesAuto(used)
})

// 网络速率格式化：固定 KB/s，无数据时 0.00 KB/s
function formatRate(bps: number): string {
  if (!isFinite(bps) || bps <= 0) return '0.00 KB/s'
  const kbps = bps / 1024
  return kbps.toFixed(2) + ' KB/s'
}
const netRxRate = computed(() => props.netRate?.rx ?? 0)
const netTxRate = computed(() => props.netRate?.tx ?? 0)

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

function onAction(kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause') {
  emit('action', kind, props.container)
}

function onMoreCmd(cmd: string) {
  if (cmd === 'detail') emit('detail', props.container)
  else if (cmd === 'logs') emit('logs', props.container)
  else if (cmd === 'openDir') emit('openDir', props.container)
  else if (cmd === 'rename') emit('rename', props.container)
  else if (cmd === 'remove') emit('remove', props.container)
}
</script>

<template>
  <div class="ctr-card" :class="{ running: isRunning }">
    <!-- 左侧状态彩条 -->
    <div class="state-bar" :style="{ background: stateStyle.bar }" />

    <div class="card-inner">
      <!-- 图标（占整个卡片高度，垂直居中）+ 右侧两行内容 -->
      <div class="head-block">
        <ContainerIcon :image="container.image" :size="40" class="ctr-icon" />

        <div class="info">
          <!-- 第1行：名称 + ID + 项目 + 端口 -->
          <div class="row-main">
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

          <!-- 第2行：监控(左) + 操作(右) 两端对齐 -->
          <div class="row-second">
            <!-- 监控：运行中显示 CPU/内存/网络；非运行中显示状态文案 -->
            <div class="metrics-wrap">
              <template v-if="isRunning">
                <span class="metric">
                  <span class="metric-label">CPU</span>
                  <span class="metric-val mono-num">{{ cpuPct.toFixed(2) }} %</span>
                </span>
                <span class="metric">
                  <span class="metric-label">内存</span>
                  <span class="metric-val mono-num">{{ memUsedText }}</span>
                </span>
                <span class="metric">
                  <span class="metric-label">网络</span>
                  <span class="metric-val net-val mono-num">
                    <span class="tx">↑ {{ formatRate(netTxRate) }}</span>
                    <span class="rx">↓ {{ formatRate(netRxRate) }}</span>
                  </span>
                </span>
              </template>
              <span v-else class="state-text" :style="{ color: stateStyle.color }">{{ stateStyle.label }}</span>
            </div>

            <!-- 操作 -->
            <div class="row-actions">
              <template v-if="isRunning">
                <el-button size="small" :icon="VideoPause" @click="onAction('pause')">暂停</el-button>
                <el-button size="small" :icon="RefreshRight" @click="onAction('restart')">重启</el-button>
                <el-button size="small" :icon="Monitor" @click="emit('terminal', container)">终端</el-button>
              </template>
              <template v-else-if="isPaused">
                <el-button size="small" type="primary" :icon="VideoPlay" @click="onAction('unpause')">恢复</el-button>
                <el-button size="small" :icon="Monitor" @click="emit('terminal', container)">终端</el-button>
              </template>
              <template v-else>
                <el-button size="small" type="primary" :icon="VideoPlay" @click="onAction('start')">启动</el-button>
              </template>
              <el-dropdown trigger="click" @command="onMoreCmd" class="more-btn">
                <el-button size="small" text :icon="MoreFilled" />
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="detail" :icon="View">详情</el-dropdown-item>
                    <el-dropdown-item command="logs" :icon="Document">日志</el-dropdown-item>
                    <el-dropdown-item command="rename" :icon="EditPen">重命名</el-dropdown-item>
                    <el-dropdown-item command="openDir" :icon="FolderOpened">目录</el-dropdown-item>
                    <el-dropdown-item command="remove" :icon="Delete" divided>删除...</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  VideoPlay, VideoPause, RefreshRight, Monitor, Document, MoreFilled, Connection, Box, View, Delete, EditPen,
} from '@element-plus/icons-vue'
export default {
  components: { VideoPlay, VideoPause, RefreshRight, Monitor, Document, MoreFilled, Connection, Box, View, Delete, EditPen },
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
.ctr-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
}
.ctr-card:hover .state-bar {
  width: 5px;
  filter: saturate(1.3) brightness(1.1);
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
  padding: 10px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* ===== 图标占卡片高度 + 右侧两行内容 ===== */
.head-block {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
  flex: 1;
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

/* 第1行：名称/ID/项目 + 端口 */
.row-main {
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
  max-width: 200px;
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

/* 端口按钮 */
.port-btn {
  margin-left: auto;
  color: var(--el-text-color-secondary);
  font-weight: 600;
  padding: 4px 8px;
}
.port-btn:hover { color: var(--el-color-primary); }

/* 第2行：监控(左) + 操作(右) 两端对齐 */
.row-second {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}
.metrics-wrap {
  display: flex;
  align-items: center;
  gap: 18px;
  flex-wrap: wrap;
  min-width: 0;
}
.state-text {
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
}

/* 指标 */
.metric {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
  white-space: nowrap;
}
.metric-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.metric-val {
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  white-space: nowrap;
}
.mono-num {
  font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', Consolas, 'Courier New', monospace;
  font-variant-numeric: tabular-nums;
}
.net-val {
  display: inline-flex;
  gap: 8px;
}
.net-val .rx { color: #1d9bf0; }
.net-val .tx { color: #39d0d8; }

/* 操作 */
.row-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
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
