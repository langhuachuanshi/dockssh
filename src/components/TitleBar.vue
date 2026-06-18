<script setup lang="ts">
/**
 * 自定义标题栏。
 * - 左侧：DockSSH logo + 名称
 * - 右侧：最小化 / 最大化 / 关闭 按钮
 * - 整个标题栏可拖动移动窗口（data-tauri-drag-region），
 *   按钮区域阻止冒泡，避免点按钮触发拖动。
 */
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, FullScreen, Close } from '@element-plus/icons-vue'

const win = getCurrentWindow()

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
  <div class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <el-icon class="logo"><Box /></el-icon>
      <span class="name">DockSSH</span>
    </div>

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
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color);
  flex-shrink: 0;
  user-select: none;
}
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 14px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.logo {
  font-size: 16px;
  color: var(--el-color-primary);
}
.win-controls {
  display: flex;
  height: 100%;
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
