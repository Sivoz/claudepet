pub mod watcher;
pub mod parser;
pub mod cli;
pub mod settings;
pub mod session;
pub mod permissions;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PetState {
    Idle,
    Thinking,
    Coding,
    Success,
    Error,
    Waiting,
    Sleeping,
}

impl PetState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PetState::Idle => "idle",
            PetState::Thinking => "thinking",
            PetState::Coding => "coding",
            PetState::Success => "success",
            PetState::Error => "error",
            PetState::Waiting => "waiting",
            PetState::Sleeping => "sleeping",
        }
    }
}

/// JSONL 中每一行的顶层结构（所有字段可选以容错）
#[derive(Debug, Clone, Deserialize)]
pub struct JsonlEntry {
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub message: Option<MessagePayload>,
    pub uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePayload {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
    pub stop_reason: Option<String>,
    pub model: Option<String>,
}

/// 推送到前端的精简事件
#[derive(Debug, Clone, Serialize)]
pub struct PetEvent {
    pub state: String,
    pub session_id: Option<String>,
    pub project_name: Option<String>,
    pub detail: Option<String>,
}

/// 根据 JSONL 条目判断宠物状态
pub fn resolve_state(entry: &JsonlEntry) -> Option<PetState> {
    let entry_type = entry.entry_type.as_deref()?;

    match entry_type {
        // 忽略内部类型
        "file-history-snapshot" | "last-prompt" => None,

        // 用户消息 → thinking
        "user" => Some(PetState::Thinking),

        // AI 回复
        "assistant" => {
            let msg = entry.message.as_ref()?;
            // 检查 content 数组中是否包含 tool_use
            if let Some(serde_json::Value::Array(contents)) = &msg.content {
                let has_tool_use = contents.iter().any(|c| {
                    c.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                });
                if has_tool_use {
                    return Some(PetState::Coding);
                }
            }

            // stop_reason = "end_turn" → success
            if msg.stop_reason.as_deref() == Some("end_turn") {
                return Some(PetState::Success);
            }

            // 只有 thinking 块 → thinking
            if let Some(serde_json::Value::Array(contents)) = &msg.content {
                let only_thinking = contents.iter().all(|c| {
                    c.get("type").and_then(|t| t.as_str()) == Some("thinking")
                });
                if only_thinking {
                    return Some(PetState::Thinking);
                }
            }

            Some(PetState::Thinking)
        }

        // 工具结果
        "tool_result" => {
            if let Some(msg) = &entry.message {
                if let Some(serde_json::Value::Array(contents)) = &msg.content {
                    let has_error = contents.iter().any(|c| {
                        c.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false)
                    });
                    if has_error {
                        return Some(PetState::Error);
                    }
                }
            }
            Some(PetState::Coding)
        }

        _ => None,
    }
}
