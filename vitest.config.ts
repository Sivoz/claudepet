import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@tauri-apps/api/core': resolve(__dirname, 'src/__mocks__/tauri.ts'),
      '@tauri-apps/api/event': resolve(__dirname, 'src/__mocks__/tauri-event.ts'),
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
  },
})
