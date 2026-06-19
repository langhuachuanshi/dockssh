<script setup lang="ts">
/**
 * 仓库（registry）列表页（只读）。
 * 读取远程机 ~/.docker/config.json 的 auths 段，列出已登录的 registry 地址。
 * 登录/登出/推送/拉取等写操作暂未实现。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const registries = ref<string[]>([])
const loading = ref(false)

async function refresh() {
  loading.value = true
  try {
    registries.value = await api.listRegistries(hostId.value)
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
      <span class="title">已登录的仓库</span>
      <el-button :icon="Refresh" @click="refresh">刷新</el-button>
    </div>

    <div class="table-wrap" v-loading="loading">
      <el-table :data="registries.map((r) => ({ url: r }))" size="default" height="100%">
        <el-table-column label="仓库地址" min-width="320">
          <template #default="{ row }">
            <span class="mono">{{ row.url }}</span>
          </template>
        </el-table-column>
        <el-table-column label="类型" width="140">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">
              {{ row.url.includes('docker.io') ? 'Docker Hub' : '私有仓库' }}
            </el-tag>
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="没有已登录的仓库" />
        </template>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts">
import { Refresh } from '@element-plus/icons-vue'
export default { components: { Refresh } }
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
.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.table-wrap {
  flex: 1;
  padding: 0 24px 16px;
  overflow: hidden;
}
</style>
