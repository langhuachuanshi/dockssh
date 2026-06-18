<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)
const cid = computed(() => route.params.cid as string)
const name = computed(() => (route.query.name as string) || cid.value)

const logEl = ref<HTMLDivElement>()
const lines = ref<string[]>([])
const autoScroll = ref(true)
const tail = ref('500')
let unlisten: (() => void) | null = null

async function start() {
  lines.value = []
  await api.startLogs(hostId.value, cid.value, tail.value)
  unlisten = await api.onLogChunk(hostId.value, cid.value, (chunk) => {
    // 按行追加，保留原始换行结构
    const newLines = chunk.split('\n')
    for (const l of newLines) {
      if (l === '' && lines.value[lines.value.length - 1] === '') continue
      lines.value.push(l)
    }
    // 限制最大行数，避免内存膨胀
    if (lines.value.length > 10000) {
      lines.value = lines.value.slice(-8000)
    }
    if (autoScroll.value) {
      nextTick(() => scrollToBottom())
    }
  })
}

function scrollToBottom() {
  if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
}

function clearLogs() {
  lines.value = []
}

onMounted(async () => {
  await store.ensureConnected(hostId.value)
  await start()
})
onUnmounted(() => {
  unlisten?.()
  api.stopLogs(hostId.value, cid.value).catch(() => {})
})
</script>

<template>
  <div class="page">
    <div class="toolbar">
      <div class="flex gap-12 flex-center">
        <el-button :icon="ArrowLeft" text @click="$router.back()">返回</el-button>
        <span class="title">日志：{{ name }}</span>
        <span class="cid mono">{{ cid.slice(0, 12) }}</span>
      </div>
      <div class="flex gap-8 flex-center">
        <el-input-number v-model="tail" :min="50" :step="100" size="small" />
        <span class="tip">初始行数</span>
        <el-button size="small" :icon="Delete" @click="clearLogs">清空</el-button>
        <el-checkbox v-model="autoScroll" label="自动滚动" />
      </div>
    </div>
    <div ref="logEl" class="log-view selectable mono">
      <div v-for="(l, i) in lines" :key="i" class="log-line">{{ l }}</div>
      <div v-if="!lines.length" class="empty-wait">等待日志输出…</div>
    </div>
  </div>
</template>

<script lang="ts">
import { ArrowLeft, Delete } from '@element-plus/icons-vue'
export default { components: { ArrowLeft, Delete } }
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
  padding: 10px 24px;
  border-bottom: 1px solid var(--el-border-color);
  background: var(--el-bg-color);
}
.title {
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.cid {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.tip {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.log-view {
  flex: 1;
  overflow-y: auto;
  padding: 12px 24px;
  font-size: 12.5px;
  line-height: 1.55;
  background: var(--el-fill-color-darker);
}
.log-line {
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--el-text-color-regular);
}
.empty-wait {
  color: var(--el-text-color-secondary);
  text-align: center;
  margin-top: 40px;
}
</style>
