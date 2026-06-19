/**
 * 路由配置。
 * 布局（标题栏+侧栏+工具条）由 App.vue 常驻提供，路由只切换内容区。
 *   /hosts                    —— 主机管理（增删改，从 ActivityBar 进入）
 *   /host/:id/dashboard       —— 概览（主机级）
 *   /host/:id/containers      —— 容器（主机级）
 *   /host/:id/containers/:cid/logs  —— 容器日志
 *   /host/:id/images          —— 镜像（主机级）
 *   /registries /templates /config —— 全局页（应用级，不跟主机走，ActivityBar 进入）
 */
import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/hosts' },
    {
      // 独立终端窗口（带 tabs，承接主窗口投递的终端）
      path: '/terminal-window',
      name: 'terminal-window',
      component: () => import('@/views/terminal/TerminalWindowView.vue'),
    },
    {
      path: '/hosts',
      name: 'hosts',
      component: () => import('@/views/hosts/HostList.vue'),
    },
    {
      path: '/host/:id/dashboard',
      name: 'dashboard',
      component: () => import('@/views/dashboard/DashboardView.vue'),
    },
    {
      path: '/host/:id/containers',
      name: 'containers',
      component: () => import('@/views/containers/ContainerList.vue'),
    },
    {
      path: '/host/:id/containers/:cid/logs',
      name: 'container-logs',
      component: () => import('@/views/containers/ContainerLogs.vue'),
    },
    {
      path: '/host/:id/images',
      name: 'images',
      component: () => import('@/views/images/ImageList.vue'),
    },
    {
      path: '/host/:id/compose',
      name: 'compose',
      component: () => import('@/views/compose/ComposeList.vue'),
    },
    {
      path: '/host/:id/networks',
      name: 'networks',
      component: () => import('@/views/networks/NetworkList.vue'),
    },
    {
      path: '/host/:id/volumes',
      name: 'volumes',
      component: () => import('@/views/volumes/VolumeList.vue'),
    },
    {
      path: '/host/:id/files',
      name: 'files',
      component: () => import('@/views/files/FileExplorer.vue'),
    },
    // ===== 全局页面（应用级，不跟主机走）=====
    {
      path: '/registries',
      name: 'registries',
      component: () => import('@/views/dev/ComingSoon.vue'),
      props: { title: '仓库：私有仓库登录信息管理（开发中）' },
    },
    {
      path: '/templates',
      name: 'templates',
      component: () => import('@/views/dev/ComingSoon.vue'),
      props: { title: '项目模板：保存常用 compose 模板，一键部署（开发中）' },
    },
    {
      path: '/config',
      name: 'config',
      component: () => import('@/views/dev/ComingSoon.vue'),
      props: { title: '配置：应用全局设置（开发中）' },
    },
  ],
})

export default router
