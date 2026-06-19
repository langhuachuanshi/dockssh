<script setup lang="ts">
/**
 * 通用容器图标（单色 SVG，跟随 currentColor）。
 *
 * 一个简化的集装箱轮廓 + 顶部吊装点，表达"容器"语义。
 * 不依赖任何外部资源，作为 ContainerIcon 的最终兜底。
 */
withDefaults(defineProps<{ size?: number | string }>(), { size: 32 })
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="ctr-default-icon"
  >
    <!-- 顶部吊装横梁 -->
    <path d="M5 7h14" />
    <path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
    <!-- 箱体 -->
    <rect x="4" y="7" width="16" height="13" rx="1.5" />
    <!-- 竖向纹理 -->
    <path d="M8.5 7v13M12 7v13M15.5 7v13" opacity="0.55" />
  </svg>
</template>

<style scoped>
.ctr-default-icon {
  display: block;
  color: var(--el-text-color-secondary);
}
</style>
