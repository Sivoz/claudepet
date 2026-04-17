use std::sync::atomic::{AtomicU32, Ordering};
use tauri::Manager;

static PENDING_COUNT: AtomicU32 = AtomicU32::new(0);

/// 发送权限请求通知
pub fn send_permission_notification(app: &tauri::AppHandle, tool_name: &str) {
    // macOS 原生通知
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("ClaudePet — 权限请求")
        .body(format!("工具 {} 请求审批", tool_name))
        .show();

    // 更新 Dock badge
    let count = PENDING_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    set_dock_badge(count);
}

/// 权限处理完毕，减少计数
pub fn clear_one_permission() {
    let prev = PENDING_COUNT.fetch_sub(1, Ordering::Relaxed);
    let count = if prev > 0 { prev - 1 } else { 0 };
    set_dock_badge(count);
}

/// 清零
pub fn clear_all_permissions() {
    PENDING_COUNT.store(0, Ordering::Relaxed);
    set_dock_badge(0);
}

#[cfg(target_os = "macos")]
fn set_dock_badge(count: u32) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;

    let label = if count == 0 {
        String::new()
    } else {
        count.to_string()
    };

    // MainThreadMarker 在 macOS GUI 应用中总是可以获取的
    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        let tile = app.dockTile();
        let ns_str = NSString::from_str(&label);
        tile.setBadgeLabel(Some(&ns_str));
    }
}

#[cfg(not(target_os = "macos"))]
fn set_dock_badge(_count: u32) {
    // 非 macOS 平台暂不支持
}
