type MigrateFn = (v: Record<string, unknown>) => Record<string, unknown>

const migrations: Record<number, MigrateFn> = {
  1: (v) => ({ ...v, schemaVersion: 1 }),
  // 未来新增迁移示例:
  // 2: (v) => ({ ...v, newField: 'default', schemaVersion: 2 }),
}

/**
 * 运行迁移链，将数据从当前 schemaVersion 升级到 latest
 */
export function runMigrations(raw: unknown, latest: number): Record<string, unknown> {
  let v: Record<string, unknown> = (raw && typeof raw === 'object') ? { ...(raw as Record<string, unknown>) } : { schemaVersion: 0 }

  while ((v.schemaVersion as number ?? 0) < latest) {
    const next = (v.schemaVersion as number ?? 0) + 1
    const fn = migrations[next]
    if (!fn) break
    v = fn(v)
  }
  return v
}
