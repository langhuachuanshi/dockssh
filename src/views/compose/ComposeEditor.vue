<script setup lang="ts">
/**
 * Compose 文件编辑器（CodeMirror 6）。
 *
 * - 行号 + YAML 高亮 + one-dark 主题（与 Element Plus 深色 UI 契合）
 * - 通过 path（远程 compose 文件绝对路径）读取/保存
 * - 内容变更本地维护；保存时回写远端
 *
 * 由父组件控制可见性并传入 path；本组件负责加载内容、维护编辑器实例。
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { EditorState } from '@codemirror/state'
import { EditorView, lineNumbers } from '@codemirror/view'
import { yaml } from '@codemirror/lang-yaml'
import { oneDark } from '@codemirror/theme-one-dark'
import * as api from '@/api'
import { useHostsStore } from '@/store/hosts'
import { useRoute } from 'vue-router'

const props = defineProps<{
  /** 远程 compose 文件绝对路径；为空时编辑器不加载 */
  path: string
}>()
const emit = defineEmits<{ saved: [] }>()

const route = useRoute()
const store = useHostsStore()
const hostId = computed(() => route.params.id as string)

const editorEl = ref<HTMLDivElement>()
// CodeMirror 实例用 shallowRef 持有，避免 Vue 深度代理（实例无需响应式）
const view = shallowRef<EditorView | null>(null)
const loading = ref(false)
const saving = ref(false)
// 是否有未保存修改
const dirty = ref(false)

// one-dark 下编辑区底色偏深，让编辑器铺满并跟随主题
const extensions = [
  lineNumbers(),
  EditorView.lineWrapping,
  yaml(),
  oneDark,
  EditorView.theme({
    '&': { height: '100%', fontSize: '13px' },
    '.cm-scroller': { fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace" },
  }),
  EditorView.updateListener.of((u) => {
    if (u.docChanged) dirty.value = true
  }),
]

function destroyView() {
  view.value?.destroy()
  view.value = null
}

async function load() {
  if (!props.path) return
  loading.value = true
  try {
    await store.ensureConnected(hostId.value)
    const content = await api.readComposeFile(hostId.value, props.path)
    destroyView()
    // 确保挂载容器在 DOM 中（弹窗 v-if 首次渲染/切换时尤其需要）
    await nextTick()
    if (editorEl.value) {
      view.value = new EditorView({
        state: EditorState.create({ doc: content, extensions }),
        parent: editorEl.value,
      })
    }
    dirty.value = false
  } catch (e) {
    ElMessage.error(`读取失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function save() {
  if (!view.value) return
  saving.value = true
  try {
    const content = view.value.state.doc.toString()
    await api.saveComposeFile(hostId.value, props.path, content)
    dirty.value = false
    ElMessage.success('已保存')
    emit('saved')
  } catch (e) {
    ElMessage.error(`保存失败：${e}`)
  } finally {
    saving.value = false
  }
}

// 首次挂载即加载（弹窗内每次打开都会重建本组件）
onMounted(() => load())

// path 变化（切换编辑目标）时重新加载
watch(() => props.path, (p) => {
  if (p) load()
})

onBeforeUnmount(() => destroyView())

defineExpose({ save })
</script>

<template>
  <div class="editor-host" v-loading="loading">
    <div ref="editorEl" class="cm-box" />
  </div>
</template>

<style scoped>
.editor-host {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border: 1px solid var(--el-border-color);
  border-radius: 6px;
  overflow: hidden;
}
.cm-box {
  height: 100%;
  min-height: 0;
}
.cm-box :deep(.cm-editor) {
  height: 100%;
}
.cm-box :deep(.cm-scroller) {
  overflow: auto;
}
</style>
