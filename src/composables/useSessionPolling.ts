import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'
import type { PetState } from '@/types/claude'

interface SessionInfoRaw {
  session_id: string
  project_name: string
  project_path: string
  file_path: string
  state: string
  detail: string | null
  last_activity: number
}

/**
 * 每 5 秒轮询 get_active_sessions，补充事件驱动无法覆盖的场景
 */
export function useSessionPolling() {
  const claudeStore = useClaudeStore()
  const timer = ref<ReturnType<typeof setInterval> | null>(null)

  async function poll() {
    try {
      const raw = await invoke<SessionInfoRaw[]>(INVOKE_KEY.GET_ACTIVE_SESSIONS)
      claudeStore.setSessions(
        raw.map((s) => ({
          id: s.session_id,
          projectName: s.project_name,
          projectPath: s.project_path,
          filePath: s.file_path,
          state: s.state as PetState,
          detail: s.detail,
          lastActivity: s.last_activity,
        })),
      )
    } catch (e) {
      console.error('Failed to poll sessions:', e)
    }
  }

  onMounted(() => {
    poll()
    timer.value = setInterval(poll, 5000)
  })

  onUnmounted(() => {
    if (timer.value) clearInterval(timer.value)
  })
}
