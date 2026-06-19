<script setup lang="ts">
/**
 * 应用根布局：tabs 现在并入标题栏，省掉一行高度。
 * ┌──────────────────────────────────────────┐
 * │ ▣ DockSSH │ ┌prod×┐ staging×      - □ × │  TitleBar(含 tabs)
 * ├────┬──────┬──────────────────────────────┤
 * │    │      │                              │
 * │轨道│ 导航栏│       内容区 router-view      │
 * │    │      │                              │
 * └────┴──────┴──────────────────────────────┘
 * - 活动栏(轨道)常驻，放「连接管理器」入口
 * - tabs 已移入标题栏（HostTabs 组件），由 TitleBar 控制
 * - 导航栏仅在已选中主机(路由带 :id)时显示
 * - 主机管理页(/hosts)时只有轨道 + 内容区，铺满
 */
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import TitleBar from '@/components/TitleBar.vue'
import ActivityBar from '@/components/ActivityBar.vue'
import AppSidebar from '@/components/AppSidebar.vue'
import TerminalPanel from '@/components/TerminalPanel.vue'
import { useHostsStore } from '@/store/hosts'
import { useTabsStore } from '@/store/tabs'

const route = useRoute()
const store = useHostsStore()
const tabsStore = useTabsStore()

// 当前路由是否选中了某台主机（导航栏显示依据）
const hasHostContext = computed(() => !!route.params.id)

onMounted(async () => {
  await store.refresh()
  // 刷新后若路由指向某主机，把它补进 tabs（保持 tabs 一致）
  const id = route.params.id as string | undefined
  if (id && !tabsStore.tabs.includes(id)) {
    tabsStore.open(id)
  }
})
</script>

<template>
  <div class="app-shell">
    <TitleBar />
    <div class="body">
      <ActivityBar />
      <AppSidebar v-if="hasHostContext" />
      <div class="main">
        <div class="content">
          <router-view v-slot="{ Component, route }">
            <keep-alive :include="['DashboardView', 'ContainerList', 'ImageList']">
              <component :is="Component" :key="route.fullPath" />
            </keep-alive>
          </router-view>
        </div>
        <!-- 终端面板：底部抽屉，非模态，与路由平级以保证会话保活 -->
        <TerminalPanel />
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
