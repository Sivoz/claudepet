import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useCatStore } from '@/stores/cat'

describe('useCatStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('has correct default values', () => {
    const store = useCatStore()
    expect(store.opacity).toBe(1)
    expect(store.sizePct).toBe(100)
    expect(store.mirror).toBe(false)
    expect(store.opacityMode).toBe('manual')
  })

  it('setSizePct updates size', () => {
    const store = useCatStore()
    store.setSizePct(150)
    expect(store.sizePct).toBe(150)
  })

  it('can toggle mirror', () => {
    const store = useCatStore()
    store.mirror = true
    expect(store.mirror).toBe(true)
  })
})
