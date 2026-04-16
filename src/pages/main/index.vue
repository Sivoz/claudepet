<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useClaudeStore } from '@/stores/claude'
import { useCatStore } from '@/stores/cat'
import { useClaudeEvents } from '@/composables/useClaudeEvents'
import { useClaudeState } from '@/composables/useClaudeState'
import { useModel } from '@/composables/useModel'
import { useSessionPolling } from '@/composables/useSessionPolling'
import { usePermissions } from '@/composables/usePermissions'
import type { PetState } from '@/types/claude'

const claudeStore = useClaudeStore()
const catStore = useCatStore()
const appWindow = getCurrentWebviewWindow()
const { approve, deny } = usePermissions()

useClaudeEvents()
useClaudeState()
useModel('pet-canvas')
useSessionPolling()

// 直接监听菜单事件（不经过 useTauriListen 封装，更可靠）
const unlisteners: (() => void)[] = []
onMounted(async () => {
  unlisteners.push(await listen<string>('toggle-session', (e) => {
    claudeStore.toggleSessionVisibility(e.payload)
  }))
  unlisteners.push(await listen<number>('change-label-size', (e) => {
    claudeStore.labelSize = e.payload
  }))
})
onUnmounted(() => {
  unlisteners.forEach((fn) => fn())
})

const stateConfig: Record<PetState, { label: string; color: string; dot: string }> = {
  idle:     { label: '空闲',     color: '#9ca3af', dot: '#9ca3af' },
  thinking: { label: '思考中…',  color: '#93c5fd', dot: '#60a5fa' },
  coding:   { label: '编码中…',  color: '#86efac', dot: '#4ade80' },
  success:  { label: '已完成',   color: '#6ee7b7', dot: '#34d399' },
  error:    { label: '出错了',   color: '#fca5a5', dot: '#f87171' },
  waiting:  { label: '等待审批', color: '#fdba74', dot: '#fb923c' },
  sleeping: { label: '休眠中',   color: '#c4b5fd', dot: '#a78bfa' },
}

/** 只显示未隐藏的会话 */
const visibleSessions = computed(() =>
  claudeStore.sessions.filter((s) => claudeStore.isSessionVisible(s.id)),
)

const hasPending = computed(() => claudeStore.pendingPermissions.length > 0)

/** 没有活跃可见会话时，显示全局 petState 作为回退 */
const currentState = computed(() => stateConfig[claudeStore.petState])

/** 标签大小样式 */
const labelStyle = computed(() => {
  const sz = claudeStore.labelSize
  return {
    fontSize: `${sz}px`,
    dotSize: `${Math.max(4, Math.round(sz * 0.7))}px`,
    nameMaxWidth: `${Math.round(sz * 8)}px`,
    gap: `${Math.max(3, Math.round(sz * 0.4))}px`,
  }
})

function stateOf(state: PetState) {
  return stateConfig[state] || stateConfig.idle
}

async function startDrag() {
  await appWindow.startDragging()
}

async function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  await invoke('show_context_menu')
}
</script>

