import { EVENT_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'
import { useTauriListen } from './useTauriListen'
import type { PetState, PermissionRequest } from '@/types/claude'

interface PetEvent {
  state: string
  session_id: string | null
  project_name: string | null
  detail: string | null
  input_tokens: number | null
  output_tokens: number | null
  model: string | null
}

/** 活跃状态前端兜底超时（略大于后端 30 秒，避免双重触发） */
const ACTIVE_TIMEOUT_MS = 35_000

/**
 * 监听 Rust 推送的 Claude Code 事件，更新 store
 */
export function useClaudeEvents() {
  const claudeStore = useClaudeStore()

  let activeTimer: ReturnType<typeof setTimeout> | null = null

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
        inputTokens: event.input_tokens || 0,
        outputTokens: event.output_tokens || 0,
        startedAt: Date.now(),
        model: event.model,
      })
    }

    // 累加 token 消耗
    if (event.input_tokens || event.output_tokens) {
      claudeStore.addTokens(event.input_tokens || 0, event.output_tokens || 0)
    }

    // dev 模式下记录状态转移路径
    if (import.meta.env.DEV) {
      console.debug(`[state] ${claudeStore.petState} → ${state}`, event.detail ?? '')
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

    // 前端兜底：活跃状态超时回 idle（后端 watchdog 的保险层）
    if (activeTimer) clearTimeout(activeTimer)
    if (state === 'thinking' || state === 'coding') {
      activeTimer = setTimeout(() => {
        if (claudeStore.petState === 'thinking' || claudeStore.petState === 'coding') {
          console.warn('[claudepet] active state frontend timeout, fallback to idle')
          claudeStore.setPetState('idle')
        }
      }, ACTIVE_TIMEOUT_MS)
    }
  })

  // 监听权限请求
  useTauriListen<PermissionRequest>(EVENT_KEY.PERMISSION_REQUEST, (req) => {
    claudeStore.addPermission(req)

    // 切换到 waiting 状态
    const prevState = claudeStore.petState
    claudeStore.setPetState('waiting')

    // hook 脚本 120 秒超时后会自行返回 "ask"
    // 前端留 5 秒缓冲（125s），确保 hook 脚本和 Rust 清理先执行
    setTimeout(() => {
      claudeStore.removePermission(req.requestId)
      // 超时后如果仍是 waiting 且无其他待审批，恢复之前状态
      if (claudeStore.petState === 'waiting' && claudeStore.pendingPermissions.length === 0) {
        claudeStore.setPetState(prevState)
      }
    }, 125_000)
  })

}
