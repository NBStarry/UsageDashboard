// 把当前用量快照写成紧凑、已格式化的 widget.json,供原生 App Widget(Kotlin)读取渲染。
// 仅移动端调用(桌面端无小组件)。写入路径 home_dir()/widget.json 与 Kotlin 端
// context.dataDir/widget.json 一致(移动端 home_dir() = app 沙盒根目录)。
// 时间以 epoch 毫秒输出,由 Kotlin 按本地时区格式化(避开 chrono::Local 在 Android 的不确定性)。

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::paths::home_dir;
use crate::state::{ServiceSnapshot, ServiceStatus};

fn money(v: f64, currency: &str) -> String {
    format!("{}{:.2}", currency, v)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

// 每个服务的两行展示文本(对齐卡片:余额型显示余额/消耗,窗口型显示前两个窗口)。
fn lines(status: &ServiceStatus) -> (String, String) {
    match status {
        ServiceStatus::Loading => ("加载中…".to_string(), String::new()),
        ServiceStatus::Error { message } => (truncate(message, 22), String::new()),
        ServiceStatus::Ok { usage, .. } | ServiceStatus::Stale { usage, .. } => {
            if let Some(b) = &usage.balance {
                (
                    format!("余额 {}", money(b.balance, &b.currency)),
                    format!("消耗 {}", money(b.used, &b.currency)),
                )
            } else {
                let mut it = usage.windows.iter();
                let fmt = |w: &crate::models::UsageWindow| format!("{} {}%", w.label, w.pct.round() as i64);
                let l1 = it.next().map(fmt).unwrap_or_default();
                let l2 = it.next().map(fmt).unwrap_or_default();
                if l1.is_empty() {
                    ("无数据".to_string(), String::new())
                } else {
                    (l1, l2)
                }
            }
        }
    }
}

pub fn write_snapshot(snapshots: &[ServiceSnapshot], last_updated: Option<DateTime<Utc>>) {
    let updated_ms = last_updated.map(|t| t.timestamp_millis()).unwrap_or(0);
    let services: Vec<_> = snapshots
        .iter()
        .map(|s| {
            let (l1, l2) = lines(&s.status);
            json!({
                "title": s.config.title,
                "accent": s.config.accent,
                "line1": l1,
                "line2": l2,
            })
        })
        .collect();
    let doc = json!({ "updatedAtMs": updated_ms, "services": services });
    if let Some(home) = home_dir() {
        let _ = std::fs::write(home.join("widget.json"), doc.to_string());
    }
}
