<script setup lang="ts">
/**
 * 存储卷卡片组件（单行卡片，紧凑两行布局）。
 *
 *   第1行：文件夹图标 + 名称 + 驱动标签(模式) + 时间(右) + 删除按钮(右)
 *   第2行：存储卷目录(可点击打开对应文件) + 挂载点(超出显示 ...)
 *
 * 注意：Docker volume 仅暴露一个宿主机路径 Mountpoint，「存储卷目录」与
 * 「挂载点」同源——前者作为可点击链接跳转文件管理器，后者作为截断文本展示。
 */
import { computed } from 'vue'
import type { Volume } from '@/api/types'

const props = defineProps<{
  volume: Volume
}>()

const emit = defineEmits<{
  (e: 'openDir', v: Volume): void
  (e: 'remove', v: Volume): void
}>()

const hasMountpoint = computed(() => !!props.volume.mountpoint)

// 解析 docker CreatedAt（如 "2024-01-01 12:00:00 +0800 CST"）成时间戳，用于排序/展示
const createdTs = computed(() => parseDockerTime(props.volume.created))
const createdText = computed(() => formatTime(createdTs.value, props.volume.created))

function parseDockerTime(s: string): number {
  if (!s) return 0
  // 去掉末尾 "CST" 之类的时区缩写，Date 无法可靠解析；优先按 +0800 偏移
  const cleaned = s.replace(/\s+CST\b/i, '').trim()
  const t = Date.parse(cleaned)
  return isNaN(t) ? 0 : t
}

function formatTime(ts: number, raw: string): string {
  if (!ts) return raw || '—'
  const d = new Date(ts)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}
</script>

<template>
  <div class="vol-card">
    <!-- 左侧装饰条 -->
    <div class="state-bar" />

    <div class="card-inner">
      <!-- 第1行：图标 + 名称 + 驱动 -->
      <div class="row-info">
        <div class="title-block">
          <el-icon class="vol-icon"><Coin /></el-icon>
          <span class="name mono" :title="volume.name">{{ volume.name }}</span>
          <el-tag size="small" type="info" effect="plain">{{ volume.driver }}</el-tag>
        </div>
      </div>

      <!-- 第2行：挂载点(截断) + 时间 + 打开目录(图标) + 删除(图标) -->
      <div class="row-paths">
        <template v-if="hasMountpoint">
          <span class="field-label">挂载点</span>
          <span class="mountpoint mono" :title="volume.mountpoint">{{ volume.mountpoint }}</span>
        </template>
        <span v-else class="mountpoint c-dim">无挂载点信息</span>
        <div class="right-block">
          <span class="time">{{ createdText }}</span>
          <div class="action-group">
            <!-- 存储卷目录：点击跳转文件管理器 -->
            <el-tooltip content="打开目录" placement="top" :show-after="300">
              <el-button
                size="small"
                text
                type="primary"
                :icon="FolderOpened"
                class="icon-btn"
                @click="emit('openDir', volume)"
              />
            </el-tooltip>
            <el-tooltip content="删除" placement="top" :show-after="300">
              <el-button
                size="small"
                text
                type="danger"
                :icon="Delete"
                class="icon-btn"
                @click="emit('remove', volume)"
              />
            </el-tooltip>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { Coin, FolderOpened, Delete } from '@element-plus/icons-vue'
export default { components: { Coin, FolderOpened, Delete } }
</script>

<style scoped>
.vol-card {
  position: relative;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  transition: transform 0.18s ease, box-shadow 0.18s ease;
}
.vol-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
}
.vol-card:hover .state-bar {
  width: 5px;
  filter: saturate(1.3) brightness(1.1);
}

/* 左侧装饰条 */
.state-bar {
  width: 3px;
  flex-shrink: 0;
  background: var(--el-color-primary);
  transition: width 0.18s ease, filter 0.18s ease;
}

.card-inner {
  flex: 1;
  min-width: 0;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 第1行 */
.row-info {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.title-block {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.vol-icon {
  font-size: 18px;
  color: var(--el-color-primary);
  flex-shrink: 0;
}
.name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 360px;
}
.time {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}

/* 第2行：挂载点在左，时间 + 图标按钮在右 */
.row-paths {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.field-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}
/* 挂载点：超出显示省略号 */
.mountpoint {
  font-size: 12px;
  color: var(--el-text-color-regular);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1;
}
.right-block {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-shrink: 0;
  margin-left: auto;
}
/* 按钮组：两个图标按钮紧贴 */
.action-group {
  display: flex;
  align-items: center;
  gap: 2px;
}
/* 纯图标按钮：放大图标与点击热区，与容器卡操作按钮视觉一致 */
.icon-btn.el-button {
  height: 30px;
  width: 30px;
  padding: 0;
}
/* 清掉 Element Plus 默认 .el-button + .el-button 的 12px 左外间距，
   按钮间距统一交给 .action-group 的 gap 控制 */
.icon-btn.el-button + .icon-btn.el-button {
  margin-left: 0;
}
.icon-btn.el-button :deep(.el-icon) {
  font-size: 17px;
}
.icon-btn.el-button--danger:hover {
  background: var(--el-color-danger-light-9);
}
.c-dim {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
