import { ref } from 'vue'
import { defineStore } from 'pinia'
import { loadFromStorage, autoPersist } from '@/utils/storage'

export type OpacityMode = 'manual' | 'proximity' | 'focusAware' | 'statusDriven'

const STORAGE_KEY = 'claude-pet-cat'

interface PersistedCat {
  opacity: number
  opacityMode: OpacityMode
  sizePct: number
  mirror: boolean
}

export const useCatStore = defineStore('cat', () => {
  const saved = loadFromStorage<PersistedCat>(STORAGE_KEY, {
    opacity: 1,
    opacityMode: 'manual',
    sizePct: 100,
    mirror: false,
  })

  const opacity = ref(saved.opacity)
  const opacityMode = ref<OpacityMode>(saved.opacityMode)
  const sizePct = ref(saved.sizePct)
  const mirror = ref(saved.mirror)

  autoPersist(STORAGE_KEY, [opacity, opacityMode, sizePct, mirror], () => ({
    opacity: opacity.value,
    opacityMode: opacityMode.value,
    sizePct: sizePct.value,
    mirror: mirror.value,
  }))

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
