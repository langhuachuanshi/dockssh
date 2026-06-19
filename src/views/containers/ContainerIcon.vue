<script setup lang="ts">
/**
 * 容器图标组件。
 *
 * 解析镜像名 → slug，按优先级取图标：
 *   1. 本地缓存（AppData/dockssh/logos/<slug>.svg，带品牌色）
 *   2. iconify CDN 下载（带品牌色，成功后自动写入缓存）
 *   3. 内置 LOGO_MAP（常用镜像单色兜底，离线可用）
 *   4. 通用容器图标（最终兜底）
 *
 * 在线时图标显示品牌原色；离线时回落到内置单色 / 通用图标。
 */
import { computed, ref, watch } from 'vue'
import * as api from '@/api'
import ContainerDefaultIcon from './ContainerDefaultIcon.vue'

const props = withDefaults(
  defineProps<{
    image: string
    size?: number | string
  }>(),
  { size: 32 },
)

/**
 * 内置常用镜像 logo 映射。
 * 单色 SVG（path 用 currentColor 填充），离线也能显示。
 * slug 取镜像名第一段：nginx:1.25 → nginx；library/redis → redis。
 */
/**
 * 内置常用镜像 logo 映射（离线兜底，单色 currentColor）。
 * 在线时优先走 iconify（带品牌色），失败才用这些。
 */
