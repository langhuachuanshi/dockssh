<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Image } from '@/api/types'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const images = ref<Image[]>([])
const loading = ref(false)
const search = ref('')

const filtered = computed(() =>
  images.value.filter((i) => {
    if (!search.value) return true
    const kw = search.value.toLowerCase()
    return (
      i.repository.toLowerCase().includes(kw) ||
      i.tag.toLowerCase().includes(kw) ||
      i.id.toLowerCase().includes(kw)
    )
  }),
)

async function refresh() {
  loading.value = true
  try {
    images.value = await api.listImages(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function remove(img: Image) {
  try {
    await ElMessageBox.confirm(
      `确认删除镜像「${img.repository}:${img.tag}」？`,
      '删除确认',
      { type: 'warning' },
    )
    await api.removeImage(hostId.value, img.id, true)
    ElMessage.success('已删除')
    await refresh()
  } catch {
    /* cancel */
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
        placeholder="搜索仓库名/标签/ID"
        :prefix-icon="Search"
        clearable
        style="width: 280px"
      />
      <div class="toolbar-right">
        <el-tooltip content="拉取镜像功能开发中" placement="top">
          <el-button :icon="Download" disabled>拉取镜像</el-button>
        </el-tooltip>
        <el-button :icon="Refresh" @click="refresh">刷新</el-button>
      </div>
    </div>

    <div class="table-wrap" v-loading="loading">
      <el-table :data="filtered" size="default" height="100%">
        <el-table-column label="仓库" min-width="220" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="mono">{{ row.repository }}</span>
          </template>
        </el-table-column>
        <el-table-column label="标签" width="120">
          <template #default="{ row }">
            <el-tag size="small" effect="dark">{{ row.tag }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="ID" width="160">
          <template #default="{ row }">
            <span class="mono muted">{{ row.id.replace('sha256:', '').slice(0, 12) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="大小" width="110">
          <template #default="{ row }">{{ row.size }}</template>
        </el-table-column>
        <el-table-column label="创建时间" min-width="160">
          <template #default="{ row }">{{ row.created }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              type="danger"
              :icon="Delete"
              @click="remove(row)"
            />
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="没有镜像" />
        </template>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts">
import { Search, Refresh, Delete, Download } from '@element-plus/icons-vue'
export default { name: 'ImageList', components: { Search, Refresh, Delete, Download } }
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
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
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
