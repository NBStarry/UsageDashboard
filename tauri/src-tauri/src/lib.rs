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

use state::AppState;

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
