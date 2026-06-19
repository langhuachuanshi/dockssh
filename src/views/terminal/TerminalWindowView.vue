<script setup lang="ts">
/**
 * 独立终端窗口页（tabs 嵌入 48px 标题栏，带窗口控制按钮）。
 *
 * 布局（与主窗口 TitleBar 风格统一）：
 * ┌──────────────────────────────────────────────┐
 * │ 终端 │ ┌tab1×┐ tab2×            - □ × │  标题栏 48px
 * ├──────────────────────────────────────────────┤
 * │                                              │
 * │              全屏终端区                        │
 * │                                              │
 * └──────────────────────────────────────────────┘
 *
 * - 标题栏左：窗口名（可拖动窗口）
 * - 标题栏中：终端 tabs（切换 + 单个关闭），空白处可拖窗
 * - 标题栏右：最小化 / 最大化 / 关闭
 * - 终端区：所有 tab 渲染，v-show 切换（保活）
 *
 * 终端来源：主窗口「弹出」投递（attach_terminal → 队列 → 本窗口拉取）。
 * 独立窗口有自己的 Pinia store 实例（与主窗口 webview 内存隔离）。
 */
import { computed, nextTick, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Close, Minus, FullScreen } from '@element-plus/icons-vue'
import Terminal from '@/components/Terminal.vue'
import { useTerminalsStore } from '@/store/terminals'
import * as api from '@/api'
import { unmarkMigrating } from '@/utils/terminal-migrate'

const win = getCurrentWindow()
const store = useTerminalsStore()

// 每个 tab 的 Terminal 组件实例引用，用于激活时 refit
const termRefs = ref<Record<string, InstanceType<typeof Terminal> | null>>({})
function setTermRef(id: string, el: any) {
  termRefs.value[id] = el
}

function activate(id: string) {
  store.setActive(id)
  nextTick(() => {
    termRefs.value[id]?.refit()
  })
}

function closeTab(id: string) {
  store.close(id)
}

async function minimize() {
  await win.minimize()
}
async function toggleMaximize() {
  await win.toggleMaximize()
}
async function close() {
  await win.close()
}

/** 从后端拉取待投递队列，逐个加为本窗口的 tab。幂等（openAttached 去重）。 */
async function takePending() {
  try {
    const list = await api.takePendingTerminals()
    for (const t of list) {
      unmarkMigrating(t.session_id)
      store.openAttached(t.session_id, t.name)
    }
  } catch {
    /* 忽略 */
  }
}

// 窗口加载完成：立即拉取积压 + 监听后续投递通知
takePending()
api.onAttachTerminalNotify(takePending).catch(() => {})

/**
 * tabs 区鼠标按下兜底拖窗（与主窗口 TitleBar 同机制）：
 * el-tabs 内部容器撑满会盖住 drag region，点 tab 间空白时主动 startDragging。
 * 命中 tab 项本身则不拖动，交给 tab 点击处理。
 */
async function onTabsAreaMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement | null
  if (target?.closest('.tab-item')) return
  try {
    await win.startDragging()
  } catch {
    /* 拖动失败忽略 */
  }
}

const hasTabs = computed(() => store.tabs.length > 0)
</script>

<template>
  <div class="terminal-window">
    <!-- 标题栏 48px：左窗口名 + 中 tabs + 右窗口按钮 -->
    <div class="titlebar">
      <!-- 左：窗口名（可拖动） -->
      <div class="logo-area" data-tauri-drag-region>
        <span class="name">终端</span>
      </div>

      <!-- 中：终端 tabs（空白处可拖窗） -->
      <div
        class="tabs-area"
        data-tauri-drag-region
        @mousedown="onTabsAreaMouseDown"
      >
        <div v-if="hasTabs" class="tabs-row">
          <div
            v-for="t in store.tabs"
            :key="t.id"
            class="tab-item"
            :class="{ active: t.id === store.activeId }"
            @click="activate(t.id)"
          >
            <span class="tab-name">{{ t.name }}</span>
            <el-icon
              class="tab-action tab-close"
              :size="12"
              title="关闭"
              @click.stop="closeTab(t.id)"
            >
              <Close />
            </el-icon>
          </div>
        </div>
        <div v-else class="tabs-empty" data-tauri-drag-region />
      </div>

      <!-- 右：窗口控制按钮（不拖动） -->
      <div class="win-controls">
        <button class="ctrl-btn" title="最小化" @click="minimize">
          <el-icon><Minus /></el-icon>
        </button>
        <button class="ctrl-btn" title="最大化" @click="toggleMaximize">
          <el-icon><FullScreen /></el-icon>
        </button>
        <button class="ctrl-btn close" title="关闭" @click="close">
          <el-icon><Close /></el-icon>
        </button>
      </div>
    </div>

    <!-- 终端区：所有 tab 渲染，v-show 切换（保活） -->
    <div class="terminal-area">
      <div
        v-for="t in store.tabs"
        :key="t.id"
        v-show="t.id === store.activeId"
        class="terminal-cell"
      >
        <Terminal :ref="(el) => setTermRef(t.id, el)" :tab="t" />
      </div>
      <div v-if="!hasTabs" class="empty-hint">
        从主窗口弹出终端即可在此显示
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-window {
  width: 100vw;
  height: 100vh;
  background: #1e1e1e;
  display: flex;
  flex-direction: column;
}

/* ===== 标题栏 48px（与主窗口一致） ===== */
.titlebar {
  height: 48px;
  flex-shrink: 0;
  display: flex;
  align-items: stretch;
  background: var(--el-bg-color);
  user-select: none;
}
.logo-area {
  display: flex;
  align-items: center;
  padding: 0 14px 0 16px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  flex-shrink: 0;
}
.tabs-area {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: stretch;
}
.tabs-empty {
  flex: 1;
}
.tabs-row {
  display: flex;
  gap: 2px;
  align-items: stretch;
}
.tab-item {
  position: relative; /* 下划线指示条绝对定位的锚 */
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 14px;
  color: var(--el-text-color-secondary);
  cursor: pointer;
  white-space: nowrap;
  font-size: 14px;
  transition: color 0.15s;
}
.tab-item:hover {
  color: var(--el-text-color-primary);
}
.tab-item.active {
  color: #cccccc;
}
/* 下划线指示条：absolute 定位在 tab 底部，默认透明收缩，
   active 时展开 + 显色 + 过渡动画。 */
.tab-item::after {
  content: '';
  position: absolute;
  left: 10px;
  right: 10px;
  bottom: 0;
  height: 2px;
  border-radius: 1px;
  background: var(--el-color-primary);
  transform: scaleX(0);
  transform-origin: center;
  opacity: 0;
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.tab-item.active::after {
  transform: scaleX(1);
  opacity: 1;
}
.tab-name {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tab-action {
  border-radius: 2px;
  padding: 1px;
  color: var(--el-text-color-secondary);
  transition: background 0.15s, color 0.15s;
}
.tab-close:hover {
  background: #f14c4c;
  color: #1e1e1e;
}

/* ===== 窗口控制按钮（与主窗口一致） ===== */
.win-controls {
  display: flex;
  height: 100%;
  flex-shrink: 0;
}
.ctrl-btn {
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  color: var(--el-text-color-regular);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s;
}
.ctrl-btn:hover {
  background: var(--el-fill-color-light);
}
.ctrl-btn.close:hover {
  background: var(--el-color-danger);
  color: #fff;
}

/* ===== 终端区 ===== */
.terminal-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}
.terminal-cell {
  position: absolute;
  inset: 0;
}
.empty-hint {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 13px;
}
</style>