const LOGO_MAP: Record<string, string> = {
  nginx: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2L3 4.5v6c0 5.5 3.8 10.5 9 11.5 5.2-1 9-6 9-11.5v-6L12 2zm-1 13l-3-3 1.4-1.4L11 12.2l4.6-4.6L17 9l-6 6z"/></svg>',
  redis: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 3C7 3 3 4.6 3 6.5v3c0 1.9 4 3.5 9 3.5s9-1.6 9-3.5v-3C21 4.6 17 3 12 3zm0 2c3.3 0 6 .9 6 2s-2.7 2-6 2-6-.9-6-2 2.7-2 6-2zM3 12v3c0 1.9 4 3.5 9 3.5s9-1.6 9-3.5v-3c0 1.9-4 3.5-9 3.5s-9-1.6-9-3.5zm0 5v3c0 1.9 4 3.5 9 3.5s9-1.6 9-3.5v-3c0 1.9-4 3.5-9 3.5s-9-1.6-9-3.5z"/></svg>',
  mysql: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M5.5 4C4 4 3 5 3 6.5c0 .8.3 1.5.8 2.2.5.7 1.2 1 2 1 .6 0 1.2-.2 1.7-.6.5-.4.8-1 .8-1.7 0-1.5-1.3-2.4-2.8-2.4zm12 .5c-.5 0-.9.1-1.3.4-.4.3-.7.7-.9 1.2l-.3 1.5-.3 1.5h-.1c-.4 0-.7.1-1 .3-.3.2-.5.5-.6.8v.1l.7.3c.2-.3.4-.4.7-.4.2 0 .4.1.5.3.1.2.2.4.2.7v6c0 .3-.1.6-.2.8l-.5.5c-.2.1-.5.2-.8.2h-.1v.7h4v-.7c-.4 0-.7-.1-1-.3-.2-.2-.4-.5-.4-.8V9.5l3.5 9.5h.5L19 9v6.5c0 .3-.1.6-.2.8l-.5.5c-.2.1-.5.2-.8.2v.7h4v-.7c-.3 0-.6-.1-.8-.3-.2-.2-.3-.5-.3-.8v-7c0-.5.2-1 .5-1.3.3-.4.7-.5 1.2-.5V6c-.6 0-1.2.2-1.6.6-.5.4-.8.9-1 1.5l-.3 1.5-.4 1.4c-.3-.3-.7-.4-1.1-.4-.4 0-.8.1-1.1.4.2-.7.4-1.6.6-2.6l.3-1.5c.1-.5.4-.9.7-1.2.3-.3.7-.4 1.1-.4V4.5h-1zM6 6c.8 0 1.5.7 1.5 1.5S6.8 9 6 9s-1.5-.7-1.5-1.5S5.2 6 6 6z"/></svg>',
  postgres: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M21 6.5c-.3-1-1-1.7-2-2-.8-.3-1.8-.3-2.8 0-.7.2-1.4.6-2 1-.4-.1-.8-.2-1.2-.2-.7 0-1.4.1-2 .4-.5-.3-1.1-.6-1.7-.8C7.7 4.4 6 4.7 5 6c-.7 1-1 2.3-.8 3.8.1.6.3 1.2.5 1.8-.3.8-.5 1.7-.5 2.6 0 2.6 1.5 4.7 3.7 5.6.7.3 1.5.4 2.3.4.5 0 1-.1 1.4-.2.4.3.9.5 1.4.6.5.1 1 .2 1.5.2 1.5 0 2.9-.6 3.8-1.7.9-1.1 1.3-2.6 1.1-4.2.3-.5.5-1.1.6-1.7.2-1 .1-2-.2-2.9.4-.5.7-1.1.8-1.8.1-.4 0-.7-.1-1z"/></svg>',
  postgresql: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M21 6.5c-.3-1-1-1.7-2-2-.8-.3-1.8-.3-2.8 0-.7.2-1.4.6-2 1-.4-.1-.8-.2-1.2-.2-.7 0-1.4.1-2 .4-.5-.3-1.1-.6-1.7-.8C7.7 4.4 6 4.7 5 6c-.7 1-1 2.3-.8 3.8.1.6.3 1.2.5 1.8-.3.8-.5 1.7-.5 2.6 0 2.6 1.5 4.7 3.7 5.6.7.3 1.5.4 2.3.4.5 0 1-.1 1.4-.2.4.3.9.5 1.4.6.5.1 1 .2 1.5.2 1.5 0 2.9-.6 3.8-1.7.9-1.1 1.3-2.6 1.1-4.2.3-.5.5-1.1.6-1.7.2-1 .1-2-.2-2.9.4-.5.7-1.1.8-1.8.1-.4 0-.7-.1-1z"/></svg>',
  mariadb: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M3 7c0 2 1 4 3 5 1 .5 2 .5 3 0 .5-.3 1-.7 1.5-1.2.3.7.8 1.3 1.5 1.7.7.4 1.5.5 2.3.3.8-.2 1.5-.7 2-1.4.5.6 1.2 1 2 1.1.8.1 1.6-.1 2.2-.6-.3.5-.7 1-1.2 1.3-.8.5-1.7.7-2.6.6-.9-.1-1.7-.5-2.4-1.1-.6.5-1.3.8-2.1.9-.8.1-1.6 0-2.3-.4-.7-.3-1.3-.9-1.7-1.6-.6.6-1.4 1-2.3 1.1C5 13 3.5 11 3 7z"/></svg>',
  mongo: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2c-1 3-4 5-4 9 0 3 2 5 4 5s4-2 4-5c0-4-3-6-4-9zm0 4c.5 1.5 2 2.5 2 5 0 1.5-1 2.5-2 2.5s-2-1-2-2.5c0-2.5 1.5-3.5 2-5zM7 17c.5 2 2 3 5 3s4.5-1 5-3c-1 1.5-2.5 2-5 2s-4-.5-5-2z"/></svg>',
  mongodb: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2c-1 3-4 5-4 9 0 3 2 5 4 5s4-2 4-5c0-4-3-6-4-9zm0 4c.5 1.5 2 2.5 2 5 0 1.5-1 2.5-2 2.5s-2-1-2-2.5c0-2.5 1.5-3.5 2-5zM7 17c.5 2 2 3 5 3s4.5-1 5-3c-1 1.5-2.5 2-5 2s-4-.5-5-2z"/></svg>',
  rabbitmq: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M4 4h16a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-6v3h-1l-3-3H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zm12 4v4h2V8h-2z"/></svg>',
  kafka: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M8 3a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm8 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm-4 7a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm-4 7a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm8 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM9 5.5l5 4-1 1.5-5-4 1-1.5zm0 6l5 4-1 1.5-5-4 1-1.5z"/></svg>',
  node: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2L3 7v10l9 5 9-5V7l-9-5zm0 2.3l6.9 3.8v7.8L12 19.7 5.1 15.9V8.1L12 4.3zm-.5 4.2c-1.6 0-2.7.8-2.7 2.1 0 1.2.8 1.7 2.4 2.1l.7.2c.9.2 1.2.4 1.2.8 0 .4-.4.7-1.2.7-1 0-1.5-.3-1.7-1.1l-1.5.9c.4 1.3 1.5 1.9 3.1 1.9 1.7 0 2.9-.8 2.9-2.2 0-1.3-.9-1.8-2.6-2.2l-.7-.2c-.8-.2-1.2-.4-1.2-.8 0-.3.3-.6 1-.6.7 0 1.1.2 1.3.9l1.5-.9c-.4-1.1-1.3-1.6-2.8-1.6z"/></svg>',
  python: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M11.5 3c-2 0-3.5 1.5-3.5 3.5v2h5v.5H6C4 9 3 10.5 3 12.5S4 16 6 16h1.5v-2c0-2 1.5-3.5 3.5-3.5h4c1.5 0 2.5-1 2.5-2.5v-4C19.5 3 18 3 16 3h-4.5zm-1.5 2a1 1 0 1 1 0 2 1 1 0 0 1 0-2zM16 8.5v2c0 2-1.5 3.5-3.5 3.5h-4C7 14 6 15 6 16.5v4c0 2 1.5 2.5 3.5 2.5h4c2 0 3.5-1.5 3.5-3.5v-2H12v-.5h6.5c2 0 3-1.5 3-3.5S20.5 9 18.5 9H16zm-1.5 9a1 1 0 1 1 0 2 1 1 0 0 1 0-2z"/></svg>',
  golang: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M3 10h2v4H3v-4zm14-3c2 0 4 1.5 4 4s-2 4-4 4h-5c-2 0-4-1.5-4-4s2-4 4-4h5zm-.5 2c-1 0-2 .8-2 2s1 2 2 2 2-.8 2-2-1-2-2-2zm-12 3c.5 0 1 .5 1 1s-.5 1-1 1-1-.5-1-1 .5-1 1-1z"/></svg>',
  java: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M9 3v2h6V3H9zm0 3c-1 3-1 6 0 9 .5 1.5 1.5 2.5 3 2.5s2.5-1 3-2.5c1-3 1-6 0-9H9zm1 8c.5 1 1 1.5 2 1.5s1.5-.5 2-1.5c.5 1.5.5 3 0 4.5-.5 1-1.5 1.5-2 1.5s-1.5-.5-2-1.5c-.5-1.5-.5-3 0-4.5z"/></svg>',
  grafana: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 4a6 6 0 0 1 6 6h-2a4 4 0 0 0-4-4V6zm-6 6a6 6 0 0 1 6-6v2a4 4 0 0 0-4 4H6zm6 6a6 6 0 0 1-6-6h2a4 4 0 0 0 4 4v2zm6-6a6 6 0 0 1-6 6v-2a4 4 0 0 0 4-4h2z"/></svg>',
  prometheus: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2a7 7 0 0 0-7 7c0 2 1 4 2.5 5v3h1v2h7v-2h1v-3C18 13 19 11 19 9a7 7 0 0 0-7-7zm-2 7a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0zm4 0a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0z"/></svg>',
  docker: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M3 11h2v2H3v-2zm3 0h2v2H6v-2zm3 0h2v2H9v-2zm3 0h2v2h-2v-2zm-6-3h2v2H6V8zm3 0h2v2H9V8zm3 0h2v2h-2V8zm0-3h2v2h-2V5zm5 6.5c-.5 0-1 .1-1.4.3-.3-1.3-1.4-2-1.4-2s-.7.9-.8 2.1c-.5-.3-1-.4-1.6-.4-.7 0-1.4.2-2 .6 1.8.5 3 2.1 3 4 0 .2 0 .4-.1.6.4.2.8.3 1.3.3 2.5 0 4-2 4-4.5 0-.7-.4-1-1-1z"/></svg>',
  alpine: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2L2 22h20L12 2zm0 4l7 14H5l7-14z"/></svg>',
  ubuntu: '<svg viewBox="0 0 24 24" width="100%" height="100%"><circle cx="12" cy="12" r="10" fill="currentColor"/><circle cx="12" cy="6" r="1.8" fill="#fff"/><circle cx="17" cy="15" r="1.8" fill="#fff"/><circle cx="7" cy="15" r="1.8" fill="#fff"/></svg>',
  debian: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm-1 5c2 0 4 1.5 4 4 0 1.5-.5 2.5-1.5 3 .5.5 1 1 1 2 0 2-1.5 3-3.5 3-1 0-2-.5-2.5-1.5l1-1c.5.5 1 1 1.5 1 1 0 1.5-.5 1.5-1.5s-.5-1.5-1.5-1.5h-.5v-1.5c1.5-.2 2-.7 2-1.5 0-.8-.5-1.3-1.5-1.3-.7 0-1.3.4-1.7 1l-1-1C7.5 8 9 7 11 7z"/></svg>',
  traefik: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 3L3 8v8l9 5 9-5V8l-9-5zm0 2.3l6.5 3.6L12 12.5 5.5 8.9 12 5.3zM5 10.5l6 3.4v6.3l-6-3.4v-6.3zm14 0v6.3l-6 3.4v-6.3l6-3.4z"/></svg>',
  caddy: '<svg viewBox="0 0 24 24" width="100%" height="100%"><path fill="currentColor" d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm-3 6l3 2 3-2v3l-3 2-3-2V8zm-3 4l3 2v3l-3-2v-3zm12 0v3l-3 2v-3l3-2z"/></svg>',
}

