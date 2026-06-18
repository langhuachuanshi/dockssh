<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Container } from '@/api/types'

const route = useRoute()
const router = useRouter()
const store = useHostsStore()

const hostId = computed(() => route.params.id as string)

const containers = ref<Container[]>([])
const loading = ref(false)
const search = ref('')
const onlyRunning = ref(false)

let pollTimer: number | null = null

const filtered = computed(() =>
  containers.value.filter((c) => {
    if (onlyRunning.value && c.state !== 'running') return false
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      c.name.toLowerCase().includes(kw) ||
      c.image.toLowerCase().includes(kw) ||
      c.id.toLowerCase().includes(kw)
    )
  }),
)

async function refresh() {
  loading.value = true
  try {
    containers.value = await api.listContainers(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function action(kind: 'start' | 'stop' | 'restart', c: Container) {
  try {
    if (kind === 'start') await api.startContainer(hostId.value, c.id)
    else if (kind === 'stop') await api.stopContainer(hostId.value, c.id)
    else await api.restartContainer(hostId.value, c.id)
    ElMessage.success(`已${kind === 'start' ? '启动' : kind === 'stop' ? '停止' : '重启'}`)
    await refresh()
  } catch (e) {
    ElMessage.error(`操作失败：${e}`)
  }
}

async function remove(c: Container) {
  try {
    await ElMessageBox.confirm(`确认删除容器「${c.name}」？`, '删除确认', {
      type: 'warning',
    })
    await api.removeContainer(hostId.value, c.id, true)
    ElMessage.success('已删除')
    await refresh()
  } catch {
    /* cancel */
  }
}

function viewLogs(c: Container) {
  router.push({
    name: 'container-logs',
    params: { id: hostId.value, cid: c.id },
    query: { name: c.name },
  })
}

function stateTag(s: string) {
  return s === 'running' ? 'success' : 'info'
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  refresh()
  pollTimer = window.setInterval(refresh, 8000)
})
onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <div class="flex gap-12 flex-center">
        <el-input
          v-model="search"
          placeholder="搜索名称/镜像/ID"
          :prefix-icon="Search"
          clearable
          style="width: 260px"
        />
        <el-checkbox v-model="onlyRunning">仅运行中</el-checkbox>
      </div>
      <el-button :icon="Refresh" @click="refresh">刷新</el-button>
    </div>

    <div class="table-wrap" v-loading="loading">
      <el-table :data="filtered" size="default" height="100%">
        <el-table-column label="名称" min-width="180">
          <template #default="{ row }">
            <el-link type="primary" :underline="false" @click="viewLogs(row)">
              {{ row.name }}
            </el-link>
          </template>
        </el-table-column>
        <el-table-column label="镜像" min-width="180" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="mono c-dim">{{ row.image }}</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="stateTag(row.state)" size="small" effect="dark">
              {{ row.state }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="运行情况" min-width="130">
          <template #default="{ row }">
            <span class="c-dim">{{ row.status }}</span>
          </template>
        </el-table-column>
        <el-table-column label="端口" min-width="180">
          <template #default="{ row }">
            <div class="ports">
              <el-tag v-for="(p, i) in row.ports" :key="i" size="small" type="info" effect="plain">
                {{ p }}
              </el-tag>
              <span v-if="!row.ports.length" class="c-dim">—</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button-group>
              <el-button
                v-if="row.state !== 'running'"
                size="small"
                :icon="VideoPlay"
                @click="action('start', row)"
              />
              <el-button
                v-else
                size="small"
                :icon="VideoPause"
                @click="action('stop', row)"
              />
              <el-button size="small" :icon="RefreshRight" @click="action('restart', row)" />
              <el-button size="small" :icon="Document" @click="viewLogs(row)" />
              <el-button size="small" type="danger" :icon="Delete" @click="remove(row)" />
            </el-button-group>
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="没有容器" />
        </template>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts">
import {
  Search,
  Refresh,
  VideoPlay,
  VideoPause,
  RefreshRight,
  Document,
  Delete,
} from '@element-plus/icons-vue'
export default {
  components: { Search, Refresh, VideoPlay, VideoPause, RefreshRight, Document, Delete },
}
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  border-bottom: 1px solid var(--el-border-color);
}
.table-wrap {
  flex: 1;
  padding: 0 24px 16px;
  overflow: hidden;
}
.c-dim {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.ports {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
</style>
