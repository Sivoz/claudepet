import { ref, watch } from 'vue'
import { defineStore } from 'pinia'

const STORAGE_KEY = 'claude-pet-general'

interface PersistedGeneral {
  autoStart: boolean
  language: string
}

function load(): PersistedGeneral {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch { /* ignore */ }
  return { autoStart: false, language: 'zh-CN' }
}

export const useGeneralStore = defineStore('general', () => {
  const saved = load()

  const autoStart = ref(saved.autoStart)
  const language = ref(saved.language)

  watch([autoStart, language], () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      autoStart: autoStart.value,
      language: language.value,
    }))
  })

  return {
    autoStart,
    language,
  }
})
