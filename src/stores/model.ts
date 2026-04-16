import { ref, watch } from 'vue'
import { defineStore } from 'pinia'

const STORAGE_KEY = 'claude-pet-model'

export const useModelStore = defineStore('model', () => {
  const saved = localStorage.getItem(STORAGE_KEY)
  const currentModel = ref(saved || 'mahou')

  watch(currentModel, (v) => {
    localStorage.setItem(STORAGE_KEY, v)
  })

  function setModel(name: string) {
    currentModel.value = name
  }

  return {
    currentModel,
    setModel,
  }
})