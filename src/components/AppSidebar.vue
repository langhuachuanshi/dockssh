<script setup lang="ts">
/**
 * 应用常驻侧栏：
 * - 顶部：当前主机切换器（下拉列出所有主机，含在线状态 + 添加入口）
 * - 下方：导航菜单（概览/容器/镜像）
 *
 * 未选中主机或主机未连接时，导航项禁用并提示先连接主机。
 */
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { useHostsStore } from '@/store/hosts'
import * as api from '@/api'
import HostFormDialog from '@/views/hosts/HostFormDialog.vue'
import { ref } from 'vue'

const route = useRoute()
const router = useRouter()
const store = useHostsStore()

const dialogVisible = ref(false)
const connecting = ref(false)

// 当前路由的主机 id
const currentHostId = computed(() => (route.params.id as string) || '')
const currentHost = computed(
  () => store.hosts.find((h) => h.id === currentHostId.value) || null,
)
const activeMenu = computed(() => route.name as string)

// 选中主机后可用的导航
const menus = [
  { name: 'dashboard', label: '概览', icon: 'Odometer' },
  { name: 'containers', label: '容器', icon: 'Box' },
  { name: 'images', label: '镜像', icon: 'Files' },
]

// 菜单是否可用：必须选中且在线的主机
const menuEnabled = computed(
  () => !!currentHostId.value && store.isOnline(currentHostId.value),
)

// 切换主机：下拉选中某主机 → 连接 → 跳概览
async function switchHost(id: string) {
  if (id === currentHostId.value) return
  connecting.value = true
  try {
    if (!store.isOnline(id)) {
      const res = await store.connect(id)
      ElMessage.success(`已连接：${res.probe.hostname}`)
    }
    store.select(id)
    router.push({ name: 'dashboard', params: { id } })
  } catch (e) {
    ElMessage.error(`连接失败：${e}`)
  } finally {
    connecting.value = false
  }
}

// 下拉菜单：主机列表 + 添加
function handleCommand(cmd: string) {
  if (cmd === 'add') {
    dialogVisible.value = true
  } else {
    switchHost(cmd)
  }
}

async function onSaved() {
  dialogVisible.value = false
  await store.refresh()
}

// 断开当前主机
async function disconnectCurrent() {
  if (!currentHostId.value) return
  try {
    await api.disconnectHost(currentHostId.value)
    store.onlineMap[currentHostId.value] = false
    ElMessage.success('已断开')
  } catch (e) {
    ElMessage.error(`断开失败：${e}`)
  }
}

function go(name: string) {
  if (!menuEnabled.value) return
  router.push({ name, params: { id: currentHostId.value } })
}

function manageHosts() {
  // 跳主机管理（保留路由）
  router.push({ name: 'hosts' })
}
</script>

<template>
  <aside class="sidebar">
    <!-- 主机切换器 -->
    <el-dropdown trigger="click" class="host-switch" @command="handleCommand">
      <div class="switch-trigger">
        <el-icon
          class="state-dot"
          :color="currentHost && store.isOnline(currentHost.id) ? 'var(--el-color-success)' : 'var(--el-text-color-secondary)'"
        >
          <component :is="currentHost && store.isOnline(currentHost.id) ? 'CircleCheckFilled' : 'CirclePlus'" />
        </el-icon>
        <span class="host-name">
          {{ currentHost ? currentHost.name : '选择主机' }}
        </span>
        <el-icon class="caret"><ArrowDown /></el-icon>
      </div>
      <template #dropdown>
        <el-dropdown-menu>
          <el-dropdown-item
            v-for="h in store.hosts"
            :key="h.id"
            :command="h.id"
          >
            <el-icon :color="store.isOnline(h.id) ? 'var(--el-color-success)' : 'var(--el-text-color-secondary)'">
              <component :is="store.isOnline(h.id) ? 'CircleCheckFilled' : 'CircleClose'" />
            </el-icon>
            <span>{{ h.name }}</span>
            <span class="addr">{{ h.user }}@{{ h.host }}</span>
          </el-dropdown-item>
          <el-dropdown-item divided command="add" :icon="Plus">
            添加主机
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>

    <!-- 当前主机地址 + 操作 -->
    <div class="host-meta" v-if="currentHost">
      <div class="meta-addr mono">{{ currentHost.user }}@{{ currentHost.host }}</div>
      <div class="meta-actions">
        <el-button
          v-if="store.isOnline(currentHost.id)"
          text
          size="small"
          :icon="SwitchButton"
          @click="disconnectCurrent"
        >
          断开
        </el-button>
        <el-button text size="small" :icon="Setting" @click="manageHosts">
          管理
        </el-button>
      </div>
    </div>

    <!-- 导航 -->
    <el-menu
      :default-active="activeMenu"
      class="nav"
      :class="{ disabled: !menuEnabled }"
      @select="go"
    >
      <el-menu-item
        v-for="m in menus"
        :key="m.name"
        :index="m.name"
        :disabled="!menuEnabled"
      >
        <el-icon><component :is="m.icon" /></el-icon>
        <span>{{ m.label }}</span>
      </el-menu-item>
    </el-menu>

    <!-- 未连接提示 -->
    <div class="hint" v-if="!menuEnabled">
      <el-icon><InfoFilled /></el-icon>
      <span>{{ store.hosts.length === 0 ? '点击上方添加主机' : '选择并连接一台主机' }}</span>
    </div>

    <HostFormDialog v-model="dialogVisible" :host="null" @saved="onSaved" />
  </aside>
</template>

<script lang="ts">
import {
  ArrowDown,
  CircleCheckFilled,
  CirclePlus,
  CircleClose,
  Plus,
  SwitchButton,
  Setting,
  InfoFilled,
  Odometer,
  Box,
  Files,
} from '@element-plus/icons-vue'
export default {
  components: {
    ArrowDown, CircleCheckFilled, CirclePlus, CircleClose, Plus,
    SwitchButton, Setting, InfoFilled, Odometer, Box, Files,
  },
}
</script>

<style scoped>
.sidebar {
  width: 200px;
  background: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}
.host-switch {
  padding: 10px;
}
.switch-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--el-border-color);
  border-radius: 6px;
  cursor: pointer;
  background: var(--el-fill-color-blank);
}
.switch-trigger:hover {
  border-color: var(--el-color-primary);
}
.state-dot {
  font-size: 14px;
}
.host-name {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.caret {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.addr {
  margin-left: 8px;
  color: var(--el-text-color-secondary);
  font-size: 11px;
}
.host-meta {
  padding: 8px 14px;
  border-top: 1px solid var(--el-border-color);
  border-bottom: 1px solid var(--el-border-color);
}
.meta-addr {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  word-break: break-all;
  margin-bottom: 4px;
}
.meta-actions {
  display: flex;
  gap: 4px;
}
.nav {
  border-right: none;
  padding-top: 8px;
}
.nav.disabled :deep(.el-menu-item) {
  opacity: 0.4;
}
.hint {
  margin: 16px 14px;
  padding: 10px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-radius: 6px;
  display: flex;
  align-items: flex-start;
  gap: 6px;
}
</style>
