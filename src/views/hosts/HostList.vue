<script setup lang="ts">
/**
 * 连接管理器：主机列表。
 *
 * - 顶部一个分组下拉筛选（全部/各分组/未分组）+ 名称/IP 搜索框，卡片平铺展示
 * - 单击卡片：未连接→连接，已连接→进入概览
 * - hover 右上角 ⋯ 菜单：编辑 / 删除 / 断开
 * - 卡片显示：状态、名称、地址、OS、Docker 版本
 */
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useHostsStore } from '@/store/hosts'
import { useTabsStore } from '@/store/tabs'
import { useTerminalsStore } from '@/store/terminals'
import * as api from '@/api'
import type { Host } from '@/api/types'
import HostFormDialog from './HostFormDialog.vue'

const router = useRouter()
const store = useHostsStore()
const tabsStore = useTabsStore()
const terminalsStore = useTerminalsStore()

const dialogVisible = ref(false)
const editing = ref<Host | null>(null)
const connecting = ref<string | null>(null)

// 分组筛选：'' = 全部
const groupFilter = ref('')
// 名称/IP 搜索关键字（大小写不敏感）
const keyword = ref('')

onMounted(async () => {
  await store.refresh()
})

// 已有分组（去重），供筛选下拉 + 新增/编辑候选
const groupOptions = computed(() => {
  const set = new Set<string>()
  store.hosts.forEach((h) => h.group && set.add(h.group))
  return [...set]
})

// 当前筛选 + 搜索下的主机
const filteredHosts = computed(() => {
  const g = groupFilter.value
  const kw = keyword.value.trim().toLowerCase()
  return store.hosts.filter((h) => {
    if (g && (h.group ?? '') !== g) return false
    if (kw) {
      const hay = `${h.name}\n${h.host}\n${h.user}`.toLowerCase()
      if (!hay.includes(kw)) return false
    }
    return true
  })
})

function openCreate() {
  editing.value = null
  dialogVisible.value = true
}

function openEdit(host: Host) {
  editing.value = host
  dialogVisible.value = true
}

async function onSaved() {
  dialogVisible.value = false
  await store.refresh()
}

async function remove(host: Host) {
  try {
    await ElMessageBox.confirm(
      `确定删除主机「${host.name}」吗？此操作会同时清除其凭据。`,
      '删除确认',
      { type: 'warning' },
    )
    await api.deleteHost(host.id)
    tabsStore.remove(host.id)
    ElMessage.success('已删除')
    await store.refresh()
  } catch {
    /* 用户取消 */
  }
}

// 单击卡片：直接连接并进入（防重入：connecting 锁）
function onCardClick(host: Host) {
  connect(host)
}

// 卡片 ⋯ 菜单命令分发：edit / delete / disconnect
function onCardCommand(command: string, host: Host) {
  if (command === 'edit') openEdit(host)
  else if (command === 'delete') remove(host)
  else disconnect(host)
}

/** 连接阶段 → 卡片上的进度文案 */
function phaseText(host: Host): string {
  switch (store.phaseOf(host.id)) {
    case 'connecting':
      return '正在建立连接…'
    case 'auth':
      return '正在认证…'
    case 'probing':
      return '正在探测环境…'
    default:
      return '连接中…'
  }
}

/** 错误分类 → 友好标题 + 建议操作文案 */
function errorHint(kind: string): { title: string; tip: string } {
  switch (kind) {
    case 'timeout':
      return {
        title: '连接超时',
        tip: '请检查网络是否通畅，或主机是否在线。',
      }
    case 'network':
      return {
        title: '无法连接到主机',
        tip: '请检查地址、端口是否正确，以及防火墙是否放行。',
      }
    case 'auth':
      return {
        title: '认证被拒绝',
        tip: '用户名或密码/密钥不正确，请检查凭据。',
      }
    case 'credential':
      return {
        title: '本地凭据缺失',
        tip: '未读取到保存的密码或密钥口令，请重新编辑主机填写凭据。',
      }
    case 'notfound':
      return { title: '主机不存在', tip: '该主机配置可能已被删除。' }
    default:
      return { title: '连接失败', tip: '请稍后重试，或检查目标主机的 Docker 环境。' }
  }
}

