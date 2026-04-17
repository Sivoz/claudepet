use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

/// 应用配置，所有字段都有默认值
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// 空闲多少秒后进入 sleeping 状态（默认 300 = 5 分钟）
    pub idle_sleep_secs: u64,
    /// 会话活跃窗口（秒），超过视为非活跃（默认 600 = 10 分钟）
    pub session_window_secs: u64,
    /// Hook 超时（秒）（默认 120）
    pub hook_timeout_secs: u64,
    /// JSONL 文件变更 debounce 毫秒数（默认 300）
    pub jsonl_debounce_ms: u64,
    /// 活跃状态（thinking/coding）超时秒数，无新事件自动回 idle（默认 30）
    pub active_state_timeout_secs: u64,
    /// 会话兜底轮询间隔（秒）（默认 30）
    pub session_poll_fallback_secs: u64,
    /// 光标靠近时检测间隔（毫秒）（默认 32）
    pub cursor_track_near_ms: u64,
    /// 光标远离时检测间隔（毫秒）（默认 150）
    pub cursor_track_far_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            idle_sleep_secs: 300,
            session_window_secs: 600,
            hook_timeout_secs: 120,
            jsonl_debounce_ms: 300,
            active_state_timeout_secs: 30,
            session_poll_fallback_secs: 30,
            cursor_track_near_ms: 32,
            cursor_track_far_ms: 150,
        }
    }
}

/// 配置文件路径
fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude-pet").join("config.toml"))
}

/// 从文件加载配置，不存在则创建默认配置文件
pub fn load() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };

    if !path.exists() {
        // 创建默认配置文件
        let default = AppConfig::default();
        if let Ok(content) = toml::to_string_pretty(&default) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let header = "# ClaudePet 配置文件\n# 修改后自动生效，无需重启\n\n";
            let _ = std::fs::write(&path, format!("{}{}", header, content));
            tracing::info!(?path, "default config.toml created");
        }
        return default;
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<AppConfig>(&content) {
            Ok(config) => {
                tracing::info!(?path, "config loaded");
                config
            }
            Err(e) => {
                tracing::error!(?path, "invalid config.toml: {}, using defaults", e);
                AppConfig::default()
            }
        },
        Err(e) => {
            tracing::error!(?path, "failed to read config.toml: {}, using defaults", e);
            AppConfig::default()
        }
    }
}

/// 共享的配置状态
pub type SharedConfig = Arc<RwLock<AppConfig>>;

/// 创建共享配置
pub fn shared() -> SharedConfig {
    Arc::new(RwLock::new(load()))
}

/// 启动配置文件热重载监听
pub fn start_watching(
    config: SharedConfig,
    app_handle: tauri::AppHandle,
) -> Option<Debouncer<notify::RecommendedWatcher>> {
    let path = config_path()?;
    let dir = path.parent()?.to_path_buf();

    let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();

    let cfg = config;
    std::thread::spawn(move || {
        while let Ok(result) = rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!("config watcher error: {:?}", e);
                    continue;
                }
            };

            let config_changed = events.iter().any(|e| {
                e.path.file_name().and_then(|n| n.to_str()) == Some("config.toml")
            });

            if !config_changed {
                continue;
            }

            let new_config = load();
            if let Ok(mut guard) = cfg.write() {
                *guard = new_config.clone();
                tracing::info!("config hot-reloaded");
            }
            let _ = app_handle.emit("config-changed", &new_config);
        }
    });

    let mut debouncer = new_debouncer(Duration::from_millis(500), tx).ok()?;
    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)
        .ok()?;

    tracing::info!("config file watcher started");
    Some(debouncer)
}
