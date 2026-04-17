use std::path::PathBuf;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::claude::settings;

/// 前端显示的权限请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub request_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: Option<String>,
    pub timestamp: u64,
}

/// 权限响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub decision: String,
    pub reason: Option<String>,
}

/// 监听 ~/.claude-pet/requests/ 目录，检测新的权限请求
pub fn start_permission_watching(
    app_handle: tauri::AppHandle,
) -> anyhow::Result<Debouncer<notify::RecommendedWatcher>> {
    let requests_dir = settings::requests_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    std::fs::create_dir_all(&requests_dir)?;

    let app = app_handle;
    let watch_dir = requests_dir.clone();

    let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();

    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!("Permission watcher error: {:?}", e);
                    continue;
                }
            };

            for event in events {
                let path = &event.path;

                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }

                if !path.exists() {
                    continue;
                }

                // 读取并解析请求文件
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("Failed to read permission request {:?}: {}", path, e);
                        continue;
                    }
                };

                let request: PermissionRequest = match serde_json::from_str(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("Failed to parse permission request {:?}: {}", path, e);
                        continue;
                    }
                };

                tracing::info!(
                    "Permission request received: {} for tool {}",
                    request.request_id,
                    request.tool_name
                );

                if let Err(e) = app.emit("permission-request", &request) {
                    tracing::error!("Failed to emit permission-request: {}", e);
                }
            }
        }
    });

    let mut debouncer = new_debouncer(Duration::from_millis(100), tx)?;

    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::NonRecursive)?;

    tracing::info!("Watching permission requests at {:?}", requests_dir);

    Ok(debouncer)
}

/// 写入权限响应文件
pub fn respond(request_id: &str, decision: &str, reason: Option<&str>) -> anyhow::Result<()> {
    let responses_dir = settings::responses_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    std::fs::create_dir_all(&responses_dir)?;

    let response = PermissionResponse {
        decision: decision.to_string(),
        reason: reason.map(|s| s.to_string()),
    };

    let response_path = responses_dir.join(format!("{}.json", request_id));
    let tmp_path = responses_dir.join(format!("{}.tmp", request_id));
    let content = serde_json::to_string(&response)?;
    std::fs::write(&tmp_path, &content)?;
    std::fs::rename(&tmp_path, &response_path)?;

    tracing::info!("Permission response written: {} = {}", request_id, decision);
    Ok(())
}

/// 设置/取消拦截模式标志文件
pub fn set_intercept_active(active: bool) -> anyhow::Result<()> {
    let flag_path = intercept_flag_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    if active {
        if let Some(parent) = flag_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&flag_path, "1")?;
        tracing::info!("Intercept mode activated");
    } else {
        let _ = std::fs::remove_file(&flag_path);
        tracing::info!("Intercept mode deactivated");
    }

    Ok(())
}

/// 检查拦截模式是否激活
pub fn is_intercept_active() -> bool {
    intercept_flag_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

fn intercept_flag_path() -> Option<PathBuf> {
    settings::signal_dir().map(|d| d.join("intercept-active"))
}

/// 清理超过 2 分钟的过期请求/响应文件
pub fn cleanup_stale_files() {
    let now = std::time::SystemTime::now();
    let max_age = Duration::from_secs(120);

    for dir_fn in [settings::requests_dir, settings::responses_dir] {
        if let Some(dir) = dir_fn() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if now.duration_since(modified).unwrap_or_default() > max_age {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }
}
