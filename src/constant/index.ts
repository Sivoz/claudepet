/** Tauri invoke command 名称 */
export const INVOKE_KEY = {
  GET_CLAUDE_STATUS: 'get_claude_status',
  START_WATCHER: 'start_watcher',
  STOP_WATCHER: 'stop_watcher',
  SEND_CLAUDE_MESSAGE: 'send_claude_message',
  GET_CLAUDE_SESSIONS: 'get_claude_sessions',
  SET_WINDOW_OPACITY: 'set_window_opacity',
  CHECK_HOOK_STATUS: 'check_hook_status',
  INSTALL_NOTIFICATION_HOOK: 'install_notification_hook',
  UNINSTALL_NOTIFICATION_HOOK: 'uninstall_notification_hook',
  SET_SKIN: 'set_skin',
  SET_SCALE: 'set_scale',
  SET_MIRROR: 'set_mirror',
  SET_OPACITY: 'set_opacity',
  GET_ACTIVE_SESSIONS: 'get_active_sessions',
  RESPOND_PERMISSION: 'respond_permission',
  INSTALL_PRETOOLUSE_HOOK: 'install_pretooluse_hook',
  UNINSTALL_PRETOOLUSE_HOOK: 'uninstall_pretooluse_hook',
  CHECK_PRETOOLUSE_HOOK_STATUS: 'check_pretooluse_hook_status',
  SET_INTERCEPT_ACTIVE: 'set_intercept_active',
  GET_INTERCEPT_ACTIVE: 'get_intercept_active',
} as const

/** Tauri 事件名称 */
export const EVENT_KEY = {
  CLAUDE_EVENT: 'claude-event',
  CLAUDE_STATE_CHANGED: 'claude-state-changed',
  CLAUDE_SESSION_STARTED: 'claude-session-started',
  CLAUDE_SESSION_ENDED: 'claude-session-ended',
  PERMISSION_REQUEST: 'permission-request',
} as const
