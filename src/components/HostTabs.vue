<script setup lang="ts">
/**
 * 顶部主机标签行（嵌入标题栏）：直接使用 el-tabs 原版样式。
 * - 每个已打开主机一个 tab：状态点 + 名称
 * - 点 tab 切换激活主机（跳到该主机的概览）
 * - tab 上的 × 关闭（@tab-remove）
 * - 拖拽 tab 可重排序（HTML5 拖拽，委托到 el-tabs__nav）
 *
 * 主机管理页(/hosts)时本组件不显示。
 */
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useTabsStore } from '@/store/tabs'
import { useHostsStore } from '@/store/hosts'

const router = useRouter()
const tabsStore = useTabsStore()
const hostsStore = useHostsStore()

// el-tabs v-model 绑定当前激活的 host id
const activeId = computed<string>({
  get: () => tabsStore.activeId || '',
  set: (val) => val && tabsStore.setActive(val),
})

// tabs 中每个 host id → host 信息
const tabHosts = computed(() =>
  tabsStore.tabs
    .map((id) => hostsStore.hosts.find((h) => h.id === id))
    .filter((h): h is NonNullable<typeof h> => !!h),
)

// 点击 tab：切换激活主机
function onTabClick(name: string | number) {
  const id = String(name)
  if (tabsStore.activeId === id) return
  tabsStore.setActive(id)
  router.push({ name: 'dashboard', params: { id } })
}

// 关闭 tab（点 ×）
function onTabRemove(name: string | number) {
  const id = String(name)
  const next = tabsStore.close(id)
  if (next) {
    router.push({ name: 'dashboard', params: { id: next } })
  } else {
    // 全部关闭 → 回主机管理页
    router.push({ name: 'hosts' })
  }
}

// ===== 拖拽排序 =====
// el-tabs 的 tab 项没有 draggable，挂载后手动给它加上；事件委托到 nav 容器。
const navRef = ref<HTMLElement | null>(null)
const draggingIndex = ref<number | null>(null)

/** 给当前所有 .el-tabs__item 标记 draggable */
function enableItemDraggable() {
  const nav = navRef.value
  if (!nav) return
  nav.querySelectorAll('.el-tabs__item').forEach((el) => {
    ;(el as HTMLElement).draggable = true
  })
}

function itemIndexFromEvent(e: DragEvent): number {
  const target = (e.target as HTMLElement)?.closest('.el-tabs__item')
  if (!target || !navRef.value) return -1
  const items = [...navRef.value.querySelectorAll('.el-tabs__item')]
  return items.indexOf(target)
}

function onDragStart(e: DragEvent) {
  const idx = itemIndexFromEvent(e)
  if (idx < 0) return
  draggingIndex.value = idx
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    // Firefox 需要 setData 才能触发拖拽
    e.dataTransfer.setData('text/plain', String(idx))
  }
}

function onDragOver(e: DragEvent) {
  if (draggingIndex.value === null) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  if (draggingIndex.value === null) return
  const to = itemIndexFromEvent(e)
  if (to >= 0) {
    tabsStore.move(draggingIndex.value, to)
  }
  draggingIndex.value = null
}

function onDragEnd() {
  draggingIndex.value = null
}

onMounted(() => {
  nextTick(enableItemDraggable)
})
// tabs 数量变化时（新增/关闭），新 tab 项也要标记 draggable
watch(
  () => tabsStore.tabs.length,
  () => nextTick(enableItemDraggable),
)
</script>

<template>
  <div class="host-tabs">
    <div
      ref="navRef"
      class="tabs-nav-host"
      @dragstart="onDragStart"
      @dragover="onDragOver"
      @drop="onDrop"
      @dragend="onDragEnd"
    >
      <el-tabs
        v-model="activeId"
        @tab-click="(t) => onTabClick(t.paneName as string)"
        @tab-remove="onTabRemove"
      >
        <el-tab-pane
          v-for="h in tabHosts"
          :key="h.id"
          :name="h.id"
          :closable="true"
        >
          <template #label>
            <span class="tab-label">
              <el-icon
                class="dot"
                :color="hostsStore.isOnline(h.id) ? 'var(--el-color-success)' : 'var(--el-text-color-secondary)'"
              >
                <component :is="hostsStore.isOnline(h.id) ? 'CircleCheckFilled' : 'CircleClose'" />
              </el-icon>
              <span class="name">{{ h.name }}</span>
            </span>
          </template>
        </el-tab-pane>
      </el-tabs>
    </div>
  </div>
</template>

<script lang="ts">
import { CircleCheckFilled, CircleClose } from '@element-plus/icons-vue'
export default { components: { CircleCheckFilled, CircleClose } }
</script>

<style scoped>
.host-tabs {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: stretch;
  background: transparent;
}
.tabs-nav-host {
  flex: 1;
  min-width: 0;
  display: flex;
}
/* el-tabs header 高度填满标题栏（48px），tab 底部对齐 */
.host-tabs :deep(.el-tabs) {
  --el-tabs-header-height: 48px;
  width: 100%;
}
.host-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 0 8px;
  border: none;
}
.host-tabs :deep(.el-tabs__nav-wrap)::after {
  /* 去掉 el-tabs 默认底部分隔线，标题栏整体观感 */
  display: none;
}
/* 隐藏 tab 内容区（本组件只用作切换器，内容由路由控制） */
.host-tabs :deep(.el-tabs__content) {
  display: none;
}
/* tab 项可拖拽时的光标 */
.host-tabs :deep(.el-tabs__item) {
  cursor: grab;
}
.host-tabs :deep(.el-tabs__item):active {
  cursor: grabbing;
}
.tab-label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.dot {
  font-size: 12px;
}
.name {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
