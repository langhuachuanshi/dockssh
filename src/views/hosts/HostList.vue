<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useHostsStore } from '@/store/hosts'
import * as api from '@/api'
import type { Host } from '@/api/types'
import HostFormDialog from './HostFormDialog.vue'

const router = useRouter()
const store = useHostsStore()

const dialogVisible = ref(false)
const editing = ref<Host | null>(null)
const connecting = ref<string | null>(null)

onMounted(async () => {
  await store.refresh()
})

function openCreate() {
  editing.value = null
  dialogVisible.value = true
}

function openEdit(host: Host) {
  editing.value = host
  dialogVisible.value = true
}

async function onSaved() {
  dialogVisible.value = false
  await store.refresh()
}

async function remove(host: Host) {
  try {
    await ElMessageBox.confirm(
      `确定删除主机「${host.name}」吗？此操作会同时清除其凭据。`,
      '删除确认',
      { type: 'warning' },
    )
    await api.deleteHost(host.id)
    ElMessage.success('已删除')
    await store.refresh()
  } catch {
    /* 用户取消 */
  }
}

async function connect(host: Host) {
  connecting.value = host.id
  try {
    const res = await store.connect(host.id)
    ElMessage.success(`已连接：${res.probe.hostname}（Docker ${res.probe.docker_version}）`)
    store.select(host.id)
    router.push({ name: 'dashboard', params: { id: host.id } })
  } catch (e) {
    ElMessage.error(`连接失败：${e}`)
  } finally {
    connecting.value = null
  }
}

async function disconnect(host: Host) {
  try {
    await store.disconnect(host.id)
    ElMessage.success('已断开')
  } catch (e) {
    ElMessage.error(`断开失败：${e}`)
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <span class="subtitle">通过 SSH 连接远程服务器管理 Docker，连接成功后从左侧导航操作</span>
      <el-button type="primary" :icon="Plus" @click="openCreate">
        添加主机
      </el-button>
    </header>

    <div v-loading="store.loading" class="host-grid">
      <el-card
        v-for="h in store.hosts"
        :key="h.id"
        class="host-card"
        shadow="never"
        body-style="display:flex;flex-direction:column;gap:12px;height:100%;"
      >
        <div class="card-top">
          <div class="flex-col gap-8">
            <div class="flex gap-8 flex-center">
              <el-icon :color="store.isOnline(h.id) ? 'var(--el-color-success)' : 'var(--el-text-color-secondary)'">
                <component :is="store.isOnline(h.id) ? 'CircleCheck' : 'CircleClose'" />
              </el-icon>
              <span class="host-name">{{ h.name }}</span>
            </div>
            <span class="host-addr mono">{{ h.user }}@{{ h.host }}:{{ h.port }}</span>
          </div>
          <el-dropdown trigger="click">
            <el-button :icon="MoreFilled" text />
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item :icon="Edit" @click="openEdit(h)">编辑</el-dropdown-item>
                <el-dropdown-item :icon="Delete" divided @click="remove(h)">
                  删除
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>

        <div class="card-probe" v-if="store.probeMap[h.id]">
          <el-tag size="small" type="info" effect="dark">{{ store.probeMap[h.id].os }}</el-tag>
          <el-tag v-if="store.probeMap[h.id].docker_version" size="small" type="info" effect="dark">
            Docker {{ store.probeMap[h.id].docker_version }}
          </el-tag>
          <el-tag v-if="store.probeMap[h.id].is_wsl2" size="small" type="info" effect="dark">WSL2</el-tag>
        </div>

        <div class="card-actions">
          <el-button
            v-if="!store.isOnline(h.id)"
            type="primary"
            :loading="connecting === h.id"
            @click="connect(h)"
          >
            连接
          </el-button>
          <template v-else>
            <el-button @click="store.select(h.id); router.push({ name: 'dashboard', params: { id: h.id } })">
              管理
            </el-button>
            <el-button :icon="SwitchButton" @click="disconnect(h)">断开</el-button>
          </template>
        </div>
      </el-card>

      <div v-if="!store.loading && store.hosts.length === 0" class="empty">
        <el-empty description="还没有主机，点击右上角添加一台" />
      </div>
    </div>

    <HostFormDialog
      v-model="dialogVisible"
      :host="editing"
      @saved="onSaved"
    />
  </div>
</template>

<script lang="ts">
import {
  Plus,
  MoreFilled,
  Edit,
  Delete,
  SwitchButton,
  CircleCheck,
  CircleClose,
} from '@element-plus/icons-vue'
export default {
  components: { Plus, MoreFilled, Edit, Delete, SwitchButton, CircleCheck, CircleClose },
}
</script>

<style scoped>
.page {
  height: 100%;
  overflow-y: auto;
  padding: 24px 32px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}
.title {
  font-size: 22px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.subtitle {
  color: var(--el-text-color-secondary);
  font-size: 13px;
  margin-top: 4px;
}
.host-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}
.host-card {
  transition: border-color 0.2s;
}
.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}
.host-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.host-addr {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.card-probe {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.card-actions {
  display: flex;
  gap: 8px;
  margin-top: auto;
}
.empty {
  grid-column: 1 / -1;
}
</style>
