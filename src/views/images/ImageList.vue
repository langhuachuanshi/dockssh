<script setup lang="ts">
/**
 * 镜像列表页（单列横卡视图）。
 *
 * 每镜像一张横向卡片，视觉与容器列表统一：左彩条 + 图标 + 两行内容。
 * 镜像无运行状态，第二行展示「大小 · 创建时间」+ 右侧「创建容器 / 更多」操作。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Image } from '@/api/types'
import ContainerDefaultIcon from '@/views/containers/ContainerDefaultIcon.vue'
import CreateContainerDialog from '@/views/containers/CreateContainerDialog.vue'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const images = ref<Image[]>([])
const loading = ref(false)
const search = ref('')

// 多选 + 批量删除（卡片视图自行维护选中集）
const selected = ref<Image[]>([])
function toggleCheck(img: Image, val: boolean) {
  if (val) {
    if (!selected.value.some((s) => s.id === img.id)) selected.value.push(img)
  } else {
    const idx = selected.value.findIndex((s) => s.id === img.id)
    if (idx >= 0) selected.value.splice(idx, 1)
  }
}
const batching = ref(false)
async function batchRemove() {
  if (!selected.value.length) return
  const count = selected.value.length
  try {
    await ElMessageBox.confirm(
      `确认删除选中的 ${count} 个镜像？被容器引用的镜像将强制删除。`,
      '批量删除',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  batching.value = true
  try {
    await api.removeImages(
      hostId.value,
      selected.value.map((i) => i.id),
      true,
    )
    ElMessage.success(`已删除 ${count} 个镜像`)
    await refresh()
  } catch (e) {
    ElMessage.error(`批量删除失败：${e}`)
    await refresh()
  } finally {
    batching.value = false
  }
}

// 创建容器向导
const createVisible = ref(false)
const presetImage = ref('')
function openCreate(img?: Image) {
  presetImage.value = img ? `${img.repository}:${img.tag}` : ''
  createVisible.value = true
}
function onCreated() {
  refresh()
}

// ===== 排序：按创建时间字符串降序（新镜像在前）=====
// docker CreatedAt 形如 "2026-06-25 01:34:45 +0800 CST"，去掉 CST 后可被 Date 解析。
function parseCreated(s: string): number {
  if (!s) return 0
  const cleaned = s.replace(/\s*CST\s*$/i, '').trim()
  const t = Date.parse(cleaned)
  return isNaN(t) ? 0 : t
}

/** 把 docker 的 CreatedAt 字符串格式化成 "YYYY-MM-DD HH:mm:ss"。
 * 优先用解析出的时间戳重新格式化（时区一致）；解析失败则原样回显。 */
function formatCreated(s: string): string {
  const ts = parseCreated(s)
  if (!ts) return s
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return (
    d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) +
    ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds())
  )
}

const filtered = computed(() => {
  const result = images.value.filter((i) => {
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      i.repository.toLowerCase().includes(kw) ||
      i.tag.toLowerCase().includes(kw) ||
      i.id.toLowerCase().includes(kw)
    )
  })
  result.sort((a, b) => parseCreated(b.created) - parseCreated(a.created))
  return result
})

