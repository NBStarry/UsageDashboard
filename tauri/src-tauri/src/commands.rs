// Tauri 命令层:前端 invoke 的入口。
// 约定:全部返回 Result<_, String>;改配置后 save 并 emit("config-updated")。
// 行为对齐 Sources/UsageBar/UsageStore.swift 的各 mutator。

use tauri::{AppHandle, Emitter, State};

use crate::models::AppConfig;
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

// 请求把主屏小组件固定到桌面(弹系统确认框)。仅 Android,经 JNI 调
// AppWidgetManager.requestPinAppWidget(只用 framework 类,避开 JNI 找不到 app 类的问题)。
#[cfg(target_os = "android")]
fn request_pin_widget_provider(provider_class: &str) -> Result<bool, String> {
    use jni::objects::{JObject, JValue};
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("vm:{e}"))?;
    let mut env = vm.attach_current_thread().map_err(|e| format!("attach:{e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let mgr = env
        .call_static_method(
            "android/appwidget/AppWidgetManager",
            "getInstance",
            "(Landroid/content/Context;)Landroid/appwidget/AppWidgetManager;",
            &[JValue::Object(&context)],
        )
        .and_then(|v| v.l())
        .map_err(|e| format!("getInstance:{e}"))?;

    let supported = env
        .call_method(&mgr, "isRequestPinAppWidgetSupported", "()Z", &[])
        .and_then(|v| v.z())
        .map_err(|e| format!("supported:{e}"))?;
    if !supported {
        return Err("当前桌面不支持一键添加,请从微件列表手动拖入".to_string());
    }

    let name = env
        .new_string(provider_class)
        .map_err(|e| format!("str:{e}"))?;
    let name_obj = JObject::from(name);
    let cn = env
        .new_object(
            "android/content/ComponentName",
            "(Landroid/content/Context;Ljava/lang/String;)V",
            &[JValue::Object(&context), JValue::Object(&name_obj)],
        )
        .map_err(|e| format!("componentName:{e}"))?;

    let ok = env
        .call_method(
            &mgr,
            "requestPinAppWidget",
            "(Landroid/content/ComponentName;Landroid/os/Bundle;Landroid/app/PendingIntent;)Z",
            &[
                JValue::Object(&cn),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        )
        .and_then(|v| v.z())
        .map_err(|e| format!("requestPin:{e}"))?;
    Ok(ok)
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn request_pin_widget() -> Result<bool, String> {
    request_pin_widget_provider("app.usagedashboard.UsageWidgetProvider")
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn request_pin_double_widget() -> Result<bool, String> {
    request_pin_widget_provider("app.usagedashboard.UsageDoubleWidgetProvider")
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn request_pin_widget() -> Result<bool, String> {
    Err("仅 Android 支持添加主屏小组件".to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn request_pin_double_widget() -> Result<bool, String> {
    Err("仅 Android 支持添加主屏小组件".to_string())
}

// 设置中转 relay 配置。手机端目前固定使用中转模式:
// - url 必填。
// - secret 首次必填;后续改地址时可留空,沿用已保存的 secret。
#[tauri::command]
pub async fn set_relay_config(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    secret: String,
    _enabled: bool,
) -> Result<(), String> {
    let relay;
    {
        let mut cfg = state.config.lock().unwrap();
        let u = url.trim();
        if u.is_empty() {
            return Err("中转地址必填".to_string());
        }
        let input_secret = secret.trim();
        let next_secret = if input_secret.is_empty() {
            cfg.relay
                .as_ref()
                .map(|r| r.secret.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "首次连接需要填写中转密钥".to_string())?
        } else {
            input_secret.to_string()
        };
        cfg.proxy_url = None;
        relay = crate::models::RelayConfig {
            url: u.to_string(),
            secret: next_secret,
            enabled: true,
        };
        cfg.relay = Some(relay.clone());
    }
    commit_config(&app, &state)?;

    let payload = crate::fetchers::fetch_relay(&relay)
        .await
        .map_err(|e| format!("配置已保存,但连接失败:{}", e))?;
    let lu = payload
        .ts
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    state.apply_relay(payload.services);
    *state.last_updated.lock().unwrap() = Some(lu);
    #[cfg(mobile)]
    {
        let snaps = state.snapshots();
        let lu = *state.last_updated.lock().unwrap();
        crate::widget::write_snapshot(&snaps, lu);
    }
    let _ = app.emit("usage-updated", ());
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
