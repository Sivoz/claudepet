import { watch, type WatchSource } from 'vue'
import { runMigrations } from '@/stores/_migrations'

const CURRENT_SCHEMA_VERSION = 1

/**
 * 从 localStorage 加载数据，自动执行 schema 迁移
 */
export function loadFromStorage<T>(key: string, defaults: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (raw) {
      const parsed = JSON.parse(raw)
      const migrated = runMigrations(parsed, CURRENT_SCHEMA_VERSION)
      return { ...defaults, ...migrated } as T
    }
  } catch { /* ignore corrupt data */ }
  return { ...defaults, schemaVersion: CURRENT_SCHEMA_VERSION } as T
}

/**
 * 自动将响应式数据持久化到 localStorage
 * @param key localStorage key
 * @param sources 要监听的响应式数据源
 * @param serialize 序列化函数，将当前值转为可 JSON 的对象
 */
export function autoPersist<T>(
  key: string,
  sources: WatchSource | WatchSource[],
  serialize: () => T,
) {
  watch(sources, () => {
    localStorage.setItem(key, JSON.stringify(serialize()))
  }, { deep: true })
}
