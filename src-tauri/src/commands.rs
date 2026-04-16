use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

use notify_debouncer_mini::Debouncer;
use tauri::{Emitter, Manager};
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

use crate::claude::watcher;
use crate::claude::settings;
use crate::claude::permissions;
use crate::claude::session::SessionManager;
use crate::claude::PetState;

/// Watcher 状态（JSONL 监听 + 信号文件监听 + 权限请求监听）
pub struct WatcherState {
    pub jsonl_watcher: Mutex<Option<Debouncer<notify::RecommendedWatcher>>>,
    pub signal_watcher: Mutex<Option<Debouncer<notify::RecommendedWatcher>>>,
    pub permission_watcher: Mutex<Option<Debouncer<notify::RecommendedWatcher>>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            jsonl_watcher: Mutex::new(None),
            signal_watcher: Mutex::new(None),
            permission_watcher: Mutex::new(None),
        }
    }
}

/// 当前尺寸百分比（全局，线程安全）
pub static CURRENT_SIZE_PCT: AtomicU32 = AtomicU32::new(100);

#[tauri::command]
pub fn get_claude_status() -> Result<String, String> {
    Ok(PetState::Idle.as_str().to_string())
}

#[tauri::command]
pub fn start_watcher(
    app: tauri::AppHandle,
    state: tauri::State<WatcherState>,
    session_mgr: tauri::State<Arc<Mutex<SessionManager>>>,
) -> Result<(), String> {
    let mut guard = state.jsonl_watcher.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let debouncer =
        watcher::start_watching(app, session_mgr.inner().clone()).map_err(|e| e.to_string())?;
    *guard = Some(debouncer);
    log::info!("Claude Code watcher started");
    Ok(())
}

#[tauri::command]
pub fn stop_watcher(state: tauri::State<WatcherState>) -> Result<(), String> {
    let mut guard = state.jsonl_watcher.lock().map_err(|e| e.to_string())?;
    if guard.take().is_some() {
        log::info!("Claude Code watcher stopped");
    }
    Ok(())
}

#[tauri::command]
pub fn resize_window(window: tauri::WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    use tauri::LogicalSize;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    log::info!("Window resized to {}x{}", width, height);
    Ok(())
}

