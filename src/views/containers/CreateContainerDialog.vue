<script setup lang="ts">
/**
 * 创建容器向导（el-steps 三步）。
 * 步骤1：镜像（本地已有下拉）+ 容器名 + 资源限制(CPU/内存,可留空不限) + 开机自启
 * 步骤2：端口 + 存储挂载 + 环境变量 + 网络 + 启动命令
 * 步骤3：确认 + 创建（docker run -d）
 */
import { computed, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import type {
  CreateContainerOpts,
  Image,
  Network,
  PortMapping,
  VolumeMount,
} from '@/api/types'

const props = defineProps<{
  modelValue: boolean
  hostId: string
  /** 预填镜像名（从镜像页入口传入，如 "nginx:latest"） */
  presetImage?: string
  /** 创建成功后回调 */
  onCreated?: () => void
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
})

// ===== 步骤 =====
const active = ref(0)
const steps = ['基础配置', '端口与存储', '确认创建']

// ===== 表单数据 =====
const images = ref<Image[]>([])
const networks = ref<Network[]>([])

const form = ref({
  image: '',           // 镜像名（repository:tag）
  name: '',            // 容器名
  cpuLimit: '',        // CPU 核数，空=不限
  memLimit: '',        // 内存上限，空=不限
  restart: 'no',       // 重启策略
  // 步骤2
  ports: [] as PortMapping[],
  volumes: [] as VolumeMount[],
  envs: [] as string[],
  network: '',         // 网络
  command: '',         // 启动命令
})

const saving = ref(false)

// 镜像下拉选项：去重 repository:tag
const imageOptions = computed(() => {
  const seen = new Set<string>()
  const out: { label: string; value: string }[] = []
  for (const img of images.value) {
    const value = `${img.repository}:${img.tag}`
    if (img.repository === '<none>' || seen.has(value)) continue
    seen.add(value)
    out.push({ label: `${value}  (${img.size})`, value })
  }
  return out
})

const networkOptions = computed(() =>
  networks.value.map((n) => ({ label: n.name, value: n.name })),
)

// ===== 动态行增删 =====
function addPort() {
  form.value.ports.push({ host: '', container: '', protocol: '' })
}
function removePort(i: number) {
  form.value.ports.splice(i, 1)
}
function addVolume() {
  form.value.volumes.push({ host: '', container: '', readOnly: false })
}
function removeVolume(i: number) {
  form.value.volumes.splice(i, 1)
}
function addEnv() {
  form.value.envs.push('')
}
function removeEnv(i: number) {
  form.value.envs.splice(i, 1)
}

// ===== 校验 =====
function validateStep(n: number): string | null {
  if (n === 0) {
    if (!form.value.image.trim()) return '请选择镜像'
    if (form.value.cpuLimit && isNaN(Number(form.value.cpuLimit))) return 'CPU 限制必须是数字'
    if (form.value.memLimit && !/^\d+[kmgt]?b?$/i.test(form.value.memLimit)) {
      return '内存限制格式错误，如 512m / 1g'
    }
  }
  return null
}

function next() {
  const err = validateStep(active.value)
  if (err) {
    ElMessage.warning(err)
    return
  }
  active.value = Math.min(active.value + 1, steps.length - 1)
}
function prev() {
  active.value = Math.max(active.value - 1, 0)
}

// ===== 加载数据 =====
async function loadOptions() {
  try {
    const [imgs, nets] = await Promise.all([
      api.listImages(props.hostId),
      api.listNetworks(props.hostId),
    ])
    images.value = imgs
    networks.value = nets
  } catch (e) {
    ElMessage.error(`加载镜像/网络列表失败：${e}`)
  }
}

function resetForm() {
  form.value = {
    image: props.presetImage || '',
    name: '',
    cpuLimit: '',
    memLimit: '',
    restart: 'no',
    ports: [],
    volumes: [],
    envs: [],
    network: '',
    command: '',
  }
  active.value = 0
}

// ===== 打开时初始化 =====
watch(
  () => props.modelValue,
  async (v) => {
    if (v) {
      resetForm()
      await loadOptions()
    }
  },
)

