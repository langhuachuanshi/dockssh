<script setup lang="ts">
/**
 * Compose 项目详情弹窗（参考图布局）。
 *
 * 结构：标题区（图标+名称，副行路径）→ 基础信息 → tab（容器 / 日志）→ 底部按钮。
 *
 * - 容器 tab：列出本项目容器（从 listContainers 按 compose_project 过滤）
 * - 日志 tab：合并订阅本项目所有容器的实时日志（复用 start_logs/stop_logs/onLogChunk）
 *   切到日志 tab 才开始订阅；切走/关闭时停止，避免空跑。
 */
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useTerminalsStore } from '@/store/terminals'
import type { ComposeProject, Container } from '@/api/types'
import ContainerCard from '@/views/containers/ContainerCard.vue'

const props = defineProps<{
  modelValue: boolean
  project: ComposeProject | null
}>()
const emit = defineEmits<{
  'update:modelValue': [val: boolean]
  edit: [p: ComposeProject]
  refreshed: []
}>()

const route = useRoute()
const terminals = useTerminalsStore()
const hostId = computed(() => route.params.id as string)

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
})

const activeTab = ref<'containers' | 'logs'>('containers')
const containers = ref<Container[]>([])
const loadingContainers = ref(false)

// 状态样式（与卡片保持一致）
const stateInfo = computed(() => {
  const p = props.project
  if (!p) return { tag: 'info' as const, label: '—', color: '#909399' }
  if (p.containers === 0) return { tag: 'info' as const, label: '无容器', color: '#909399' }
  if (p.running === p.containers) return { tag: 'success' as const, label: '正在运行', color: '#67c23a' }
  if (p.running === 0) return { tag: 'info' as const, label: '已停止', color: '#909399' }
  return { tag: 'warning' as const, label: '部分运行', color: '#e6a23c' }
})

async function loadContainers() {
  if (!props.project) return
  loadingContainers.value = true
  try {
    const all = await api.listContainers(hostId.value)
    containers.value = all.filter(
      (c) => c.compose_project === props.project!.name,
    )
  } catch (e) {
    ElMessage.error(`加载容器失败：${e}`)
  } finally {
    loadingContainers.value = false
  }
}

// ===== 容器操作（转发给现有 container API，完成后刷新本列表 + 通知外层） =====
async function containerAction(kind: 'start' | 'stop' | 'restart' | 'pause' | 'unpause', c: Container) {
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
    await loadContainers()
    emit('refreshed')
  } catch (e) {
    ElMessage.error(`操作失败：${e}`)
  }
}

// 容器「打开目录」在详情弹窗内无文件管理上下文，降级为提示
function openDir(c: Container) {
  ElMessage.info(`「${c.name}」目录打开请在容器列表页操作`)
}

// 打开容器终端（与容器列表页一致的复用语义）
function openTerminal(c: Container) {
  terminals.open(hostId.value, c.id, c.name)
}

// 容器的日志/详情按钮 → 切到本项目日志 tab
function goLogs() {
  activeTab.value = 'logs'
}

// ===== 日志（合并项目所有容器） =====
const logEl = ref<HTMLDivElement>()
const logLines = ref<string[]>([])
const logAutoScroll = ref(true)
// 每个容器的取消监听函数 + 停止句柄
let logUnlistens: Array<() => void> = []

async function startProjectLogs() {
  logLines.value = []
  logUnlistens = []
  if (!containers.value.length) {
    logLines.value = ['（该项目暂无容器）']
    return
  }
  // 为每个容器订阅日志流（注意：先挂监听再启动，避免历史首包被丢）
  for (const c of containers.value) {
    const un = await api.onLogChunk(hostId.value, c.id || c.name, (chunk) => {
      const prefix = `[${c.name}] `
      for (const line of chunk.text.split('\n')) {
        if (line === '') continue
        logLines.value.push(prefix + line)
      }
      if (logLines.value.length > 8000) {
        logLines.value = logLines.value.slice(-6000)
      }
      if (logAutoScroll.value) nextTick(scrollLogBottom)
    })
    logUnlistens.push(un)
    try {
      await api.startLogs(hostId.value, c.id || c.name, { tail: '200' })
    } catch {
      /* 单个失败不阻断其余 */
    }
  }
}

function stopProjectLogs() {
  for (const c of containers.value) {
    api.stopLogs(hostId.value, c.id || c.name).catch(() => {})
  }
  for (const un of logUnlistens) un()
  logUnlistens = []
}

