/**
 * 路由配置。
 * 布局（标题栏+侧栏+工具条）由 App.vue 常驻提供，路由只切换内容区。
 *   /hosts                    —— 主机管理（增删改，从侧栏"管理"进入）
 *   /host/:id/dashboard       —— 概览
 *   /host/:id/containers      —— 容器
 *   /host/:id/containers/:cid/logs  —— 容器日志
 *   /host/:id/images          —— 镜像
 */
import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/hosts' },
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
  ],
})

export default router