#[tauri::command]
pub fn check_hook_status() -> Result<bool, String> {
    settings::is_hook_installed().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_notification_hook() -> Result<(), String> {
    settings::install_hook().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_notification_hook() -> Result<(), String> {
    settings::uninstall_hook().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_skin(app: tauri::AppHandle, skin: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.emit("change-skin", &skin).map_err(|e| e.to_string())?;
    }
    log::info!("Skin changed to: {}", skin);
    Ok(())
}

#[tauri::command]
pub fn set_scale(app: tauri::AppHandle, pct: u32) -> Result<(), String> {
    CURRENT_SIZE_PCT.store(pct, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window("main") {
        w.emit("change-scale", pct).map_err(|e| e.to_string())?;
    }
    log::info!("Scale changed to: {}%", pct);
    Ok(())
}

#[tauri::command]
pub fn set_mirror(app: tauri::AppHandle, mirror: bool) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.emit("change-mirror", mirror).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_opacity(app: tauri::AppHandle, opacity: f64) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.emit("change-opacity", opacity).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---- 多会话管理命令 ----

#[tauri::command]
pub fn get_active_sessions(
    session_mgr: tauri::State<Arc<Mutex<SessionManager>>>,
) -> Result<Vec<crate::claude::session::SessionInfo>, String> {
    let mgr = session_mgr.lock().map_err(|e| e.to_string())?;
    Ok(mgr.get_active_sessions())
}

// ---- 权限审批命令 ----

#[tauri::command]
pub fn respond_permission(request_id: String, decision: String) -> Result<(), String> {
    permissions::respond(&request_id, &decision, Some("ClaudePet user decision"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_pretooluse_hook() -> Result<(), String> {
    settings::install_pretooluse_hook().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_pretooluse_hook() -> Result<(), String> {
    settings::uninstall_pretooluse_hook().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn check_pretooluse_hook_status() -> Result<bool, String> {
    settings::is_pretooluse_hook_installed().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_intercept_active(active: bool) -> Result<(), String> {
    permissions::set_intercept_active(active).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_intercept_active() -> Result<bool, String> {
    Ok(permissions::is_intercept_active())
}

/// 皮肤列表
pub const SKINS: &[(&str, &str)] = &[
    ("mahou", "🌸 Sakura · 魔法少女"),
    ("neko", "🐱 Mochi · 小猫咪"),
    ("kitsune", "🦊 Kitsune · 狐灵"),
    ("usagi", "🐰 Usagi · 兔兔"),
    ("devil", "😈 Imp · 小恶魔"),
    ("elf", "🧚 Sylph · 精灵"),
    ("yami", "🌑 Yami · 暗影"),
    ("nexus", "🤖 Nexus · 赛博"),
];

/// 尺寸选项 (百分比)
const SIZES: &[(u32, &str)] = &[
    (50, "50%"), (60, "60%"), (70, "70%"), (80, "80%"), (90, "90%"),
    (100, "100%"), (110, "110%"), (120, "120%"), (130, "130%"), (140, "140%"), (150, "150%"),
];

/// 构建完整菜单（托盘和右键共用）
pub fn build_full_menu(
    app: &tauri::AppHandle,
    session_mgr: &Arc<Mutex<SessionManager>>,
) -> Result<Menu<tauri::Wry>, String> {
    let current_pct = CURRENT_SIZE_PCT.load(Ordering::Relaxed);

    // 皮肤子菜单
    let skin_items: Vec<MenuItem<tauri::Wry>> = SKINS
        .iter()
        .map(|(id, label)| {
            MenuItem::with_id(app, format!("skin_{}", id), *label, true, None::<&str>)
                .expect("skin menu item")
        })
        .collect();
    let skin_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
        skin_items.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
    let skin_sub = Submenu::with_items(app, "切换皮肤", true, &skin_refs)
        .map_err(|e| e.to_string())?;

    // 大小子菜单（CheckMenuItem 标记当前选中项）
    let size_items: Vec<CheckMenuItem<tauri::Wry>> = SIZES
        .iter()
        .map(|(pct, label)| {
            CheckMenuItem::with_id(
                app,
                format!("size_{}", pct),
                *label,
                true,
                *pct == current_pct,
                None::<&str>,
            )
            .expect("size menu item")
        })
        .collect();
    let size_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
        size_items.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
    let size_sub = Submenu::with_items(app, "调整大小", true, &size_refs)
        .map_err(|e| e.to_string())?;

    // 会话显隐子菜单
    let sessions = session_mgr
        .lock()
        .map_err(|e| e.to_string())?
        .get_active_sessions();

    let session_sub = if !sessions.is_empty() {
        let session_items: Vec<MenuItem<tauri::Wry>> = sessions
            .iter()
            .map(|s| {
                MenuItem::with_id(
                    app,
                    format!("session_toggle_{}", s.session_id),
                    format!("{} · {}", s.project_name, s.state),
                    true,
                    None::<&str>,
                )
                .expect("session menu item")
            })
            .collect();
        let session_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
            session_items.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
        Some(
            Submenu::with_items(app, "会话", true, &session_refs)
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    // 状态标签大小子菜单
    let label_sizes: &[(u32, &str)] = &[
        (8, "小"), (10, "中"), (12, "大"), (14, "特大"),
    ];
    let label_size_items: Vec<MenuItem<tauri::Wry>> = label_sizes
        .iter()
        .map(|(sz, label)| {
            MenuItem::with_id(
                app,
                format!("label_size_{}", sz),
                *label,
                true,
                None::<&str>,
            )
            .expect("label size menu item")
        })
        .collect();
    let label_size_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
        label_size_items.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
    let label_size_sub = Submenu::with_items(app, "标签大小", true, &label_size_refs)
        .map_err(|e| e.to_string())?;

    // 其他项
    let sep = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let settings_item = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    if let Some(session_sub) = &session_sub {
        Menu::with_items(
            app,
            &[&skin_sub, &size_sub, &label_size_sub, session_sub, &sep, &settings_item, &quit],
        )
        .map_err(|e| e.to_string())
    } else {
        Menu::with_items(app, &[&skin_sub, &size_sub, &label_size_sub, &sep, &settings_item, &quit])
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn show_context_menu(
    window: tauri::WebviewWindow,
    session_mgr: tauri::State<Arc<Mutex<SessionManager>>>,
) -> Result<(), String> {
    let menu = build_full_menu(window.app_handle(), session_mgr.inner())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())?;
    Ok(())
}
