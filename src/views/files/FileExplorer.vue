<script setup lang="ts">
/**
 * 文件资源管理器（1:1 复刻 Windows 资源管理器）。
 *
 * 布局自上而下：
 *   导航条（后退/前进/上一级）→ 地址栏（面包屑 ↔ 文本输入）→ 搜索框
 *   → 工具栏（刷新/新建/上传/下载/重命名/删除/视图切换/排序）
 *   → 主区（大图标视图 / 详细列表视图）→ 状态栏
 *
 * 后端走 SFTP（commands::files）。所有路径用绝对路径。
 * 默认进入 SSH 用户主目录（file_home）。
 */
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open, save } from '@tauri-apps/plugin-dialog'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { DirListing, FileEntry } from '@/api/types'
import FileIcon from '@/components/FileIcon.vue'

const route = useRoute()
const store = useHostsStore()

const hostId = computed(() => route.params.id as string)

// ===== 当前目录状态 =====
const currentPath = ref('')
const entries = ref<FileEntry[]>([])
const loading = ref(false)

// ===== 导航历史栈（后退/前进） =====
const history = ref<string[]>([])
const histIdx = ref(-1) // 指向 history 中当前路径

// ===== 选中态 =====
const selected = ref<Set<string>>(new Set()) // 存 name

// ===== 视图与排序 =====
type ViewMode = 'large' | 'detail'
type SortKey = 'name' | 'modified' | 'size' | 'type'
// 视图模式持久化到 localStorage，避免切走再回来又被重置成默认
// 默认大图标视图（更直观），用户手动切到列表后记住选择
const VIEW_STORAGE_KEY = 'dockssh:file-view-mode'
const viewMode = ref<ViewMode>(
  (localStorage.getItem(VIEW_STORAGE_KEY) as ViewMode) || 'large',
)
watch(viewMode, (m) => localStorage.setItem(VIEW_STORAGE_KEY, m))
const sortKey = ref<SortKey>('name')
const sortAsc = ref(true)

// ===== 搜索 =====
const search = ref('')

// ===== 文本预览弹窗 =====
const previewVisible = ref(false)
const previewName = ref('')
const previewContent = ref('')
const previewLoading = ref(false)

// ===== 重命名/新建内联输入 =====
const renamingName = ref<string | null>(null) // 正在重命名的条目名
const renamingValue = ref('')
// 是否正在新建文件夹（内联命名态）
const creating = ref(false)
const creatingValue = ref('')
// 临时项输入框引用（大图标/列表两个视图共用一个 ref，谁渲染谁挂载）
const creatingInputRef = ref<HTMLInputElement | null>(null)

// 生成不重名的默认名（Windows 风格）：新建文件夹 / 新建文件夹 (2) ...
function defaultName(): string {
  const base = '新建文件夹'
  const existing = new Set(entries.value.map((e) => e.name))
  if (!existing.has(base)) return base
  let i = 2
  while (existing.has(`${base} (${i})`)) i++
  return `${base} (${i})`
}

// ===== 路径输入框 =====
const pathEditing = ref(false)
const pathInput = ref('')
const pathInputRef = ref<{ focus?: () => void; input?: HTMLInputElement } | null>(null)

// ===== 计算属性 =====
const sortedEntries = computed(() => {
  const arr = [...entries.value]
  const dir = sortAsc.value ? 1 : -1
  arr.sort((a, b) => {
    // 目录始终优先
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
    let cmp = 0
    switch (sortKey.value) {
      case 'name':
        cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase())
        break
      case 'modified':
        cmp = (a.modified || 0) - (b.modified || 0)
        break
      case 'size':
        cmp = a.size - b.size
        break
      case 'type':
        cmp = extOf(a.name).localeCompare(extOf(b.name))
        break
    }
    return cmp * dir
  })
  return arr
})

const filtered = computed(() => {
  if (!search.value.trim()) return sortedEntries.value
  const kw = search.value.trim().toLowerCase()
  return sortedEntries.value.filter((e) => e.name.toLowerCase().includes(kw))
})

