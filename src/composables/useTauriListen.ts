import { onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * 封装 Tauri 事件监听，自动在组件卸载时取消
 */
export function useTauriListen<T>(event: string, handler: (payload: T) => void) {
  let unlisten: UnlistenFn | null = null

  onMounted(async () => {
    unlisten = await listen<T>(event, (e) => {
      handler(e.payload)
    })
  })

  onUnmounted(() => {
    unlisten?.()
  })
}
