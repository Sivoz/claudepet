import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useClaudeStore } from '@/stores/claude'

describe('useClaudeStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('has correct default state', () => {
    const store = useClaudeStore()
    expect(store.petState).toBe('idle')
    expect(store.isWatching).toBe(false)
    expect(store.sessions).toEqual([])
    expect(store.pendingPermissions).toEqual([])
    expect(store.totalTokens).toBe(0)
  })

  it('setPetState changes state', () => {
    const store = useClaudeStore()
    store.setPetState('coding')
    expect(store.petState).toBe('coding')
  })

  it('addPermission and removePermission work', () => {
    const store = useClaudeStore()
    const req = { requestId: 'r1', sessionId: 's1', toolName: 'Bash', toolInput: '{}', timestamp: 1000 }
    store.addPermission(req)
    expect(store.pendingPermissions).toHaveLength(1)

    // 重复添加不会重复
    store.addPermission(req)
    expect(store.pendingPermissions).toHaveLength(1)

    store.removePermission('r1')
    expect(store.pendingPermissions).toHaveLength(0)
  })

  it('addTokens accumulates', () => {
    const store = useClaudeStore()
    store.addTokens(100, 50)
    store.addTokens(200, 100)
    expect(store.totalTokens).toBe(450)
  })

  it('resetTokens clears total', () => {
    const store = useClaudeStore()
    store.addTokens(100, 50)
    store.resetTokens()
    expect(store.totalTokens).toBe(0)
  })

  it('toggleSessionVisibility works', () => {
    const store = useClaudeStore()
    expect(store.isSessionVisible('s1')).toBe(true)

    store.toggleSessionVisibility('s1')
    expect(store.isSessionVisible('s1')).toBe(false)

    store.toggleSessionVisibility('s1')
    expect(store.isSessionVisible('s1')).toBe(true)
  })
})
