import { EVENT_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'
import { useTauriListen } from './useTauriListen'
import type { PetState, PermissionRequest } from '@/types/claude'

interface PetEvent {
  state: string
  session_id: string | null
  project_name: string | null
  detail: string | null
}

/**
 * 监听 Rust 推送的 Claude Code 事件，更新 store
 */
export function useClaudeEvents() {
  const claudeStore = useClaudeStore()

  useTauriListen<PetEvent>(EVENT_KEY.CLAUDE_EVENT, (event) => {
    const state = event.state as PetState

    // 更新会话信息
    if (event.session_id) {
      claudeStore.upsertSession({
        id: event.session_id,
        projectName: event.project_name || 'unknown',
        projectPath: '',
        filePath: '',
        state,
        detail: event.detail,
        lastActivity: Date.now(),
      })
    }

    // 驱动宠物状态：使用 primary session 或最新活跃会话
    const isPrimary =
      !claudeStore.primarySessionId ||
      claudeStore.primarySessionId === event.session_id
    if (isPrimary) {
      claudeStore.setPetState(state)
    }

    // 收到 success 后 3 秒回到 idle
    if (state === 'success' && isPrimary) {
      setTimeout(() => {
        if (claudeStore.petState === 'success') {
          claudeStore.setPetState('idle')
        }
      }, 3000)
    }
  })

  // 监听权限请求
  useTauriListen<PermissionRequest>(EVENT_KEY.PERMISSION_REQUEST, (req) => {
    claudeStore.addPermission(req)
  })

}
