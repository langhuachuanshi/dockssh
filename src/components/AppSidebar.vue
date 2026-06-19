<script setup lang="ts">
/**
 * 导航栏（内容区左侧）：当前主机的页面导航。
 *
 * 当前主机信息（名称/状态/地址）由顶部 HostTabs 承载，本栏只放纯导航菜单。
 * 仅在选中主机(路由带 :id)时出现。
 */
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

// 当前路由的主机 id
const currentHostId = computed(() => (route.params.id as string) || '')
const activeMenu = computed(() => route.name as string)

// 导航：主机级资源（容器/项目/镜像/网络/存储卷/文件）。
// 全局页（仓库/项目模板/配置）已移至 ActivityBar，不跟主机走。
const menus = [
  { name: 'dashboard', label: '概览', icon: 'Odometer', disabled: false },
  { name: 'containers', label: '容器', icon: 'Box', disabled: false },
  { name: 'compose', label: '项目', icon: 'Connection', disabled: false },
  { name: 'images', label: '镜像', icon: 'Files', disabled: false },
  { name: 'networks', label: '网络', icon: 'Share', disabled: false },
  { name: 'volumes', label: '存储卷', icon: 'Coin', disabled: false },
  { name: 'files', label: '文件', icon: 'FolderOpened', disabled: false },
]

function go(name: string) {
  const m = menus.find((x) => x.name === name)
  if (m?.disabled) return
  router.push({ name, params: { id: currentHostId.value } })
}
</script>

<template>
  <aside class="sidebar">
    <el-menu
      :default-active="activeMenu"
      class="nav"
      @select="go"
    >
      <el-menu-item
        v-for="m in menus"
        :key="m.name"
        :index="m.name"
        :disabled="m.disabled"
      >
        <el-icon><component :is="m.icon" /></el-icon>
        <span>{{ m.label }}</span>
        <el-tag v-if="m.disabled" size="small" effect="plain" class="dev-tag">开发中</el-tag>
      </el-menu-item>
    </el-menu>
  </aside>
</template>

<script lang="ts">
import {
  Odometer,
  Box,
  Connection,
  Files,
  Share,
  Coin,
  FolderOpened,
} from '@element-plus/icons-vue'
export default {
  components: {
    Odometer, Box, Connection, Files,
    Share, Coin, FolderOpened,
  },
}
</script>

<style scoped>
.sidebar {
  width: 200px;
  background: var(--el-bg-color);
  flex-shrink: 0;
  overflow-y: auto;
  overflow-x: hidden;
}
.nav {
  border-right: none;
  padding-top: 8px;
}
.dev-tag {
  margin-left: auto;
  transform: scale(0.85);
}
</style>
