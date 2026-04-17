import { ref } from 'vue'
import { defineStore } from 'pinia'

export type ToastType = 'info' | 'success' | 'error' | 'warning'

export interface Toast {
  id: number
  message: string
  type: ToastType
  duration: number
}

let nextId = 0

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function show(message: string, type: ToastType = 'info', duration = 3000) {
    const id = nextId++
    toasts.value.push({ id, message, type, duration })
    if (duration > 0) {
      setTimeout(() => remove(id), duration)
    }
  }

  function remove(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  function error(message: string, duration = 5000) {
    show(message, 'error', duration)
  }

  function success(message: string, duration = 3000) {
    show(message, 'success', duration)
  }

  function warning(message: string, duration = 4000) {
    show(message, 'warning', duration)
  }

  return { toasts, show, remove, error, success, warning }
})
