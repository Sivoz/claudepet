pub mod watcher;
pub mod parser;
pub mod cli;
pub mod settings;
pub mod session;
pub mod permissions;
pub mod state_machine;
pub mod ipc;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct MessagePayload {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
    pub stop_reason: Option<String>,
    pub model: Option<String>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageInfo {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// 推送到前端的精简事件
#[derive(Debug, Clone, Serialize)]
pub struct PetEvent {
    pub state: String,
    pub session_id: Option<String>,
    pub project_name: Option<String>,
    pub detail: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub model: Option<String>,
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

            // 先检查 stop_reason：end_turn / max_tokens / stop_sequence 均为任务结束
            // 注意 stop_reason: "tool_use" 表示 assistant 决定调工具，不是结束
            match msg.stop_reason.as_deref() {
                Some("end_turn") | Some("max_tokens") | Some("stop_sequence") => {
                    return Some(PetState::Success);
                }
                _ => {}
            }

            // 检查 content 数组中是否包含 tool_use
            if let Some(serde_json::Value::Array(contents)) = &msg.content {
                let has_tool_use = contents.iter().any(|c| {
                    c.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                });
                if has_tool_use {
                    return Some(PetState::Coding);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(entry_type: &str) -> JsonlEntry {
        JsonlEntry {
            entry_type: Some(entry_type.to_string()),
            message: None,
            uuid: None,
            parent_uuid: None,
            session_id: None,
            timestamp: None,
            permission_mode: None,
        }
    }

    fn make_assistant_entry(content: serde_json::Value, stop_reason: Option<&str>) -> JsonlEntry {
        JsonlEntry {
            entry_type: Some("assistant".to_string()),
            message: Some(MessagePayload {
                role: Some("assistant".to_string()),
                content: Some(content),
                stop_reason: stop_reason.map(|s| s.to_string()),
                model: None,
                usage: None,
            }),
            uuid: None,
            parent_uuid: None,
            session_id: None,
            timestamp: None,
            permission_mode: None,
        }
    }

    #[test]
    fn user_message_maps_to_thinking() {
        let entry = make_entry("user");
        assert_eq!(resolve_state(&entry), Some(PetState::Thinking));
    }

    #[test]
    fn assistant_with_tool_use_maps_to_coding() {
        let content = serde_json::json!([
            {"type": "text", "text": "Let me check..."},
            {"type": "tool_use", "name": "Read", "input": {}}
        ]);
        let entry = make_assistant_entry(content, None);
        assert_eq!(resolve_state(&entry), Some(PetState::Coding));
    }

    #[test]
    fn assistant_end_turn_maps_to_success() {
        let content = serde_json::json!([
            {"type": "text", "text": "Done!"}
        ]);
        let entry = make_assistant_entry(content, Some("end_turn"));
        assert_eq!(resolve_state(&entry), Some(PetState::Success));
    }

    #[test]
    fn assistant_only_thinking_maps_to_thinking() {
        let content = serde_json::json!([
            {"type": "thinking", "thinking": "hmm..."}
        ]);
        let entry = make_assistant_entry(content, None);
        assert_eq!(resolve_state(&entry), Some(PetState::Thinking));
    }

    #[test]
    fn tool_result_with_error_maps_to_error() {
        let entry = JsonlEntry {
            entry_type: Some("tool_result".to_string()),
            message: Some(MessagePayload {
                role: Some("tool".to_string()),
                content: Some(serde_json::json!([
                    {"type": "text", "text": "Error", "is_error": true}
                ])),
                stop_reason: None,
                model: None,
                usage: None,
            }),
            uuid: None,
            parent_uuid: None,
            session_id: None,
            timestamp: None,
            permission_mode: None,
        };
        assert_eq!(resolve_state(&entry), Some(PetState::Error));
    }

    #[test]
    fn tool_result_without_error_maps_to_coding() {
        let entry = JsonlEntry {
            entry_type: Some("tool_result".to_string()),
            message: Some(MessagePayload {
                role: Some("tool".to_string()),
                content: Some(serde_json::json!([
                    {"type": "text", "text": "file contents..."}
                ])),
                stop_reason: None,
                model: None,
                usage: None,
            }),
            uuid: None,
            parent_uuid: None,
            session_id: None,
            timestamp: None,
            permission_mode: None,
        };
        assert_eq!(resolve_state(&entry), Some(PetState::Coding));
    }

    #[test]
    fn internal_types_return_none() {
        assert_eq!(resolve_state(&make_entry("file-history-snapshot")), None);
        assert_eq!(resolve_state(&make_entry("last-prompt")), None);
    }

    #[test]
    fn unknown_type_returns_none() {
        assert_eq!(resolve_state(&make_entry("some-unknown-type")), None);
    }

    #[test]
    fn assistant_max_tokens_maps_to_success() {
        let content = serde_json::json!([
            {"type": "text", "text": "I ran out of tokens..."}
        ]);
        let entry = make_assistant_entry(content, Some("max_tokens"));
        assert_eq!(resolve_state(&entry), Some(PetState::Success));
    }

    #[test]
    fn assistant_stop_sequence_maps_to_success() {
        let content = serde_json::json!([
            {"type": "text", "text": "done"}
        ]);
        let entry = make_assistant_entry(content, Some("stop_sequence"));
        assert_eq!(resolve_state(&entry), Some(PetState::Success));
    }

    #[test]
    fn assistant_tool_use_stop_reason_maps_to_coding() {
        // stop_reason: "tool_use" 表示 assistant 决定调工具，不是结束
        let content = serde_json::json!([
            {"type": "text", "text": "Let me read that file"},
            {"type": "tool_use", "name": "Read", "input": {"path": "/tmp/x"}}
        ]);
        let entry = make_assistant_entry(content, Some("tool_use"));
        assert_eq!(resolve_state(&entry), Some(PetState::Coding));
    }

    #[test]
    fn assistant_no_stop_reason_no_tool_maps_to_thinking() {
        let content = serde_json::json!([
            {"type": "text", "text": "Hmm let me think..."}
        ]);
        let entry = make_assistant_entry(content, None);
        assert_eq!(resolve_state(&entry), Some(PetState::Thinking));
    }

    #[test]
    fn no_type_returns_none() {
        let entry = JsonlEntry {
            entry_type: None,
            message: None,
            uuid: None,
            parent_uuid: None,
            session_id: None,
            timestamp: None,
            permission_mode: None,
        };
        assert_eq!(resolve_state(&entry), None);
    }
}
