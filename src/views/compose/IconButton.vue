<script setup lang="ts">
/**
 * 纯图标按钮（启停等操作用）。
 *
 * - 不带圆形/背景，单独显示一个大号图标，看得清楚
 * - 支持运行中态：running=true 时图标变绿 + 微光晕
 * - loading 态：转圈
 *
 * 用法：
 *   <IconButton :icon="VideoPlay" :running="false" tip="启动" @click="..." />
 */
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    /** 图标组件 */
    icon: any
    /** 是否处于「运行/激活」态（影响颜色，如运行中=绿） */
    running?: boolean
    /** tooltip 文案 */
    tip?: string
    /** loading（转圈，禁用点击） */
    loading?: boolean
    /** 自定义图标颜色（覆盖 running 的默认绿） */
    color?: string
  }>(),
  { running: false, loading: false },
)

const emit = defineEmits<{ click: [e: MouseEvent] }>()

function onClick(e: MouseEvent) {
  if (props.loading) return
  emit('click', e)
}

const iconColor = computed(() => props.color ?? (props.running ? '#67c23a' : 'var(--el-text-color-secondary)'))
</script>

<template>
  <button
    class="icon-btn"
    :class="{ 'is-running': running, 'is-loading': loading }"
    :title="tip"
    @click="onClick"
  >
    <el-icon v-if="!loading" :size="22" :color="iconColor">
      <component :is="icon" />
    </el-icon>
    <el-icon v-else :size="22" class="loading-icon"><Loading /></el-icon>
  </button>
</template>

<script lang="ts">
import { Loading } from '@element-plus/icons-vue'
export default { components: { Loading } }
</script>

<style scoped>
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  background: transparent;
  border-radius: 8px;
  cursor: pointer;
  padding: 0;
  transition: background 0.15s ease, transform 0.15s ease;
}
.icon-btn:hover {
  background: var(--el-fill-color-light);
}
.icon-btn:active {
  transform: scale(0.92);
}
.icon-btn.is-loading {
  cursor: progress;
}

/* 运行中：图标绿 + 微光晕 */
.icon-btn.is-running .el-icon {
  filter: drop-shadow(0 0 4px rgba(103, 194, 58, 0.5));
}

.loading-icon {
  animation: spin 0.8s linear infinite;
  color: var(--el-text-color-secondary);
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
