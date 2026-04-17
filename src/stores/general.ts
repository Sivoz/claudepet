import { ref } from 'vue'
import { defineStore } from 'pinia'
import { loadFromStorage, autoPersist } from '@/utils/storage'

const STORAGE_KEY = 'claude-pet-general'

interface PersistedGeneral {
  autoStart: boolean
  language: string
}

export const useGeneralStore = defineStore('general', () => {
  const saved = loadFromStorage<PersistedGeneral>(STORAGE_KEY, {
    autoStart: false,
    language: 'zh-CN',
  })

  const autoStart = ref(saved.autoStart)
  const language = ref(saved.language)

  autoPersist(STORAGE_KEY, [autoStart, language], () => ({
    autoStart: autoStart.value,
    language: language.value,
  }))

  return {
    autoStart,
    language,
  }
})
