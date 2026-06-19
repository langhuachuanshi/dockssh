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
import {
  Close,
  ArrowDown,
  ArrowUp,
  Bottom,
  Back,
  Right,
  Expand,
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useTerminalsStore } from '@/store/terminals'
import type { DockSide } from '@/store/terminals'
import * as api from '@/api'
import Terminal from './Terminal.vue'
import { markMigrating } from '@/utils/terminal-migrate'

/**
 * floating：是否运行在独立终端窗口（而非主窗口内嵌面板）。
 * - true（独立窗口）：隐藏停靠按钮、弹出按钮、展开/收起、关闭全部、拖拽把手，
 *   只保留 tabs 切换 + 单个 tab 关闭。独立窗口本身就是浮动的，这些操作无意义。
 * - false（主窗口面板，默认）：完整功能。
 */
const props = defineProps<{ floating?: boolean }>()
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

/** 切换停靠方向，切换后所有终端重新 fit（尺寸语义变了）。 */
function dockTo(side: DockSide) {
  if (store.dockSide === side) return
  store.setDockSide(side)
  nextTick(() => {
    Object.values(termRefs.value).forEach((t) => t?.refit())
  })
}

/** 把当前激活的终端「撕离」为独立窗口：
 *  - detach 移除主窗口该 tab（终端消失），但保留后端 PTY 会话
 *  - 独立窗口接管同一个 sessionId（新建 xterm 订阅），实现会话迁移而非镜像
 *  独立窗口可拖到主窗口外/第二显示器。 */
async function popout() {
  const tab = store.activeTab
  if (!tab || !tab.sessionId) {
    ElMessage.warning('终端尚未就绪，请稍候')
    return
  }
  const sid = tab.sessionId
  const name = tab.name
  // 先标记迁移：detach 会移除 tab 触发面板 Terminal unmount，
  // 此时不能 ptyKill（session 要交给独立窗口），靠标记跳过。
  markMigrating(sid)
  const detached = store.detach(tab.id)
  if (!detached) {
    return
  }
  // 投递到独立终端窗口：后端确保窗口存在并向其 emit attach-terminal
  try {
    await api.attachTerminal(sid, name)
  } catch (e) {
    ElMessage.error(`投递到独立窗口失败：${e}`)
  }
}

/** 是否横向停靠（左/右），决定尺寸绑定用 width 还是 height。 */
const isHorizontal = computed(
  () => store.dockSide === 'left' || store.dockSide === 'right',
)

/** 面板内联样式：底部绑 height，左右绑 width。 */
const panelStyle = computed(() => {
  if (isHorizontal.value) return { width: `${store.width}px` }
  return { height: `${store.height}px` }
})

/** 当前停靠位的动态 class，用于切换边框/把手/动画方向。 */
const dockClass = computed(() => `dock-${store.dockSide}`)

// ===== 拖拽调整尺寸（支持上下/左右两个轴向） =====
const dragging = ref(false)

