import { describe, it, expect } from 'vitest'
import { runMigrations } from '@/stores/_migrations'

describe('runMigrations', () => {
  it('migrates null to version 1', () => {
    const result = runMigrations(null, 1)
    expect(result.schemaVersion).toBe(1)
  })

  it('migrates old data without schemaVersion', () => {
    const result = runMigrations({ opacity: 0.5 }, 1)
    expect(result.schemaVersion).toBe(1)
    expect(result.opacity).toBe(0.5)
  })

  it('does not re-migrate already current data', () => {
    const result = runMigrations({ opacity: 0.8, schemaVersion: 1 }, 1)
    expect(result.schemaVersion).toBe(1)
    expect(result.opacity).toBe(0.8)
  })

  it('handles corrupt input gracefully', () => {
    const result = runMigrations('not an object', 1)
    expect(result.schemaVersion).toBe(1)
  })
})
