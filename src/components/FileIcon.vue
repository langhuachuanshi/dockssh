<script setup lang="ts">
/**
 * 文件类型图标组件（VSCode Material Icon Theme 风格）。
 *
 * 资源来源：material-extensions/vscode-material-icon-theme（MIT）。
 * 仅打包了一份「常用扩展名/文件名/文件夹名」对应的 SVG 子集（约 70 个），
 * 配合 mapping.json 做扩展名 → 图标名的查表。
 *
 * 匹配优先级：特殊文件名（package.json 等）> 扩展名 > 通用 file/folder。
 */
import { computed } from 'vue'
import mapping from '@/assets/file-icons/mapping.json'

// eager + as: 'raw' 一次性把本地所有 SVG 内联成字符串，键为图标名。
const icons = import.meta.glob('@/assets/file-icons/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

// 把键里的路径前缀去掉，得到 iconName -> svgString
const iconSvg: Record<string, string> = {}
for (const [path, svg] of Object.entries(icons)) {
  const name = path.split('/').pop()!.replace(/\.svg$/, '')
  iconSvg[name] = svg
}

const props = withDefaults(
  defineProps<{
    /** 文件名（用于取扩展名 / 特殊文件名匹配） */
    name: string
    /** 是否目录 */
    isDir: boolean
    /** 图标大小（px），默认 32 */
    size?: number
  }>(),
  { size: 32 },
)

type MapData = { fileExt: Record<string, string>; fileName: Record<string, string>; folder: Record<string, string> }
const data = mapping as MapData

// 解析出应使用的图标名
const iconName = computed(() => {
  if (props.isDir) {
    return data.folder[props.name.toLowerCase()] || 'folder'
  }
  const lower = props.name.toLowerCase()
  // 1. 特殊文件名（全名匹配）
  if (data.fileName[lower]) return data.fileName[lower]
  // 2. 扩展名
  const dot = lower.lastIndexOf('.')
  if (dot > 0) {
    const ext = lower.slice(dot + 1)
    if (data.fileExt[ext]) return data.fileExt[ext]
  }
  // 3. 通用文件
  return 'file'
})

// 取 SVG 字符串，找不到则回退通用图标
const svgStr = computed(() => iconSvg[iconName.value] || iconSvg.file || '')

const wrapStyle = computed(() => ({
  width: `${props.size}px`,
  height: `${props.size}px`,
}))
</script>

<template>
  <span class="file-icon" :style="wrapStyle" v-html="svgStr" />
</template>

<script lang="ts">
export default { name: 'FileIcon' }
</script>

<style scoped>
.file-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  line-height: 0;
}
/* 内联的 svg 撑满容器 */
.file-icon :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