async function connect(host: Host) {
  if (connecting.value) return
  connecting.value = host.id
  try {
    if (!store.isOnline(host.id)) {
      const res = await store.connect(host.id)
      ElMessage.success(`已连接：${res.probe.hostname}（Docker ${res.probe.docker_version}）`)
    }
    store.select(host.id)
    tabsStore.open(host.id)
    router.push({ name: 'dashboard', params: { id: host.id } })
  } catch (e) {
    // store.connect 抛出的是 StructuredError {kind,message}
    const err = (e as { kind?: string; message?: string }) ?? {}
    const kind = typeof err.kind === 'string' ? err.kind : 'other'
    const message = typeof err.message === 'string' ? err.message : String(e)
    const hint = errorHint(kind)
    // 失败弹窗：标题 + 详细信息 + 建议操作 + 重试/编辑按钮
    ElMessageBox.alert(
      `<div style="line-height:1.7">
        <div style="color:var(--el-text-color-secondary);font-size:12px;margin-top:4px">${hint.tip}</div>
        <div style="margin-top:10px;padding:8px 10px;background:var(--el-fill-color-light);border-radius:6px;font-size:12px;color:var(--el-text-color-secondary);word-break:break-all">${message}</div>
      </div>`,
      hint.title,
      {
        dangerouslyUseHTMLString: true,
        type: 'error',
        confirmButtonText: '重试',
        cancelButtonText: '编辑主机',
        showCancelButton: true,
        distinguishCancelAndClose: true,
      },
    )
      .then(() => {
        // 用户点「重试」
        connect(host)
      })
      .catch((action: string) => {
        // cancel = 用户点了「编辑主机」；close = 关闭，什么都不做
        if (action === 'cancel') openEdit(host)
      })
  } finally {
    connecting.value = null
  }
}

