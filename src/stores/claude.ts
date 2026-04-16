import { ref, watch } from 'vue'
import { defineStore } from 'pinia'
import type { PetState, ClaudeSession, PermissionRequest } from '@/types/claude'

/** localStorage 持久化 key */
const STORAGE_KEY = 'claude-pet-settings'

interface PersistedSettings {
  hiddenSessionIds: string[]
  labelSize: number
}

function loadSettings(): PersistedSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch { /* ignore */ }
  return { hiddenSessionIds: [], labelSize: 10 }
}

function saveSettings(settings: PersistedSettings) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings))
}

export const useClaudeStore = defineStore('claude', () => {
  const saved = loadSettings()

  const petState = ref<PetState>('idle')
  const activeSession = ref<ClaudeSession | null>(null)
  const isWatching = ref(false)
  const sessions = ref<ClaudeSession[]>([])
  const pendingPermissions = ref<PermissionRequest[]>([])
  const primarySessionId = ref<string | null>(null)
  // 用数组代替 Set，Vue 3 对数组的响应式追踪更可靠
  const hiddenSessionIds = ref<string[]>(saved.hiddenSessionIds)
  const labelSize = ref(saved.labelSize)

  // 自动持久化
  watch([hiddenSessionIds, labelSize], () => {
    saveSettings({
      hiddenSessionIds: hiddenSessionIds.value,
      labelSize: labelSize.value,
    })
  }, { deep: true })

  function setPetState(state: PetState) {
    petState.value = state
  }

  function setActiveSession(session: ClaudeSession | null) {
    activeSession.value = session
  }

  function setWatching(watching: boolean) {
    isWatching.value = watching
  }

  function setSessions(list: ClaudeSession[]) {
    sessions.value = list
  }

  function upsertSession(session: ClaudeSession) {
    const idx = sessions.value.findIndex((s) => s.id === session.id)
    if (idx >= 0) {
      sessions.value[idx] = session
    } else {
      sessions.value.push(session)
    }
    sessions.value.sort((a, b) => b.lastActivity - a.lastActivity)
  }

  function addPermission(req: PermissionRequest) {
    if (!pendingPermissions.value.find((p) => p.requestId === req.requestId)) {
      pendingPermissions.value.push(req)
    }
  }

  function removePermission(requestId: string) {
    pendingPermissions.value = pendingPermissions.value.filter(
      (p) => p.requestId !== requestId,
    )
  }

  function setPrimarySession(sessionId: string | null) {
    primarySessionId.value = sessionId
  }

  function toggleSessionVisibility(sessionId: string) {
    const idx = hiddenSessionIds.value.indexOf(sessionId)
    if (idx >= 0) {
      hiddenSessionIds.value.splice(idx, 1)
    } else {
      hiddenSessionIds.value.push(sessionId)
    }
  }

  function isSessionVisible(sessionId: string): boolean {
    return !hiddenSessionIds.value.includes(sessionId)
  }

  return {
    petState,
    activeSession,
    isWatching,
    sessions,
    pendingPermissions,
    primarySessionId,
    setPetState,
    setActiveSession,
    setWatching,
    setSessions,
    upsertSession,
    addPermission,
    removePermission,
    setPrimarySession,
    hiddenSessionIds,
    toggleSessionVisibility,
    isSessionVisible,
    labelSize,
  }
})