<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { INVOKE_KEY, EVENT_KEY } from '@/constant'
import { useToastStore } from '@/stores/toast'

const toastStore = useToastStore()

interface HookStatus {
  hookEnabled: boolean
  pretoolHookEnabled: boolean
  interceptActive: boolean
}

const activeTab = ref<'appearance' | 'claude' | 'advanced'>('appearance')

// ---- Claude Code Hook ----
const hookEnabled = ref(false)
const hookLoading = ref(false)
const pretoolHookEnabled = ref(false)
const pretoolHookLoading = ref(false)
const interceptActive = ref(false)
const interceptLoading = ref(false)

function broadcastHookStatus() {
  emit(EVENT_KEY.HOOK_STATUS_CHANGED, {
    hookEnabled: hookEnabled.value,
    pretoolHookEnabled: pretoolHookEnabled.value,
    interceptActive: interceptActive.value,
  } as HookStatus)
}

let unlistenHookStatus: (() => void) | null = null

onMounted(async () => {
  try {
    hookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_HOOK_STATUS)
    pretoolHookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_PRETOOLUSE_HOOK_STATUS)
    interceptActive.value = await invoke<boolean>(INVOKE_KEY.GET_INTERCEPT_ACTIVE)
  } catch (e) {
    toastStore.error(`获取 Hook 状态失败: ${e}`)
  }
  await loadConfig()

  // 监听其他窗口的状态变更
  unlistenHookStatus = await listen<HookStatus>(EVENT_KEY.HOOK_STATUS_CHANGED, (e) => {
    hookEnabled.value = e.payload.hookEnabled
    pretoolHookEnabled.value = e.payload.pretoolHookEnabled
    interceptActive.value = e.payload.interceptActive
  })
})

onUnmounted(() => {
  unlistenHookStatus?.()
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
    broadcastHookStatus()
    toastStore.success(hookEnabled.value ? '通知 Hook 已启用' : '通知 Hook 已关闭')
  } catch (e) {
    toastStore.error(`切换 Hook 失败: ${e}`)
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
    broadcastHookStatus()
    toastStore.success(pretoolHookEnabled.value ? 'PreToolUse Hook 已安装' : 'PreToolUse Hook 已卸载')
  } catch (e) {
    toastStore.error(`切换 PreToolUse Hook 失败: ${e}`)
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
    broadcastHookStatus()
    toastStore.success(interceptActive.value ? '拦截模式已开启' : '拦截模式已关闭')
  } catch (e) {
    toastStore.error(`切换拦截模式失败: ${e}`)
  } finally {
    interceptLoading.value = false
  }
}

async function openLogDir() {
  try {
    await invoke(INVOKE_KEY.OPEN_LOG_DIR)
  } catch (e) {
    toastStore.error(`打开日志目录失败: ${e}`)
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

// ---- 配置中心 ----
interface AppConfig {
  idle_sleep_secs: number
  session_window_secs: number
  hook_timeout_secs: number
  jsonl_debounce_ms: number
  session_poll_fallback_secs: number
  cursor_track_near_ms: number
  cursor_track_far_ms: number
}

const appConfig = ref<AppConfig>({
  idle_sleep_secs: 300,
  session_window_secs: 600,
  hook_timeout_secs: 120,
  jsonl_debounce_ms: 300,
  session_poll_fallback_secs: 30,
  cursor_track_near_ms: 32,
  cursor_track_far_ms: 150,
})

async function loadConfig() {
  try {
    appConfig.value = await invoke<AppConfig>(INVOKE_KEY.GET_CONFIG)
  } catch { /* use defaults */ }
}

async function saveConfig() {
  try {
    await invoke(INVOKE_KEY.UPDATE_CONFIG, { newConfig: appConfig.value })
    toastStore.success('配置已保存')
  } catch (e) {
    toastStore.error(`保存配置失败: ${e}`)
  }
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
      <button
        class="tab-item"
        :class="{ active: activeTab === 'advanced' }"
        @click="activeTab = 'advanced'"
      >高级</button>
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

        <div class="divider" />

        <div class="section-label">调试</div>
        <div class="row">
          <span class="row-label">日志文件</span>
          <button class="action-btn" @click="openLogDir">打开日志目录</button>
        </div>
        <div class="hook-desc">
          <p>日志保存在 <code>~/.claude-pet/logs/</code>，按日滚动。</p>
        </div>
      </template>

      <!-- 高级 Tab -->
      <template v-if="activeTab === 'advanced'">
        <div class="section-label">时间参数</div>
        <div class="row">
          <span class="row-label">空闲睡眠（秒）</span>
          <input type="number" class="num-input" v-model.number="appConfig.idle_sleep_secs" min="30" max="3600" />
        </div>
        <div class="row">
          <span class="row-label">Hook 超时（秒）</span>
          <input type="number" class="num-input" v-model.number="appConfig.hook_timeout_secs" min="10" max="600" />
        </div>
        <div class="row">
          <span class="row-label">会话窗口（秒）</span>
          <input type="number" class="num-input" v-model.number="appConfig.session_window_secs" min="60" max="3600" />
        </div>

        <div class="divider" />
        <div class="hook-desc">
          <p>更多参数可直接编辑 <code>~/.claude-pet/config.toml</code></p>
        </div>
        <div class="row" style="margin-top: 12px">
          <span></span>
          <button class="action-btn save-btn" @click="saveConfig">保存配置</button>
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
/* 操作按钮 */
.action-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 12px;
  border: 1px solid rgba(255,255,255,0.15);
  background: rgba(255,255,255,0.06);
  color: rgba(255,255,255,0.7);
  cursor: pointer;
  transition: all 0.15s;
}
/* 数字输入 */
.num-input {
  width: 70px;
  background: rgba(255,255,255,0.08);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 6px;
  color: inherit;
  padding: 4px 8px;
  font-size: 12px;
  outline: none;
  text-align: right;
}
.num-input:focus {
  border-color: rgba(255,255,255,0.25);
}
.save-btn {
  background: rgba(74,222,128,0.15);
  border-color: rgba(74,222,128,0.3);
  color: #4ade80;
}

.action-btn:hover {
  background: rgba(255,255,255,0.12);
  border-color: rgba(255,255,255,0.25);
}

.hook-desc code {
  color: rgba(255,255,255,0.5);
}
.hook-desc p + p {
  margin-top: 4px;
}
</style>
