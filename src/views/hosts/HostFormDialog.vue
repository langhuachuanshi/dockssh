<script setup lang="ts">
/**
 * 主机新增/编辑弹窗。
 *
 * 字段：服务器 / 连接名称 / 端口 / 验证方式(密码|密钥)
 *   密码 → 用户名 + 密码
 *   密钥 → 用户名 + 密钥(文件选择) + 密钥口令
 *   分组(下拉，可新建)
 *
 * - 表单项不显示必填星号（hide-required-asterisk），保存时才统一校验
 * - 密码与密钥口令仅在填写时提交，留空表示不改（编辑场景）
 */
import { computed, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import * as api from '@/api'
import type { AuthType, Host } from '@/api/types'

const props = defineProps<{
  modelValue: boolean
  host: Host | null
  /** 已有分组名列表，用于下拉候选 */
  groups?: string[]
}>()
const emit = defineEmits<{
  'update:modelValue': [val: boolean]
  saved: []
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
})

const form = ref({
  id: undefined as string | undefined,
  name: '',
  host: '',
  port: 22,
  user: 'root',
  auth_type: 'password' as AuthType,
  key_path: '',
  password: '',
  passphrase: '',
  group: '' as string,
})
const saving = ref(false)
const groupOptions = computed(() => props.groups ?? [])

function resetForm() {
  form.value = {
    id: undefined,
    name: '',
    host: '',
    port: 22,
    user: 'root',
    auth_type: 'password',
    key_path: '',
    password: '',
    passphrase: '',
    group: '',
  }
}

watch(
  () => props.modelValue,
  (v) => {
    if (!v) return
    if (props.host) {
      form.value = {
        id: props.host.id,
        name: props.host.name,
        host: props.host.host,
        port: props.host.port,
        user: props.host.user,
        auth_type: props.host.auth_type,
        key_path: props.host.key_path ?? '',
        // 编辑时不预填凭据（安全），留空表示不改
        password: '',
        passphrase: '',
        group: props.host.group ?? '',
      }
    } else {
      resetForm()
    }
  },
)

const rules = {
  name: [{ required: true, message: '请输入连接名称', trigger: 'blur' }],
  host: [{ required: true, message: '请输入服务器地址', trigger: 'blur' }],
  user: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
}

const formRef = ref()

// 选择密钥文件（Tauri 原生文件框）
async function pickKey() {
  try {
    const selected = await open({
      title: '选择私钥文件',
      multiple: false,
      directory: false,
    })
    if (typeof selected === 'string' && selected) {
      form.value.key_path = selected
    }
  } catch {
    /* 用户取消 */
  }
}

async function submit() {
  await formRef.value?.validate()
  saving.value = true
  try {
    await api.saveHost({
      id: form.value.id,
      name: form.value.name,
      host: form.value.host,
      port: Number(form.value.port),
      user: form.value.user,
      auth_type: form.value.auth_type,
      key_path: form.value.auth_type === 'key' ? form.value.key_path || null : null,
      password: form.value.auth_type === 'password' ? form.value.password || undefined : undefined,
      passphrase: form.value.auth_type === 'key' ? form.value.passphrase || undefined : undefined,
      group: form.value.group || null,
    })
    ElMessage.success(props.host ? '已更新' : '已添加')
    emit('saved')
  } catch (e) {
    ElMessage.error(`保存失败：${e}`)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="host ? '编辑主机' : '添加主机'"
    width="460px"
    :close-on-click-modal="false"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      :hide-required-asterisk="true"
      label-width="88px"
      label-position="left"
    >
      <el-form-item label="服务器" prop="host">
        <el-input v-model="form.host" placeholder="IP 或域名" />
      </el-form-item>
      <el-form-item label="连接名称" prop="name">
        <el-input v-model="form.name" placeholder="如：生产服务器" />
      </el-form-item>
      <el-form-item label="端口">
        <el-input-number v-model="form.port" :min="1" :max="65535" />
      </el-form-item>

      <el-divider content-position="left">验证方式</el-divider>
      <el-form-item label="验证方式">
        <el-radio-group v-model="form.auth_type">
          <el-radio value="password">密码验证</el-radio>
          <el-radio value="key">密钥验证</el-radio>
        </el-radio-group>
      </el-form-item>

      <!-- 密码验证 -->
      <template v-if="form.auth_type === 'password'">
        <el-form-item label="用户名" prop="user">
          <el-input v-model="form.user" />
        </el-form-item>
        <el-form-item label="密码">
          <el-input
            v-model="form.password"
            type="password"
            show-password
            :placeholder="host ? '留空表示不修改' : '请输入密码'"
          />
        </el-form-item>
      </template>

      <!-- 密钥验证 -->
      <template v-else>
        <el-form-item label="用户名" prop="user">
          <el-input v-model="form.user" />
        </el-form-item>
        <el-form-item label="密钥">
          <el-input
            v-model="form.key_path"
            placeholder="点击右侧选择密钥文件"
            readonly
          >
            <template #append>
              <el-button :icon="FolderOpened" @click="pickKey">选择</el-button>
            </template>
          </el-input>
        </el-form-item>
        <el-form-item label="密钥口令">
          <el-input
            v-model="form.passphrase"
            type="password"
            show-password
            :placeholder="host ? '留空表示不修改' : '无私钥口令可留空'"
          />
        </el-form-item>
      </template>

      <el-divider content-position="left">归类</el-divider>
      <el-form-item label="分组">
        <el-select
          v-model="form.group"
          placeholder="选择或输入分组（可留空）"
          filterable
          allow-create
          clearable
          default-first-option
          style="width: 100%"
        >
          <el-option v-for="g in groupOptions" :key="g" :label="g" :value="g" />
        </el-select>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="saving" @click="submit">
        保存
      </el-button>
    </template>
  </el-dialog>
</template>

<script lang="ts">
import { FolderOpened } from '@element-plus/icons-vue'
export default { components: { FolderOpened } }
</script>
