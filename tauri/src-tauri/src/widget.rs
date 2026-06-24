// 把当前用量快照写成 widget.json,供原生 App Widget(Kotlin)读取渲染。
// 仅移动端调用。写入 home_dir()/widget.json(= Kotlin 端 context.dataDir/widget.json)。
// 输出结构化数据(窗口型给 windows 的 label/pct,余额型给 balance),让小组件画进度条;
// 每个小组件实例绑定一个服务 id,单卡片展示。时间用 epoch 毫秒,由 Kotlin 本地化。

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::paths::home_dir;
use crate::state::{ServiceSnapshot, ServiceStatus};

fn service_json(s: &ServiceSnapshot) -> Value {
    let mut v = json!({
        "id": s.config.id,
        "title": s.config.title,
        "accent": s.config.accent,
    });
    match &s.status {
        ServiceStatus::Loading => {
            v["kind"] = json!("loading");
        }
        ServiceStatus::Error { message } => {
            v["kind"] = json!("error");
            v["message"] = json!(message);
        }
        ServiceStatus::Ok { usage, .. } | ServiceStatus::Stale { usage, .. } => {
            v["kind"] = json!("ok");
            if let Some(p) = &usage.plan {
                v["plan"] = json!(p);
            }
            if let Some(b) = &usage.balance {
                v["balance"] = json!({
                    "balance": b.balance,
                    "used": b.used,
                    "currency": b.currency,
                    "requestCount": b.request_count,
                });
            } else {
                let wins: Vec<Value> = usage
                    .windows
                    .iter()
                    .map(|w| {
                        let mut wj = json!({ "label": w.label, "pct": w.pct });
                        if let Some(r) = w.reset_at {
                            wj["resetAtMs"] = json!(r.timestamp_millis());
                        }
                        wj
                    })
                    .collect();
                v["windows"] = json!(wins);
            }
        }
    }
    v
}

pub fn write_snapshot(snapshots: &[ServiceSnapshot], last_updated: Option<DateTime<Utc>>) {
    let updated_ms = last_updated.map(|t| t.timestamp_millis()).unwrap_or(0);
    let services: Vec<Value> = snapshots.iter().map(service_json).collect();
    let doc = json!({ "updatedAtMs": updated_ms, "services": services });
    if let Some(home) = home_dir() {
        let _ = std::fs::write(home.join("widget.json"), doc.to_string());
    }
}
