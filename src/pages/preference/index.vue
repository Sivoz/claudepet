<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { INVOKE_KEY } from '@/constant'

const activeTab = ref<'appearance' | 'claude'>('appearance')

// ---- Claude Code Hook ----
const hookEnabled = ref(false)
const hookLoading = ref(false)
const pretoolHookEnabled = ref(false)
const pretoolHookLoading = ref(false)
const interceptActive = ref(false)
const interceptLoading = ref(false)

onMounted(async () => {
  try {
    hookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_HOOK_STATUS)
    pretoolHookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_PRETOOLUSE_HOOK_STATUS)
    interceptActive.value = await invoke<boolean>(INVOKE_KEY.GET_INTERCEPT_ACTIVE)
  } catch (e) {
    console.error('Failed to check hook status:', e)
  }
})

async function toggleHook() {
  hookLoading.value = true
  try {
    if (hookEnabled.value) {
      await invoke(INVOKE_KEY.UNINSTALL_NOTIFICATION_HOOK)
      hookEnabled.value = false
    } else {
      await invoke(INVOKE_KEY.INSTALL_NOTIFICATION_HOOK)
      hookEnabled.value = true
    }
  } catch (e) {
    console.error('Failed to toggle hook:', e)
  } finally {
    hookLoading.value = false
  }
}

async function togglePretoolHook() {
  pretoolHookLoading.value = true
  try {
    if (pretoolHookEnabled.value) {
      await invoke(INVOKE_KEY.UNINSTALL_PRETOOLUSE_HOOK)
      pretoolHookEnabled.value = false
      // 卸载时同时关闭拦截模式
      interceptActive.value = false
      await invoke(INVOKE_KEY.SET_INTERCEPT_ACTIVE, { active: false })
    } else {
      await invoke(INVOKE_KEY.INSTALL_PRETOOLUSE_HOOK)
      pretoolHookEnabled.value = true
    }
  } catch (e) {
    console.error('Failed to toggle pretool hook:', e)
  } finally {
    pretoolHookLoading.value = false
  }
}

async function toggleIntercept() {
  interceptLoading.value = true
  try {
    const newVal = !interceptActive.value
    await invoke(INVOKE_KEY.SET_INTERCEPT_ACTIVE, { active: newVal })
    interceptActive.value = newVal
  } catch (e) {
    console.error('Failed to toggle intercept:', e)
  } finally {
    interceptLoading.value = false
  }
}

// ---- 外观 ----
const sizePct = ref(100)
const mirror = ref(false)
const opacity = ref(1)

const sizeOptions = [50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150]

function onSizeChange(pct: number) {
  sizePct.value = pct
  invoke(INVOKE_KEY.SET_SCALE, { pct })
}

function onMirrorChange() {
  mirror.value = !mirror.value
  invoke(INVOKE_KEY.SET_MIRROR, { mirror: mirror.value })
}

function onOpacityChange(val: number) {
  opacity.value = val
  invoke(INVOKE_KEY.SET_OPACITY, { opacity: val })
}

// ---- 模型/皮肤 ----
const currentSkin = ref('mahou')

const skins = [
  { key: 'mahou', label: '🌸 魔法少女' },
  { key: 'neko', label: '🐱 小猫咪' },
  { key: 'kitsune', label: '🦊 狐灵' },
  { key: 'usagi', label: '🐰 兔兔' },
  { key: 'devil', label: '😈 小恶魔' },
  { key: 'elf', label: '🧚 精灵' },
  { key: 'yami', label: '🌑 暗影' },
  { key: 'nexus', label: '🤖 赛博' },
]

function onSkinChange(key: string) {
  currentSkin.value = key
  invoke(INVOKE_KEY.SET_SKIN, { skin: key })
}
</script>