async function disconnect(host: Host) {
  try {
    await store.disconnect(host.id)
    // 同步清理该主机下所有终端 tab（断开 SSH 后 PTY 会话已失效）
    await terminalsStore.closeByHost(host.id).catch(() => {})
    ElMessage.success('已断开')
  } catch (e) {
    ElMessage.error(`断开失败：${e}`)
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <span class="subtitle">连接远程服务器管理 Docker，选择一台主机开始</span>
      <div class="header-actions">
        <el-input
          v-model="keyword"
          placeholder="搜索名称 / IP / 用户名"
          clearable
          :prefix-icon="Search"
          style="width: 220px"
        />
        <el-select
          v-model="groupFilter"
          placeholder="全部分组"
          clearable
          size="default"
          style="width: 160px"
        >
          <el-option label="全部分组" value="" />
          <el-option
            v-for="g in groupOptions"
            :key="g"
            :label="g"
            :value="g"
          />
        </el-select>
        <el-button type="primary" :icon="Plus" @click="openCreate">
          添加主机
        </el-button>
      </div>
    </header>

    <div v-loading="store.loading" class="host-content">
      <div class="host-grid">
        <div
          v-for="h in filteredHosts"
          :key="h.id"
          class="host-card"
          :class="{ online: store.isOnline(h.id), connecting: connecting === h.id }"
          @click="onCardClick(h)"
        >
          <div class="card-top">
            <el-icon
              class="state-dot"
              :color="store.isOnline(h.id) ? 'var(--el-color-success)' : 'var(--el-text-color-secondary)'"
            >
              <component :is="store.isOnline(h.id) ? 'CircleCheck' : 'CircleClose'" />
            </el-icon>
            <span class="host-name">{{ h.name }}</span>
            <span v-if="h.group" class="group-tag">{{ h.group }}</span>
            <el-dropdown
              trigger="click"
              @click.stop
              @command="(c: string) => onCardCommand(c, h)"
            >
              <el-icon class="more-btn" @click.stop><MoreFilled /></el-icon>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item :icon="Edit" command="edit">编辑</el-dropdown-item>
                  <el-dropdown-item v-if="store.isOnline(h.id)" :icon="SwitchButton" command="disconnect">断开</el-dropdown-item>
                  <el-dropdown-item :icon="Delete" divided command="delete">删除</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>

          <div class="host-addr mono">{{ h.user }}@{{ h.host }}:{{ h.port }}</div>

          <!-- 在线：探测信息 -->
          <div class="probe-tags" v-if="store.isOnline(h.id) && store.probeMap[h.id]">
            <el-tag size="small" type="info" effect="dark">{{ store.probeMap[h.id].os }}</el-tag>
            <el-tag v-if="store.probeMap[h.id].docker_version" size="small" type="info" effect="dark">
              Docker {{ store.probeMap[h.id].docker_version }}
            </el-tag>
            <el-tag v-if="store.probeMap[h.id].is_wsl2" size="small" type="info" effect="dark">WSL2</el-tag>
          </div>
          <!-- 离线：连接阶段提示 / 失败提示 -->
          <div class="offline-status" v-else>
            <span v-if="connecting === h.id" class="loading-text">{{ phaseText(h) }}</span>
            <span v-else-if="store.phaseOf(h.id) === 'failed'" class="fail-text">连接失败</span>
          </div>
        </div>
      </div>

      <div v-if="!store.loading && filteredHosts.length === 0" class="empty">
        <el-empty :description="keyword ? '没有匹配的主机' : (groupFilter ? '该分组下没有主机' : '还没有主机，点击右上角添加一台')" />
      </div>
    </div>

    <HostFormDialog
      v-model="dialogVisible"
      :host="editing"
      :groups="groupOptions"
      @saved="onSaved"
    />
  </div>
</template>

<script lang="ts">
import {
  Plus,
  MoreFilled,
  Edit,
  Delete,
  SwitchButton,
  CircleCheck,
  CircleClose,
  Search,
} from '@element-plus/icons-vue'
export default {
  components: { Plus, MoreFilled, Edit, Delete, SwitchButton, CircleCheck, CircleClose, Search },
}
</script>

<style scoped>
.page {
  height: 100%;
  overflow-y: auto;
  padding: 20px 28px;
  display: flex;
  flex-direction: column;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
  flex-shrink: 0;
}
.subtitle {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}
.host-content {
  flex: 1;
  min-height: 0;
}

/* ===== 卡片网格 ===== */
.host-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 14px;
}

/* ===== 单张卡片 ===== */
.host-card {
  position: relative;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  border-radius: 12px;
  padding: 16px 18px;
  cursor: pointer;
  /* 多属性过渡，让每个反馈都顺滑 */
  transition: transform 0.22s cubic-bezier(0.34, 1.2, 0.64, 1),
    box-shadow 0.22s ease, border-color 0.22s ease, background 0.22s ease;
  overflow: hidden;
  will-change: transform;
}
/* 在线卡片：右上一个柔和的状态光晕 */
.host-card.online::after {
  content: '';
  position: absolute;
  top: -40px;
  right: -40px;
  width: 100px;
  height: 100px;
  border-radius: 50%;
  background: radial-gradient(circle, var(--el-color-success) 0%, transparent 70%);
  opacity: 0.1;
  pointer-events: none;
  transition: opacity 0.3s ease;
}

/* hover：上浮 + 阴影 + 轻微放大，不用边框变色 */
.host-card:hover {
  transform: translateY(-3px) scale(1.01);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  border-color: var(--el-border-color-hover);
}
.host-card.online:hover::after {
  opacity: 0.2;
}

/* 在线状态点：常驻呼吸动画 */
.state-dot {
  animation: state-breathe 2.4s ease-in-out infinite;
}
.host-card:hover .state-dot {
  animation-duration: 1.2s;
}
@keyframes state-breathe {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.18); }
}

/* hover 时主机名微移，增强联动感 */
.host-name,
.host-addr {
  transition: color 0.22s ease;
}
.host-card:hover .host-addr {
  color: var(--el-text-color-regular);
}

/* 连接中：卡片轻微高亮脉冲 */
.host-card.connecting {
  animation: card-pulse 1s ease-in-out infinite;
}
@keyframes card-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(64, 158, 255, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(64, 158, 255, 0); }
}

.card-top {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.state-dot {
  font-size: 15px;
  flex-shrink: 0;
}
.host-name {
  flex: 1;
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.group-tag {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 1px 6px;
  flex-shrink: 0;
}
.more-btn {
  font-size: 16px;
  color: var(--el-text-color-secondary);
  cursor: pointer;
  border-radius: 4px;
  padding: 2px;
  flex-shrink: 0;
  transition: color 0.15s, background 0.15s, transform 0.18s;
}
.more-btn:hover {
  color: var(--el-text-color-primary);
  background: var(--el-fill-color-light);
  transform: rotate(90deg);
}
.host-addr {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.probe-tags {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  min-height: 24px;
  /* hover 时标签组淡入感 */
  transition: opacity 0.22s ease;
}
.offline-status {
  min-height: 24px;
}
.loading-text {
  font-size: 12px;
  color: var(--el-color-primary);
}
.fail-text {
  font-size: 12px;
  color: var(--el-color-danger);
}
.empty {
  margin-top: 40px;
}
</style>
