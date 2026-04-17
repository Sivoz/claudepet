import { onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY, EVENT_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'
import { useTauriListen } from './useTauriListen'
import type { PetState } from '@/types/claude'

interface SessionInfoRaw {
  session_id: string
  project_name: string
  project_path: string
  file_path: string
  state: string
  detail: string | null
  last_activity: number
  input_tokens: number
  output_tokens: number
  started_at: number
  model: string | null
}

function mapSessions(raw: SessionInfoRaw[]) {
  return raw.map((s) => ({
    id: s.session_id,
    projectName: s.project_name,
    projectPath: s.project_path,
    filePath: s.file_path,
    state: s.state as PetState,
    detail: s.detail,
    lastActivity: s.last_activity,
    inputTokens: s.input_tokens,
    outputTokens: s.output_tokens,
    startedAt: s.started_at,
    model: s.model,
  }))
}

/**
 * 事件驱动的会话同步，辅以 30 秒兜底轮询
 */
export function useSessionPolling() {
  const claudeStore = useClaudeStore()
  let timer: ReturnType<typeof setInterval> | null = null

  // 监听 Rust 主动推送的会话变更
  useTauriListen<SessionInfoRaw[]>(EVENT_KEY.SESSION_CHANGED, (sessions) => {
    claudeStore.setSessions(mapSessions(sessions))
  })

  // 兜底轮询：处理启动时和 watcher 可能遗漏的场景
  async function poll() {
    try {
      const raw = await invoke<SessionInfoRaw[]>(INVOKE_KEY.GET_ACTIVE_SESSIONS)
      claudeStore.setSessions(mapSessions(raw))
    } catch (e) {
      console.error('Failed to poll sessions:', e)
    }
  }

  onMounted(() => {
    poll()
    timer = setInterval(poll, 30_000)
  })

  onUnmounted(() => {
    if (timer) clearInterval(timer)
  })
}
