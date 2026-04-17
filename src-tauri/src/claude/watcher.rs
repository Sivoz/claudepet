use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::Emitter;

use crate::claude::parser::IncrementalParser;
use crate::claude::session::{self, SessionManager};
use crate::claude::settings;
use crate::claude::state_machine::StateMachine;
use crate::claude::{resolve_state, PetEvent, PetState};
use crate::config::SharedConfig;

/// Watchdog 状态：追踪最后事件时间和当前状态，用于超时兜底
pub struct WatchdogState {
    pub current: PetState,
    pub last_event_at: Instant,
}

impl WatchdogState {
    pub fn new() -> Self {
        Self {
            current: PetState::Idle,
            last_event_at: Instant::now(),
        }
    }
}

/// 获取 Claude projects 目录
fn claude_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// 从 .jsonl 文件路径中提取项目名
fn project_name_from_path(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    let folder_name = parent.file_name()?.to_str()?;
    Some(session::decode_project_name(folder_name))
}

/// 启动 Claude Code JSONL 文件监听
/// 返回 debouncer 以保持监听存活，调用方须持有返回值
pub fn start_watching(
    app_handle: tauri::AppHandle,
    session_manager: Arc<Mutex<SessionManager>>,
    config: SharedConfig,
) -> anyhow::Result<Debouncer<notify::RecommendedWatcher>> {
    let projects_dir = claude_projects_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    if !projects_dir.exists() {
        std::fs::create_dir_all(&projects_dir)?;
    }

    let parser = Arc::new(Mutex::new(IncrementalParser::new()));
    let state_machine = Arc::new(Mutex::new(StateMachine::new()));
    let watchdog = Arc::new(Mutex::new(WatchdogState::new()));

    // 启动 watchdog 定时任务
    {
        let watchdog = Arc::clone(&watchdog);
        let app = app_handle.clone();
        let config = Arc::clone(&config);
        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                let timeout_secs = config
                    .read()
                    .map(|c| c.active_state_timeout_secs)
                    .unwrap_or(30);
                let timeout = Duration::from_secs(timeout_secs);

                let should_reset = {
                    let wd = watchdog.lock().unwrap();
                    matches!(wd.current, PetState::Thinking | PetState::Coding)
                        && wd.last_event_at.elapsed() > timeout
                };

                if should_reset {
                    let elapsed = watchdog.lock().unwrap().last_event_at.elapsed();
                    tracing::info!(
                        elapsed_secs = elapsed.as_secs(),
                        "active state timeout, reverting to idle"
                    );
                    watchdog.lock().unwrap().current = PetState::Idle;

                    let pet_event = PetEvent {
                        state: "idle".to_string(),
                        session_id: None,
                        project_name: None,
                        detail: Some("watchdog_timeout".to_string()),
                        input_tokens: None,
                        output_tokens: None,
                        model: None,
                    };
                    let _ = app.emit("claude-event", &pet_event);
                }
            }
        });
    }

    let app = app_handle;

    let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();

    // 后台线程处理文件变更事件
    let watchdog_for_thread = Arc::clone(&watchdog);
    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!("Watcher error: {:?}", e);
                    continue;
                }
            };
            for event in events {
                let path = &event.path;

                // 只处理 .jsonl 文件
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                // 增量解析
                let entries = {
                    let mut p = parser.lock().unwrap();
                    p.parse_new_entries(path)
                };

                let project_name = project_name_from_path(path);

                for entry in &entries {
                    if let Some(state) = resolve_state(entry) {
                        let detail = extract_detail(entry);

                        // 提取 token 用量
                        let (input_tokens, output_tokens) = entry
                            .message
                            .as_ref()
                            .and_then(|m| m.usage.as_ref())
                            .map(|u| (u.input_tokens, u.output_tokens))
                            .unwrap_or((None, None));

                        // 提取模型名
                        let model = entry
                            .message
                            .as_ref()
                            .and_then(|m| m.model.clone());

                        // 确定 session_id：优先用 entry 中的，否则从文件名提取
                        let session_id = entry
                            .session_id
                            .clone()
                            .or_else(|| session::extract_session_id_from_path(path));

                        // 更新会话管理器，并推送会话列表变更
                        if let Some(sid) = &session_id {
                            let mut mgr = session_manager.lock().unwrap();
                            mgr.update_session(
                                sid, state.as_str(), detail.clone(), path,
                                input_tokens, output_tokens, model.clone(),
                            );
                            let sessions = mgr.get_active_sessions();
                            drop(mgr); // 释放锁后再 emit
                            let _ = app.emit("session-changed", &sessions);
                        }

                        // 通过状态机校验转移合法性
                        if let Ok(mut sm) = state_machine.lock() {
                            sm.transition(state.clone());
                        }

                        // 更新 watchdog 状态
                        if let Ok(mut wd) = watchdog_for_thread.lock() {
                            wd.current = state.clone();
                            wd.last_event_at = Instant::now();
                        }

                        let pet_event = PetEvent {
                            state: state.as_str().to_string(),
                            session_id,
                            project_name: project_name.clone(),
                            detail,
                            input_tokens,
                            output_tokens,
                            model,
                        };

                        if let Err(e) = app.emit("claude-event", &pet_event) {
                            tracing::error!("Failed to emit claude-event: {}", e);
                        }
                    }
                }
            }
        }
    });

    // 创建 debounced watcher（300ms 合并）
    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)?;

    debouncer
        .watcher()
        .watch(&projects_dir, RecursiveMode::Recursive)?;

    tracing::info!("Watching Claude projects at {:?}", projects_dir);

    Ok(debouncer)
}

/// 启动信号文件监听（检测 Notification hook 写入的 waiting-signal）
pub fn start_signal_watching(
    app_handle: tauri::AppHandle,
) -> anyhow::Result<Debouncer<notify::RecommendedWatcher>> {
    let dir = settings::signal_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    std::fs::create_dir_all(&dir)?;

    let signal_file = settings::signal_file_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let app = app_handle;

    let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();

    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!("Signal watcher error: {:?}", e);
                    continue;
                }
            };

            for event in events {
                if event.path == signal_file {
                    let pet_event = PetEvent {
                        state: "waiting".to_string(),
                        session_id: None,
                        project_name: None,
                        detail: Some("permission_prompt".to_string()),
                        input_tokens: None,
                        output_tokens: None,
                        model: None,
                    };

                    if let Err(e) = app.emit("claude-event", &pet_event) {
                        tracing::error!("Failed to emit waiting event: {}", e);
                    }
                    tracing::info!("Permission prompt detected, pet state -> waiting");
                }
            }
        }
    });

    let mut debouncer = new_debouncer(Duration::from_millis(200), tx)?;

    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)?;

    tracing::info!("Watching signal file at {:?}", dir);

    Ok(debouncer)
}

/// 从 JSONL 条目中提取可读的详情文本
fn extract_detail(entry: &crate::claude::JsonlEntry) -> Option<String> {
    let msg = entry.message.as_ref()?;
    let content = msg.content.as_ref()?;

    if let serde_json::Value::Array(arr) = content {
        arr.iter().find_map(|item| {
            let item_type = item.get("type")?.as_str()?;
            match item_type {
                "text" => {
                    let text = item.get("text")?.as_str()?;
                    Some(text.chars().take(100).collect::<String>())
                }
                "tool_use" => {
                    let name = item.get("name")?.as_str()?;
                    Some(name.to_string())
                }
                _ => None,
            }
        })
    } else {
        None
    }
}
