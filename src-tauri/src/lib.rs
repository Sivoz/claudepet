mod claude;
mod commands;

use std::sync::{Arc, Mutex};

use tauri::{
    Emitter,
    Manager,
    menu::{MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconEvent,
};

use claude::session::SessionManager;
use commands::WatcherState;

fn handle_menu_event(app: &tauri::AppHandle, event_id: &str) {
    let window = app.get_webview_window("main");

    // 皮肤切换事件: skin_mahou, skin_neko, ...
    if let Some(skin_key) = event_id.strip_prefix("skin_") {
        if let Some(w) = &window {
            let _ = w.emit("change-skin", skin_key.to_string());
            log::info!("Skin changed to: {}", skin_key);
        }
        return;
    }

    // 尺寸调整事件: size_50, size_60, ..., size_150 (百分比)
    if let Some(pct_str) = event_id.strip_prefix("size_") {
        if let Ok(pct) = pct_str.parse::<u32>() {
            commands::CURRENT_SIZE_PCT.store(pct, std::sync::atomic::Ordering::Relaxed);
            if let Some(w) = &window {
                let _ = w.emit("change-scale", pct);
                log::info!("Scale changed to: {}%", pct);
            }
        }
        return;
    }

    // 会话显隐切换: session_toggle_{session_id}
    if let Some(session_id) = event_id.strip_prefix("session_toggle_") {
        // 必须用 app.emit 广播，window.emit 前端 listen 收不到
        let _ = app.emit("toggle-session", session_id.to_string());
        log::info!("Session visibility toggled: {}", session_id);
        return;
    }

    // 状态标签大小: label_size_{size}
    if let Some(size_str) = event_id.strip_prefix("label_size_") {
        if let Ok(size) = size_str.parse::<u32>() {
            let _ = app.emit("change-label-size", size);
            log::info!("Label size changed to: {}", size);
        }
        return;
    }

    match event_id {
        "settings" => {
            // 如果设置窗口已存在，直接聚焦
            if let Some(w) = app.get_webview_window("preference") {
                let _ = w.show();
                let _ = w.set_focus();
            } else {
                // 创建设置窗口
                let _w = tauri::WebviewWindowBuilder::new(
                    app,
                    "preference",
                    tauri::WebviewUrl::App("/preference".into()),
                )
                .title("ClaudePet 设置")
                .inner_size(360.0, 420.0)
                .resizable(false)
                .build();
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// 获取鼠标所在屏幕的 frame；找不到则 fallback 到 mainScreen
#[cfg(target_os = "macos")]
fn screen_for_mouse() -> objc2_foundation::NSRect {
    use objc2::runtime::AnyClass;

    let cursor = objc2_app_kit::NSEvent::mouseLocation();

    // 遍历所有屏幕，找到包含光标的那个
    let screens: Vec<objc2_foundation::NSRect> = unsafe {
        let cls = AnyClass::get(c"NSScreen").unwrap();
        let arr: *const objc2_foundation::NSArray<objc2_app_kit::NSScreen> =
            objc2::msg_send![cls, screens];
        if arr.is_null() {
            vec![]
        } else {
            (*arr)
                .iter()
                .map(|s| {
                    let s_ref: &objc2_app_kit::NSScreen = &s;
                    let frame: objc2_foundation::NSRect = objc2::msg_send![s_ref, frame];
                    frame
                })
                .collect()
        }
    };

    for frame in &screens {
        if cursor.x >= frame.origin.x
            && cursor.x < frame.origin.x + frame.size.width
            && cursor.y >= frame.origin.y
            && cursor.y < frame.origin.y + frame.size.height
        {
            return *frame;
        }
    }

    // fallback: mainScreen
    unsafe {
        let cls = AnyClass::get(c"NSScreen").unwrap();
        let screen: *const objc2::runtime::AnyObject = objc2::msg_send![cls, mainScreen];
        if !screen.is_null() {
            objc2::msg_send![screen, frame]
        } else {
            objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(1920.0, 1080.0),
            )
        }
    }
}

/// macOS: 后台轮询光标位置，动态切换窗口穿透 + 跨屏跟随
#[cfg(target_os = "macos")]
fn start_passthrough_tracking(window: tauri::WebviewWindow) {
    // 初始状态：穿透
    let _ = window.set_ignore_cursor_events(true);

    tauri::async_runtime::spawn(async move {
        let mut cursor_over = false;
        let mut last_screen = screen_for_mouse();

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(32)).await;

            // —— 屏幕切换检测 ——
            let cur_screen = screen_for_mouse();
            if (cur_screen.origin.x - last_screen.origin.x).abs() > 1.0
                || (cur_screen.origin.y - last_screen.origin.y).abs() > 1.0
                || (cur_screen.size.width - last_screen.size.width).abs() > 1.0
                || (cur_screen.size.height - last_screen.size.height).abs() > 1.0
            {
                // 计算宠物在旧屏幕上的相对位置
                if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
                    let sf = window.scale_factor().unwrap_or(1.0);
                    let wx = pos.x as f64 / sf;
                    let wy = pos.y as f64 / sf;
                    let ww = size.width as f64 / sf;
                    let wh = size.height as f64 / sf;

                    // 旧屏幕坐标系下的相对位置（macOS 坐标：左下角原点）
                    let rel_x = (wx - last_screen.origin.x) / (last_screen.size.width - ww);
                    let rel_y = (wy - last_screen.origin.y) / (last_screen.size.height - wh);

                    // clamp 到 [0, 1]
                    let rel_x = rel_x.clamp(0.0, 1.0);
                    let rel_y = rel_y.clamp(0.0, 1.0);

                    // 映射到新屏幕
                    let new_x = cur_screen.origin.x + rel_x * (cur_screen.size.width - ww);
                    let new_y = cur_screen.origin.y + rel_y * (cur_screen.size.height - wh);

                    let _ = window.set_position(tauri::LogicalPosition::new(new_x, new_y));
                    log::info!(
                        "Pet moved to screen at ({}, {}), size {}x{}",
                        cur_screen.origin.x,
                        cur_screen.origin.y,
                        cur_screen.size.width,
                        cur_screen.size.height
                    );
                }
                last_screen = cur_screen;
            }

            // —— 光标穿透检测 ——
            let over = is_cursor_over_pet(&window);
            if over != cursor_over {
                cursor_over = over;
                let _ = window.set_ignore_cursor_events(!over);
            }
        }
    });
}

/// 判断光标是否在宠物有效区域内
#[cfg(target_os = "macos")]
fn is_cursor_over_pet(window: &tauri::WebviewWindow) -> bool {
    use objc2_app_kit::NSEvent;

    // 获取光标位置（macOS 坐标系：左下角原点，逻辑点）
    let cursor = NSEvent::mouseLocation();

    // 获取鼠标所在屏幕的高度用于坐标翻转（多屏修正）
    let screen_frame = screen_for_mouse();
    let screen_h: f64 = screen_frame.origin.y + screen_frame.size.height;

    // 获取窗口位置和尺寸（Tauri 返回物理像素，左上角原点）
    let Ok(pos) = window.outer_position() else { return false };
    let Ok(size) = window.outer_size() else { return false };
    let sf = window.scale_factor().unwrap_or(1.0);

    // 物理像素 → 逻辑点
    let wx = pos.x as f64 / sf;
    let wy_top = pos.y as f64 / sf;
    let ww = size.width as f64 / sf;
    let wh = size.height as f64 / sf;

    // 将窗口坐标转换为 macOS 坐标系（左下角原点）
    let win_top_ns = screen_h - wy_top;
    let win_bottom_ns = win_top_ns - wh;

    // 内缩边距：排除窗口四周的透明空白
    let mx = 12.0; // 左右边距
    let mt = 8.0;  // 顶部（状态标签上方的少量空白）
    let mb = 12.0; // 底部

    cursor.x >= wx + mx
        && cursor.x <= wx + ww - mx
        && cursor.y >= win_bottom_ns + mb
        && cursor.y <= win_top_ns - mt
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(WatcherState::new())
        .manage({
            let mut mgr = SessionManager::new();
            mgr.scan_existing_sessions();
            Arc::new(Mutex::new(mgr))
        })
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_claude_status,
            commands::start_watcher,
            commands::stop_watcher,
            commands::show_context_menu,
            commands::resize_window,
            commands::check_hook_status,
            commands::install_notification_hook,
            commands::uninstall_notification_hook,
            commands::set_skin,
            commands::set_scale,
            commands::set_mirror,
            commands::set_opacity,
            commands::get_active_sessions,
            commands::respond_permission,
            commands::install_pretooluse_hook,
            commands::uninstall_pretooluse_hook,
            commands::check_pretooluse_hook_status,
            commands::set_intercept_active,
            commands::get_intercept_active,
        ])
        .setup(|app| {
            // macOS: 通过 NSWindow + WKWebView API 实现完全透明
            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::{NSColor, NSWindow};

                if let Some(window) = app.get_webview_window("main") {
                    let ns_window = window.ns_window().unwrap();
                    let ns_window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
                    ns_window.setOpaque(false);
                    ns_window.setBackgroundColor(Some(&NSColor::clearColor()));

                    // 让窗口出现在所有 Space 上（四指滑动切换桌面时跟随）
                    use objc2_app_kit::NSWindowCollectionBehavior;
                    ns_window.setCollectionBehavior(
                        NSWindowCollectionBehavior::CanJoinAllSpaces
                            | NSWindowCollectionBehavior::FullScreenAuxiliary,
                    );

                    // 关键：WKWebView 默认绘制白色背景，必须显式关闭
                    unsafe {
                        use objc2::sel;
                        use objc2::runtime::Bool;

                        if let Some(content_view) = ns_window.contentView() {
                            for subview in content_view.subviews().iter() {
                                let sv: &objc2_app_kit::NSView = &subview;
                                let responds: Bool = objc2::msg_send![sv, respondsToSelector: sel!(_setDrawsBackground:)];
                                if responds.as_bool() {
                                    let _: () = objc2::msg_send![sv, _setDrawsBackground: Bool::NO];
                                    log::info!("Set WKWebView drawsBackground = NO");
                                }
                            }
                        }
                    }

                    // 定位到鼠标所在屏幕的右上角
                    {
                        let screen_frame = screen_for_mouse();

                        let win_w = 220.0_f64;
                        let _win_h = 250.0_f64;
                        let margin = 20.0_f64;
                        let x = screen_frame.size.width - win_w - margin;
                        let y = margin;

                        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
                        let _ = window.show();
                        log::info!(
                            "Window positioned at ({}, {}), screen {}x{}",
                            x,
                            y,
                            screen_frame.size.width,
                            screen_frame.size.height
                        );
                    }

                    // 启动窗口穿透跟踪
                    start_passthrough_tracking(window);
                    log::info!("Passthrough tracking started");
                }
            }

            // macOS 应用菜单栏（左上角 app 名称下）
            {
                let app_menu = {
                    let settings = MenuItem::with_id(
                        app.handle(),
                        "settings",
                        "设置…",
                        true,
                        Some("CmdOrCtrl+,"),
                    )?;
                    let sep = PredefinedMenuItem::separator(app.handle())?;
                    let hide = PredefinedMenuItem::hide(app.handle(), Some("隐藏 ClaudePet"))?;
                    let hide_others = PredefinedMenuItem::hide_others(app.handle(), Some("隐藏其他"))?;
                    let show_all = PredefinedMenuItem::show_all(app.handle(), Some("全部显示"))?;
                    let sep2 = PredefinedMenuItem::separator(app.handle())?;
                    let quit = PredefinedMenuItem::quit(app.handle(), Some("退出 ClaudePet"))?;
                    Submenu::with_items(
                        app.handle(),
                        "ClaudePet",
                        true,
                        &[&settings, &sep, &hide, &hide_others, &show_all, &sep2, &quit],
                    )?
                };

                let window_menu = {
                    let close = PredefinedMenuItem::close_window(app.handle(), Some("关闭窗口"))?;
                    let minimize = PredefinedMenuItem::minimize(app.handle(), Some("最小化"))?;
                    Submenu::with_items(
                        app.handle(),
                        "窗口",
                        true,
                        &[&close, &minimize],
                    )?
                };

                let menu = tauri::menu::Menu::with_items(app.handle(), &[&app_menu, &window_menu])?;
                app.set_menu(menu)?;
            }

            // 构建托盘菜单（与右键菜单一致）
            let session_mgr = app.state::<Arc<Mutex<SessionManager>>>().inner().clone();
            let tray_menu = commands::build_full_menu(app.handle(), &session_mgr)
                .expect("failed to build tray menu");

            if let Some(tray) = app.tray_by_id("main") {
                tray.set_menu(Some(tray_menu))?;
                tray.on_menu_event(move |app, event| {
                    handle_menu_event(app, event.id().as_ref());
                });
                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                });
            }

            // 启动时自动开始监听 Claude Code
            let app_handle = app.handle().clone();
            let session_mgr = app.state::<Arc<Mutex<SessionManager>>>().inner().clone();
            match claude::watcher::start_watching(app_handle, session_mgr) {
                Ok(debouncer) => {
                    let state = app.state::<WatcherState>();
                    let mut guard = state.jsonl_watcher.lock().unwrap();
                    *guard = Some(debouncer);
                    log::info!("Claude Code watcher auto-started");
                }
                Err(e) => {
                    log::warn!("Failed to auto-start watcher: {}", e);
                }
            }

            // 启动信号文件监听（Notification hook）
            let app_handle = app.handle().clone();
            match claude::watcher::start_signal_watching(app_handle) {
                Ok(debouncer) => {
                    let state = app.state::<WatcherState>();
                    let mut guard = state.signal_watcher.lock().unwrap();
                    *guard = Some(debouncer);
                    log::info!("Signal file watcher auto-started");
                }
                Err(e) => {
                    log::warn!("Failed to auto-start signal watcher: {}", e);
                }
            }

            // 启动权限请求监听
            let app_handle = app.handle().clone();
            match claude::permissions::start_permission_watching(app_handle) {
                Ok(debouncer) => {
                    let state = app.state::<WatcherState>();
                    let mut guard = state.permission_watcher.lock().unwrap();
                    *guard = Some(debouncer);
                    log::info!("Permission watcher auto-started");
                }
                Err(e) => {
                    log::warn!("Failed to auto-start permission watcher: {}", e);
                }
            }

            log::info!("ClaudePet started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