// 从镜像名解析 slug：library/redis:7 → redis
function parseSlug(image: string): string {
  if (!image) return ''
  // 去掉 registry 前缀（含 . 或 : 或 localhost），取最后一段
  const noTag = image.split(':')[0]
  const parts = noTag.split('/')
  // 最后一段是镜像名
  let name = parts[parts.length - 1] || ''
  name = name.trim().toLowerCase()
  // 去掉常见后缀
  name = name.replace(/[-_](alpine|slim|latest|full)$/, '')
  return name
}

const slug = computed(() => parseSlug(props.image))

// 图标内容：内置 → 缓存/下载 → 默认
type State = 'builtin' | 'loaded' | 'loading' | 'fallback'
const state = ref<State>('loading')
const svgContent = ref<string>('')

// 正在下载的 slug 集合，避免并发重复请求
const inFlight = new Set<string>()

async function load() {
  const s = slug.value
  if (!s) {
    state.value = 'fallback'
    return
  }

  state.value = 'loading'
  try {
    // 1. 本地缓存（带品牌色，最快）
    const cached = await api.getCachedLogo(s)
    if (cached) {
      svgContent.value = cached
      state.value = 'loaded'
      return
    }

    // 2. 避免并发
    if (inFlight.has(s)) return
    inFlight.add(s)

    // 3. iconify 下载（带品牌色，成功后写入缓存）
    try {
      const svg = await api.fetchLogo(s)
      svgContent.value = svg
      state.value = 'loaded'
      return
    } catch (e) {
      console.warn(`[logo] ${s} 下载失败:`, e)
    } finally {
      inFlight.delete(s)
    }

    // 4. 内置 MAP（离线兜底，单色）
    if (LOGO_MAP[s]) {
      svgContent.value = LOGO_MAP[s]
      state.value = 'builtin'
      return
    }

    // 5. 通用容器图标
    state.value = 'fallback'
  } catch (e) {
    console.warn(`[logo] ${s} 加载失败:`, e)
    // 兜底：内置 MAP
    if (LOGO_MAP[s]) {
      svgContent.value = LOGO_MAP[s]
      state.value = 'builtin'
    } else {
      state.value = 'fallback'
    }
  }
}

watch(() => props.image, load, { immediate: true })
</script>

<template>
  <span class="ctr-icon" :style="{ width: typeof size === 'number' ? size + 'px' : size, height: typeof size === 'number' ? size + 'px' : size }">
    <!-- 已加载 SVG（内置 / 缓存 / CDN） -->
    <span
      v-if="(state === 'builtin' || state === 'loaded') && svgContent"
      class="ctr-icon-svg"
      v-html="svgContent"
    />
    <!-- 加载中：用默认图标占位（避免闪烁） -->
    <ContainerDefaultIcon v-else :size="size" />
  </span>
</template>

<style scoped>
.ctr-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  /* 不设 color：让内置 MAP 的 currentColor 跟随文本色，
     iconify 彩色 SVG 用自身 fill 不受影响 */
  color: var(--el-text-color-regular);
  transition: color 0.2s;
}
.ctr-icon-svg {
  display: inline-flex;
  width: 100%;
  height: 100%;
}
/* SVG 撑满容器；不强制 fill，保留 iconify 品牌原色 */
.ctr-icon-svg :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
