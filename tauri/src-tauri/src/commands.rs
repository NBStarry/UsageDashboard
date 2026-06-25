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

// 手机端录入 Claude OAuth accessToken,写入 home_dir()/.claude/.credentials.json
// (结构对齐 credentials::claude_token 读取的 claudeAiOauth.accessToken)。
// 仅移动端使用:桌面端该文件由 Claude Code CLI 维护,不应被覆盖。
#[tauri::command]
pub fn save_claude_credentials(
    _state: State<'_, AppState>,
    access_token: String,
) -> Result<(), String> {
    // 防护:仅移动端。桌面端 home_dir() 是真实主目录,写入会覆盖 Claude Code CLI 的凭证。
    if !cfg!(mobile) {
        return Err("仅移动端支持 app 内录入 Claude 凭证".to_string());
    }
    let tok = access_token.trim();
    if tok.is_empty() {
        return Err("accessToken 不能为空".to_string());
    }
    let home = crate::paths::home_dir().ok_or("无法定位主目录")?;
    let dir = home.join(".claude");
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败:{}", e))?;
    let json = serde_json::json!({ "claudeAiOauth": { "accessToken": tok } });
    fs::write(dir.join(".credentials.json"), json.to_string())
        .map_err(|e| format!("写入凭证失败:{}", e))?;
    Ok(())
}

// 手机端录入 Codex/GPT 凭证,写入 home_dir()/.codex/auth.json
// (结构对齐 credentials::codex_creds 读取的 tokens.access_token / tokens.account_id)。
// 仅移动端使用:桌面端该文件由 Codex CLI 维护。
#[tauri::command]
pub fn save_codex_credentials(
    _state: State<'_, AppState>,
    access_token: String,
    account_id: String,
) -> Result<(), String> {
    // 防护:仅移动端。桌面端 home_dir() 是真实主目录,写入会覆盖 Codex CLI 的凭证。
    if !cfg!(mobile) {
        return Err("仅移动端支持 app 内录入 Codex 凭证".to_string());
    }
    let tok = access_token.trim();
    let acc = account_id.trim();
    if tok.is_empty() || acc.is_empty() {
        return Err("access_token 和 account_id 均必填".to_string());
    }
    let home = crate::paths::home_dir().ok_or("无法定位主目录")?;
    let dir = home.join(".codex");
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败:{}", e))?;
    let json = serde_json::json!({ "tokens": { "access_token": tok, "account_id": acc } });
    fs::write(dir.join("auth.json"), json.to_string())
        .map_err(|e| format!("写入凭证失败:{}", e))?;
    Ok(())
}

// 设置/清除 HTTP 代理(空串=清除)。立即生效:写入 http 全局 + 存配置 + 重新取数。
// 主要用于 GFW 下让 Claude/Codex 经代理(如 http://127.0.0.1:7897,模拟器 http://10.0.2.2:7897)。
#[tauri::command]
pub async fn set_proxy_url(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    let trimmed = url.trim();
    let value = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
    state.config.lock().unwrap().proxy_url = value.clone();
    crate::http::set_proxy(value);
    commit_config(&app, &state)?;
    state.refresh(&app).await;
    let _ = app.emit("usage-updated", ());
    Ok(())
}

// 请求把主屏小组件固定到桌面(弹系统确认框)。仅 Android,经 JNI 调
// AppWidgetManager.requestPinAppWidget(只用 framework 类,避开 JNI 找不到 app 类的问题)。
#[cfg(target_os = "android")]
#[tauri::command]
pub fn request_pin_widget() -> Result<bool, String> {
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
        .new_string("app.usagedashboard.UsageWidgetProvider")
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

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn request_pin_widget() -> Result<bool, String> {
    Err("仅 Android 支持添加主屏小组件".to_string())
}

// 打开 New-API 网关网页登录(WebLoginActivity),登录后自动提取令牌写入凭证文件。
// 经 JNI 用 action + setPackage 启动(只用 framework Intent 类,避开 JNI 找不到 app 类)。
#[cfg(target_os = "android")]
#[tauri::command]
pub fn login_new_api(
    _state: State<'_, AppState>,
    base_url: String,
    credential_file: String,
) -> Result<(), String> {
    let clean = credential_file.trim();
    if clean.is_empty() || std::path::Path::new(clean).is_absolute() || clean.contains("..") {
        return Err("凭证文件名非法".to_string());
    }
    if base_url.trim().is_empty() {
        return Err("请先填写网关地址 baseUrl".to_string());
    }
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败:{}", e))?;
    let file_path = dir.join(clean).to_string_lossy().to_string();

    use jni::objects::{JObject, JValue};
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("vm:{e}"))?;
    let mut env = vm.attach_current_thread().map_err(|e| format!("attach:{e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let action = env
        .new_string("app.usagedashboard.LOGIN_NEWAPI")
        .map_err(|e| e.to_string())?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&JObject::from(action))],
        )
        .map_err(|e| format!("intent:{e}"))?;

    let pkg = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .and_then(|v| v.l())
        .map_err(|e| format!("pkg:{e}"))?;
    env.call_method(
        &intent,
        "setPackage",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&pkg)],
    )
    .map_err(|e| format!("setPackage:{e}"))?;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)], // FLAG_ACTIVITY_NEW_TASK
    )
    .map_err(|e| format!("addFlags:{e}"))?;

    // putExtra("baseUrl", ...)
    let k1 = env.new_string("baseUrl").map_err(|e| e.to_string())?;
    let v1 = env.new_string(base_url.trim()).map_err(|e| e.to_string())?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&JObject::from(k1)), JValue::Object(&JObject::from(v1))],
    )
    .map_err(|e| format!("extra1:{e}"))?;
    // putExtra("filePath", ...)
    let k2 = env.new_string("filePath").map_err(|e| e.to_string())?;
    let v2 = env.new_string(&file_path).map_err(|e| e.to_string())?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&JObject::from(k2)), JValue::Object(&JObject::from(v2))],
    )
    .map_err(|e| format!("extra2:{e}"))?;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|e| format!("startActivity:{e}"))?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn login_new_api(
    _state: State<'_, AppState>,
    base_url: String,
    credential_file: String,
) -> Result<(), String> {
    let _ = (base_url, credential_file);
    Err("仅移动端支持网页登录".to_string())
}

// 设置中转 relay 配置(url, secret, enabled)。写入 config 后立即刷新。
// enabled=true 时 url 和 secret 均必填;enabled=false 时仅持久化关闭状态。
#[tauri::command]
pub async fn set_relay_config(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    secret: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        let u = url.trim();
        if enabled && (u.is_empty() || secret.trim().is_empty()) {
            return Err("中转地址和密钥必填".to_string());
        }
        cfg.relay = Some(crate::models::RelayConfig {
            url: u.to_string(),
            secret: secret.trim().to_string(),
            enabled,
        });
    }
    commit_config(&app, &state)?;
    state.refresh(&app).await;
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
