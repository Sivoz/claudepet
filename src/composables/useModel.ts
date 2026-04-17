import { onMounted, onUnmounted, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'
import { useModelStore } from '@/stores/model'
import { useClaudeStore } from '@/stores/claude'
import { useCatStore } from '@/stores/cat'
import { PixelPet } from '@/utils/pixel-pet'

/** 像素画基础尺寸 */
const PET_BASE = 48
/** 100% 对应的像素缩放倍数 */
const BASE_SCALE = 4

/** 百分比 → 像素缩放倍数 */
function pctToScale(pct: number) {
  return (pct / 100) * BASE_SCALE
}

/** 根据缩放倍数计算窗口尺寸 */
function calcWindowSize(scale: number) {
  const canvas = Math.round(PET_BASE * scale)
  return { width: canvas + 28, height: canvas + 50 }
}

/**
 * 像素宠物生命周期管理
 */
export function useModel(containerId: string) {
  const modelStore = useModelStore()
  const claudeStore = useClaudeStore()
  const catStore = useCatStore()
  let pet: PixelPet | null = null
  const unlisteners: (() => void)[] = []

  onMounted(async () => {
    const container = document.getElementById(containerId)
    if (!container) return

    const initScale = pctToScale(catStore.sizePct)
    pet = new PixelPet(container, initScale)
    pet.skin = modelStore.currentModel
    pet.state = claudeStore.petState
    pet.mirror = catStore.mirror
    pet.opacity = catStore.opacity

    // 监听皮肤切换
    unlisteners.push(await listen<string>('change-skin', (event) => {
      modelStore.setModel(event.payload)
    }))

    // 监听大小切换（接收百分比值）
    unlisteners.push(await listen<number>('change-scale', async (event) => {
      const pct = event.payload
      catStore.setSizePct(pct)
      const scale = pctToScale(pct)
      if (pet) pet.setScale(scale)
      const { width, height } = calcWindowSize(scale)
      await invoke(INVOKE_KEY.RESIZE_WINDOW, { width, height })
    }))

    // 监听镜像切换
    unlisteners.push(await listen<boolean>('change-mirror', (event) => {
      catStore.mirror = event.payload
      if (pet) pet.mirror = event.payload
    }))

    // 监听透明度变化
    unlisteners.push(await listen<number>('change-opacity', (event) => {
      catStore.opacity = event.payload
      if (pet) pet.opacity = event.payload
    }))
  })

  watch(() => modelStore.currentModel, (name) => {
    if (pet) pet.skin = name
  })

  watch(() => claudeStore.petState, (state) => {
    if (pet) pet.state = state
  })

  onUnmounted(() => {
    pet?.destroy()
    pet = null
    unlisteners.forEach(fn => fn())
    unlisteners.length = 0
  })
}