// 地址栏面包屑：把 /root/home/user 切成可点击的段
const breadcrumbs = computed(() => {
  const p = currentPath.value
  if (!p) return []
  const parts = p.split('/').filter(Boolean)
  const list: { name: string; path: string }[] = []
  let acc = ''
  for (const part of parts) {
    acc += '/' + part
    list.push({ name: part, path: acc })
  }
  return list
})

const selectedEntries = computed(() =>
  entries.value.filter((e) => selected.value.has(e.name)),
)
const canGoBack = computed(() => histIdx.value > 0)
const canGoForward = computed(() => histIdx.value < history.value.length - 1)
const canGoUp = computed(() => !!currentPath.value && currentPath.value !== '/')

// ===== 工具函数 =====
function extOf(name: string): string {
  const i = name.lastIndexOf('.')
  return i > 0 ? name.slice(i + 1).toLowerCase() : ''
}
function joinPath(dir: string, name: string): string {
  if (dir.endsWith('/')) return dir + name
  return dir + '/' + name
}
function formatSize(b: number): string {
  if (b < 1024) return `${b} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let v = b / 1024
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(1)} ${units[i]}`
}
function formatDate(ts: number | null): string {
  if (!ts) return '—'
  const d = new Date(ts * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}
function typeName(e: FileEntry): string {
  if (e.is_dir) return '文件夹'
  if (e.is_symlink) return '快捷方式'
  const ext = extOf(e.name)
  return ext ? `${ext.toUpperCase()} 文件` : '文件'
}

// ===== 导航核心 =====
/** 打开目录。push=true 时记入历史（正常导航）。 */
async function openDir(path: string, push = true) {
  if (!path) return
  loading.value = true
  selected.value = new Set()
  renamingName.value = null
  creating.value = false
  try {
    const listing: DirListing = await api.listDir(hostId.value, path)
    currentPath.value = listing.path
    entries.value = listing.entries
    if (push) {
      history.value = history.value.slice(0, histIdx.value + 1)
      history.value.push(listing.path)
      histIdx.value = history.value.length - 1
    }
  } catch (e) {
    ElMessage.error(`打开目录失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function goBack() {
  if (!canGoBack.value) return
  histIdx.value -= 1
  await loadFromHistory(history.value[histIdx.value])
}
async function goForward() {
  if (!canGoForward.value) return
  histIdx.value += 1
  await loadFromHistory(history.value[histIdx.value])
}
async function loadFromHistory(path: string) {
  loading.value = true
  selected.value = new Set()
  try {
    const listing = await api.listDir(hostId.value, path)
    currentPath.value = listing.path
    entries.value = listing.entries
  } catch (e) {
    ElMessage.error(`打开目录失败：${e}`)
  } finally {
    loading.value = false
  }
}
async function goUp() {
  if (!canGoUp.value) return
  const p = currentPath.value
  const idx = p.slice(0, -1).lastIndexOf('/')
  const parent = idx >= 0 ? p.slice(0, idx + 1) : '/'
  await openDir(parent || '/')
}
async function refresh() {
  await openDir(currentPath.value, false)
}

// 路径输入框
function startEditPath() {
  pathInput.value = currentPath.value
  pathEditing.value = true
  nextTick(() => pathInputRef.value?.focus?.())
}
async function commitPath() {
  pathEditing.value = false
  const p = pathInput.value.trim()
  if (p && p !== currentPath.value) await openDir(p)
}
function cancelPath() {
  pathEditing.value = false
}

// ===== 双击进入或预览 =====
function onItemDblClick(e: FileEntry) {
  if (e.is_dir) {
    openDir(joinPath(currentPath.value, e.name))
  } else {
    previewFile(e)
  }
}

async function previewFile(e: FileEntry) {
  const ext = extOf(e.name)
  const textLike = [
    'txt', 'log', 'md', 'json', 'yaml', 'yml', 'conf', 'ini', 'sh',
    'toml', 'xml', 'csv', 'env', 'properties', 'js', 'ts', 'py', 'go', 'rs',
  ]
  if (!textLike.includes(ext)) {
    ElMessage.info('该类型暂不支持预览，请使用下载')
    return
  }
  previewName.value = e.name
  previewVisible.value = true
  previewLoading.value = true
  previewContent.value = ''
  try {
    previewContent.value = await api.fileReadText(hostId.value, joinPath(currentPath.value, e.name))
  } catch (err) {
    previewContent.value = `读取失败：${err}`
  } finally {
    previewLoading.value = false
  }
}

// ===== 选择 =====
function onItemMouseDown(e: FileEntry, ev: MouseEvent) {
  if (ev.ctrlKey || ev.metaKey) {
    if (selected.value.has(e.name)) selected.value.delete(e.name)
    else selected.value.add(e.name)
  } else {
    selected.value = new Set([e.name])
    return
  }
  selected.value = new Set(selected.value)
}
function clearSelection() {
  if (selected.value.size) selected.value = new Set()
}

// ===== 新建（文件夹 / 文件） =====
/** 开始新建文件夹：插入临时项 + 聚焦输入框 + 选中默认名 */
function startCreate() {
  creating.value = true
  creatingValue.value = defaultName()
  selected.value = new Set()
}

// 新建临时项出现时自动聚焦 + 全选默认名（Windows 行为）
watch(creating, (c) => {
  if (!c) return
  nextTick(() => {
    const el = creatingInputRef.value
    if (!el) return
    el.focus()
    el.select()
  })
})
async function commitCreate() {
  const name = creatingValue.value.trim()
  creating.value = false
  if (!name) return
  const fullPath = joinPath(currentPath.value, name)
  try {
    await api.fileMkdir(hostId.value, fullPath)
    ElMessage.success('已创建')
    await refresh()
  } catch (e) {
    ElMessage.error(`创建失败：${e}`)
  }
}
function cancelCreate() {
  creating.value = false
}

function startRename(e: FileEntry) {
  renamingName.value = e.name
  renamingValue.value = e.name
}
async function commitRename(e: FileEntry) {
  const name = renamingValue.value.trim()
  renamingName.value = null
  if (!name || name === e.name) return
  try {
    await api.fileRename(
      hostId.value,
      joinPath(currentPath.value, e.name),
      joinPath(currentPath.value, name),
    )
    ElMessage.success('已重命名')
    await refresh()
  } catch (err) {
    ElMessage.error(`重命名失败：${err}`)
  }
}

async function removeSelected() {
  const items = selectedEntries.value
  if (!items.length) return
  try {
    await ElMessageBox.confirm(
      `确认删除 ${items.length} 个项目？此操作不可撤销。`,
      '删除确认',
      { type: 'warning' },
    )
  } catch {
    return
  }
  let ok = 0
  for (const e of items) {
    try {
      await api.fileRemove(hostId.value, joinPath(currentPath.value, e.name), e.is_dir)
      ok++
    } catch (err) {
      ElMessage.error(`删除「${e.name}」失败：${err}`)
    }
  }
  if (ok) ElMessage.success(`已删除 ${ok} 项`)
  await refresh()
}

async function downloadSelected() {
  const items = selectedEntries.value.filter((e) => !e.is_dir)
  if (!items.length) {
    ElMessage.info('请选择文件（不支持下载文件夹）')
    return
  }
  for (const e of items) {
    const local = await save({ defaultPath: e.name })
    if (!local) continue
    try {
      await api.fileDownload(hostId.value, joinPath(currentPath.value, e.name), local)
      ElMessage.success(`已下载「${e.name}」`)
    } catch (err) {
      ElMessage.error(`下载失败：${err}`)
    }
  }
}

async function upload() {
  const picked = await open({ multiple: true, title: '选择要上传的文件' })
  if (!picked) return
  const files = Array.isArray(picked) ? picked : [picked]
  let ok = 0
  for (const f of files) {
    try {
      await api.fileUpload(hostId.value, f, currentPath.value)
      ok++
    } catch (err) {
      ElMessage.error(`上传「${f}」失败：${err}`)
    }
  }
  if (ok) ElMessage.success(`已上传 ${ok} 个文件`)
  await refresh()
}

// ===== 排序 =====
function toggleSort(key: SortKey) {
  if (sortKey.value === key) sortAsc.value = !sortAsc.value
  else {
    sortKey.value = key
    sortAsc.value = true
  }
}
// 排序下拉
function onSortCommand(cmd: string) {
  if (cmd === 'toggleAsc') {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = cmd as SortKey
  }
}
// 更多下拉：重命名 / 上一级 / 编辑路径
function onMoreCommand(cmd: string) {
  if (cmd === 'rename' && selectedEntries.value.length === 1) {
    startRename(selectedEntries.value[0])
  } else if (cmd === 'up') {
    goUp()
  } else if (cmd === 'editPath') {
    startEditPath()
  }
}

// ===== 初始化 =====
async function init() {
  await store.ensureConnected(hostId.value)
  // 若路由带了 path query（从容器/卷跳转），优先用它
  const qpath = route.query.path as string | undefined
  if (qpath) {
    await openDir(qpath)
    return
  }
  try {
    const home = await api.fileHome(hostId.value)
    await openDir(home)
  } catch (e) {
    ElMessage.error(`无法打开主目录：${e}`)
  }
}

// 路由 query.path 变化时（从其他页跳转进来）重新打开
watch(
  () => route.query.path,
  (p) => {
    if (p && typeof p === 'string' && p !== currentPath.value) openDir(p)
  },
)

onMounted(init)
</script>

<template>
  <div class="explorer">
    <!-- 第一行：导航按钮 + 地址栏 + 搜索 -->
    <div class="navbar">
      <div class="nav-controls">
        <!-- 后退 + 前进 合并一组 -->
        <el-button-group class="nav-group">
          <el-button :icon="Back" :disabled="!canGoBack" @click="goBack" title="后退" />
          <el-button :icon="Right" :disabled="!canGoForward" @click="goForward" title="前进" />
        </el-button-group>
        <!-- 刷新：单独按钮，正常间隔 -->
        <el-button :icon="Refresh" @click="refresh" title="刷新" />
      </div>

      <!-- 地址栏：干净的路径条，面包屑 ↔ 文本输入 -->
      <div
        class="addr-bar"
        :class="{ editing: pathEditing }"
        @click.self="startEditPath"
      >
        <template v-if="!pathEditing">
          <span class="crumb-root" @click="openDir('/')" title="根目录">
            <el-icon><HomeFilled /></el-icon>
          </span>
          <template v-for="(c, i) in breadcrumbs" :key="c.path">
            <span class="crumb-sep">›</span>
            <span
              class="crumb"
              :class="{ active: i === breadcrumbs.length - 1 }"
              @click="openDir(c.path)"
            >{{ c.name }}</span>
          </template>
        </template>
        <el-input
          v-else
          ref="pathInputRef"
          v-model="pathInput"
          class="path-input"
          placeholder="输入绝对路径，如 /root/logs"
          @keyup.enter="commitPath"
          @keyup.esc="cancelPath"
          @blur="commitPath"
        />
      </div>

      <!-- 搜索框 -->
      <el-input
        v-model="search"
        :prefix-icon="Search"
        placeholder="搜索当前文件夹"
        clearable
        size="small"
        class="search-box"
      />
    </div>

    <!-- 第二行：命令工具栏 -->
    <div class="toolbar">
      <div class="left-tools">
        <el-button :icon="Upload" size="small" @click="upload">上传</el-button>
        <el-button :icon="FolderAdd" size="small" @click="startCreate()">新建文件夹</el-button>
        <el-button :icon="Download" size="small" :disabled="!selectedEntries.length" @click="downloadSelected">下载</el-button>
        <el-button :icon="Delete" size="small" type="danger" :disabled="!selectedEntries.length" @click="removeSelected">删除</el-button>
        <!-- 更多：重命名/上一级/打开路径编辑 等次要操作收进下拉 -->
        <el-dropdown trigger="click" @command="onMoreCommand">
          <el-button size="small">
            更多<el-icon class="more-caret"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="rename" :disabled="selectedEntries.length !== 1" :icon="EditPen">重命名</el-dropdown-item>
              <el-dropdown-item command="up" :disabled="!canGoUp" :icon="Top">上一级</el-dropdown-item>
              <el-dropdown-item command="editPath" :icon="EditPen">编辑路径</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
      <div class="right-tools">
        <!-- 排序下拉（图标按钮触发） -->
        <el-dropdown trigger="click" @command="onSortCommand">
          <el-button size="small" :icon="Sort" title="排序" />
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="name" :class="{ 'is-active': sortKey === 'name' }">名称</el-dropdown-item>
              <el-dropdown-item command="modified" :class="{ 'is-active': sortKey === 'modified' }">修改日期</el-dropdown-item>
              <el-dropdown-item command="type" :class="{ 'is-active': sortKey === 'type' }">类型</el-dropdown-item>
              <el-dropdown-item command="size" :class="{ 'is-active': sortKey === 'size' }">大小</el-dropdown-item>
              <el-dropdown-item divided command="toggleAsc">{{ sortAsc ? '降序' : '升序' }}</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <!-- 视图切换 -->
        <el-radio-group v-model="viewMode" size="small">
          <el-radio-button value="detail" title="详细列表"><el-icon><List /></el-icon></el-radio-button>
          <el-radio-button value="large" title="大图标"><el-icon><Grid /></el-icon></el-radio-button>
        </el-radio-group>
      </div>
    </div>

    <!-- 主区 -->
    <div class="main-area" v-loading="loading">
      <el-empty v-if="!loading && !filtered.length" description="文件夹为空" />

      <!-- 大图标视图 -->
      <div v-else-if="viewMode === 'large'" class="icon-grid" @mousedown="clearSelection">
        <!-- 新建文件夹临时项 -->
        <div v-if="creating" class="icon-cell creating">
          <FileIcon name="" :is-dir="true" :size="56" />
          <input
            ref="creatingInputRef"
            v-model="creatingValue"
            class="rename-input native-input"
            @mousedown.stop
            @keyup.enter="commitCreate"
            @keyup.esc="cancelCreate"
            @blur="commitCreate"
          />
        </div>
        <div
          v-for="e in filtered"
          :key="e.name"
          class="icon-cell"
          :class="{ selected: selected.has(e.name) }"
          @mousedown.stop="onItemMouseDown(e, $event)"
          @dblclick="onItemDblClick(e)"
        >
          <FileIcon :name="e.name" :is-dir="e.is_dir" :size="56" />
          <el-input
            v-if="renamingName === e.name"
            v-model="renamingValue"
            size="small"
            class="rename-input"
            autofocus
            @mousedown.stop
            @keyup.enter="commitRename(e)"
            @blur="commitRename(e)"
          />
          <div v-else class="icon-name" :title="e.name">{{ e.name }}</div>
        </div>
      </div>

      <!-- 详细列表视图 -->
      <div v-else class="detail-view" @mousedown="clearSelection">
        <div class="detail-header">
          <span class="col-name sortable" @click="toggleSort('name')">
            名称
            <i v-if="sortKey === 'name'" class="sort-arrow">{{ sortAsc ? '▲' : '▼' }}</i>
          </span>
          <span class="col-date sortable" @click="toggleSort('modified')">
            修改日期
            <i v-if="sortKey === 'modified'" class="sort-arrow">{{ sortAsc ? '▲' : '▼' }}</i>
          </span>
          <span class="col-type sortable" @click="toggleSort('type')">
            类型
            <i v-if="sortKey === 'type'" class="sort-arrow">{{ sortAsc ? '▲' : '▼' }}</i>
          </span>
          <span class="col-size sortable" @click="toggleSort('size')">
            大小
            <i v-if="sortKey === 'size'" class="sort-arrow">{{ sortAsc ? '▲' : '▼' }}</i>
          </span>
        </div>

        <div class="detail-body">
          <!-- 新建文件夹临时行，对齐表头列 -->
          <div v-if="creating" class="detail-row creating-row">
            <span class="col-icon"><FileIcon name="" :is-dir="true" :size="20" /></span>
            <span class="col-name">
              <input
                ref="creatingInputRef"
                v-model="creatingValue"
                class="native-input"
                @mousedown.stop
                @keyup.enter="commitCreate"
                @keyup.esc="cancelCreate"
                @blur="commitCreate"
              />
            </span>
            <span class="col-date"></span>
            <span class="col-type">文件夹</span>
            <span class="col-size"></span>
          </div>
          <div
            v-for="e in filtered"
            :key="e.name"
            class="detail-row"
            :class="{ selected: selected.has(e.name) }"
            @mousedown.stop="onItemMouseDown(e, $event)"
            @dblclick="onItemDblClick(e)"
          >
            <span class="col-icon"><FileIcon :name="e.name" :is-dir="e.is_dir" :size="20" /></span>
            <span class="col-name">
              <el-input
                v-if="renamingName === e.name"
                v-model="renamingValue"
                size="small"
                autofocus
                @mousedown.stop
                @keyup.enter="commitRename(e)"
                @blur="commitRename(e)"
              />
              <span v-else :title="e.name">{{ e.name }}</span>
            </span>
            <span class="col-date">{{ formatDate(e.modified) }}</span>
            <span class="col-type">{{ typeName(e) }}</span>
            <span class="col-size">{{ e.is_dir ? '' : formatSize(e.size) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="statusbar">
      <span>{{ filtered.length }} 项</span>
      <span v-if="selectedEntries.length" class="muted">· 已选择 {{ selectedEntries.length }} 项</span>
    </div>

    <!-- 文本预览抽屉 -->
    <el-drawer
      v-model="previewVisible"
      :title="previewName"
      size="55%"
      direction="rtl"
    >
      <pre v-loading="previewLoading" class="preview-pre">{{ previewContent }}</pre>
    </el-drawer>
  </div>
</template>

<script lang="ts">
import {
  Back, Right, Top, HomeFilled, EditPen, Search, Refresh,
  FolderAdd, Upload, Download, Delete, Grid, List, ArrowDown, Sort,
} from '@element-plus/icons-vue'
export default {
  components: {
    Back, Right, Top, HomeFilled, EditPen, Search, Refresh,
    FolderAdd, Upload, Download, Delete, Grid, List, ArrowDown, Sort,
  },
}
</script>

<style scoped>
.explorer {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  background: var(--el-bg-color);
}

/* 导航条 */
.navbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border-bottom: 1px solid var(--el-border-color);
}
.nav-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
/* 导航区图标按钮（后退/前进/刷新）统一为方形、紧凑 */
.nav-controls .el-button {
  padding: 0;
  width: 30px;
  height: 30px;
  min-height: 30px;
}
.nav-group .el-button {
  /* 后退/前进合并后去掉中间圆角，更像 Win11 */
  border-radius: 0;
}
.more-caret {
  margin-left: 2px;
  font-size: 12px;
}
.addr-bar {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: 32px;
  padding: 0 8px;
  background: var(--el-fill-color-light);
  border: 1px solid var(--el-border-color);
  border-radius: 6px;
  cursor: text;
  transition: border-color 0.15s, background 0.15s;
  overflow: hidden;
}
.addr-bar:hover {
  background: var(--el-fill-color);
  border-color: var(--el-border-color-hover);
}
/* 编辑/聚焦时整条加主题色细边 */
.addr-bar.editing {
  border-color: var(--el-color-primary);
  background: var(--el-bg-color);
}
/* 编辑态的 input 完全透明融入外层条，不产生第二层边框 */
.addr-bar :deep(.path-input) {
  --el-input-bg-color: transparent;
  --el-input-border-color: transparent;
  --el-input-hover-border-color: transparent;
  --el-input-focus-border-color: transparent;
  width: 100%;
}
.addr-bar :deep(.path-input .el-input__wrapper) {
  background: transparent;
  box-shadow: none !important;
  padding: 0;
}
.addr-bar :deep(.path-input .el-input__inner) {
  color: var(--el-text-color-primary);
  height: 30px;
  line-height: 30px;
}
.crumb-root {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  color: var(--el-text-color-secondary);
}
.crumb-root:hover { color: var(--el-color-primary); }
.crumb-sep {
  color: var(--el-text-color-placeholder);
  margin: 0 2px;
  flex-shrink: 0;
}
.crumb {
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
  color: var(--el-text-color-regular);
  white-space: nowrap;
}
.crumb:hover { background: var(--el-fill-color-dark); color: var(--el-color-primary); }
.crumb.active { color: var(--el-text-color-primary); font-weight: 600; }
.search-box { width: 220px; }

/* 工具栏 */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 12px;
  border-bottom: 1px solid var(--el-border-color);
  gap: 8px;
}
.left-tools { display: flex; align-items: center; gap: 8px; }
.right-tools { display: flex; align-items: center; gap: 8px; }
/* 覆盖 element-plus 的 .el-button + .el-button { margin-left: 12px }，
   该默认外边距会让 flex gap 之外再叠加间距，导致按钮间隔不对等。
   这里统一交给父级 gap: 8px 控制，所有相邻按钮间隔完全一致。 */
.left-tools .el-button + .el-button,
.right-tools .el-button + .el-button {
  margin-left: 0;
}
/* el-dropdown 默认 inline，确保作为 flex 子项正确参与 gap */
.left-tools :deep(.el-dropdown),
.right-tools :deep(.el-dropdown) {
  display: inline-flex;
}
/* 排序/间隔下拉菜单里的当前项高亮 */
:deep(.el-dropdown-menu .is-active) {
  color: var(--el-color-primary);
  font-weight: 600;
}

/* 主区 */
.main-area {
  flex: 1;
  overflow: auto;
  padding: 4px;
}

/* 大图标视图 */
.icon-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 6px;
  padding: 8px;
  align-content: start;
  height: 100%;
}
.icon-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px;
  border-radius: 6px;
  cursor: default;
  user-select: none;
}
.icon-cell:hover { background: var(--el-fill-color-light); }
.icon-cell.selected {
  background: var(--el-color-primary-light-9);
  outline: 1px solid var(--el-color-primary-light-5);
}
.icon-cell.creating {
  background: var(--el-color-primary-light-9);
  outline: 1px solid var(--el-color-primary-light-5);
}
.icon-name {
  font-size: 12px;
  text-align: center;
  max-width: 90px;
  word-break: break-all;
  line-height: 1.3;
}
.rename-input { width: 88px; }

/* 原生输入框（新建临时项用，避免 el-input 在小尺寸下样式问题） */
.native-input {
  width: 100%;
  min-width: 0;
  max-width: 140px;
  height: 26px;
  padding: 0 6px;
  font-size: 12px;
  border: 1px solid var(--el-color-primary);
  border-radius: 3px;
  outline: none;
  background: var(--el-bg-color);
  color: var(--el-text-color-primary);
}
.icon-cell .native-input { width: 88px; text-align: center; }

/* 详细列表视图 */
.detail-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.detail-header {
  display: grid;
  grid-template-columns: 1fr 160px 110px 90px;
  padding: 6px 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  border-bottom: 1px solid var(--el-border-color);
  position: sticky;
  top: 0;
  background: var(--el-bg-color);
  z-index: 1;
}
.detail-header .col-name { display: flex; }
.sortable { cursor: pointer; display: flex; align-items: center; gap: 4px; }
.sortable:hover { color: var(--el-color-primary); }
.sort-arrow { font-size: 10px; font-style: normal; }

.detail-body { flex: 1; }
.detail-row {
  display: grid;
  grid-template-columns: 24px 1fr 160px 110px 90px;
  align-items: center;
  padding: 4px 12px 4px 4px;
  font-size: 13px;
  cursor: default;
  user-select: none;
  border-radius: 3px;
}
.detail-row .col-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.detail-row .col-date,
.detail-row .col-type,
.detail-row .col-size {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.detail-row .col-icon {
  display: inline-flex;
  justify-content: center;
}
.detail-row:hover { background: var(--el-fill-color-light); }
.detail-row.selected {
  background: var(--el-color-primary-light-9);
}
.creating-row { background: var(--el-fill-color); }

/* 状态栏 */
.statusbar {
  display: flex;
  gap: 6px;
  padding: 4px 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  border-top: 1px solid var(--el-border-color);
}
.muted { color: var(--el-text-color-placeholder); }

/* 预览 */
.preview-pre {
  font-family: var(--el-font-family-mono, monospace);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  padding: 8px;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  min-height: 200px;
}
</style>
