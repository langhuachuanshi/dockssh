<script setup lang="ts">
/**
 * 应用根布局：常驻三段结构。
 * ┌─────────────────────────────────┐
 * │           自定义标题栏            │  TitleBar
 * ├──────────┬──────────────────────┤
 * │          │     顶部工具条         │  TopBar
 * │   侧栏    ├──────────────────────┤
 * │ (主机+导航)│                      │
 * │          │     内容区 router-view │
 * └──────────┴──────────────────────┘
 */
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import TitleBar from '@/components/TitleBar.vue'
import AppSidebar from '@/components/AppSidebar.vue'
import AppTopBar from '@/components/AppTopBar.vue'
import { useHostsStore } from '@/store/hosts'

const route = useRoute()
const store = useHostsStore()

// 顶部工具条显示的标题（按当前路由名映射）
const pageTitle = computed(() => {
  const map: Record<string, string> = {
    dashboard: '概览',
    containers: '容器',
    'container-logs': '容器日志',
    images: '镜像',
  }
  return map[route.name as string] || 'DockSSH'
})

onMounted(() => {
  store.refresh()
})
</script>

<template>
  <div class="app-shell">
    <TitleBar />
    <div class="body">
      <AppSidebar />
      <div class="main">
        <AppTopBar :title="pageTitle" />
        <div class="content">
          <router-view />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
}
.body {
  flex: 1;
  display: flex;
  overflow: hidden;
}
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
