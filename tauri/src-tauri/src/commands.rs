// Tauri 命令层:前端 invoke 的入口。
// 约定:全部返回 Result<_, String>;改配置后 save 并 emit("config-updated")。
// 行为对齐 Sources/UsageBar/UsageStore.swift 的各 mutator。

use std::fs;

use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::models::AppConfig;
use crate::paths::config_dir;
use crate::state::{AppState, ServiceSnapshot};

// 改配置后统一:重建状态 → 保存 → emit config-updated。
fn commit_config(app: &AppHandle, state: &AppState) -> Result<(), String> {
    state.rebuild_states();
    state.save_config()?;
    let _ = app.emit("config-updated", ());
    Ok(())
}

#[tauri::command]
pub fn get_snapshots(state: State<'_, AppState>) -> Vec<ServiceSnapshot> {
    state.snapshots()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub async fn refresh_now(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.refresh(&app).await;
    Ok(())
}

#[tauri::command]
pub fn set_service_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().unwrap();
        let Some(svc) = config.services.iter_mut().find(|c| c.id == id) else {
            return Err(format!("未知服务:{}", id));
        };
        svc.enabled = enabled;
    }
    commit_config(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn move_service(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    delta: i64,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().unwrap();
        let Some(idx) = config.services.iter().position(|c| c.id == id) else {
            return Err(format!("未知服务:{}", id));
        };
        let len = config.services.len() as i64;
        let new_index = (idx as i64 + delta).clamp(0, len - 1) as usize;
        if new_index == idx {
            return Ok(());
        }
        let item = config.services.remove(idx);
        config.services.insert(new_index, item);
    }
    commit_config(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn set_display_content(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    item: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().unwrap();
        let Some(svc) = config.services.iter_mut().find(|c| c.id == id) else {
            return Err(format!("未知服务:{}", id));
        };
        let d = &mut svc.display;
        match item.as_str() {
            "plan" => d.plan = enabled,
            "fiveHour" => d.five_hour = enabled,
            "weekly" => d.weekly = enabled,
            "resetCountdown" => d.reset_countdown = enabled,
            "updatedAt" => d.updated_at = enabled,
            "balance" => d.balance = enabled,
            "used" => d.used = enabled,
            "requestCount" => d.request_count = enabled,
            "models" => d.models = enabled,
            other => return Err(format!("未知展示项:{}", other)),
        }
    }
    commit_config(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn set_alerts_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.config.lock().unwrap().alerts.enabled = enabled;
    commit_config(&app, &state)?;
    state.refresh_active_alerts();
    let _ = app.emit("usage-updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_alert_threshold(
    app: AppHandle,
    state: State<'_, AppState>,
    percent: f64,
) -> Result<(), String> {
    state.config.lock().unwrap().alerts.minimum_usage_percent = percent.clamp(0.0, 100.0);
    commit_config(&app, &state)?;
    state.refresh_active_alerts();
    let _ = app.emit("usage-updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_alert_rule(
    app: AppHandle,
    state: State<'_, AppState>,
    rule: crate::models::UsageAlertRule,
) -> Result<(), String> {
    state.config.lock().unwrap().alerts.rule = rule;
    commit_config(&app, &state)?;
    state.refresh_active_alerts();
    let _ = app.emit("usage-updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_alert_cooldown_minutes(
    app: AppHandle,
    state: State<'_, AppState>,
    minutes: i64,
) -> Result<(), String> {
    // 对齐 Swift:cooldownSeconds = max(60, minutes * 60)。
    state.config.lock().unwrap().alerts.cooldown_seconds = (minutes * 60).max(60);
    commit_config(&app, &state)?;
    Ok(())
}

// 把手动录入的 New-API 凭证写入 config_dir()/<file_name>。
// 复用 credentials 的路径校验语义:非空、非绝对路径、不含 ".."。
#[tauri::command]
pub fn save_new_api_credentials(
    _state: State<'_, AppState>,
    file_name: String,
    json: String,
) -> Result<(), String> {
    let clean = file_name.trim();
    if clean.is_empty() {
        return Err("文件名不能为空".to_string());
    }
    if std::path::Path::new(clean).is_absolute() || clean.contains("..") {
        return Err("文件名非法".to_string());
    }
    // 校验是合法 JSON,避免写入垃圾。
    serde_json::from_str::<Value>(&json).map_err(|e| format!("凭证不是合法 JSON:{}", e))?;

    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败:{}", e))?;
    let path = dir.join(clean);
    fs::write(&path, json).map_err(|e| format!("写入凭证失败:{}", e))?;
    Ok(())
}

// 前端据此切换移动端布局(隐藏 quit、安全区 padding)。
// cfg!(mobile) 由 tauri-build 注入,Android/iOS 为 true,桌面为 false。
#[tauri::command]
pub fn is_mobile() -> bool {
    cfg!(mobile)
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