// ===== 提交创建 =====
async function submit() {
  const err = validateStep(0)
  if (err) {
    ElMessage.warning(err)
    active.value = 0
    return
  }
  const opts: CreateContainerOpts = {
    image: form.value.image.trim(),
    name: form.value.name.trim(),
    cpuLimit: form.value.cpuLimit.trim(),
    memLimit: form.value.memLimit.trim(),
    restartPolicy: form.value.restart,
    network: form.value.network,
    command: form.value.command.trim(),
    ports: form.value.ports.filter((p) => p.container.trim()),
    volumes: form.value.volumes.filter((v) => v.host.trim() && v.container.trim()),
    envs: form.value.envs.map((e) => e.trim()).filter(Boolean),
  }
  saving.value = true
  try {
    const id = await api.createContainer(props.hostId, opts)
    ElMessage.success(`容器已创建：${id.slice(0, 12)}`)
    visible.value = false
    props.onCreated?.()
  } catch (err) {
    ElMessage.error(`创建失败：${err}`)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <el-dialog
    v-model="visible"
    title="创建容器"
    width="620px"
    top="6vh"
    :close-on-click-modal="false"
    destroy-on-close
  >
    <!-- 步骤条 -->
    <el-steps :active="active" finish-status="success" align-center class="steps">
      <el-step v-for="(s, i) in steps" :key="i" :title="s" />
    </el-steps>

    <div class="form-area">
      <!-- 步骤1：基础配置 -->
      <el-form v-show="active === 0" label-width="90px" label-position="right">
        <el-form-item label="镜像" required>
          <el-select
            v-model="form.image"
            filterable
            placeholder="选择本地镜像"
            class="full-width"
          >
            <el-option
              v-for="opt in imageOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="容器名称">
          <el-input v-model="form.name" placeholder="留空则自动生成" />
        </el-form-item>
        <el-divider content-position="left">资源限制</el-divider>
        <el-form-item label="CPU 限制">
          <el-input v-model="form.cpuLimit" placeholder="留空=不限，如 1.5（核）" />
        </el-form-item>
        <el-form-item label="内存限制">
          <el-input v-model="form.memLimit" placeholder="留空=不限，如 512m / 1g" />
        </el-form-item>
        <el-form-item label="开机自启">
          <el-select v-model="form.restart" class="full-width">
            <el-option label="否" value="no" />
            <el-option label="是（always）" value="always" />
            <el-option label="除非手动停止（unless-stopped）" value="unless-stopped" />
            <el-option label="失败时重启（on-failure）" value="on-failure" />
          </el-select>
        </el-form-item>
      </el-form>

      <!-- 步骤2：端口与存储 -->
      <el-form v-show="active === 1" label-width="90px" label-position="right">
        <!-- 端口映射 -->
        <el-divider content-position="left">端口映射</el-divider>
        <div v-for="(p, i) in form.ports" :key="'p' + i" class="dyn-row">
          <el-input v-model="p.host" placeholder="宿主端口" class="dyn-col" />
          <span class="dyn-arrow">→</span>
          <el-input v-model="p.container" placeholder="容器端口" class="dyn-col" />
          <el-select v-model="p.protocol" placeholder="协议" class="dyn-proto">
            <el-option label="tcp" value="tcp" />
            <el-option label="udp" value="udp" />
          </el-select>
          <el-button :icon="Delete" circle size="small" @click="removePort(i)" />
        </div>
        <el-button :icon="Plus" size="small" text @click="addPort">添加端口</el-button>

        <!-- 存储挂载 -->
        <el-divider content-position="left">存储位置（卷挂载）</el-divider>
        <div v-for="(v, i) in form.volumes" :key="'v' + i" class="dyn-row">
          <el-input v-model="v.host" placeholder="宿主路径" class="dyn-col" />
          <span class="dyn-arrow">→</span>
          <el-input v-model="v.container" placeholder="容器路径" class="dyn-col" />
          <el-checkbox v-model="v.readOnly">只读</el-checkbox>
          <el-button :icon="Delete" circle size="small" @click="removeVolume(i)" />
        </div>
        <el-button :icon="Plus" size="small" text @click="addVolume">添加挂载</el-button>

        <!-- 环境变量 -->
        <el-divider content-position="left">环境变量</el-divider>
        <div v-for="(_env, i) in form.envs" :key="'e' + i" class="dyn-row">
          <el-input v-model="form.envs[i]" placeholder="KEY=VALUE" class="env-input" />
          <el-button :icon="Delete" circle size="small" @click="removeEnv(i)" />
        </div>
        <el-button :icon="Plus" size="small" text @click="addEnv">添加变量</el-button>

        <!-- 网络 + 命令 -->
        <el-divider content-position="left">其他</el-divider>
        <el-form-item label="网络">
          <el-select v-model="form.network" clearable placeholder="默认（bridge）" class="full-width">
            <el-option
              v-for="opt in networkOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="启动命令">
          <el-input v-model="form.command" placeholder="覆盖默认命令（可选）" />
        </el-form-item>
      </el-form>

      <!-- 步骤3：确认 -->
      <div v-show="active === 2" class="confirm-area">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="镜像">{{ form.image || '—' }}</el-descriptions-item>
          <el-descriptions-item label="容器名">{{ form.name || '自动生成' }}</el-descriptions-item>
          <el-descriptions-item label="CPU 限制">{{ form.cpuLimit || '不限' }}</el-descriptions-item>
          <el-descriptions-item label="内存限制">{{ form.memLimit || '不限' }}</el-descriptions-item>
          <el-descriptions-item label="开机自启">{{ form.restart }}</el-descriptions-item>
          <el-descriptions-item label="端口">
            <span v-if="!form.ports.length">无</span>
            <span v-else>
              {{ form.ports.filter(p => p.container).map(p => `${p.host || '?'}:${p.container}${p.protocol ? '/' + p.protocol : ''}`).join(', ') }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="挂载">
            <span v-if="!form.volumes.length">无</span>
            <span v-else>
              {{ form.volumes.filter(v => v.host && v.container).map(v => `${v.host}:${v.container}${v.readOnly ? '(ro)' : ''}`).join(', ') }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="环境变量">
            <span v-if="!form.envs.filter(Boolean).length">无</span>
            <span v-else>{{ form.envs.filter(Boolean).join(', ') }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="网络">{{ form.network || '默认' }}</el-descriptions-item>
          <el-descriptions-item label="启动命令">{{ form.command || '默认' }}</el-descriptions-item>
        </el-descriptions>
        <div class="confirm-cmd">
          <div class="confirm-cmd-title">将执行：</div>
          <code class="mono">docker run -d {{ form.image }}{{ form.name ? ' --name ' + form.name : '' }}</code>
        </div>
      </div>
    </div>

    <!-- 底部按钮 -->
    <template #footer>
      <div class="footer">
        <el-button @click="visible = false">取消</el-button>
        <el-button v-if="active > 0" @click="prev">上一步</el-button>
        <el-button v-if="active < steps.length - 1" type="primary" @click="next">下一步</el-button>
        <el-button
          v-else
          type="primary"
          :loading="saving"
          :icon="Check"
          @click="submit"
        >
          创建容器
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script lang="ts">
import { Check, Plus, Delete } from '@element-plus/icons-vue'
export default {
  components: { Check, Plus, Delete },
}
</script>

<style scoped>
.steps {
  margin-bottom: 24px;
}
.form-area {
  min-height: 280px;
  max-height: 56vh;
  overflow-y: auto;
  padding-right: 4px;
}
.full-width {
  width: 100%;
}
.dyn-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.dyn-col {
  flex: 1;
}
.dyn-arrow {
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}
.dyn-proto {
  width: 90px;
  flex-shrink: 0;
}
.env-input {
  flex: 1;
}
.confirm-area {
  padding: 4px 0;
}
.confirm-cmd {
  margin-top: 16px;
  padding: 10px 12px;
  background: var(--el-fill-color-light);
  border-radius: 6px;
  font-size: 13px;
}
.confirm-cmd-title {
  color: var(--el-text-color-secondary);
  margin-bottom: 4px;
}
.confirm-cmd .mono {
  color: var(--el-color-primary);
  word-break: break-all;
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
