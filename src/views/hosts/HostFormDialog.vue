<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import * as api from '@/api'
import type { AuthType, Host } from '@/api/types'

const props = defineProps<{
  modelValue: boolean
  host: Host | null
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
  verify_host_key: false,
})
const saving = ref(false)

watch(
  () => props.modelValue,
  (v) => {
    if (v) {
      if (props.host) {
        form.value = {
          id: props.host.id,
          name: props.host.name,
          host: props.host.host,
          port: props.host.port,
          user: props.host.user,
          auth_type: props.host.auth_type,
          key_path: props.host.key_path ?? '',
          password: '', // 编辑时不预填密码（安全），留空表示不改
          verify_host_key: props.host.verify_host_key,
        }
      } else {
        form.value = {
          id: undefined,
          name: '',
          host: '',
          port: 22,
          user: 'root',
          auth_type: 'password',
          key_path: '',
          password: '',
          verify_host_key: false,
        }
      }
    }
  },
)

const rules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  host: [{ required: true, message: '请输入主机地址', trigger: 'blur' }],
  user: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
}

const formRef = ref()

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
      key_path: form.value.auth_type === 'key' ? form.value.key_path : null,
      password: form.value.password || undefined,
      verify_host_key: form.value.verify_host_key,
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
    width="480px"
    :close-on-click-modal="false"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="84px"
      label-position="left"
    >
      <el-form-item label="名称" prop="name">
        <el-input v-model="form.name" placeholder="如：生产服务器" />
      </el-form-item>
      <el-form-item label="主机地址" prop="host">
        <el-input v-model="form.host" placeholder="IP 或域名" />
      </el-form-item>
      <el-form-item label="端口">
        <el-input-number v-model="form.port" :min="1" :max="65535" />
      </el-form-item>
      <el-form-item label="用户名" prop="user">
        <el-input v-model="form.user" />
      </el-form-item>
      <el-form-item label="认证方式">
        <el-radio-group v-model="form.auth_type">
          <el-radio value="password">密码</el-radio>
          <el-radio value="key">密钥</el-radio>
          <el-radio value="agent">ssh-agent</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item v-if="form.auth_type === 'password'" label="密码">
        <el-input
          v-model="form.password"
          type="password"
          show-password
          :placeholder="host ? '留空表示不修改密码' : '请输入密码'"
        />
      </el-form-item>
      <el-form-item v-if="form.auth_type === 'key'" label="私钥路径">
        <el-input
          v-model="form.key_path"
          placeholder="如 ~/.ssh/id_rsa 或 C:\\Users\\you\\.ssh\\id_rsa"
        />
      </el-form-item>
      <el-form-item label="校验指纹">
        <el-switch v-model="form.verify_host_key" />
        <span class="tip">关闭则在首次连接时不校验 host key</span>
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

<style scoped>
.tip {
  margin-left: 12px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