async function refresh() {
  loading.value = true
  try {
    images.value = await api.listImages(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

// 删除单个镜像
async function remove(img: Image) {
  try {
    await ElMessageBox.confirm(
      `确认删除镜像「${img.repository}:${img.tag}」？`,
      '删除确认',
      { type: 'warning' },
    )
    await api.removeImage(hostId.value, img.id, true)
    ElMessage.success('已删除')
    await refresh()
  } catch {
    /* cancel */
  }
}

// 点击复制
async function copyText(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success(`已复制${label}`)
  } catch {
    ElMessage.info(text)
  }
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
        placeholder="搜索仓库名/标签/ID"
        :prefix-icon="Search"
        clearable
        style="width: 280px"
      />
      <div class="toolbar-right">
        <el-button
          v-if="selected.length"
          type="danger"
          :icon="Delete"
          :loading="batching"
          @click="batchRemove"
        >删除选中 ({{ selected.length }})</el-button>
        <el-button :icon="VideoPlay" type="primary" @click="openCreate()">创建容器</el-button>
        <el-tooltip content="拉取镜像功能开发中" placement="top">
          <el-button :icon="Download" disabled>拉取镜像</el-button>
        </el-tooltip>
        <el-button :icon="Refresh" @click="refresh">刷新</el-button>
      </div>
    </div>

    <div class="card-wrap" v-loading="loading">
      <div v-if="filtered.length" class="card-grid">
        <div v-for="img in filtered" :key="img.id" class="img-card">
          <!-- 左侧彩条 -->
          <div class="state-bar" />

          <div class="card-inner">
            <el-checkbox
              :model-value="selected.some((s) => s.id === img.id)"
              class="card-check"
              @change="(val: boolean) => toggleCheck(img, val)"
            />

            <ContainerDefaultIcon :size="40" class="img-icon" />

            <div class="info">
              <!-- 第1行：仓库名 -->
              <div class="row-name">
                <span
                  class="name mono"
                  :title="`点击复制 ${img.repository}`"
                  @click="copyText(img.repository, '仓库名')"
                >{{ img.repository }}</span>
              </div>

              <!-- 第2行：创建时间 · 标签 · 大小 -->
              <div class="row-meta">
                <span class="metric-val mono-num">{{ formatCreated(img.created) }}</span>
                <el-tag size="small" effect="dark">{{ img.tag }}</el-tag>
                <span class="metric-val mono-num">{{ img.size }}</span>
              </div>
            </div>

            <!-- 右侧操作：垂直居中两个图标按钮 -->
            <div class="row-actions">
              <el-tooltip content="运行" placement="top" :show-after="300">
                <el-button
                  :icon="CaretRight"
                  type="primary"
                  circle
                  @click="openCreate(img)"
                />
              </el-tooltip>
              <el-tooltip content="删除镜像" placement="top" :show-after="300">
                <el-button
                  :icon="Delete"
                  circle
                  class="del-btn"
                  @click="remove(img)"
                />
              </el-tooltip>
            </div>
          </div>
        </div>
      </div>
      <el-empty v-else description="没有镜像" />
    </div>

    <!-- 创建容器向导 -->
    <CreateContainerDialog
      v-model="createVisible"
      :host-id="hostId"
      :preset-image="presetImage"
      :on-created="onCreated"
    />
  </div>
</template>

<script lang="ts">
import { Search, Refresh, Delete, Download, CaretRight, VideoPlay } from '@element-plus/icons-vue'

export default {
  name: 'ImageList',
  components: { Search, Refresh, Delete, Download, CaretRight, VideoPlay },
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
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
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

.img-card {
  position: relative;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease;
}
.img-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
  border-color: var(--el-color-primary-light-5);
}
.img-card:hover .state-bar {
  width: 5px;
}

/* 左侧彩条（镜像无状态，用主色点缀，与容器卡片视觉统一） */
.state-bar {
  width: 3px;
  flex-shrink: 0;
  background: var(--el-color-primary);
  transition: width 0.18s ease;
}

.card-inner {
  flex: 1;
  min-width: 0;
  padding: 10px 16px;
  display: flex;
  align-items: center;
  gap: 14px;
}

.card-check {
  flex-shrink: 0;
  height: 40px;
}
.img-icon {
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

/* 第1行：仓库名 */
.row-name {
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
  display: inline-block;
  max-width: 100%;
}
.name:hover { color: var(--el-color-primary); }

/* 第2行：创建时间 · 标签 · 大小 */
.row-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  min-width: 0;
}
.metric-val {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}

/* 右侧操作：垂直居中两个图标按钮 */
.row-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.row-actions :deep(.el-icon) {
  font-size: 20px;
}
.del-btn {
  color: var(--el-text-color-secondary);
}
.del-btn:hover {
  color: var(--el-color-danger);
}

.mono-num {
  font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', Consolas, 'Courier New', monospace;
  font-variant-numeric: tabular-nums;
}

.mono {
  font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', Consolas, 'Courier New', monospace;
}
</style>
