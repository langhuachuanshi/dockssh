<script setup lang="ts">
/**
 * 项目（compose）列表页 —— 卡片视图。
 *
 * 通过扫描容器 labels 中的 com.docker.compose.project 聚合出 compose 项目。
 * 每个项目一张纵向卡片：
 *   [左彩条] [项目图标] [第1行: 名称+状态  第2行: 创建时间·路径] [开关] [菜单⋮]
 *
 * - 开关：ON = up -d，OFF = stop（仅停止，保留容器）
 * - 菜单（按状态变化）：构建/启动/停止/重启/清理/删除/详情/显示日志
 * - 编辑配置：居中大弹窗 + CodeMirror
 * - 详情：参考图布局的弹窗（容器 / 日志 tab）
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { ComposeProject } from '@/api/types'
import ComposeEditor from './ComposeEditor.vue'
import IconButton from './IconButton.vue'
import ComposeDetail from './ComposeDetail.vue'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const projects = ref<ComposeProject[]>([])
const loading = ref(false)
const search = ref('')
// 正在执行操作的项目名集合（禁用相关交互）
const busy = ref<Set<string>>(new Set())

const filtered = computed(() =>
  projects.value
    .filter((p) => {
      if (!search.value) return true
      return p.name.toLowerCase().includes(search.value.toLowerCase())
    })
    // 默认按创建时间倒序（最新在最上）
    .slice()
    .sort((a, b) => (a.created < b.created ? 1 : a.created > b.created ? -1 : 0)),
)

async function refresh() {
  loading.value = true
  try {
    projects.value = await api.listComposeProjects(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

// ===== 状态样式 =====
// 全部运行 success / 部分运行 warning / 全停或无容器 info
function stateOf(p: ComposeProject): { tag: 'success' | 'warning' | 'info'; label: string; color: string } {
  if (p.containers === 0) return { tag: 'info', label: '已停止', color: '#909399' }
  if (p.running === p.containers) return { tag: 'success', label: '正在运行', color: '#67c23a' }
  if (p.running === 0) return { tag: 'info', label: '已停止', color: '#909399' }
  return { tag: 'warning', label: '部分运行', color: '#e6a23c' }
}
const isRunning = (p: ComposeProject) => p.running > 0

// ===== 操作（统一带 busy 锁 + 刷新） =====
async function act(
  p: ComposeProject,
  fn: () => Promise<string>,
  okMsg: string,
) {
  if (busy.value.has(p.name)) return
  busy.value.add(p.name)
  try {
    await fn()
    ElMessage.success(okMsg)
    await refresh()
  } catch (e) {
    ElMessage.error(`操作失败：${e}`)
  } finally {
    busy.value.delete(p.name)
  }
}

function build(p: ComposeProject) {
  act(p, () => api.buildComposeProject(hostId.value, p.name), '已构建')
}
function up(p: ComposeProject) {
  act(p, () => api.upComposeProject(hostId.value, p.name), '已启动')
}
function stop(p: ComposeProject) {
  act(p, () => api.stopComposeProject(hostId.value, p.name), '已停止')
}
function restart(p: ComposeProject) {
  act(p, () => api.restartComposeProject(hostId.value, p.name), '已重启')
}

// 开关：根据当前状态决定 up / stop
async function onToggle(p: ComposeProject) {
  if (isRunning(p)) stop(p)
  else up(p)
}

// 危险操作：清理 / 删除（均 = down）走确认
async function destroy(p: ComposeProject) {
  try {
    await ElMessageBox.confirm(
      `将对项目「${p.name}」执行 docker compose down：停止并移除容器/网络（保留 compose 文件）。确认？`,
      '删除确认',
      { type: 'warning' },
    )
  } catch {
    return
  }
  act(p, () => api.downComposeProject(hostId.value, p.name), '已删除')
}

// ===== 菜单命令分发 =====
type MenuCmd = 'build' | 'up' | 'stop' | 'restart' | 'clean' | 'delete' | 'detail' | 'logs'
function onCommand(cmd: MenuCmd, p: ComposeProject) {
  switch (cmd) {
    case 'build': return build(p)
    case 'up': return up(p)
    case 'stop': return stop(p)
    case 'restart': return restart(p)
    case 'clean':
    case 'delete': return destroy(p)
    case 'detail': return openDetail(p)
    case 'logs': return openLogs(p)
  }
}

// ===== 详情弹窗 =====
const detailVisible = ref(false)
const detailProject = ref<ComposeProject | null>(null)
function openDetail(p: ComposeProject) {
  detailProject.value = p
  detailVisible.value = true
}
function openLogs(p: ComposeProject) {
  detailProject.value = p
  detailVisible.value = true
  // 切到日志 tab：ComposeDetail 通过 watch(activeTab) 处理，这里无法直接设；
  // 默认进容器 tab，用户点「日志」即可。保持简单。
}

// ===== 编辑配置弹窗 =====
const editVisible = ref(false)
const editProject = ref<ComposeProject | null>(null)
const editorRef = ref<InstanceType<typeof ComposeEditor>>()

function openEdit(p: ComposeProject) {
  if (!p.config_files) {
    ElMessage.warning('未识别到 compose 文件路径，无法编辑')
    return
  }
  editProject.value = p
  editVisible.value = true
}
// 详情弹窗里点「编辑配置」
function openEditFromDetail(p: ComposeProject) {
  detailVisible.value = false
  openEdit(p)
}
async function doSave() {
  await editorRef.value?.save()
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  refresh()
})
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <el-input
        v-model="search"
        placeholder="搜索项目名"
        :prefix-icon="Search"
        clearable
        style="width: 280px"
      />
      <el-button :icon="Refresh" @click="refresh">刷新</el-button>
    </div>

    <div class="card-wrap" v-loading="loading">
      <div v-if="filtered.length" class="card-list">
        <div
          v-for="p in filtered"
          :key="p.name"
          class="proj-card"
          :class="{ running: isRunning(p), busy: busy.has(p.name) }"
        >
          <!-- 左侧状态彩条（全高，hover 加宽，参考容器卡片） -->
          <div class="state-bar" :style="{ background: stateOf(p).color }" />

          <div class="card-inner">
            <!-- 图标 + 两行信息 -->
            <div class="head-block">
              <div class="icon" :style="{ background: stateOf(p).color }">
                <el-icon><Box /></el-icon>
              </div>
              <div class="info">
                <!-- 第1行：名称 + 状态 -->
                <div class="line1">
                  <span class="name mono" @click="openDetail(p)">{{ p.name }}</span>
                  <span class="state-text" :style="{ color: stateOf(p).color }">
                    {{ stateOf(p).label }}
                  </span>
                </div>
                <!-- 第2行：创建时间 · 路径（占剩余宽度，超出省略） -->
                <div class="line2">
                  <span class="created">{{ p.created || '—' }}</span>
                  <span class="sep">·</span>
                  <span class="path mono" :title="p.config_files || ''">
                    {{ p.config_files || '未识别到 compose 文件路径' }}
                  </span>
                </div>
              </div>
            </div>

            <!-- 操作行：启停图标按钮 + 菜单 -->
            <div class="row-actions">
              <!-- 启停图标按钮：运行中=停止图标(绿)，未运行=启动图标(灰) -->
              <IconButton
                :icon="isRunning(p) ? VideoPause : VideoPlay"
                :running="isRunning(p)"
                :loading="busy.has(p.name)"
                :tip="isRunning(p) ? '停止' : '启动'"
                @click="onToggle(p)"
              />

              <!-- 菜单 -->
              <el-dropdown trigger="click" @command="(c: MenuCmd) => onCommand(c, p)">
                <IconButton :icon="MoreFilled" tip="更多操作" />
                <template #dropdown>
                  <el-dropdown-menu>
                    <!-- 运行中：停止 / 重启 / 清理 / 删除 -->
                    <template v-if="isRunning(p)">
                      <el-dropdown-item :icon="VideoPause" command="stop">停止</el-dropdown-item>
                      <el-dropdown-item :icon="RefreshRight" command="restart">重启</el-dropdown-item>
                      <el-dropdown-item :icon="Delete" command="clean">清理</el-dropdown-item>
                      <el-dropdown-item :icon="Delete" command="delete" divided>删除</el-dropdown-item>
                    </template>
                    <!-- 未运行：构建 / 启动 / 删除 -->
                    <template v-else>
                      <el-dropdown-item :icon="Cpu" command="build">构建</el-dropdown-item>
                      <el-dropdown-item :icon="VideoPlay" command="up">启动</el-dropdown-item>
                      <el-dropdown-item :icon="Delete" command="delete" divided>删除</el-dropdown-item>
                    </template>
                    <el-dropdown-item :icon="Document" command="logs" divided>显示日志</el-dropdown-item>
                    <el-dropdown-item :icon="View" command="detail">详情</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </div>
        </div>
      </div>
      <el-empty v-else description="没有 compose 项目" />
    </div>

    <!-- 详情弹窗 -->
    <ComposeDetail
      v-model="detailVisible"
      :project="detailProject"
      @edit="openEditFromDetail"
    />

    <!-- 编辑配置弹窗 -->
    <el-dialog
      v-model="editVisible"
      :title="`编辑 compose 文件${editProject?.name ? '：' + editProject.name : ''}`"
      width="70%"
      top="6vh"
      :close-on-click-modal="false"
      destroy-on-close
    >
      <template v-if="editVisible && editProject?.config_files">
        <div class="dlg-path mono" :title="editProject.config_files">
          {{ editProject.config_files }}
        </div>
        <div class="dlg-editor">
          <ComposeEditor
            ref="editorRef"
            :path="editProject.config_files"
            @saved="refresh"
          />
        </div>
      </template>
      <template #footer>
        <el-button @click="editVisible = false">关闭</el-button>
        <el-button type="primary" :icon="Check" @click="doSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts">
import {
  Search, Refresh, Box, MoreFilled,
  VideoPlay, VideoPause, RefreshRight, Cpu,
  Delete, Document, View, Check,
} from '@element-plus/icons-vue'
export default {
  components: {
    Search, Refresh, Box, MoreFilled,
    VideoPlay, VideoPause, RefreshRight, Cpu,
    Delete, Document, View, Check,
  },
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

.card-wrap {
  flex: 1;
  padding: 16px 24px;
  overflow: auto;
}
.card-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.proj-card {
  position: relative;
  display: flex;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.18s ease, box-shadow 0.18s ease, opacity 0.15s;
}
/* hover：彩条加宽变亮 + 整卡轻微抬升 + 操作区填充（对齐容器卡片） */
.proj-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
}
.proj-card:hover .state-bar {
  width: 5px;
  filter: saturate(1.3) brightness(1.1);
}
.proj-card.busy {
  opacity: 0.7;
}