<template>
  <div class="w-screen h-screen overflow-hidden bg-transparent flex flex-col items-center">
    <div class="main-content">
      <!-- 多会话状态标签（上方） -->
      <div
        class="status-stack pointer-events-none"
        :style="{ opacity: catStore.opacity }"
      >
        <!-- 有可见会话时：每个会话一行 -->
        <template v-if="visibleSessions.length > 0">
          <div
            v-for="session in visibleSessions"
            :key="session.id"
            class="status-label"
            :style="{
              color: stateOf(session.state).color,
              fontSize: labelStyle.fontSize,
              gap: labelStyle.gap,
            }"
          >
            <span
              class="status-dot"
              :style="{
                backgroundColor: stateOf(session.state).dot,
                width: labelStyle.dotSize,
                height: labelStyle.dotSize,
              }"
            />
            <span class="session-name" :style="{ maxWidth: labelStyle.nameMaxWidth }">{{ session.projectName }}</span>
            <span class="session-state-text">{{ stateOf(session.state).label }}</span>
          </div>
        </template>

        <!-- 没有可见会话时：显示全局状态 -->
        <div
          v-else
          class="status-label"
          :style="{
            color: currentState.color,
            fontSize: labelStyle.fontSize,
            gap: labelStyle.gap,
          }"
        >
          <span
            class="status-dot"
            :style="{
              backgroundColor: currentState.dot,
              width: labelStyle.dotSize,
              height: labelStyle.dotSize,
            }"
          />
          {{ currentState.label }}
        </div>
      </div>

      <!-- 宠物画布 -->
      <div
        id="pet-canvas"
        class="pet-area"
        @mousedown.left="startDrag"
        @contextmenu="onContextMenu"
      />

      <!-- 待审批快捷条（宠物下方） -->
      <div v-if="hasPending" class="approval-bar">
        <div
          v-for="perm in claudeStore.pendingPermissions.slice(0, 3)"
          :key="perm.requestId"
          class="approval-item"
        >
          <span class="approval-tool">{{ perm.toolName }}</span>
          <button class="btn-approve" @click="approve(perm.requestId)">&#10003;</button>
          <button class="btn-deny" @click="deny(perm.requestId)">&#10005;</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

/* 状态标签栈 — 128-bit 风格相框 */
.status-stack {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  margin-bottom: 4px;
  padding: 4px 8px;
  position: relative;
  background: linear-gradient(
    180deg,
    rgba(60, 50, 80, 0.65) 0%,
    rgba(30, 25, 50, 0.75) 100%
  );
  border-radius: 3px;
  /* 外层深色边 → 高光边 → 内阴影边 → 底部暗边：经典凹凸相框 */
  border: 1.5px solid rgba(120, 100, 160, 0.5);
  box-shadow:
    /* 外框暗边 */
    0 0 0 1px rgba(10, 8, 20, 0.6),
    /* 内顶高光 */
    inset 0 1px 0 rgba(180, 160, 220, 0.3),
    /* 内底暗边 */
    inset 0 -1px 0 rgba(10, 8, 20, 0.4),
    /* 内左高光 */
    inset 1px 0 0 rgba(180, 160, 220, 0.2),
    /* 内右暗边 */
    inset -1px 0 0 rgba(10, 8, 20, 0.3);
  /* 像素化边缘 */
  image-rendering: pixelated;
  transition: opacity 0.3s ease;
}

.status-label {
  display: inline-flex;
  align-items: center;
  font-weight: 600;
  letter-spacing: 0.3px;
  white-space: nowrap;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  text-shadow: 0 1px 2px rgba(0,0,0,0.7);
  transition: color 0.3s ease;
  line-height: 1.3;
}

.status-dot {
  border-radius: 50%;
  flex-shrink: 0;
  animation: pulse-dot 2s ease-in-out infinite;
}

.session-name {
  overflow: hidden;
  text-overflow: ellipsis;
  opacity: 0.7;
}

.session-state-text {
  opacity: 1;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.pet-area {
  cursor: grab;
  animation: float 2s ease-in-out infinite;
}
.pet-area:active {
  cursor: grabbing;
}
@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}

/* 待审批快捷条 */
.approval-bar {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-top: 2px;
}
.approval-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  border-radius: 6px;
  background: rgba(251,146,60,0.15);
  font-size: 10px;
  color: #fdba74;
}
.approval-tool {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}
.btn-approve, .btn-deny {
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}
.btn-approve {
  background: rgba(74,222,128,0.2);
  color: #4ade80;
}
.btn-approve:hover {
  background: rgba(74,222,128,0.4);
}
.btn-deny {
  background: rgba(248,113,113,0.2);
  color: #f87171;
}
.btn-deny:hover {
  background: rgba(248,113,113,0.4);
}
</style>
