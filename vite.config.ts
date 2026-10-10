import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// Tauri 期望固定端口，失败则报错而非静默换端口
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [vue()],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },

  // Tauri 需要固定端口
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 1421 }
      : undefined,
    watch: {
      // 不要监听 Rust 侧，避免无谓重载
      ignored: ['**/src-tauri/**'],
    },
  },

  // 生产构建优化
  build: {
    target: 'chrome110',       // WebView2 版本足够新
    minify: 'esbuild',
    sourcemap: false,
    chunkSizeWarningLimit: 1500,
  },

  // Tauri 相关环境变量前缀
  envPrefix: ['VITE_', 'TAURI_'],
})
