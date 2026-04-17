import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'
import { useToastStore } from '@/stores/toast'

/**
 * 权限审批操作
 */
export function usePermissions() {
  const claudeStore = useClaudeStore()
  const toastStore = useToastStore()

  function restoreFromWaiting() {
    if (claudeStore.petState === 'waiting' && claudeStore.pendingPermissions.length === 0) {
      claudeStore.setPetState('idle')
    }
  }

  async function approve(requestId: string) {
    try {
      await invoke(INVOKE_KEY.RESPOND_PERMISSION, {
        requestId,
        decision: 'allow',
      })
      claudeStore.removePermission(requestId)
      restoreFromWaiting()
    } catch (e) {
      toastStore.error(`审批失败: ${e}`)
    }
  }

  async function deny(requestId: string) {
    try {
      await invoke(INVOKE_KEY.RESPOND_PERMISSION, {
        requestId,
        decision: 'deny',
      })
      claudeStore.removePermission(requestId)
      restoreFromWaiting()
    } catch (e) {
      toastStore.error(`拒绝失败: ${e}`)
    }
  }

  return { approve, deny }
}
