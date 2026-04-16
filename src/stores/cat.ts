import { ref, watch } from 'vue'
import { defineStore } from 'pinia'

export type OpacityMode = 'manual' | 'proximity' | 'focusAware' | 'statusDriven'

const STORAGE_KEY = 'claude-pet-cat'

interface PersistedCat {
  opacity: number
  opacityMode: OpacityMode
  sizePct: number
  mirror: boolean
}

function load(): PersistedCat {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch { /* ignore */ }
  return { opacity: 1, opacityMode: 'manual', sizePct: 100, mirror: false }
}

export const useCatStore = defineStore('cat', () => {
  const saved = load()

  const opacity = ref(saved.opacity)
  const opacityMode = ref<OpacityMode>(saved.opacityMode)
  const sizePct = ref(saved.sizePct)
  const mirror = ref(saved.mirror)

  watch([opacity, opacityMode, sizePct, mirror], () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      opacity: opacity.value,
      opacityMode: opacityMode.value,
      sizePct: sizePct.value,
      mirror: mirror.value,
    }))
  })

  function setSizePct(pct: number) {
    sizePct.value = pct
  }

  return {
    opacity,
    opacityMode,
    sizePct,
    mirror,
    setSizePct,
  }
})
