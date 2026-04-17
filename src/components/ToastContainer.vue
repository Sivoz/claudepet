<script setup lang="ts">
import { useToastStore } from '@/stores/toast'

const toastStore = useToastStore()

const typeStyles: Record<string, { bg: string; border: string; color: string }> = {
  info:    { bg: 'rgba(96,165,250,0.15)',  border: 'rgba(96,165,250,0.4)',  color: '#93c5fd' },
  success: { bg: 'rgba(74,222,128,0.15)',  border: 'rgba(74,222,128,0.4)',  color: '#86efac' },
  error:   { bg: 'rgba(248,113,113,0.15)', border: 'rgba(248,113,113,0.4)', color: '#fca5a5' },
  warning: { bg: 'rgba(251,146,60,0.15)',  border: 'rgba(251,146,60,0.4)',  color: '#fdba74' },
}
</script>

<template>
  <div class="toast-container">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toastStore.toasts"
        :key="toast.id"
        class="toast-item"
        :style="{
          background: typeStyles[toast.type].bg,
          borderColor: typeStyles[toast.type].border,
          color: typeStyles[toast.type].color,
        }"
        @click="toastStore.remove(toast.id)"
      >
        {{ toast.message }}
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 12px;
  right: 12px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 6px;
  pointer-events: none;
  max-width: 300px;
}
.toast-item {
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid;
  font-size: 12px;
  font-weight: 500;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  backdrop-filter: blur(8px);
  cursor: pointer;
  pointer-events: auto;
  word-break: break-word;
}
.toast-enter-active {
  transition: all 0.3s ease;
}
.toast-leave-active {
  transition: all 0.2s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(20px);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(20px);
}
</style>
