import { describe, it, expect, beforeEach } from 'vitest'
import { loadFromStorage } from '@/utils/storage'

describe('loadFromStorage', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('returns defaults when key does not exist', () => {
    const result = loadFromStorage('missing', { a: 1, b: 'x' })
    expect(result.a).toBe(1)
    expect(result.b).toBe('x')
  })

  it('merges saved values with defaults', () => {
    localStorage.setItem('test-key', JSON.stringify({ a: 42 }))
    const result = loadFromStorage('test-key', { a: 1, b: 'x' })
    expect(result.a).toBe(42)
    expect(result.b).toBe('x')
  })

  it('returns defaults on corrupt JSON', () => {
    localStorage.setItem('bad-key', 'NOT JSON')
    const result = loadFromStorage('bad-key', { a: 1 })
    expect(result.a).toBe(1)
  })

  it('adds schemaVersion to loaded data', () => {
    const result = loadFromStorage('missing', { x: 1 })
    expect((result as any).schemaVersion).toBe(1)
  })
})
