import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'

/**
 * 权限审批操作
 */
export function usePermissions() {
  const claudeStore = useClaudeStore()

  async function approve(requestId: string) {
    try {
      await invoke(INVOKE_KEY.RESPOND_PERMISSION, {
        requestId,
        decision: 'allow',
      })
      claudeStore.removePermission(requestId)
    } catch (e) {
      console.error('Failed to approve permission:', e)
    }
  }

  async function deny(requestId: string) {
    try {
      await invoke(INVOKE_KEY.RESPOND_PERMISSION, {
        requestId,
        decision: 'deny',
      })
      claudeStore.removePermission(requestId)
    } catch (e) {
      console.error('Failed to deny permission:', e)
    }
  }

  return { approve, deny }
}
