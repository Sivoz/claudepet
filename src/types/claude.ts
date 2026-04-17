export type PetState =
  | 'idle'
  | 'thinking'
  | 'coding'
  | 'success'
  | 'error'
  | 'waiting'
  | 'sleeping'

export interface ClaudeEvent {
  type?: string
  subtype?: string
  is_error?: boolean
  permission_request?: boolean
  content?: unknown
  timestamp?: number
}

export interface ClaudeSession {
  id: string
  projectName: string
  projectPath: string
  filePath: string
  state: PetState
  detail: string | null
  lastActivity: number
  inputTokens: number
  outputTokens: number
  startedAt: number
  model: string | null
}

export interface PermissionRequest {
  requestId: string
  sessionId: string
  toolName: string
  toolInput: string | null
  timestamp: number
}
