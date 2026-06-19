<script setup lang="ts">
/**
 * 网络列表页（只读）。
 * docker network ls —— 列出所有 docker 网络。
 * 写操作（创建/删除/连接容器）暂未实现，按钮留占位。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Network } from '@/api/types'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const networks = ref<Network[]>([])
const loading = ref(false)
const search = ref('')

const filtered = computed(() =>
  networks.value.filter((n) => {
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      n.name.toLowerCase().includes(kw) ||
      n.driver.toLowerCase().includes(kw) ||
      n.id.toLowerCase().includes(kw)
    )
  }),
)

async function refresh() {
  loading.value = true
  try {
    networks.value = await api.listNetworks(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  refresh()
})
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <el-input
        v-model="search"
        placeholder="搜索名称/驱动/ID"
        :prefix-icon="Search"
        clearable
        style="width: 280px"
      />
      <el-button :icon="Refresh" @click="refresh">刷新</el-button>
    </div>

    <div class="table-wrap" v-loading="loading">
      <el-table :data="filtered" size="default" height="100%">
        <el-table-column label="名称" min-width="200">
          <template #default="{ row }">
            <span class="mono">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column label="驱动" width="130">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">{{ row.driver }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="作用域" width="100">
          <template #default="{ row }">{{ row.scope }}</template>
        </el-table-column>
        <el-table-column label="ID" min-width="160">
          <template #default="{ row }">
            <span class="mono muted">{{ row.id.slice(0, 12) }}</span>
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="没有网络" />
        </template>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts">
import { Search, Refresh } from '@element-plus/icons-vue'
export default { components: { Search, Refresh } }
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
.muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
