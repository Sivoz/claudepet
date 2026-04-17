<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { INVOKE_KEY, EVENT_KEY } from '@/constant'
import type { PetState, ClaudeSession, PermissionRequest } from '@/types/claude'

// ---- 会话数据 ----
interface SessionInfoRaw {
  session_id: string
  project_name: string
  project_path: string
  file_path: string
  state: string
  detail: string | null
  last_activity: number
  input_tokens: number
  output_tokens: number
  started_at: number
  model: string | null
}

const sessions = ref<ClaudeSession[]>([])
const pendingPermissions = ref<PermissionRequest[]>([])
const petState = ref<PetState>('idle')
const hookEnabled = ref(false)
const pretoolHookEnabled = ref(false)
const interceptActive = ref(false)
const activeTab = ref<string>('')
const now = ref(Date.now())
const appStartTime = Date.now()

function mapSession(s: SessionInfoRaw): ClaudeSession {
  return {
    id: s.session_id,
    projectName: s.project_name,
    projectPath: s.project_path,
    filePath: s.file_path,
    state: s.state as PetState,
    detail: s.detail,
    lastActivity: s.last_activity,
    inputTokens: s.input_tokens,
    outputTokens: s.output_tokens,
    startedAt: s.started_at,
    model: s.model,
  }
}

async function pollSessions() {
  try {
    const raw = await invoke<SessionInfoRaw[]>(INVOKE_KEY.GET_ACTIVE_SESSIONS)
    sessions.value = raw.map(mapSession)
  } catch (e) {
    console.error('Failed to poll sessions:', e)
  }
}

const sortedSessions = computed(() =>
  [...sessions.value].sort((a, b) => b.lastActivity - a.lastActivity),
)

const sessionCount = computed(() => sessions.value.length)
const activeSessions = computed(() =>
  sessions.value.filter((s) => s.state !== 'idle' && s.state !== 'sleeping'),
)

const totalTokens = computed(() =>
  sessions.value.reduce((sum, s) => sum + s.inputTokens + s.outputTokens, 0),
)

const activeSessionDetail = computed(() =>
  sessions.value.find((s) => s.id === activeTab.value) || null,
)

// 自动选中第一个会话，或会话消失时切到第一个
watch(sortedSessions, (list) => {
  if (list.length > 0 && !list.find((s) => s.id === activeTab.value)) {
    activeTab.value = list[0].id
  }
})

// ---- 格式化 ----
const stateColors: Record<string, string> = {
  idle: '#6c7086',
  thinking: '#f9e2af',
  coding: '#89b4fa',
  success: '#4ade80',
  error: '#f38ba8',
  waiting: '#fab387',
  sleeping: '#9399b2',
}

const stateLabels: Record<string, string> = {
  idle: '空闲',
  thinking: '思考中',
  coding: '编码中',
  success: '完成',
  error: '出错',
  waiting: '等待中',
  sleeping: '休眠',
}

function formatTokens(n: number): string {
  if (n >= 1_0000_0000) return `${(n / 1_0000_0000).toFixed(1)}Y`
  if (n >= 1_0000) return `${(n / 1_0000).toFixed(1)}W`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
  return `${n}`
}

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ${s % 60}s`
  const h = Math.floor(m / 60)
  return `${h}h ${m % 60}m`
}

function formatTime(ts: number): string {
  if (!ts) return '-'
  const d = new Date(ts)
  return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function timeAgo(ts: number): string {
  if (!ts) return '-'
  const diff = Math.floor((now.value - ts) / 1000)
  if (diff < 60) return `${diff}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  return `${Math.floor(diff / 3600)}h ago`
}

// ---- 事件监听 ----
interface PetEvent {
  state: string
  session_id: string | null
  project_name: string | null
  detail: string | null
}

interface HookStatus {
  hookEnabled: boolean
  pretoolHookEnabled: boolean
  interceptActive: boolean
}

// ---- 健康自检 ----
interface HealthItem {
  name: string
  status: 'ok' | 'warn' | 'error'
  detail: string
}
interface HealthReport {
  items: HealthItem[]
}

const healthItems = ref<HealthItem[]>([])

async function loadHealth() {
  try {
    const report = await invoke<HealthReport>(INVOKE_KEY.CHECK_HEALTH)
    healthItems.value = report.items
  } catch { /* ignore */ }
}

