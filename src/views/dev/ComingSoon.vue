<script setup lang="ts">
/**
 * 开发中占位页：编排模板、配置 等暂不实现的菜单统一指向这里。
 */
defineProps<{ title?: string }>()
</script>

<template>
  <div class="page">
    <el-empty description="该功能开发中，敬请期待">
      <template #image>
        <el-icon :size="48" color="var(--el-text-color-secondary)">
          <Tools />
        </el-icon>
      </template>
      <div class="hint">{{ title || '此页面尚未实现' }}</div>
    </el-empty>
  </div>
</template>

<script lang="ts">
import { Tools } from '@element-plus/icons-vue'
export default { components: { Tools } }
</script>

<style scoped>
.page {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.hint {
  color: var(--el-text-color-secondary);
  font-size: 13px;
  margin-top: 8px;
}
</style>
