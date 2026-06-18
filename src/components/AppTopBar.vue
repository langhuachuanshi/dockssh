<script setup lang="ts">
/**
 * 内容区顶部工具条：左侧页面标题，右侧刷新 / 搜索 / 设置。
 * 搜索和设置暂为占位（后续接容器搜索、全局设置）。
 */
import { Refresh, Search, Setting } from '@element-plus/icons-vue'

defineProps<{ title: string }>()

function emitRefresh() {
  // 触发全局刷新事件，由内容页监听
  // 第一版先用最简：派发自定义事件，页面可按需监听
  window.dispatchEvent(new CustomEvent('dsh-refresh'))
}
</script>

<template>
  <div class="topbar">
    <span class="title">{{ title }}</span>
    <div class="actions">
      <el-input
        :prefix-icon="Search"
        placeholder="搜索"
        class="search"
        size="small"
        disabled
      />
      <el-button :icon="Refresh" circle size="small" @click="emitRefresh" />
      <el-button :icon="Setting" circle size="small" disabled />
    </div>
  </div>
</template>

<script lang="ts">
export default { components: { Refresh, Search, Setting } }
</script>

<style scoped>
.topbar {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  border-bottom: 1px solid var(--el-border-color);
  background: var(--el-bg-color);
  flex-shrink: 0;
}
.title {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.search {
  width: 200px;
}
</style>