const healthStatusIcon: Record<string, string> = {
  ok: '\u2705',
  warn: '\u26A0\uFE0F',
  error: '\u274C',
}

// ---- 统计 ----
const stateDurations = ref<Record<string, number>>({
  idle: 0, thinking: 0, coding: 0, success: 0, error: 0, waiting: 0, sleeping: 0,
})
let lastStateTs = Date.now()
let lastState: PetState = 'idle'
let longestCoding = ref(0)
let currentCodingStreak = 0
let allowCount = ref(0)
let denyCount = ref(0)

function trackStateChange(newState: PetState) {
  const elapsed = Date.now() - lastStateTs
  stateDurations.value[lastState] = (stateDurations.value[lastState] || 0) + elapsed

  if (lastState === 'coding') {
    currentCodingStreak += elapsed
    if (currentCodingStreak > longestCoding.value) longestCoding.value = currentCodingStreak
  } else {
    currentCodingStreak = newState === 'coding' ? 0 : 0
  }
  if (newState === 'coding' && lastState === 'coding') {
    // continuing coding
  } else if (newState !== 'coding') {
    currentCodingStreak = 0
  }

  lastState = newState
  lastStateTs = Date.now()
}

const totalTrackedTime = computed(() =>
  Object.values(stateDurations.value).reduce((a, b) => a + b, 0) || 1,
)

const statePercentages = computed(() => {
  const total = totalTrackedTime.value
  return Object.entries(stateDurations.value)
    .filter(([, v]) => v > 0)
    .map(([state, ms]) => ({
      state,
      label: stateLabels[state] || state,
      color: stateColors[state] || '#666',
      pct: Math.round((ms / total) * 100),
      duration: formatDuration(ms),
    }))
    .sort((a, b) => b.pct - a.pct)
})

// ---- 日志 ----
const recentLogs = ref<string[]>([])

async function loadLogs() {
  try {
    recentLogs.value = await invoke<string[]>(INVOKE_KEY.READ_RECENT_LOGS)
  } catch { /* ignore */ }
}

async function copyDiagnostics() {
  try {
    const info = await invoke<string>(INVOKE_KEY.COLLECT_DIAGNOSTICS)
    await navigator.clipboard.writeText(info)
  } catch { /* ignore */ }
}

let unlistenClaude: (() => void) | null = null
let unlistenPermission: (() => void) | null = null
let unlistenHookStatus: (() => void) | null = null
let pollTimer: ReturnType<typeof setInterval> | null = null
let nowTimer: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await pollSessions()
  await loadLogs()
  await loadHealth()
  pollTimer = setInterval(pollSessions, 3000)
  nowTimer = setInterval(() => { now.value = Date.now() }, 10_000)

  try {
    hookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_HOOK_STATUS)
    pretoolHookEnabled.value = await invoke<boolean>(INVOKE_KEY.CHECK_PRETOOLUSE_HOOK_STATUS)
    interceptActive.value = await invoke<boolean>(INVOKE_KEY.GET_INTERCEPT_ACTIVE)
  } catch { /* ignore */ }

  unlistenClaude = await listen<PetEvent>(EVENT_KEY.CLAUDE_EVENT, (e) => {
    const newState = e.payload.state as PetState
    trackStateChange(newState)
    petState.value = newState
    pollSessions()
  })

  unlistenPermission = await listen<PermissionRequest>(EVENT_KEY.PERMISSION_REQUEST, (e) => {
    const req = e.payload
    if (!pendingPermissions.value.find((p) => p.requestId === req.requestId)) {
      pendingPermissions.value.push(req)
    }
  })

  unlistenHookStatus = await listen<HookStatus>(EVENT_KEY.HOOK_STATUS_CHANGED, (e) => {
    hookEnabled.value = e.payload.hookEnabled
    pretoolHookEnabled.value = e.payload.pretoolHookEnabled
    interceptActive.value = e.payload.interceptActive
  })
})

onUnmounted(() => {
  unlistenClaude?.()
  unlistenPermission?.()
  unlistenHookStatus?.()
  if (pollTimer) clearInterval(pollTimer)
  if (nowTimer) clearInterval(nowTimer)
})
</script>

