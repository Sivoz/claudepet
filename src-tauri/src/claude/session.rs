use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 单个会话的信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub project_name: String,
    pub project_path: String,
    pub file_path: String,
    pub state: String,
    pub detail: Option<String>,
    pub last_activity: u64,
}

/// 会话管理器：追踪所有活跃的 Claude Code 会话
pub struct SessionManager {
    sessions: HashMap<String, SessionInfo>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 扫描 ~/.claude/projects/ 下所有 .jsonl 文件，初始化已有会话
    pub fn scan_existing_sessions(&mut self) {
        let projects_dir = match dirs::home_dir() {
            Some(h) => h.join(".claude").join("projects"),
            None => return,
        };

        if !projects_dir.exists() {
            return;
        }

        let Ok(project_entries) = std::fs::read_dir(&projects_dir) else {
            return;
        };

        for project_entry in project_entries.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let folder_name = project_entry.file_name().to_string_lossy().to_string();
            let project_name = decode_project_name(&folder_name);

            let Ok(files) = std::fs::read_dir(&project_path) else {
                continue;
            };

            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                let Some(session_id) = extract_session_id_from_path(&file_path) else {
                    continue;
                };

                // 用文件修改时间作为 last_activity
                let last_activity = file_entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                self.sessions.insert(
                    session_id.clone(),
                    SessionInfo {
                        session_id,
                        project_name: project_name.clone(),
                        project_path: folder_name.clone(),
                        file_path: file_path.to_string_lossy().to_string(),
                        state: "idle".to_string(),
                        detail: None,
                        last_activity,
                    },
                );
            }
        }
    }

    /// 更新或插入一个会话
    pub fn update_session(
        &mut self,
        session_id: &str,
        state: &str,
        detail: Option<String>,
        file_path: &Path,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if let Some(session) = self.sessions.get_mut(session_id) {
            session.state = state.to_string();
            session.detail = detail;
            session.last_activity = now;
        } else {
            // 新会话：从文件路径推导项目信息
            let (project_name, project_path) = extract_project_info(file_path);

            self.sessions.insert(
                session_id.to_string(),
                SessionInfo {
                    session_id: session_id.to_string(),
                    project_name,
                    project_path,
                    file_path: file_path.to_string_lossy().to_string(),
                    state: state.to_string(),
                    detail,
                    last_activity: now,
                },
            );
        }
    }

    /// 返回最近 10 分钟内有活动的会话，按最近活跃排序
    pub fn get_active_sessions(&self) -> Vec<SessionInfo> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let ten_minutes_ms = 10 * 60 * 1000;

        let mut active: Vec<SessionInfo> = self
            .sessions
            .values()
            .filter(|s| now.saturating_sub(s.last_activity) < ten_minutes_ms)
            .cloned()
            .collect();

        active.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        active
    }
}

/// 从编码的文件夹名中提取项目名（取最后一段）
/// 例：`-Users-sivo-PycharmProjects-ClaudePet` → `ClaudePet`
pub fn decode_project_name(folder: &str) -> String {
    // 文件夹名格式：前导 `-` 后跟路径段用 `-` 分隔
    // 实际是路径中的 `/` 被替换为 `-`
    // 取最后一个 `-` 之后的部分
    folder
        .rsplit('-')
        .next()
        .unwrap_or(folder)
        .to_string()
}

/// 从 .jsonl 文件名中提取 session_id（UUID 格式的文件名 stem）
pub fn extract_session_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    // 简单的 UUID 格式检查：8-4-4-4-12
    if stem.len() == 36 && stem.chars().filter(|c| *c == '-').count() == 4 {
        Some(stem.to_string())
    } else {
        None
    }
}

/// 从 .jsonl 文件路径中提取项目信息
fn extract_project_info(file_path: &Path) -> (String, String) {
    if let Some(parent) = file_path.parent() {
        let folder_name = parent
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let project_name = decode_project_name(&folder_name);
        (project_name, folder_name)
    } else {
        ("unknown".to_string(), String::new())
    }
}