<template>
  <div class="pref-root">
    <!-- 标题栏 -->
    <header class="pref-header">
      <h1>ClaudePet 设置</h1>
    </header>

    <!-- Tab 栏 -->
    <nav class="tab-bar">
      <button
        class="tab-item"
        :class="{ active: activeTab === 'appearance' }"
        @click="activeTab = 'appearance'"
      >外观</button>
      <button
        class="tab-item"
        :class="{ active: activeTab === 'claude' }"
        @click="activeTab = 'claude'"
      >Claude Code</button>
    </nav>

    <!-- 内容区 -->
    <div class="pref-content">

      <!-- 外观 Tab -->
      <template v-if="activeTab === 'appearance'">
        <!-- 皮肤 -->
        <div class="section-label">皮肤</div>
        <div class="skin-grid">
          <button
            v-for="skin in skins"
            :key="skin.key"
            class="skin-btn"
            :class="{ active: currentSkin === skin.key }"
            @click="onSkinChange(skin.key)"
          >{{ skin.label }}</button>
        </div>

        <div class="divider" />

        <!-- 尺寸 -->
        <div class="row">
          <span class="row-label">尺寸</span>
          <select
            :value="sizePct"
            class="sel"
            @change="onSizeChange(Number(($event.target as HTMLSelectElement).value))"
          >
            <option v-for="s in sizeOptions" :key="s" :value="s">{{ s }}%</option>
          </select>
        </div>

        <!-- 透明度 -->
        <div class="row">
          <span class="row-label">透明度 <span class="row-hint">{{ Math.round(opacity * 100) }}%</span></span>
          <input
            type="range"
            :min="0.1" :max="1" :step="0.05"
            :value="opacity"
            class="slider"
            @input="onOpacityChange(Number(($event.target as HTMLInputElement).value))"
          />
        </div>

        <!-- 镜像 -->
        <div class="row">
          <span class="row-label">镜像翻转</span>
          <button class="toggle" :class="{ on: mirror }" @click="onMirrorChange">
            <span class="toggle-dot" />
          </button>
        </div>
      </template>

      <!-- Claude Code Tab -->
      <template v-if="activeTab === 'claude'">
        <div class="section-label">通知 Hook</div>
        <div class="row">
          <span class="row-label">权限确认检测</span>
          <button class="toggle" :class="{ on: hookEnabled }" :disabled="hookLoading" @click="toggleHook">
            <span class="toggle-dot" />
          </button>
        </div>

        <div class="hook-status">
          <span class="status-dot" :class="hookEnabled ? 'dot-on' : 'dot-off'" />
          <span>{{ hookEnabled ? '已启用' : '未启用' }}</span>
        </div>

        <div class="hook-desc">
          <p>开启后，ClaudePet 会在 <code>~/.claude/settings.json</code> 中注册 Notification Hook。</p>
          <p>当 Claude Code 弹出权限确认时，宠物会切换到等待状态。</p>
        </div>

        <div class="divider" />

        <div class="section-label">远程审批</div>
        <div class="row">
          <span class="row-label">PreToolUse Hook</span>
          <button class="toggle" :class="{ on: pretoolHookEnabled }" :disabled="pretoolHookLoading" @click="togglePretoolHook">
            <span class="toggle-dot" />
          </button>
        </div>

        <div class="hook-status">
          <span class="status-dot" :class="pretoolHookEnabled ? 'dot-on' : 'dot-off'" />
          <span>{{ pretoolHookEnabled ? '已安装' : '未安装' }}</span>
        </div>

        <div v-if="pretoolHookEnabled" class="row" style="margin-top: 8px">
          <span class="row-label">拦截模式</span>
          <button class="toggle" :class="{ on: interceptActive }" :disabled="interceptLoading" @click="toggleIntercept">
            <span class="toggle-dot" />
          </button>
        </div>

        <div v-if="pretoolHookEnabled" class="hook-desc" style="margin-top: 8px">
          <p>安装 PreToolUse Hook 后，开启拦截模式可在宠物窗口中审批工具调用。</p>
          <p>关闭拦截模式时，所有工具调用自动放行（零延迟）。</p>
        </div>
      </template>

    </div>
  </div>
</template>

<style scoped>
.pref-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--color-surface, #1e1e2e);
  color: var(--color-text, #cdd6f4);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  overflow: hidden;
  user-select: none;
}

/* 标题 */
.pref-header {
  padding: 16px 20px 0;
  flex-shrink: 0;
}
.pref-header h1 {
  font-size: 16px;
  font-weight: 700;
  margin: 0;
}

/* Tab 栏 */
.tab-bar {
  display: flex;
  gap: 0;
  padding: 12px 20px 0;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(255,255,255,0.08);
}
.tab-item {
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 500;
  color: rgba(255,255,255,0.45);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all 0.15s;
}
.tab-item:hover {
  color: rgba(255,255,255,0.7);
}
.tab-item.active {
  color: #4ade80;
  border-bottom-color: #4ade80;
}

/* 内容滚动区 */
.pref-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px 20px;
}

/* 区域标题 */
.section-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: rgba(255,255,255,0.35);
  margin-bottom: 10px;
}

/* 分隔线 */
.divider {
  height: 1px;
  background: rgba(255,255,255,0.08);
  margin: 14px 0;
}

/* 通用行 */
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
}
.row + .row {
  margin-top: 4px;
}
.row-label {
  font-size: 13px;
}
.row-hint {
  font-size: 11px;
  color: rgba(255,255,255,0.35);
  margin-left: 6px;
}

/* 下拉框 */
.sel {
  background: rgba(255,255,255,0.08);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 6px;
  color: inherit;
  padding: 4px 8px;
  font-size: 12px;
  outline: none;
}
.sel:focus {
  border-color: rgba(255,255,255,0.25);
}

/* 滑块 */
.slider {
  width: 110px;
  accent-color: #4ade80;
}

/* 开关 */
.toggle {
  position: relative;
  width: 38px;
  height: 20px;
  border-radius: 10px;
  background: rgba(255,255,255,0.15);
  border: none;
  cursor: pointer;
  transition: background 0.2s;
  flex-shrink: 0;
}
.toggle.on {
  background: #4ade80;
}
.toggle-dot {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transition: transform 0.2s;
}
.toggle.on .toggle-dot {
  transform: translateX(18px);
}

/* 皮肤网格 */
.skin-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
}
.skin-btn {
  padding: 6px 4px;
  border-radius: 6px;
  font-size: 11px;
  text-align: center;
  border: 1px solid rgba(255,255,255,0.08);
  background: rgba(255,255,255,0.04);
  color: rgba(255,255,255,0.7);
  cursor: pointer;
  transition: all 0.15s;
}
.skin-btn:hover {
  background: rgba(255,255,255,0.08);
}
.skin-btn.active {
  background: rgba(74,222,128,0.12);
  border-color: rgba(74,222,128,0.35);
  color: #4ade80;
}

/* Hook 状态 */
.hook-status {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  font-size: 12px;
  color: rgba(255,255,255,0.45);
}
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.dot-on { background: #4ade80; }
.dot-off { background: rgba(255,255,255,0.25); }

.hook-desc {
  margin-top: 12px;
  font-size: 11px;
  color: rgba(255,255,255,0.3);
  line-height: 1.6;
}
.hook-desc code {
  color: rgba(255,255,255,0.5);
}
.hook-desc p + p {
  margin-top: 4px;
}
</style>