<template>
  <div class="status-root">
    <header class="status-header">
      <h1>Claude Code 状态</h1>
    </header>

    <div class="status-content">
      <!-- 概览 -->
      <div class="section-label">概览</div>
      <div class="overview-grid">
        <div class="overview-card">
          <div class="card-value">{{ sessionCount }}</div>
          <div class="card-label">会话总数</div>
        </div>
        <div class="overview-card">
          <div class="card-value">{{ activeSessions.length }}</div>
          <div class="card-label">活跃会话</div>
        </div>
        <div class="overview-card">
          <div class="card-value">{{ formatTokens(totalTokens) }}</div>
          <div class="card-label">Token 消耗</div>
        </div>
        <div class="overview-card">
          <div class="card-value">{{ pendingPermissions.length }}</div>
          <div class="card-label">待审批</div>
        </div>
        <div class="overview-card">
          <div class="card-value">{{ formatDuration(now - appStartTime) }}</div>
          <div class="card-label">运行时间</div>
        </div>
        <div class="overview-card">
          <div class="card-value">
            <span class="state-dot" :style="{ background: stateColors[petState] }" />
            {{ stateLabels[petState] || petState }}
          </div>
          <div class="card-label">宠物状态</div>
        </div>
      </div>

      <!-- Hook 状态 -->
      <div class="divider" />
      <div class="section-label">Hook 状态</div>
      <div class="hook-row">
        <span class="hook-name">Notification Hook</span>
        <span class="hook-badge" :class="hookEnabled ? 'badge-on' : 'badge-off'">
          {{ hookEnabled ? '已启用' : '未启用' }}
        </span>
      </div>
      <div class="hook-row">
        <span class="hook-name">PreToolUse Hook</span>
        <span class="hook-badge" :class="pretoolHookEnabled ? 'badge-on' : 'badge-off'">
          {{ pretoolHookEnabled ? '已安装' : '未安装' }}
        </span>
      </div>
      <div class="hook-row">
        <span class="hook-name">拦截模式</span>
        <span class="hook-badge" :class="interceptActive ? 'badge-on' : 'badge-off'">
          {{ interceptActive ? '开启' : '关闭' }}
        </span>
      </div>

      <!-- 待审批权限 -->
      <template v-if="pendingPermissions.length > 0">
        <div class="divider" />
        <div class="section-label">待审批权限</div>
        <div v-for="perm in pendingPermissions" :key="perm.requestId" class="perm-card">
          <div class="perm-tool">{{ perm.toolName }}</div>
          <div class="perm-meta">
            <span>会话: {{ perm.sessionId.slice(0, 8) }}...</span>
            <span>{{ formatTime(perm.timestamp) }}</span>
          </div>
          <div v-if="perm.toolInput" class="perm-input">{{ perm.toolInput }}</div>
        </div>
      </template>

      <!-- 会话 Tab 栏 -->
      <div class="divider" />
      <div class="section-label">会话</div>
      <div v-if="sortedSessions.length === 0" class="empty-hint">暂无活跃会话</div>
      <template v-else>
        <div class="tab-bar">
          <button
            v-for="session in sortedSessions"
            :key="session.id"
            class="tab-btn"
            :class="{ active: activeTab === session.id }"
            @click="activeTab = session.id"
          >
            <span class="state-dot-sm" :style="{ background: stateColors[session.state] }" />
            {{ session.projectName }}
          </button>
        </div>
      </template>

      <!-- Tab 内容：单个会话详情 -->
      <div v-if="activeSessionDetail" class="tab-content">
        <!-- 头部 -->
        <div class="detail-header">
          <div class="detail-project">{{ activeSessionDetail.projectName }}</div>
          <span
            class="detail-state-badge"
            :style="{
              background: stateColors[activeSessionDetail.state] + '22',
              color: stateColors[activeSessionDetail.state],
            }"
          >
            <span class="state-dot-sm" :style="{ background: stateColors[activeSessionDetail.state] }" />
            {{ stateLabels[activeSessionDetail.state] || activeSessionDetail.state }}
          </span>
        </div>
        <div class="detail-session-id">{{ activeSessionDetail.id }}</div>

        <!-- 统计 -->
        <div class="detail-grid">
          <div class="detail-card">
            <div class="card-value">{{ formatDuration(now - activeSessionDetail.startedAt) }}</div>
            <div class="card-label">活跃时长</div>
          </div>
          <div class="detail-card">
            <div class="card-value model-value">{{ activeSessionDetail.model || '未知' }}</div>
            <div class="card-label">模型</div>
          </div>
        </div>

        <!-- 活动详情 -->
        <div class="divider" />
        <div class="section-label">活动详情</div>
        <div class="detail-row">
          <span class="detail-key">当前操作</span>
          <span class="detail-val">{{ activeSessionDetail.detail || '-' }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-key">最后活动</span>
          <span class="detail-val">{{ formatTime(activeSessionDetail.lastActivity) }} ({{ timeAgo(activeSessionDetail.lastActivity) }})</span>
        </div>

        <!-- 路径信息 -->
        <div class="divider" />
        <div class="section-label">路径</div>
        <div class="detail-row">
          <span class="detail-key">项目</span>
          <span class="detail-val mono">{{ activeSessionDetail.projectPath || '-' }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-key">日志</span>
          <span class="detail-val mono">{{ activeSessionDetail.filePath || '-' }}</span>
        </div>
      </div>

      <!-- 统计 -->
      <div class="divider" />
      <div class="section-label">今日统计</div>
      <div class="stat-bars">
        <div v-for="sp in statePercentages" :key="sp.state" class="stat-bar-row">
          <span class="stat-label">{{ sp.label }}</span>
          <div class="stat-bar-track">
            <div class="stat-bar-fill" :style="{ width: sp.pct + '%', background: sp.color }" />
          </div>
          <span class="stat-pct">{{ sp.pct }}%</span>
          <span class="stat-dur">{{ sp.duration }}</span>
        </div>
      </div>
      <div class="stat-extra">
        <span>最长连续编码: {{ formatDuration(longestCoding) }}</span>
        <span>审批: {{ allowCount }} 通过 / {{ denyCount }} 拒绝</span>
      </div>

      <!-- 健康自检 -->
      <div class="divider" />
      <div class="section-label">健康检查</div>
      <div class="health-list">
        <div v-for="item in healthItems" :key="item.name" class="health-row">
          <span class="health-icon">{{ healthStatusIcon[item.status] || '?' }}</span>
          <span class="health-name">{{ item.name }}</span>
          <span class="health-detail">{{ item.detail }}</span>
        </div>
      </div>

      <!-- 日志 -->
      <div class="divider" />
      <div class="section-label" style="display:flex;justify-content:space-between;align-items:center">
        <span>最近日志</span>
        <button class="diag-btn" @click="copyDiagnostics">复制诊断信息</button>
      </div>
      <div v-if="recentLogs.length === 0" class="empty-hint">无 warn/error 日志</div>
      <div v-else class="log-list">
        <div v-for="(line, i) in recentLogs" :key="i" class="log-line" :class="{ 'log-error': line.includes('ERROR') }">
          {{ line }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.status-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--color-surface, #1e1e2e);
  color: var(--color-text, #cdd6f4);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  overflow: hidden;
  user-select: none;
}

.status-header {
  padding: 16px 20px 0;
  flex-shrink: 0;
}
.status-header h1 {
  font-size: 16px;
  font-weight: 700;
  margin: 0;
}

.status-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px 20px;
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: rgba(255,255,255,0.35);
  margin-bottom: 10px;
}

.divider {
  height: 1px;
  background: rgba(255,255,255,0.08);
  margin: 14px 0;
}

/* 概览卡片 */
.overview-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.overview-card {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  padding: 10px 12px;
}
.card-value {
  font-size: 18px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 6px;
}
.card-label {
  font-size: 11px;
  color: rgba(255,255,255,0.35);
  margin-top: 2px;
}

/* 状态圆点 */
.state-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.state-dot-sm {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* Tab 栏 */
.tab-bar {
  display: flex;
  gap: 0;
  border-bottom: 1px solid rgba(255,255,255,0.08);
  overflow-x: auto;
  margin: 14px 0 0;
  scrollbar-width: none;
}
.tab-bar::-webkit-scrollbar { display: none; }
.tab-btn {
  padding: 8px 14px;
  font-size: 12px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: rgba(255,255,255,0.45);
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 5px;
  transition: color 0.15s, border-color 0.15s;
}
.tab-btn:hover {
  color: rgba(255,255,255,0.7);
}
.tab-btn.active {
  color: #cdd6f4;
  border-bottom-color: #89b4fa;
}

/* Tab 内容 */
.tab-content {
  padding-top: 14px;
}

/* Hook 状态行 */
.hook-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
}
.hook-row + .hook-row {
  margin-top: 2px;
}
.hook-name {
  font-size: 13px;
}
.hook-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 500;
}
.badge-on {
  background: rgba(74,222,128,0.15);
  color: #4ade80;
}
.badge-off {
  background: rgba(255,255,255,0.06);
  color: rgba(255,255,255,0.35);
}

/* 空状态 */
.empty-hint {
  font-size: 13px;
  color: rgba(255,255,255,0.25);
  text-align: center;
  padding: 20px 0;
}

/* 会话卡片（概览 tab） */
.session-card {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  padding: 10px 12px;
}
.session-card + .session-card {
  margin-top: 6px;
}
.session-header {
  display: flex;
  align-items: center;
  gap: 6px;
}
.session-project {
  font-size: 13px;
  font-weight: 600;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.session-state {
  font-size: 11px;
  color: rgba(255,255,255,0.45);
}
.session-meta {
  display: flex;
  gap: 10px;
  margin-top: 4px;
}
.meta-item {
  font-size: 11px;
  color: rgba(255,255,255,0.3);
}

/* 权限卡片 */
.perm-card {
  background: rgba(250,179,135,0.06);
  border: 1px solid rgba(250,179,135,0.15);
  border-radius: 8px;
  padding: 10px 12px;
}
.perm-card + .perm-card {
  margin-top: 6px;
}
.perm-tool {
  font-size: 13px;
  font-weight: 600;
  color: #fab387;
}
.perm-meta {
  display: flex;
  gap: 10px;
  margin-top: 4px;
  font-size: 11px;
  color: rgba(255,255,255,0.3);
}
.perm-input {
  margin-top: 4px;
  font-size: 11px;
  color: rgba(255,255,255,0.35);
  font-family: 'SF Mono', Menlo, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 会话详情 tab */
.detail-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}
.detail-project {
  font-size: 18px;
  font-weight: 700;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.detail-state-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 10px;
  flex-shrink: 0;
}
.detail-session-id {
  font-size: 11px;
  color: rgba(255,255,255,0.25);
  font-family: 'SF Mono', Menlo, monospace;
  margin-bottom: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.detail-card {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  padding: 10px 12px;
}
.model-value {
  font-size: 13px;
  word-break: break-all;
}

.detail-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
}
.detail-key {
  color: rgba(255,255,255,0.35);
  flex-shrink: 0;
  min-width: 60px;
}
.detail-val {
  color: rgba(255,255,255,0.75);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* 健康检查 */
.health-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.health-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  padding: 4px 0;
}
.health-icon {
  width: 18px;
  text-align: center;
  flex-shrink: 0;
}
.health-name {
  width: 80px;
  flex-shrink: 0;
  color: rgba(255,255,255,0.7);
}
.health-detail {
  color: rgba(255,255,255,0.4);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 统计条 */
.stat-bars {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.stat-bar-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
}
.stat-label {
  width: 40px;
  color: rgba(255,255,255,0.5);
  text-align: right;
  flex-shrink: 0;
}
.stat-bar-track {
  flex: 1;
  height: 8px;
  background: rgba(255,255,255,0.06);
  border-radius: 4px;
  overflow: hidden;
}
.stat-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.3s;
}
.stat-pct {
  width: 28px;
  text-align: right;
  color: rgba(255,255,255,0.4);
  flex-shrink: 0;
}
.stat-dur {
  width: 45px;
  text-align: right;
  color: rgba(255,255,255,0.3);
  flex-shrink: 0;
}
.stat-extra {
  display: flex;
  justify-content: space-between;
  margin-top: 8px;
  font-size: 11px;
  color: rgba(255,255,255,0.35);
}

/* 日志 */
.log-list {
  max-height: 200px;
  overflow-y: auto;
  background: rgba(0,0,0,0.2);
  border-radius: 6px;
  padding: 8px;
}
.log-line {
  font-size: 10px;
  font-family: 'SF Mono', Menlo, monospace;
  color: rgba(255,255,255,0.5);
  line-height: 1.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.log-error {
  color: #f38ba8;
}
.diag-btn {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.12);
  background: rgba(255,255,255,0.06);
  color: rgba(255,255,255,0.5);
  cursor: pointer;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 400;
}
.diag-btn:hover {
  background: rgba(255,255,255,0.1);
}

.detail-val.mono {
  font-family: 'SF Mono', Menlo, monospace;
  font-size: 11px;
}
</style>