function scrollLogBottom() {
  if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
}

// 切 tab 时控制日志订阅
watch(activeTab, (t) => {
  if (t === 'logs') startProjectLogs()
  else stopProjectLogs()
})

// 弹窗打开/关闭、项目切换
watch(
  () => props.modelValue,
  async (open) => {
    if (open) {
      activeTab.value = 'containers'
      await loadContainers()
    } else {
      stopProjectLogs()
      logLines.value = []
    }
  },
)

function openEdit() {
  if (props.project) emit('edit', props.project)
}

defineExpose({ refreshContainers: loadContainers })
</script>

<template>
  <el-dialog
    v-model="visible"
    title="项目详情"
    width="680px"
    top="8vh"
    :close-on-click-modal="false"
    destroy-on-close
  >
    <template v-if="project">
      <!-- 标题区：图标 + 名称 + 路径 -->
      <div class="head">
        <div class="head-icon" :style="{ background: stateInfo.color }">
          <el-icon><Box /></el-icon>
        </div>
        <div class="head-text">
          <div class="head-name mono">{{ project.name }}</div>
          <div class="head-path mono" :title="project.config_files || ''">
            {{ project.config_files || '未识别到 compose 文件路径' }}
          </div>
        </div>
      </div>

      <!-- 基础信息 -->
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">状态</span>
          <span class="info-value">
            <span class="status-dot" :style="{ background: stateInfo.color }" />
            {{ stateInfo.label }}
          </span>
        </div>
        <div class="info-item">
          <span class="info-label">容器</span>
          <span class="info-value">{{ project.running }}/{{ project.containers }} 运行中</span>
        </div>
        <div class="info-item">
          <span class="info-label">创建时间</span>
          <span class="info-value">{{ project.created || '—' }}</span>
        </div>
      </div>

      <!-- tab：容器 / 日志 -->
      <el-tabs v-model="activeTab" class="detail-tabs">
        <el-tab-pane label="容器" name="containers">
          <div class="tab-body" v-loading="loadingContainers">
            <div v-if="containers.length" class="ctr-list">
              <ContainerCard
                v-for="c in containers"
                :key="c.id"
                :container="c"
                :stats="null"
                @action="(kind, cc) => containerAction(kind, cc)"
                @terminal="openTerminal"
                @logs="goLogs"
                @detail="goLogs"
                @open-dir="openDir"
              />
            </div>
            <el-empty v-else description="该项目暂无容器" />
          </div>
        </el-tab-pane>

        <el-tab-pane label="日志" name="logs">
          <div class="tab-body">
            <div class="log-toolbar">
              <el-checkbox v-model="logAutoScroll">自动滚动</el-checkbox>
            </div>
            <div ref="logEl" class="log-view selectable mono">
              <div v-for="(l, i) in logLines" :key="i" class="log-line">{{ l }}</div>
              <div v-if="!logLines.length" class="log-empty">等待日志输出…</div>
            </div>
          </div>
        </el-tab-pane>
      </el-tabs>
    </template>

    <template #footer>
      <el-button @click="visible = false">关闭</el-button>
      <el-button
        type="primary"
        :icon="EditPen"
        :disabled="!project?.config_files"
        @click="openEdit"
      >编辑配置</el-button>
    </template>
  </el-dialog>
</template>

<script lang="ts">
import { Box, EditPen } from '@element-plus/icons-vue'
export default { components: { Box, EditPen } }
</script>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 18px;
}
.head-icon {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 20px;
  flex-shrink: 0;
}
.head-text {
  min-width: 0;
}
.head-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.head-path {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  padding: 12px 14px;
  background: var(--el-fill-color-light);
  border-radius: 8px;
  margin-bottom: 16px;
}
.info-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.info-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.info-value {
  font-size: 13px;
  color: var(--el-text-color-primary);
  display: flex;
  align-items: center;
  gap: 6px;
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.detail-tabs {
  margin-top: 4px;
}
.tab-body {
  min-height: 200px;
}
.ctr-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 340px;
  overflow-y: auto;
  padding-right: 2px;
}

.log-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}
.log-view {
  height: 300px;
  overflow-y: auto;
  padding: 10px 12px;
  background: var(--el-fill-color-darker);
  border-radius: 6px;
  font-size: 12.5px;
  line-height: 1.55;
}
.log-line {
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--el-text-color-regular);
}
.log-empty {
  color: var(--el-text-color-secondary);
  text-align: center;
  margin-top: 30px;
}
</style>