/* 左侧状态彩条：flex 直接子元素，自动占满全高 */
.state-bar {
  width: 3px;
  flex-shrink: 0;
  transition: width 0.18s ease, filter 0.18s ease;
}

.card-inner {
  flex: 1;
  min-width: 0;
  padding: 12px 14px;
  display: flex;
  align-items: center;
  gap: 12px;
}

/* 头部：图标 + 两行信息 */
.head-block {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}
.icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  flex-shrink: 0;
}

/* 两行信息：占满中间剩余宽度 */
.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.line1 {
  display: flex;
  align-items: center;
  gap: 8px;
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
}
.name:hover {
  color: var(--el-color-primary);
}
.state-text {
  flex-shrink: 0;
  font-size: 12px;
}

.line2 {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  min-width: 0;
}
.created {
  flex-shrink: 0;
}
.sep {
  flex-shrink: 0;
}
.path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 操作区：启停 + 菜单，两个按钮各自独立（非按钮组） */
.row-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  padding-left: 8px;
}

/* 编辑弹窗 */
.dlg-path {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 10px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dlg-editor {
  display: flex;
  flex-direction: column;
  height: 64vh;
  min-height: 0;
}

/* 窄屏：路径行换行，操作区不挤 */
@media (max-width: 760px) {
  .card-inner {
    padding: 10px 12px;
  }
  .line2 {
    flex-wrap: wrap;
  }
}
</style>
