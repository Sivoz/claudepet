import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'
import { useClaudeStore } from '@/stores/claude'

/**
 * 前端调用 Rust 侧 Claude Code 操作
 */
export function useClaudeActions() {
  const claudeStore = useClaudeStore()

  async function startWatcher() {
    try {
      await invoke(INVOKE_KEY.START_WATCHER)
      claudeStore.setWatching(true)
    } catch (e) {
      console.error('Failed to start watcher:', e)
    }
  }

  async function stopWatcher() {
    try {
      await invoke(INVOKE_KEY.STOP_WATCHER)
      claudeStore.setWatching(false)
    } catch (e) {
      console.error('Failed to stop watcher:', e)
    }
  }

  async function getStatus(): Promise<string> {
    return invoke<string>(INVOKE_KEY.GET_CLAUDE_STATUS)
  }

  return {
    startWatcher,
    stopWatcher,
    getStatus,
  }
}
