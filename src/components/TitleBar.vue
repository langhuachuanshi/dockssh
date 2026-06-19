<script setup lang="ts">
/**
 * 自定义标题栏（高度 48）。
 * 三段结构：左 logo + 中 tabs + 右窗口按钮。
 * - logo 区与 tabs 间隙区可拖动移动窗口（data-tauri-drag-region）
 * - tabs 嵌入标题栏中部，tab 本身不标记 drag，确保可点击
 * - 按钮放在非 drag 容器里，确保点击不被拖动拦截
 *
 * 拖动机制：Tauri v2 会把带 data-tauri-drag-region 的元素作为拖动热区。
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
</script>

<template>
  <div class="titlebar">
    <!-- 左：logo（可拖动） -->
    <div class="logo-area" data-tauri-drag-region>
      <el-icon class="logo"><Box /></el-icon>
      <span class="name">DockSSH</span>
    </div>

    <!-- 中：tabs（有 tab 时显示，否则该区可拖动作空白热区） -->
    <div class="tabs-area">
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
/* tabs 区：占满中间，有 tab 时放 tabs，无 tab 时作拖动热区 */
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
