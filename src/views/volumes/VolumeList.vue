<script setup lang="ts">
/**
 * 存储卷列表页（卡片视图）。
 *
 * 每卷一张卡片，展示名称 / 存储卷目录(可点击) / 挂载点(超出显示...) /
 * 驱动(模式) / 时间 / 删除。默认按创建时间倒序排列。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import type { Volume } from '@/api/types'
import VolumeCard from './VolumeCard.vue'

const route = useRoute()
const router = useRouter()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const volumes = ref<Volume[]>([])
const loading = ref(false)
const search = ref('')

// 解析 docker CreatedAt 成时间戳，用于排序（无效/空按 0）
function parseDockerTime(s: string): number {
  if (!s) return 0
  const cleaned = s.replace(/\s+CST\b/i, '').trim()
  const t = Date.parse(cleaned)
  return isNaN(t) ? 0 : t
}

// 搜索过滤 + 默认时间倒序
const filtered = computed(() => {
  const kw = search.value.trim().toLowerCase()
  return volumes.value
    .filter((v) => {
      if (!kw) return true
      return (
        v.name.toLowerCase().includes(kw) ||
        v.driver.toLowerCase().includes(kw) ||
        (v.mountpoint || '').toLowerCase().includes(kw)
      )
    })
    .slice()
    .sort((a, b) => parseDockerTime(b.created) - parseDockerTime(a.created))
})

async function refresh() {
  loading.value = true
  try {
    volumes.value = await api.listVolumes(hostId.value)
  } catch (e) {
    ElMessage.error(`加载失败：${e}`)
  } finally {
    loading.value = false
  }
}

// 打开卷在宿主机的挂载目录：跳转文件管理器
async function openDir(v: Volume) {
  const path = v.mountpoint || (await api.inspectVolume(hostId.value, v.name)).mountpoint
  if (!path) {
    ElMessage.info(`卷「${v.name}」无挂载点`)
    return
  }
  router.push({ name: 'files', params: { id: hostId.value }, query: { path } })
}

async function remove(v: Volume) {
  try {
    await ElMessageBox.confirm(
      `确认删除卷「${v.name}」？该卷内数据将被清除。`,
      '删除确认',
      { type: 'warning' },
    )
  } catch {
    return /* cancel */
  }
  try {
    await api.removeVolume(hostId.value, v.name)
    ElMessage.success('已删除')
    await refresh()
  } catch (e) {
    ElMessage.error(`删除失败：${e}`)
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
        placeholder="搜索名称/驱动/挂载点"
        :prefix-icon="Search"
        clearable
        style="width: 280px"
      />
      <el-button :icon="Refresh" @click="refresh">刷新</el-button>
    </div>

    <!-- 卡片视图 -->
    <div class="card-wrap" v-loading="loading">
      <div v-if="filtered.length" class="card-grid">
        <VolumeCard
          v-for="v in filtered"
          :key="v.name"
          :volume="v"
          @open-dir="openDir"
          @remove="remove"
        />
      </div>
      <el-empty v-else description="没有存储卷" />
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
.card-wrap {
  flex: 1;
  padding: 16px 24px;
  overflow: auto;
}
.card-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
