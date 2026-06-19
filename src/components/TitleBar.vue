<script setup lang="ts">
/**
 * 自定义标题栏（高度 48）。
 * 三段结构：左 logo + 中 tabs + 右窗口按钮。
 *
 * 拖动机制（双重保险）：
 * 1. data-tauri-drag-region：Tauri 原生拖动热区，logo 区与 tabs 空白区都用
 * 2. onTabsAreaMouseDown：el-tabs 内部 DOM 可能撑满盖住 drag region，
 *    用 JS 兜底——鼠标按下 tabs-area 时，若命中的不是 tab 项本身，
 *    则调用 startDragging() 主动触发拖窗。点击 tab 不受影响。
 */
import { computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, FullScreen, Close } from '@element-plus/icons-vue'
import HostTabs from '@/components/HostTabs.vue'
import { useTabsStore } from '@/store/tabs'

const win = getCurrentWindow()
const tabsStore = useTabsStore()
const hasTabs = computed(() => tabsStore.tabs.length > 0)

async function minimize() {
  await win.minimize()
}

async function toggleMaximize() {
  await win.toggleMaximize()
}

async function close() {
  await win.close()
}

/**
 * tabs-area 鼠标按下兜底拖窗。
 * el-tabs 的内部容器（__header / __nav-wrap）会撑满整个 tabs-area，
 * 盖住父级的 data-tauri-drag-region，导致空白处拖不动。
 * 这里判断：若点击的不是 tab 项（.el-tabs__item）及其子元素，
 * 就主动 startDragging()，确保 tab 间空白与两侧空白都能拖动窗口。
 * 仅响应主键（button===0），避免右键菜单被吞。
 */
async function onTabsAreaMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement | null
  // 命中 tab 项本身（含关闭×、状态点等子元素）则不拖动，交给 el-tabs 处理
  if (target?.closest('.el-tabs__item')) return
  try {
    await win.startDragging()
  } catch {
    /* 拖动失败忽略（可能权限或平台差异） */
  }
}
</script>

<template>
  <div class="titlebar">
    <!-- 左：logo（可拖动） -->
    <div class="logo-area" data-tauri-drag-region>
      <el-icon class="logo"><Box /></el-icon>
      <span class="name">DockSSH</span>
    </div>

    <!-- 中：tabs。data-tauri-drag-region 标原生热区；onTabsAreaMouseDown
         兜底处理 el-tabs 撑满盖住热区的情况，确保空白处始终可拖窗。 -->
    <div
      class="tabs-area"
      data-tauri-drag-region
      @mousedown="onTabsAreaMouseDown"
    >
      <HostTabs v-if="hasTabs" />
      <div v-else class="tabs-empty" data-tauri-drag-region />
    </div>

    <!-- 右：窗口按钮（不拖动） -->
    <div class="win-controls">
      <button class="ctrl-btn" title="最小化" @click="minimize">
        <el-icon><Minus /></el-icon>
      </button>
      <button class="ctrl-btn" title="最大化" @click="toggleMaximize">
        <el-icon><FullScreen /></el-icon>
      </button>
      <button class="ctrl-btn close" title="关闭" @click="close">
        <el-icon><Close /></el-icon>
      </button>
    </div>
  </div>
</template>

<script lang="ts">
import { Box } from '@element-plus/icons-vue'
export default { components: { Box, Minus, FullScreen, Close } }
</script>

<style scoped>
.titlebar {
  height: 48px;
  display: flex;
  align-items: stretch;
  background: var(--el-bg-color);
  flex-shrink: 0;
  user-select: none;
}
.logo-area {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 14px 0 16px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  flex-shrink: 0;
}
.logo {
  font-size: 18px;
  color: var(--el-color-primary);
}
/* tabs 区：占满中间，整个标记为窗口拖动热区。HostTabs 只占内容宽度，
   两侧及 tab 间空白透出此 drag region 实现拖窗；tab 点击/关闭/排序
   因 tab 元素自身无 drag 属性而不受影响。 */
.tabs-area {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: stretch;
}
.tabs-empty {
  flex: 1;
}
/* 按钮区不标记 drag，确保可点击 */
.win-controls {
  display: flex;
  height: 100%;
  flex-shrink: 0;
}
.ctrl-btn {
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  color: var(--el-text-color-regular);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s;
}
.ctrl-btn:hover {
  background: var(--el-fill-color-light);
}
.ctrl-btn.close:hover {
  background: var(--el-color-danger);
  color: #fff;
}
</style>
