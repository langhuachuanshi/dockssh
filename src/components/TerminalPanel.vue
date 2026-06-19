<script setup lang="ts">
/**
 * 终端面板：底部抽屉式（非模态）。
 *
 * 设计回应两个核心诉求：
 * 1. 「不占据整个主窗口」—— 底部面板，只占下方一部分高度，主内容区正常可用。
 * 2. 「点击 mask 层不会取消终端」—— 本面板根本没有遮罩层（modal），
 *    点击容器列表等外部区域不会触发关闭。终端只在以下情况被销毁：
 *    - 用户点击 tab 上的 ×（显式 ptyKill）
 *    - 用户点面板「关闭全部」按钮
 *    - 对应主机被断开/删除（store.closeByHost）
 *
 * 会话保活：已打开的终端用 v-show 而非 v-if 切换显隐，
 * 切换 tab 时 xterm 实例和 SSH 会话都不销毁；面板收起也只隐藏不销毁。
 *
 * 高度可拖拽：拖动顶部把手调整面板高度（120~800px）。
 */
import { computed, nextTick, ref } from 'vue'
import { Close, ArrowDown, ArrowUp } from '@element-plus/icons-vue'
import { useTerminalsStore } from '@/store/terminals'
import Terminal from './Terminal.vue'

const store = useTerminalsStore()

// 每个 tab 的 Terminal 组件实例引用，用于激活时 refit
const termRefs = ref<Record<string, InstanceType<typeof Terminal> | null>>({})

function setTermRef(id: string, el: any) {
  termRefs.value[id] = el
}

function activate(id: string) {
  store.setActive(id)
  // 切换后容器尺寸已变，重新 fit
  nextTick(() => {
    termRefs.value[id]?.refit()
  })
}

function closeTab(id: string) {
  store.close(id)
}

function closeAll() {
  const ids = store.tabs.map((t) => t.id)
  Promise.all(ids.map((id) => store.close(id)))
}

// ===== 拖拽调整高度 =====
const dragging = ref(false)

function onDragStart(e: MouseEvent) {
  e.preventDefault()
  dragging.value = true
  const startY = e.clientY
  const startH = store.height

  const onMove = (ev: MouseEvent) => {
    const delta = startY - ev.clientY
    store.setHeight(startH + delta)
  }
  const onUp = () => {
    dragging.value = false
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
    // 拖拽结束后所有可见终端重新 fit
    nextTick(() => {
      Object.values(termRefs.value).forEach((t) => t?.refit())
    })
  }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

const panelHeight = computed(() => `${store.height}px`)
</script>

<template>
  <transition name="terminal-slide">
    <div
      v-if="store.visible && store.tabs.length"
      class="terminal-panel"
      :style="{ height: panelHeight }"
    >
      <!-- 拖拽把手 -->
      <div class="resize-handle" @mousedown="onDragStart">
        <span class="handle-bar" />
      </div>

      <!-- tab 条 + 工具按钮 -->
      <div class="panel-header">
        <div class="tabs-row">
          <div
            v-for="t in store.tabs"
            :key="t.id"
            class="tab-item"
            :class="{ active: t.id === store.activeId }"
            @click="activate(t.id)"
          >
            <span class="tab-name">{{ t.name }}</span>
            <el-icon
              class="tab-close"
              :size="12"
              @click.stop="closeTab(t.id)"
            >
              <Close />
            </el-icon>
          </div>
        </div>
        <div class="panel-actions">
          <el-tooltip :content="store.height < 500 ? '展开' : '收起'" placement="top">
            <el-icon
              class="action-btn"
              @click="store.setHeight(store.height < 500 ? 600 : 200)"
            >
              <ArrowUp v-if="store.height < 500" />
              <ArrowDown v-else />
            </el-icon>
          </el-tooltip>
          <el-tooltip content="关闭全部" placement="top">
            <el-icon class="action-btn" @click="closeAll"><Close /></el-icon>
          </el-tooltip>
        </div>
      </div>

      <!-- 终端区：所有 tab 都渲染，用 v-show 切换显隐（保活） -->
      <div class="terminal-area">
        <div
          v-for="t in store.tabs"
          :key="t.id"
          v-show="t.id === store.activeId"
          class="terminal-cell"
        >
          <Terminal :ref="(el) => setTermRef(t.id, el)" :tab="t" />
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.terminal-panel {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: #1e1e2e;
  border-top: 1px solid var(--el-border-color);
  box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.2);
  z-index: 50;
  user-select: none;
}

.resize-handle {
  height: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: ns-resize;
  background: #181825;
}
.resize-handle:hover .handle-bar {
  background: var(--el-color-primary);
}
.handle-bar {
  width: 40px;
  height: 3px;
  border-radius: 2px;
  background: var(--el-border-color);
  transition: background 0.15s;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px;
  background: #181825;
  flex-shrink: 0;
}

.tabs-row {
  display: flex;
  gap: 2px;
  overflow-x: auto;
  flex: 1;
}
.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 4px 4px 0 0;
  color: var(--el-text-color-secondary);
  cursor: pointer;
  white-space: nowrap;
  font-size: 12px;
}
.tab-item:hover {
  background: rgba(255, 255, 255, 0.05);
}
.tab-item.active {
  background: #1e1e2e;
  color: #cdd6f4;
}
.tab-name {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tab-close {
  border-radius: 2px;
  padding: 1px;
}
.tab-close:hover {
  background: rgba(255, 255, 255, 0.15);
}

.panel-actions {
  display: flex;
  gap: 4px;
  align-items: center;
}
.action-btn {
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  color: var(--el-text-color-secondary);
}
.action-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #cdd6f4;
}

.terminal-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}
.terminal-cell {
  position: absolute;
  inset: 0;
}

/* 收起/展开动画 */
.terminal-slide-enter-active,
.terminal-slide-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.terminal-slide-enter-from,
.terminal-slide-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
