<script setup lang="ts">
/**
 * 最左侧活动栏（图标轨道，宽 48px）。
 * 放置各功能入口图标，点击切换对应视图。
 *
 * 上半部分（主机）：连接管理器
 * 下半部分（全局应用）：仓库 / 项目模板 / 配置 —— 不跟主机走
 */
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

function isActive(routeName: string) {
  return route.name === routeName
}

// 主机相关入口
const hostItems = [{ key: 'hosts', label: '连接管理器', icon: 'Connection' }]
// 全局应用入口（不跟主机走）
const globalItems = [
  { key: 'registries', label: '仓库', icon: 'Files' },
  { key: 'templates', label: '项目模板', icon: 'Document' },
  { key: 'config', label: '配置', icon: 'Setting' },
]

function onClick(key: string) {
  router.push({ name: key })
}
</script>

<template>
  <nav class="activity-bar">
    <div class="group">
      <button
        v-for="item in hostItems"
        :key="item.key"
        class="bar-item"
        :class="{ active: isActive(item.key) }"
        :title="item.label"
        @click="onClick(item.key)"
      >
        <el-icon><component :is="item.icon" /></el-icon>
      </button>
    </div>

    <div class="divider" />

    <div class="group">
      <button
        v-for="item in globalItems"
        :key="item.key"
        class="bar-item"
        :class="{ active: isActive(item.key) }"
        :title="item.label"
        @click="onClick(item.key)"
      >
        <el-icon><component :is="item.icon" /></el-icon>
      </button>
    </div>
  </nav>
</template>

<script lang="ts">
import { Connection, Files, Document, Setting } from '@element-plus/icons-vue'
export default { components: { Connection, Files, Document, Setting } }
</script>

<style scoped>
.activity-bar {
  width: 48px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 0;
  gap: 4px;
  background: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
}
.group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}
.divider {
  width: 24px;
  height: 1px;
  background: var(--el-border-color);
  margin: 6px 0;
}
.bar-item {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--el-text-color-secondary);
  cursor: pointer;
  border-radius: 8px;
  position: relative;
  transition: color 0.15s, background 0.15s;
}
.bar-item:hover {
  color: var(--el-text-color-primary);
  background: var(--el-fill-color-light);
}
.bar-item.active {
  color: var(--el-color-primary);
}
/* 激活态左侧竖条，VS Code 风格 */
.bar-item.active::before {
  content: '';
  position: absolute;
  left: -8px;
  top: 8px;
  bottom: 8px;
  width: 2px;
  border-radius: 1px;
  background: var(--el-color-primary);
}
.bar-item .el-icon {
  font-size: 20px;
}
</style>
