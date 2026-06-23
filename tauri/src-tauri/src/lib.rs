mod alerts;
mod cache;
mod commands;
mod config_store;
mod credentials;
mod fetchers;
mod http;
mod models;
mod paths;
mod state;
mod tray;

use tauri::{Listener, Manager, WindowEvent};

use state::AppState;

// 后台刷新定时器睡眠下限(秒),对齐 Swift 的 60s 地板。
const MIN_REFRESH_SECONDS: u64 = 60;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config_store::load();
    let app_state = AppState::new(config);

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshots,
            commands::get_config,
            commands::refresh_now,
            commands::set_service_enabled,
            commands::move_service,
            commands::set_display_content,
            commands::set_alerts_enabled,
            commands::set_alert_threshold,
            commands::set_alert_rule,
            commands::set_alert_cooldown_minutes,
            commands::save_new_api_credentials,
            commands::quit,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // 1) 托盘。
            tray::build_tray(&handle)?;

            // 2) popover 窗口失焦自动收起(对齐 Swift popover .transient 行为)。
            if let Some(window) = app.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = win.hide();
                    }
                });
            }

            // 3) usage-updated 事件 → 同步托盘告警图标。
            //    选这条单一通路:timer / refresh_now 命令 / 告警配置变更都会 emit usage-updated,
            //    在此统一驱动托盘,避免在 state.refresh 内反向依赖 tray。
            let alert_handle = handle.clone();
            app.listen("usage-updated", move |_event| {
                let count = alert_handle.state::<AppState>().active_alert_count();
                tray::set_alert(&alert_handle, count);
            });

            // 4) 后台刷新定时器:每轮重读 config 的 refresh_seconds(取 60s 地板),
            //    sleep 后 refresh。不跨 .await 持有 Mutex guard(读完即放锁)。
            let timer_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let secs = {
                        let state = timer_handle.state::<AppState>();
                        let cfg = state.config.lock().unwrap();
                        (cfg.refresh_seconds.max(0) as u64).max(MIN_REFRESH_SECONDS)
                    };
                    tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                    let state = timer_handle.state::<AppState>();
                    state.refresh(&timer_handle).await;
                }
            });

            // 5) 启动首刷。
            let first_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = first_handle.state::<AppState>();
                state.refresh(&first_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
