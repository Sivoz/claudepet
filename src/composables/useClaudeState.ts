import { watch, ref } from 'vue'
import { useClaudeStore } from '@/stores/claude'

/**
 * 监听宠物状态变化，管理 sleep 超时
 */
export function useClaudeState() {
  const claudeStore = useClaudeStore()
  const sleepTimer = ref<ReturnType<typeof setTimeout> | null>(null)

  function resetSleepTimer() {
    if (sleepTimer.value) clearTimeout(sleepTimer.value)
    sleepTimer.value = setTimeout(() => {
      if (claudeStore.petState === 'idle') {
        claudeStore.setPetState('sleeping')
      }
    }, 5 * 60 * 1000)
  }

  watch(
    () => claudeStore.petState,
    (state) => {
      if (state !== 'sleeping' && state !== 'idle') {
        resetSleepTimer()
      }
    },
    { immediate: true },
  )

  resetSleepTimer()
}