function onDragStart(e: MouseEvent) {
  e.preventDefault()
  dragging.value = true
  const startX = e.clientX
  const startY = e.clientY
  const startW = store.width
  const startH = store.height
  const side = store.dockSide

  const onMove = (ev: MouseEvent) => {
    // 各停靠位拖拽方向不同：
    // bottom：向上拖增大高度（deltaY 负 → 增高）
    // right ：向左拖增大宽度（deltaX 负 → 增宽）
    // left  ：向右拖增大宽度（deltaX 正 → 增宽）
    if (side === 'bottom') {
      store.setHeight(startH + (startY - ev.clientY))
    } else if (side === 'right') {
      store.setWidth(startW + (startX - ev.clientX))
    } else {
      // left
      store.setWidth(startW + (ev.clientX - startX))
    }
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
</script>

<template>
  <transition :name="`terminal-slide-${store.dockSide}`">
    <div
      v-if="store.visible && store.tabs.length"
      class="terminal-panel"
      :class="[dockClass, { 'terminal-panel--floating': floating }]"
      :style="floating ? undefined : panelStyle"
    >
      <!-- 拖拽把手：仅主窗口面板需要（独立窗口铺满，无把手） -->
      <div v-if="!floating" class="resize-handle" @mousedown="onDragStart">
        <span class="handle-bar" />
      </div>

      <!-- 面板主体：header + terminal。左右停靠时与竖把手横向排列 -->
      <div class="panel-body">
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
                class="tab-action tab-close"
                :size="12"
                title="关闭"
                @click.stop="closeTab(t.id)"
              >
                <Close />
              </el-icon>
            </div>
          </div>
          <!-- 操作按钮组：仅主窗口面板需要。独立窗口无停靠/弹出/关闭全部的意义 -->
          <div v-if="!floating" class="panel-actions">
            <!-- 停靠位切换：下 / 左 / 右 -->
            <el-tooltip content="停靠到底部" placement="top">
              <el-icon
                class="action-btn dock-btn"
                :class="{ 'dock-btn--active': store.dockSide === 'bottom' }"
                @click="dockTo('bottom')"
              >
                <Bottom />
              </el-icon>
            </el-tooltip>
            <el-tooltip content="停靠到左侧" placement="top">
              <el-icon
                class="action-btn dock-btn"
                :class="{ 'dock-btn--active': store.dockSide === 'left' }"
                @click="dockTo('left')"
              >
                <Back />
              </el-icon>
            </el-tooltip>
            <el-tooltip content="停靠到右侧" placement="top">
              <el-icon
                class="action-btn dock-btn"
                :class="{ 'dock-btn--active': store.dockSide === 'right' }"
                @click="dockTo('right')"
              >
                <Right />
              </el-icon>
            </el-tooltip>
            <!-- 弹出独立窗口（可拖离主窗口） -->
            <el-tooltip content="弹出独立窗口" placement="top">
              <el-icon class="action-btn action-btn--primary" @click="popout">
                <Expand />
              </el-icon>
            </el-tooltip>
            <!-- 展开/收起（仅底部停靠有意义：调高度） -->
            <el-tooltip
              v-if="store.dockSide === 'bottom'"
              :content="store.height < 500 ? '展开' : '收起'"
              placement="top"
            >
              <el-icon
                class="action-btn action-btn--primary"
                @click="store.setHeight(store.height < 500 ? 600 : 200)"
              >
                <ArrowUp v-if="store.height < 500" />
                <ArrowDown v-else />
              </el-icon>
            </el-tooltip>
            <el-tooltip content="关闭全部" placement="top">
              <el-icon class="action-btn action-btn--danger" @click="closeAll">
                <Close />
              </el-icon>
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
    </div>
  </transition>
</template>

<style scoped>
/* ===== 面板容器：底部=column，左右=row（竖把手 + 主体） ===== */
.terminal-panel {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: #1e1e1e; /* VS Code Dark+ 编辑器背景 */
  z-index: 50;
  user-select: none;
}
/* 独立终端窗口模式：铺满父级，无停靠边框/阴影/把手 */
.terminal-panel--floating {
  flex: 1;
  border: none !important;
  box-shadow: none !important;
  z-index: auto;
}
/* 左右停靠：面板改横向，竖把手 + 主体并排 */
.dock-left,
.dock-right {
  flex-direction: row;
}
/* 底部停靠：顶边框 + 上方阴影（贴 content 顶部） */
.dock-bottom {
  border-top: 1px solid var(--el-border-color);
  box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.2);
}
/* 右侧停靠：左边框 + 左侧阴影 */
.dock-right {
  border-left: 1px solid var(--el-border-color);
  box-shadow: -4px 0 16px rgba(0, 0, 0, 0.2);
}
/* 左侧停靠：右边框 + 右侧阴影 */
.dock-left {
  border-right: 1px solid var(--el-border-color);
  box-shadow: 4px 0 16px rgba(0, 0, 0, 0.2);
}

/* ===== 主体：header + terminal，始终纵向 ===== */
.panel-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

/* ===== 拖拽把手：底部=横向(ns)，左右=竖向(ew) ===== */
.resize-handle {
  height: 6px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: ns-resize;
  background: #252526;
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
/* 左右停靠：竖向把手 */
.dock-left .resize-handle,
.dock-right .resize-handle {
  height: auto;
  width: 6px;
  cursor: ew-resize;
}
.dock-left .handle-bar,
.dock-right .handle-bar {
  width: 3px;
  height: 40px;
}
/* 左侧停靠：把手应在主体右侧（贴 content），用 order 排到后面。
   DOM 固定为 handle → panel-body，默认 handle 在前=左；
   左侧停靠要把 handle 移到右，故 handle order=2，body order=1。 */
.dock-left .resize-handle {
  order: 2;
}
.dock-left .panel-body {
  order: 1;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px;
  background: #252526; /* VS Code 标题栏/侧边栏色 */
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
  background: #1e1e1e;
  color: #cccccc;
}
.tab-name {
  max-width: 140px;
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
  /* 危险操作：与「关闭全部」一致，红底反白 */
  background: #f14c4c;
  color: #1e1e1e;
}

.panel-actions {
  display: flex;
  gap: 4px;
  align-items: center;
  flex-shrink: 0;
}
/* 操作按钮：语义化配色。
   - 展开/收起 = Blue（主操作）
   - 关闭全部 = Red（危险操作，常态即显红描边 + hover 反白强警示） */
.action-btn {
  cursor: pointer;
  font-size: 24px; /* el-icon 默认 1em，头部字号小导致图标偏小，显式放大 */
  padding: 4px; /* 点击热区 */
  border-radius: 4px;
  color: var(--el-text-color-secondary);
  transition: background 0.15s, color 0.15s;
}
.action-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #cccccc;
}
/* 主操作：蓝 */
.action-btn--primary {
  color: #3b8eea;
}
.action-btn--primary:hover {
  background: #2a2d2e;
  color: #4fc3ff;
}
/* 危险操作：红，hover 反白 */
.action-btn--danger {
  color: #f14c4c;
}
.action-btn--danger:hover {
  background: #f14c4c;
  color: #1e1e1e;
}
/* 停靠位按钮：未选中=灰，选中=蓝高亮（指示当前停靠位） */
.dock-btn {
  font-size: 18px; /* 停靠按钮比展开/收起略小，视觉分组 */
}
.dock-btn--active {
  color: #3b8eea;
  background: #2a2d2e;
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

/* ===== 滑入/滑出动画：按停靠方向 ===== */
.terminal-slide-bottom-enter-active,
.terminal-slide-bottom-leave-active,
.terminal-slide-left-enter-active,
.terminal-slide-left-leave-active,
.terminal-slide-right-enter-active,
.terminal-slide-right-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
/* 底部：从下滑入 */
.terminal-slide-bottom-enter-from,
.terminal-slide-bottom-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
/* 左侧：从左滑入 */
.terminal-slide-left-enter-from,
.terminal-slide-left-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}
/* 右侧：从右滑入 */
.terminal-slide-right-enter-from,
.terminal-slide-right-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
