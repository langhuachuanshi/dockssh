import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 期望前端以相对根部署，且开发时监听 127.0.0.1
const host = process.env.TAURI_DEV_HOST || 'localhost'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  // Tauri 用相对路径打包
  base: './',
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // Tauri dev 时确保端口固定、监听本机
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 5174,
        }
      : undefined,
    watch: {
      // 不监听 Rust 目录，避免频繁重启
      ignored: ['**/src-tauri/**'],
    },
  },
})
