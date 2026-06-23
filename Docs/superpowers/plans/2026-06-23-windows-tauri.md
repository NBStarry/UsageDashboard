# Windows 跨平台（Tauri 2）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Tauri 2 新建一个跨平台项目，交付与现有 macOS Swift 版功能对齐的 **Windows 托盘版**用量看板；现有 macOS Swift 应用原样保留。

**Architecture:** Rust 后端负责托盘、定时刷新、取数（reqwest）、配置/缓存读写、告警判定与通知；Web 前端（Svelte + TS）复刻 popover 面板与设置页，通过 Tauri `invoke` 命令调用 Rust，并监听 Rust 推送的 `usage-updated` / `config-updated` 事件。取数与告警的纯逻辑写成可单测的函数（TDD）；托盘/窗口/前端用手动验证。

**Tech Stack:** Tauri 2、Rust（serde / serde_json / reqwest[rustls] / chrono / tokio / dirs / wiremock[dev]）、Svelte 5 + TypeScript + Vite、tauri-plugin-notification、tauri-plugin-autostart。

## Global Constraints

- 目标平台本期只交付 **Windows**；不改动仓库根目录的 macOS Swift 工程，新项目全部放在 `tauri/` 子目录。
- 配置文件结构与现有完全一致（`refreshSeconds` / `alerts` / `services`），JSON 字段名 **camelCase**。
- Windows 路径：配置 `%APPDATA%\usage-bar\`、缓存 `%LOCALAPPDATA%\usage-dashboard\`、CLI 凭证 `%USERPROFILE%\.claude\.credentials.json` 与 `%USERPROFILE%\.codex\auth.json`。统一用 `dirs` crate 取目录，便于将来移动端换实现。
- 凭证存储本期用 **文件**（与 mac 版一致：New-API 凭证存 `%APPDATA%\usage-bar\<file>`）。Windows 凭据管理器 / 移动端安全存储为后续增强，不在本期。
- 进度条阈值色固定：`<75` 绿 `#3FB950` / `75–90` 琥珀 `#D29922` / `≥90` 红 `#F85149`。
- 默认配置（services 默认三项、refreshSeconds=300、alerts 默认）必须与现有 `AppConfig.default` 完全一致。
- 不打印 / 不记录 token、account_id、原始接口响应。
- 告警只作用于 `category == "subscription"` 的服务。
- 刷新间隔下限 60 秒；告警冷却下限 60 秒。
- 取数失败回退上次缓存并标 ⚠；从未成功过才显示纯错误。
- 每个任务以 `cargo test`（Rust 任务）或明确的手动验证步骤（UI/集成任务）结束，并以一次 commit 收尾。文档改动与相关代码改动放同一个 commit。

---

## 文件结构

```
tauri/
├── package.json                      # 前端 + tauri CLI 脚本
├── svelte.config.js                  # SvelteKit + adapter-static (SPA)
├── vite.config.js
├── tsconfig.json
├── src/                              # SvelteKit 前端（官方 svelte-ts 模板）
│   ├── app.html                      # HTML 壳
│   ├── routes/
│   │   ├── +layout.ts                # export const ssr = false（SPA 模式）
│   │   └── +page.svelte              # popover 根（header + 卡片列表 + footer / 设置切换）
│   ├── lib/                          # 用 $lib 别名导入（$lib → src/lib）
│   │   ├── types.ts                  # 与 Rust JSON 对齐的 TS 类型
│   │   ├── api.ts                    # invoke 封装 + 事件监听
│   │   ├── store.ts                  # Svelte store：config + states + lastUpdated
│   │   ├── theme.ts                  # 颜色 / 时间格式辅助
│   │   └── components/
│   │       ├── ServiceCard.svelte
│   │       ├── ProgressBar.svelte
│   │       ├── BalanceBody.svelte
│   │       └── SettingsView.svelte
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── icons/                        # 托盘 + 应用图标
    └── src/
        ├── main.rs                   # 入口：构建 AppState、托盘、窗口、插件、命令、定时器
        ├── models.rs                 # serde 数据模型 + 默认值
        ├── paths.rs                  # 目录解析（config/cache/home），支持测试覆盖
        ├── config_store.rs           # load/save config.json
        ├── cache.rs                  # 用量缓存读写
        ├── credentials.rs            # claude/codex/newapi 凭证读取
        ├── http.rs                   # reqwest GET JSON
        ├── fetchers.rs               # 三个取数器：网络 + 纯解析函数
        ├── alerts.rs                 # 告警判定纯逻辑
        ├── state.rs                  # AppState + 刷新编排 + 告警状态跟踪
        ├── commands.rs               # #[tauri::command] 集合
        └── tray.rs                   # 托盘图标 / 菜单 / 窗口显隐
```

---

## Phase 0：脚手架

### Task 1：脚手架 Tauri 2 + Svelte + TS 项目

**Files:**
- Create: `tauri/package.json`, `tauri/vite.config.ts`, `tauri/tsconfig.json`, `tauri/index.html`, `tauri/src/main.ts`, `tauri/src/App.svelte`
- Create: `tauri/src-tauri/Cargo.toml`, `tauri/src-tauri/tauri.conf.json`, `tauri/src-tauri/build.rs`, `tauri/src-tauri/src/main.rs`
- Create: `tauri/.gitignore`

**Interfaces:**
- Produces: 一个能在 Windows 上 `cargo build` 通过、`npm run tauri dev` 弹出空窗口的工程骨架；`cargo test` 可运行（即使暂无测试）。

- [ ] **Step 1：用脚手架生成基础工程**

在仓库根执行（交互式提示按下方选择）：

```bash
cd "A:/文档/Projects/UsageDashboard"
npm create tauri-app@latest tauri -- --template svelte-ts --manager npm
```

提示选择：前端语言 TypeScript、包管理器 npm、UI 模板 Svelte。生成后目录为 `tauri/`。

- [ ] **Step 2：确认 Cargo 包名与最小依赖**

编辑 `tauri/src-tauri/Cargo.toml`，将依赖区改为（版本以 `cargo add` 解析到的 2.x 为准）：

```toml
[package]
name = "usage-dashboard"
version = "1.0.0"
edition = "2021"

[lib]
name = "usage_dashboard_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

- [ ] **Step 3：确认 `tauri.conf.json` 基本项**

将 `tauri/src-tauri/tauri.conf.json` 的 `productName` 设为 `UsageDashboard`、`identifier` 设为 `app.usagedashboard.win`，`app.windows[0]` 暂留默认（后续 Task 12 改为隐藏/无边框）。

- [ ] **Step 4：验证构建与运行**

Run:
```bash
cd "A:/文档/Projects/UsageDashboard/tauri/src-tauri" && cargo build
cd "A:/文档/Projects/UsageDashboard/tauri" && cargo test --manifest-path src-tauri/Cargo.toml
```
Expected: `cargo build` 成功；`cargo test` 输出 `running 0 tests` 且退出码 0。

（可选手动）`cd tauri && npm install && npm run tauri dev` 应弹出一个空白窗口。

- [ ] **Step 5：Commit**

```bash
cd "A:/文档/Projects/UsageDashboard"
git add tauri/.gitignore tauri/package.json tauri/vite.config.ts tauri/tsconfig.json tauri/index.html tauri/src tauri/src-tauri
git commit -m "Scaffold Tauri 2 + Svelte/TS project for Windows"
```

---

## Phase 1：Rust 核心逻辑（TDD）

### Task 2：数据模型与默认值

**Files:**
- Create: `tauri/src-tauri/src/models.rs`
- Modify: `tauri/src-tauri/src/main.rs`（加 `mod models;`）
- Test: 同文件 `#[cfg(test)]`

**Interfaces:**
- Produces:
  - `BillingCategory`（`subscription` | `apiUsage`），`FetcherKind`（`claudeOAuth`|`codexWham`|`newAPI`|`unsupported`），`UsageAlertRule`（`usageExceedsElapsedWindowPercent`|`usageExceedsThresholdOnly`）—— 均 `#[serde(rename_all=...)]` 对齐 Swift 原始值。
  - `UsageAlertConfig { enabled, minimum_usage_percent, rule, pace_multiplier, cooldown_seconds, service_ids: Option<Vec<String>>, windows: Option<Vec<String>> }`，serde camelCase + 缺省默认。
  - `ServiceDisplayOptions`（9 个 bool，缺省全 true）。
  - `ServiceConfig { id, title, accent, category, fetcher, credential_file: Option<String>, enabled, display }`，缺省解析逻辑同 Swift（按 id 推断 category/fetcher，accent 缺省 `#8E8E93`）。
  - `AppConfig { refresh_seconds, alerts, services }` + `AppConfig::default()`（三项默认服务，见 Global Constraints）。
  - 运行态：`UsageWindow { label, pct, reset_at: Option<DateTime<Utc>> }`、`ModelEntry { name, vendor }`、`BalanceInfo { balance, used, currency, request_count: Option<i64>, models }`、`Usage { plan: Option<String>, windows, balance: Option<BalanceInfo> }`。
  - 这些运行态类型 `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]`，`reset_at` 用 chrono 默认 RFC3339 序列化，供前端消费。
  - `BalanceInfo::grouped_models(&self) -> Vec<(String, Vec<ModelEntry>)>`：按 vendor 分组，组内按 name 升序，组按 (模型数, vendor) 降序——对齐 Swift `groupedModels`。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_three_services() {
        let c = AppConfig::default();
        assert_eq!(c.refresh_seconds, 300);
        let ids: Vec<_> = c.services.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["claude", "codex", "phanrouter"]);
        assert_eq!(c.services[2].fetcher, FetcherKind::NewApi);
        assert_eq!(c.services[2].credential_file.as_deref(), Some("phanrouter.json"));
        assert_eq!(c.alerts.minimum_usage_percent, 60.0);
    }

    #[test]
    fn service_config_infers_defaults_from_id() {
        let json = serde_json::json!({"id":"claude","title":"Claude"});
        let s: ServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(s.accent, "#8E8E93");
        assert_eq!(s.category, BillingCategory::Subscription);
        assert_eq!(s.fetcher, FetcherKind::ClaudeOauth);
        assert!(s.enabled);
        assert!(s.display.five_hour);
    }

    #[test]
    fn alert_config_fills_missing_fields() {
        let a: UsageAlertConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(a.enabled);
        assert_eq!(a.cooldown_seconds, 1800);
        assert_eq!(a.rule, UsageAlertRule::UsageExceedsElapsedWindowPercent);
    }

    #[test]
    fn grouped_models_sorts_by_count_then_vendor() {
        let info = BalanceInfo {
            balance: 0.0, used: 0.0, currency: "$".into(), request_count: None,
            models: vec![
                ModelEntry { name: "b".into(), vendor: "OpenAI".into() },
                ModelEntry { name: "a".into(), vendor: "OpenAI".into() },
                ModelEntry { name: "c".into(), vendor: "Google".into() },
            ],
        };
        let g = info.grouped_models();
        assert_eq!(g[0].0, "OpenAI");
        assert_eq!(g[0].1.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(g[1].0, "Google");
    }
}
```

- [ ] **Step 2：运行测试确认失败**

Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml models`
Expected: 编译失败（类型未定义）。

- [ ] **Step 3：实现 models.rs**

实现上述所有类型。要点：
- 枚举用 `#[serde(rename_all = "camelCase")]`（`FetcherKind` 的 `newAPI` 需显式 `#[serde(rename = "newAPI")]`）。
- 用 `#[serde(default = "fn")]` 给每个可缺省字段提供默认值；`ServiceConfig` 与 `AppConfig` 用自定义 `Deserialize`（或 `#[serde(default)]` + 后处理）实现"按 id 推断 category/fetcher"。推荐用一个 `#[serde(default)]` 中间结构 + `impl From` 后处理推断。
- `AppConfig::default()` 返回三项默认服务，accent 分别 `#D97757` / `#10A37F` / `#7C5CFC`。
- 运行态结构体加 `#[derive(Clone, Serialize)]` 与 camelCase。

完整推断规则（移植 Swift `ServiceConfig.defaultCategory/defaultFetcher`）：

```rust
fn default_category(id: &str) -> BillingCategory {
    match id { "claude" | "codex" => BillingCategory::Subscription, _ => BillingCategory::ApiUsage }
}
fn default_fetcher(id: &str, category: BillingCategory) -> FetcherKind {
    match id {
        "claude" => FetcherKind::ClaudeOauth,
        "codex" => FetcherKind::CodexWham,
        _ => if category == BillingCategory::ApiUsage { FetcherKind::NewApi } else { FetcherKind::Unsupported },
    }
}
```

- [ ] **Step 4：运行测试确认通过**

Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml models`
Expected: 4 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/models.rs tauri/src-tauri/src/main.rs
git commit -m "Add data models with Swift-parity defaults"
```

---

### Task 3：目录解析（paths）

**Files:**
- Create: `tauri/src-tauri/src/paths.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod paths;`）
- Test: 同文件

**Interfaces:**
- Produces:
  - `config_dir() -> PathBuf`（`%APPDATA%\usage-bar`）
  - `cache_dir() -> PathBuf`（`%LOCALAPPDATA%\usage-dashboard`）
  - `home_dir() -> Option<PathBuf>`
  - 为可测试性：内部读 `USAGE_DASHBOARD_HOME` 环境变量覆盖根目录，测试用它指向临时目录。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_dir_under_override_root() {
        std::env::set_var("USAGE_DASHBOARD_HOME", "C:\\tmp\\udtest");
        assert!(config_dir().ends_with("usage-bar"));
        assert!(cache_dir().ends_with("usage-dashboard"));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
```

- [ ] **Step 2：运行确认失败**

Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml paths`
Expected: 编译失败。

- [ ] **Step 3：实现 paths.rs**

```rust
use std::path::PathBuf;

fn override_root() -> Option<PathBuf> {
    std::env::var_os("USAGE_DASHBOARD_HOME").map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    if let Some(r) = override_root() { return r.join("usage-bar"); }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-bar")
}

pub fn cache_dir() -> PathBuf {
    if let Some(r) = override_root() { return r.join("usage-dashboard"); }
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-dashboard")
}

pub fn home_dir() -> Option<PathBuf> {
    if let Some(r) = override_root() { return Some(r); }
    dirs::home_dir()
}
```

- [ ] **Step 4：运行确认通过**

Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml paths`
Expected: PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/paths.rs tauri/src-tauri/src/main.rs
git commit -m "Add platform path resolution with test override"
```

---

### Task 4：配置读写（config_store）

**Files:**
- Create: `tauri/src-tauri/src/config_store.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod config_store;`）
- Test: 同文件（用 `USAGE_DASHBOARD_HOME` 指向 `tempfile::tempdir()`）

**Interfaces:**
- Consumes: `models::AppConfig`、`paths::config_dir`
- Produces:
  - `load() -> AppConfig`：文件存在且可解析则返回；否则写默认并返回默认。
  - `save(cfg: &AppConfig) -> std::io::Result<()>`：pretty JSON 写入 `config_dir()/config.json`。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_writes_default_when_missing_then_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let c = load();
        assert_eq!(c.services.len(), 3);
        assert!(crate::paths::config_dir().join("config.json").exists());

        let mut next = c.clone();
        next.refresh_seconds = 120;
        save(&next).unwrap();
        assert_eq!(load().refresh_seconds, 120);
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml config_store`，Expected: 编译失败。

- [ ] **Step 3：实现 config_store.rs**

```rust
use crate::models::AppConfig;
use crate::paths;
use std::fs;

fn config_path() -> std::path::PathBuf { paths::config_dir().join("config.json") }

pub fn load() -> AppConfig {
    if let Ok(data) = fs::read(config_path()) {
        if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&data) {
            return cfg;
        }
    }
    let def = AppConfig::default();
    let _ = save(&def);
    def
}

pub fn save(cfg: &AppConfig) -> std::io::Result<()> {
    let dir = paths::config_dir();
    fs::create_dir_all(&dir)?;
    let data = serde_json::to_vec_pretty(cfg).expect("serialize config");
    fs::write(config_path(), data)
}
```

- [ ] **Step 4：运行确认通过** — Run: 同上，Expected: PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/config_store.rs tauri/src-tauri/src/main.rs
git commit -m "Add config load/save with default seeding"
```

---

### Task 5：用量缓存（cache）

**Files:**
- Create: `tauri/src-tauri/src/cache.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod cache;`）
- Test: 同文件

**Interfaces:**
- Consumes: `models::{Usage, UsageWindow, BalanceInfo, ModelEntry}`、`paths::cache_dir`
- Produces:
  - `struct Cached { pub usage: Usage, pub ts: Option<DateTime<Utc>> }`
  - `write(usage: &Usage, service: &str)`：写 `cache_dir()/<service>.json`，格式与 Swift 兼容：窗口型 `{"ts","plan","windows":[{"label","pct","resetAt"}]}`；余额型 `{"ts","balance":{"balance","used","currency","requestCount","models":[{"name","vendor"}]}}`。`resetAt`/`ts` 用 RFC3339（`withInternetDateTime`，无小数秒）。
  - `read(service: &str) -> Option<Cached>`：优先解析 balance；否则解析 windows；windows 为空且无 balance 返回 None。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[test]
    fn window_usage_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let u = Usage { plan: Some("Pro".into()), windows: vec![
            UsageWindow { label: "5 小时".into(), pct: 72.0, reset_at: None },
        ], balance: None };
        write(&u, "claude");
        let got = read("claude").unwrap();
        assert_eq!(got.usage.plan.as_deref(), Some("Pro"));
        assert_eq!(got.usage.windows[0].pct, 72.0);
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn balance_usage_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let u = Usage { plan: None, windows: vec![], balance: Some(BalanceInfo{
            balance: 18.42, used: 31.58, currency: "$".into(), request_count: Some(12),
            models: vec![ModelEntry{ name:"gpt-4.1".into(), vendor:"OpenAI".into()}],
        })};
        write(&u, "phanrouter");
        let got = read("phanrouter").unwrap();
        let b = got.usage.balance.unwrap();
        assert_eq!(b.balance, 18.42);
        assert_eq!(b.models[0].vendor, "OpenAI");
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        assert!(read("nope").is_none());
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml cache`

- [ ] **Step 3：实现 cache.rs**

用 `serde_json::Value` 手工组装/解析以保证字段名与格式和 Swift 完全一致。`ts` 写入用 `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)`；解析用 `DateTime::parse_from_rfc3339`。读取时数值字段容忍 number/string（写一个 `num(&Value)->Option<f64>` 辅助，对齐 Swift）。`request_count` 读为 `i64`。

- [ ] **Step 4：运行确认通过** — Expected: 3 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/cache.rs tauri/src-tauri/src/main.rs
git commit -m "Add usage cache compatible with existing JSON format"
```

---

### Task 6：凭证读取（credentials）

**Files:**
- Create: `tauri/src-tauri/src/credentials.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod credentials;`）
- Test: 同文件

**Interfaces:**
- Consumes: `paths::{home_dir, config_dir}`
- Produces:
  - `claude_token() -> Option<String>`：读 `home/.claude/.credentials.json` → `claudeAiOauth.accessToken`（Windows 无 Keychain，直接读文件）。
  - `enum CodexCreds { Ok{ access_token, account_id }, MissingFile, ParseError, Incomplete }`，`codex_creds() -> CodexCreds`：读 `home/.codex/auth.json`，token/account 从 `tokens.{access_token,account_id}` 或顶层取。
  - `enum NewApiCreds { Ok(NewApiCredsData), MissingFile(String), InvalidPath, Incomplete }`，`new_api_creds(file_name: &str) -> NewApiCreds`：路径校验（非空、非绝对、不含 `..`），读 `config_dir()/<file>`，字段 `baseUrl/accessToken/userId/quotaPerUnit/currency`，`quotaPerUnit` 缺省 500000，`currency` 缺省 `$`，`userId` 容忍 number/string。
  - `struct NewApiCredsData { base_url, access_token, user_id: i64, quota_per_unit: f64, currency: String }`

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        dir
    }

    #[test]
    fn reads_claude_token_from_file() {
        let dir = setup();
        let p = dir.path().join(".claude");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"abc123"}}"#).unwrap();
        assert_eq!(claude_token().as_deref(), Some("abc123"));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn codex_missing_file() {
        let _d = setup();
        assert!(matches!(codex_creds(), CodexCreds::MissingFile));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn new_api_rejects_traversal() {
        let _d = setup();
        assert!(matches!(new_api_creds("../evil.json"), NewApiCreds::InvalidPath));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn new_api_reads_fields_with_defaults() {
        let dir = setup();
        let cfgdir = crate::paths::config_dir();
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(cfgdir.join("phanrouter.json"),
            r#"{"baseUrl":"https://x/new-api","accessToken":"tok","userId":"7"}"#).unwrap();
        match new_api_creds("phanrouter.json") {
            NewApiCreds::Ok(c) => {
                assert_eq!(c.user_id, 7);
                assert_eq!(c.quota_per_unit, 500000.0);
                assert_eq!(c.currency, "$");
            }
            _ => panic!("expected ok"),
        }
        let _ = dir;
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml credentials`

- [ ] **Step 3：实现 credentials.rs**

按 Swift `CredentialStore` 移植（去掉 macOS Keychain 分支，直接读文件）。注意 `base_url` 末尾 `/` 由调用方（fetcher）裁剪，这里只读原值；空 `baseUrl`/`accessToken` → `Incomplete`。

- [ ] **Step 4：运行确认通过** — Expected: 4 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/credentials.rs tauri/src-tauri/src/main.rs
git commit -m "Add credential readers (file-based) for claude/codex/newAPI"
```

---

### Task 7：HTTP 封装（http）

**Files:**
- Create: `tauri/src-tauri/src/http.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod http;`）
- Test: 同文件（wiremock）

**Interfaces:**
- Produces: `async fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<(serde_json::Value, u16), String>`：12 秒超时；网络失败返回 `Err`（可读消息）；HTTP 状态码原样返回（不在此判定 401 等）；响应非 JSON 返回 `Err`。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    async fn returns_json_and_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u")).and(header("x-test", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server).await;
        let url = format!("{}/u", server.uri());
        let (v, status) = get_json(&url, &[("x-test", "1")]).await.unwrap();
        assert_eq!(status, 200);
        assert_eq!(v["ok"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn surfaces_non_2xx_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({})))
            .mount(&server).await;
        let url = format!("{}/u", server.uri());
        let (_v, status) = get_json(&url, &[]).await.unwrap();
        assert_eq!(status, 401);
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml http`

- [ ] **Step 3：实现 http.rs**

```rust
use std::time::Duration;

pub async fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<(serde_json::Value, u16), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("客户端初始化失败:{e}"))?;
    let mut req = client.get(url);
    for (k, v) in headers { req = req.header(*k, *v); }
    let resp = req.send().await.map_err(|e| format!("网络错误:{e}"))?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.map_err(|e| format!("网络错误:{e}"))?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| "接口返回无法解析".to_string())?;
    Ok((value, status))
}
```

- [ ] **Step 4：运行确认通过** — Expected: 2 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/http.rs tauri/src-tauri/src/main.rs
git commit -m "Add reqwest GET-JSON helper"
```

---

### Task 8：取数器与解析（fetchers）

**Files:**
- Create: `tauri/src-tauri/src/fetchers.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod fetchers;`）
- Test: 同文件（解析纯函数喂样本 JSON；不打网络）

**Interfaces:**
- Consumes: `models::*`、`credentials::*`、`http::get_json`、`cache`
- Produces（纯解析函数，可单测）：
  - `parse_claude(v: &Value, status: u16) -> Result<Usage, String>`
  - `parse_codex(v: &Value, status: u16) -> Result<Usage, String>`
  - `parse_newapi_self(v: &Value, status: u16, quota_per_unit: f64, currency: &str, title: &str) -> Result<(f64,f64,Option<i64>), String>`（返回 balance/used/request_count）
  - `parse_pricing(v: &Value) -> Vec<ModelEntry>`
  - `infer_vendor(model: &str) -> String`
  - 日期辅助：`date_from_epoch(&Value)->Option<DateTime<Utc>>`、`date_from_iso(&Value)->Option<DateTime<Utc>>`、`capitalized_plan(&Value)->Option<String>`
- Produces（网络入口）：
  - `async fn fetch_service(cfg: &ServiceConfig) -> Result<Usage, String>`：按 `cfg.fetcher` 取凭证 + 调 `get_json` + 调对应 parse 函数；`unsupported` 直接返回 `Err`。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn claude_parses_two_windows_and_plan() {
        let v = json!({
            "plan_type": "pro",
            "five_hour": {"utilization": 72.4, "resets_at": "2026-06-23T18:00:00Z"},
            "seven_day": {"utilization": 48, "resets_at": "2026-06-25T18:00:00Z"}
        });
        let u = parse_claude(&v, 200).unwrap();
        assert_eq!(u.plan.as_deref(), Some("Pro"));
        assert_eq!(u.windows.len(), 2);
        assert_eq!(u.windows[0].label, "5 小时");
        assert_eq!(u.windows[0].pct, 72.4);
        assert!(u.windows[0].reset_at.is_some());
    }

    #[test]
    fn claude_401_is_expired_error() {
        let v = json!({});
        assert!(parse_claude(&v, 401).unwrap_err().contains("过期"));
    }

    #[test]
    fn codex_converts_percent_left_to_used() {
        let v = json!({
            "rate_limits": {
                "primary": {"percent_left": 30, "reset_time_ms": 1_900_000_000_000i64},
                "secondary": {"used_percent": 12}
            }
        });
        let u = parse_codex(&v, 200).unwrap();
        assert_eq!(u.windows[0].pct, 70.0);
        assert_eq!(u.windows[1].pct, 12.0);
    }

    #[test]
    fn codex_detects_expired_envelope() {
        let v = json!({"error": {"code":"token_expired","message":"expired"}});
        assert!(parse_codex(&v, 200).unwrap_err().contains("过期"));
    }

    #[test]
    fn newapi_self_computes_money() {
        let v = json!({"success": true, "data": {"quota": 9_210_000, "used_quota": 15_790_000, "request_count": 12864}});
        let (bal, used, rc) = parse_newapi_self(&v, 200, 500000.0, "$", "X").unwrap();
        assert!((bal - 18.42).abs() < 1e-9);
        assert!((used - 31.58).abs() < 1e-9);
        assert_eq!(rc, Some(12864));
    }

    #[test]
    fn pricing_maps_vendor_id_then_infers() {
        let v = json!({
            "data": [
                {"model_name":"gpt-4.1","vendor_id":1},
                {"model_name":"claude-3.7-sonnet"},
                {"model_name":"pr-ge-foo"}
            ],
            "vendors": [{"id":1,"name":"OpenAI"}]
        });
        let m = parse_pricing(&v);
        assert_eq!(m[0].vendor, "OpenAI");
        assert_eq!(m[1].vendor, "Anthropic");
        assert_eq!(m[2].vendor, "Google");
    }

    #[test]
    fn infer_vendor_prefix_order() {
        assert_eq!(infer_vendor("pr-ge-x"), "Google");
        assert_eq!(infer_vendor("pr-g-x"), "Google");
        assert_eq!(infer_vendor("deepseek-r1"), "DeepSeek");
        assert_eq!(infer_vendor("totally-unknown"), "其他");
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml fetchers`

- [ ] **Step 3：实现 fetchers.rs**

逐字移植 Swift `UsageFetcher.swift` 的解析逻辑：
- `parse_claude`：401→`"Claude 登录已过期,请重新登录"`；非 2xx→`"接口请求失败 (HTTP n)"`；遍历 `("five_hour","5 小时")`/`("seven_day","周")`，`utilization` 容忍 number/string，`pct = (x*10).round()/10`，`resets_at` 走 `date_from_iso`；无窗口→`"未解析到用量数据(接口结构可能已变)"`。plan 走 `capitalized_plan(plan_type)`。
- `parse_codex`：401（含 200 错误信封里的 expired）→过期错误；`rate_limit`/`rate_limits` 取容器，五小时键 `["five_hour","five_hour_limit","five_hour_rate_limit","primary","primary_window"]`、周键 `["weekly","weekly_limit","weekly_rate_limit","secondary","secondary_window"]`；`used_pct`：先 `percent_left`/`remaining_percent`→`100-x`，否则 `used_percent`；`reset_at`：`reset_time_ms`（epoch）/`reset_at`（epoch 或 ISO），含 `primary_window` 嵌套兜底。
- `parse_newapi_self`：401→令牌失效；`success==false` 按 message 区分；`data.quota` 必须存在，`unit = if qpu>0 {qpu} else {500000}`，`balance=quota/unit`，`used=used_quota/unit`，`request_count` 容忍 number/string。
- `parse_pricing` + `infer_vendor`：完整搬运 vendor_id 映射与前缀/关键字规则表（**注意 `pr-ge` 必须在 `pr-g-` 之前判断**）。
- `date_from_epoch`：`raw > 1e11` 视为毫秒。
- `fetch_service`：按 fetcher 分支取凭证（缺失/解析错误返回对应中文 `Err`，文案对齐 Swift），拼 headers，调 `get_json`，网络错误透传，再调 parse；newAPI 的 `/api/pricing` 失败不致命（返回空 models）。

- [ ] **Step 4：运行确认通过** — Expected: 7 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/fetchers.rs tauri/src-tauri/src/main.rs
git commit -m "Port claude/codex/newAPI fetchers with parsing tests"
```

---

### Task 9：告警判定（alerts）

**Files:**
- Create: `tauri/src-tauri/src/alerts.rs`
- Modify: `tauri/src-tauri/src/main.rs`（`mod alerts;`）
- Test: 同文件

**Interfaces:**
- Consumes: `models::{UsageAlertConfig, UsageAlertRule, UsageWindow, ServiceConfig, Usage}`
- Produces:
  - `window_duration_seconds(label: &str) -> Option<f64>`（含 "5"→5h，含 "周"→7d）
  - `elapsed_window_percent(label: &str, reset_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Option<f64>`
  - `rule_matches(rule, usage_pct, elapsed_pct: Option<f64>, pace: f64) -> bool`
  - `alert_key(service_id, label, reset_at, rule) -> String`
  - `struct EvalResult { pub active_keys: Vec<String>, pub fires: Vec<AlertFire> }`，`struct AlertFire { pub key: String, pub title: String, pub body: String }`
  - `evaluate(service: &ServiceConfig, usage: &Usage, alerts: &UsageAlertConfig, now: DateTime<Utc>) -> EvalResult`：复刻 Swift `updateAlerts` 的判定（不含冷却，冷却在 state 层用 `lastAlertAtByKey` 处理）；非 subscription / 不在 serviceIDs / 不在 windows 过滤 → 空。

- [ ] **Step 1：写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use chrono::{Utc, Duration};

    fn sub_service() -> ServiceConfig {
        serde_json::from_value(serde_json::json!({"id":"claude","title":"Claude"})).unwrap()
    }

    #[test]
    fn threshold_only_fires_above_minimum() {
        let now = Utc::now();
        let svc = sub_service();
        let usage = Usage { plan: None, balance: None, windows: vec![
            UsageWindow { label: "5 小时".into(), pct: 80.0, reset_at: Some(now + Duration::hours(1)) },
        ]};
        let mut a = UsageAlertConfig::default();
        a.rule = UsageAlertRule::UsageExceedsThresholdOnly;
        a.minimum_usage_percent = 60.0;
        let r = evaluate(&svc, &usage, &a, now);
        assert_eq!(r.fires.len(), 1);
        assert_eq!(r.active_keys.len(), 1);
    }

    #[test]
    fn below_threshold_no_fire() {
        let now = Utc::now();
        let svc = sub_service();
        let usage = Usage { plan: None, balance: None, windows: vec![
            UsageWindow { label: "5 小时".into(), pct: 50.0, reset_at: Some(now + Duration::hours(1)) },
        ]};
        let mut a = UsageAlertConfig::default();
        a.minimum_usage_percent = 60.0;
        assert!(evaluate(&svc, &usage, &a, now).fires.is_empty());
    }

    #[test]
    fn elapsed_rule_requires_outpacing_time() {
        let now = Utc::now();
        // 5h 窗口，还剩 1h → 已流逝 80%
        let reset = now + Duration::hours(1);
        let svc = sub_service();
        let mut a = UsageAlertConfig::default();
        a.rule = UsageAlertRule::UsageExceedsElapsedWindowPercent;
        a.minimum_usage_percent = 60.0;
        a.pace_multiplier = 1.0;
        let over = Usage { plan:None, balance:None, windows: vec![
            UsageWindow{ label:"5 小时".into(), pct: 90.0, reset_at: Some(reset) }]};
        assert_eq!(evaluate(&svc, &over, &a, now).fires.len(), 1);
        let under = Usage { plan:None, balance:None, windows: vec![
            UsageWindow{ label:"5 小时".into(), pct: 70.0, reset_at: Some(reset) }]};
        assert!(evaluate(&svc, &under, &a, now).fires.is_empty());
    }

    #[test]
    fn api_usage_service_never_alerts() {
        let now = Utc::now();
        let svc: ServiceConfig = serde_json::from_value(
            serde_json::json!({"id":"phanrouter","title":"P","category":"apiUsage"})).unwrap();
        let usage = Usage { plan:None, balance:None, windows: vec![
            UsageWindow{ label:"5 小时".into(), pct: 99.0, reset_at: Some(now+Duration::hours(1))}]};
        let a = UsageAlertConfig::default();
        assert!(evaluate(&svc, &usage, &a, now).fires.is_empty());
    }
}
```

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml alerts`

- [ ] **Step 3：实现 alerts.rs**

移植 Swift `UsageStore` 的告警算法：`elapsed = duration - (reset - now)`，`elapsed_pct = clamp(elapsed/duration*100, 0, 100)`；`elapsed` 规则 `target = clamp(elapsed_pct * max(0.1, pace), 0, 100)`，`usage_pct > target`；`threshold_only` 恒真（前提已过阈值）。`body` 文案对齐 Swift `alertBody`。`alert_key` = `"{id}:{label}:{reset_epoch}:{rule_raw}"`，`reset_epoch` 用 `reset_at.map(timestamp).unwrap_or(0)`。

- [ ] **Step 4：运行确认通过** — Expected: 4 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/alerts.rs tauri/src-tauri/src/main.rs
git commit -m "Add alert evaluation logic with tests"
```

---

## Phase 2：Tauri 集成

### Task 10：应用状态与命令（state + commands）

**Files:**
- Create: `tauri/src-tauri/src/state.rs`, `tauri/src-tauri/src/commands.rs`
- Modify: `tauri/src-tauri/src/main.rs`
- Test: `state.rs` 内对 `apply_outcome` 的回退逻辑做单测

**Interfaces:**
- Consumes: 全部前述模块
- Produces:
  - `state.rs`：
    - `enum ServiceStatus { Loading, Ok{usage, fetched_at}, Stale{usage, cached_at, error}, Error(String) }`（Serialize，camelCase，带内部 `kind` tag 供前端区分，如 `#[serde(tag="kind")]`）。
    - `struct ServiceSnapshot { config: ServiceConfig, status: ServiceStatus }`（Serialize；前端列表项）。
    - `struct AppState { config: Mutex<AppConfig>, statuses: Mutex<HashMap<String, ServiceStatus>>, last_alert_at: Mutex<HashMap<String,DateTime<Utc>>>, active_alert_keys: Mutex<HashMap<String,HashSet<String>>>, last_updated: Mutex<Option<DateTime<Utc>>> }`
    - `fn snapshots(&self) -> Vec<ServiceSnapshot>`（按 config 顺序、仅 enabled，状态缺失时用 cache→Stale("加载中…") 或 Loading）。
    - `fn active_alert_count(&self) -> usize`
    - `async fn refresh(&self, app: &AppHandle)`：对每个 enabled 服务调用 `fetchers::fetch_service`，`apply_outcome` 更新状态 + 写缓存 + 跑 `alerts::evaluate`（带冷却）+ 触发通知 + 更新托盘；结束设 `last_updated` 并 `emit("usage-updated")`。
    - `fn apply_outcome(&self, id, outcome)`：成功→写 cache + Ok；失败→有 cache 则 Stale，否则 Error（**单测覆盖此函数**，不依赖网络）。
  - `commands.rs`（全部 `#[tauri::command]`，返回 `Result<_, String>`，改配置后 `save` 并 `emit("config-updated")`）：
    - `get_snapshots(state) -> Vec<ServiceSnapshot>`
    - `get_config(state) -> AppConfig`
    - `refresh_now(app, state)`
    - `set_service_enabled(app, state, id, enabled)`
    - `move_service(app, state, id, delta)`
    - `set_display_content(app, state, id, item, enabled)`
    - `set_alerts_enabled / set_alert_threshold / set_alert_rule / set_alert_cooldown_minutes`
    - `save_new_api_credentials(state, file_name, json)`：把手动录入的 New-API 凭证写入 `config_dir()/<file>`（路径复用 credentials 的校验）。
    - `quit(app)`

- [ ] **Step 1：写失败测试（apply_outcome 回退）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_falls_back_to_cache_as_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let st = AppState::new(crate::models::AppConfig::default());
        // 预置缓存
        let u = crate::models::Usage { plan: Some("Pro".into()), windows: vec![
            crate::models::UsageWindow{ label:"周".into(), pct: 10.0, reset_at: None}], balance: None };
        crate::cache::write(&u, "claude");
        st.apply_outcome("claude", Err("boom".into()));
        let s = st.statuses.lock().unwrap();
        match s.get("claude").unwrap() {
            ServiceStatus::Stale{ error, .. } => assert_eq!(error, "boom"),
            _ => panic!("expected stale"),
        }
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    fn failure_without_cache_is_error() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let st = AppState::new(crate::models::AppConfig::default());
        st.apply_outcome("codex", Err("nope".into()));
        let s = st.statuses.lock().unwrap();
        assert!(matches!(s.get("codex").unwrap(), ServiceStatus::Error(_)));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
```

注：`apply_outcome` 中触发通知/托盘的部分要与纯状态更新分离（传入可选 `AppHandle`，测试传 `None`），保证可单测。

- [ ] **Step 2：运行确认失败** — Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml state`

- [ ] **Step 3：实现 state.rs 与 commands.rs**

按上面接口实现。`refresh` 用 `futures`/`tokio::join_all` 或顺序 `await` 并发取数（可用 `tokio::task::JoinSet`）。冷却：`now - last < max(60, cooldown)` 则跳过通知但仍计入 active_keys。通知用 `tauri_plugin_notification::NotificationExt`：`app.notification().builder().title(t).body(b).show()`。

- [ ] **Step 4：运行确认通过** — Expected: 2 个测试 PASS。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/state.rs tauri/src-tauri/src/commands.rs tauri/src-tauri/src/main.rs
git commit -m "Add app state, refresh orchestration, and Tauri commands"
```

---

### Task 11：托盘、窗口与启动装配（tray + main）

**Files:**
- Create: `tauri/src-tauri/src/tray.rs`, `tauri/src-tauri/icons/tray.png`, `tauri/src-tauri/icons/tray-alert.png`
- Modify: `tauri/src-tauri/src/main.rs`, `tauri/src-tauri/tauri.conf.json`
- 验证：手动（Windows 运行）

**Interfaces:**
- Consumes: `AppState`, commands, plugins
- Produces:
  - `tray.rs`：`build_tray(app) -> TrayIcon`：左键 toggle popover 窗口、右键弹菜单（立即刷新 / 开机自启[勾选态] / 退出）；`set_alert(app, active: bool)` 切换托盘图标（普通 `tray.png` / 告警 `tray-alert.png`）与 tooltip。
  - popover 窗口：`label="main"`，`decorations:false`、`resizable:false`、`skipTaskbar:true`、`alwaysOnTop:true`、初始 `visible:false`；失焦（`WindowEvent::Focused(false)`）自动 `hide`；显示时定位到托盘附近（用鼠标位置/屏幕右下角）并 `refresh_now`。
  - `main.rs`：注册 autostart 插件（`tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)` 在 Windows 用注册表实现）、notification 插件、`manage(AppState::new(config_store::load()))`、`invoke_handler` 注册全部命令、`setup` 中 `build_tray` + 启动后台刷新定时器（`tokio::spawn` 循环 `sleep(refresh_seconds) → state.refresh`）+ 首刷。

- [ ] **Step 1：准备托盘图标**

用现有 `AppIcon.icns` 或新画一个 32×32 仪表盘风格 PNG 作 `tray.png`；`tray-alert.png` 在右上角叠加红点感叹号（对齐 Swift `alertStatusImage`）。可用任意绘图导出，提交到 `icons/`。

- [ ] **Step 2：实现 tray.rs + main.rs 装配**

按接口实现。托盘菜单项用 `MenuItemBuilder`；开机自启勾选态读 `app.autolaunch().is_enabled()`。窗口定位用 `window.set_position`。

- [ ] **Step 3：配置 tauri.conf.json 窗口**

```json
"app": {
  "windows": [
    {
      "label": "main",
      "width": 340,
      "height": 560,
      "decorations": false,
      "resizable": false,
      "skipTaskbar": true,
      "alwaysOnTop": true,
      "visible": false,
      "transparent": true
    }
  ],
  "trayIcon": { "iconPath": "icons/tray.png", "iconAsTemplate": false }
}
```

- [ ] **Step 4：手动验证（Windows）**

Run: `cd tauri && npm run tauri dev`
Expected（逐项确认并截图）：
1. 托盘出现仪表盘图标，任务栏无窗口。
2. 左键托盘 → 弹出面板；再次左键或点击别处 → 收起。
3. 右键托盘 → 出现"立即刷新 / 开机自启 / 退出"菜单；点"退出"进程结束。
4. 勾选"开机自启"后，注册表 `HKCU\...\Run` 出现对应项（`reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"`）。

- [ ] **Step 5：Commit**

```bash
git add tauri/src-tauri/src/tray.rs tauri/src-tauri/src/main.rs tauri/src-tauri/tauri.conf.json tauri/src-tauri/icons
git commit -m "Add tray icon, popover window, autostart and background refresh"
```

---

## Phase 3：前端 UI（Svelte + TS）

### Task 12：前端类型、API 与 store

**Files:**
- Create: `tauri/src/lib/types.ts`, `tauri/src/lib/api.ts`, `tauri/src/lib/store.ts`, `tauri/src/lib/theme.ts`
- 验证：`npm run check`（svelte-check）通过

**Interfaces:**
- Produces:
  - `types.ts`：`Usage / UsageWindow / BalanceInfo / ModelEntry / ServiceConfig / AppConfig / UsageAlertConfig / ServiceDisplayOptions / ServiceStatus / ServiceSnapshot`，字段与 Rust camelCase 序列化一致；`ServiceStatus` 按 `kind` 区分。
  - `api.ts`：封装 `invoke`：`getSnapshots() / getConfig() / refreshNow() / setServiceEnabled() / moveService() / setDisplayContent() / setAlertsEnabled() / setAlertThreshold() / setAlertRule() / setAlertCooldownMinutes() / saveNewApiCredentials() / quit()`；以及 `onUsageUpdated(cb) / onConfigUpdated(cb)`（`@tauri-apps/api/event` 的 `listen`）。
  - `store.ts`：Svelte writable `snapshots`、`config`、`lastUpdated`、`isRefreshing`；`init()` 拉取初值 + 注册事件监听刷新 store。
  - `theme.ts`：`barColor(pct)`（阈值色）、`hm(date)`、`resetCountdown(isoOrDate)`、accent 直接用字符串。

- [ ] **Step 0：安装依赖并清理脚手架残留**

先 `cd tauri && npm install`。从 `package.json` 移除脚手架遗留的未用依赖 `@tauri-apps/plugin-opener`（Rust 端与 capabilities 已不再用它），重新 `npm install` 同步 lockfile。文件内导入统一用 SvelteKit 的 `$lib` 别名（如 `import { barColor } from '$lib/theme'`）。

- [ ] **Step 1：实现四个文件**

`barColor`：`pct>=90 → #F85149`，`>=75 → #D29922`，else `#3FB950`。`resetCountdown`：`重置 Xh Ym 后` / `重置 Ym 后`。

- [ ] **Step 2：验证类型检查**

Run: `cd tauri && npm run check`
Expected: 0 errors（若脚手架无 `check` 脚本，加 `"check": "svelte-check --tsconfig ./tsconfig.json"`）。

- [ ] **Step 3：Commit**

```bash
git add tauri/src/lib/types.ts tauri/src/lib/api.ts tauri/src/lib/store.ts tauri/src/lib/theme.ts tauri/package.json
git commit -m "Add frontend types, Tauri API wrappers, store and theme"
```

---

### Task 13：进度条、用量卡与余额卡组件

**Files:**
- Create: `tauri/src/lib/components/ProgressBar.svelte`, `ServiceCard.svelte`, `BalanceBody.svelte`
- 验证：`npm run check` + 手动

**Interfaces:**
- Consumes: `types.ts`, `theme.ts`
- Produces:
  - `ProgressBar.svelte`：props `pct`；7px 高，轨道 `rgba(255,255,255,0.12)`，填充 `barColor(pct)`，宽度按 `pct%`，圆角 4。
  - `ServiceCard.svelte`：props `snapshot`；复刻 `ServiceCardView`：色点+标题+plan 徽章 header；按 `status.kind` 渲染 loading/ok/stale/error；窗口型按 `display.fiveHour/weekly` 过滤，显示 `label`、`Int(pct)%`（色=barColor）、进度条、`resetCountdown`（若 `display.resetCountdown`）；ok 显示 `更新于 HH:mm`（若 `display.updatedAt`）；stale 显示 `⚠ 刷新失败,显示上次结果 · HH:mm`（琥珀）；error 显示红色文案。卡片样式：圆角 14、背景 `rgba(28,28,30,0.9)`、1px `rgba(255,255,255,0.08)` 描边、padding 16/14。
  - `BalanceBody.svelte`：复刻 `BalanceCardBody`：当前余额（大号 22，accent 色）+ 历史消耗（15，灰）；累计请求；模型区可展开（总开关 chevron + 数量），展开后按 `groupedModels` 二级分组、各组可单独展开、列表区限高滚动（约 260px）。金额 `{currency}{v.toFixed(2)}`。分组排序在前端用与 Rust 一致的比较（数量降序、vendor 次序）实现一个 `groupedModels(models)` 工具。

- [ ] **Step 1：实现三个组件**

颜色/字号/间距严格照搬本计划开头读到的 SwiftUI 常量（见 Theme/各 View）。

- [ ] **Step 2：验证** — Run: `cd tauri && npm run check`，Expected: 0 errors。

- [ ] **Step 3：Commit**

```bash
git add tauri/src/lib/components/ProgressBar.svelte tauri/src/lib/components/ServiceCard.svelte tauri/src/lib/components/BalanceBody.svelte
git commit -m "Add progress bar, service card and balance body components"
```

---

### Task 14：设置页组件

**Files:**
- Create: `tauri/src/lib/components/SettingsView.svelte`
- 验证：`npm run check` + 手动

**Interfaces:**
- Consumes: `store.ts`, `api.ts`, `types.ts`
- Produces: 复刻 `DisplaySettingsView`：
  - 告警面板：启用勾选框、阈值 slider（0–100 step 5，显示百分比）、规则二选一（跑赢时间进度 / 仅超过阈值）、冷却 stepper（分钟，1–240 step 5），改动即调对应命令。
  - 渠道商列表：每个 service 一行——启用勾选框（色点+标题+类别徽章）、上移/下移按钮（首/末禁用）、展示项网格（`DisplayContent.options(for category)`：subscription=套餐/5 小时/周额度/重置倒计时/更新时间；apiUsage=当前余额/历史消耗/请求次数/模型列表/更新时间），每项勾选框调 `setDisplayContent`。
  - 底部提示 `保存到 %APPDATA%\usage-bar\config.json`。

- [ ] **Step 1：实现组件**

展示项 key 与 `ServiceDisplayOptions` 字段映射：套餐=plan、5 小时=fiveHour、周额度=weekly、重置倒计时=resetCountdown、更新时间=updatedAt、当前余额=balance、历史消耗=used、请求次数=requestCount、模型列表=models。

- [ ] **Step 2：验证** — Run: `npm run check`，Expected: 0 errors。

- [ ] **Step 3：Commit**

```bash
git add tauri/src/lib/components/SettingsView.svelte
git commit -m "Add settings view with alerts and per-service display options"
```

---

### Task 15：根面板组装（+page.svelte）

**Files:**
- Modify: `tauri/src/routes/+page.svelte`（替换脚手架默认 demo 内容）
- 验证：`npm run tauri dev`（Windows，端到端）

> 注：SvelteKit 模板下根视图是 `src/routes/+page.svelte`（无 `src/App.svelte` / `src/main.ts` / `index.html`）；SPA 由 `+layout.ts` 的 `export const ssr = false` 保证。组件从 `$lib/components/...` 导入。

**Interfaces:**
- Consumes: store、所有组件
- Produces: header（仪表盘图标 + 标题"订阅用量"/"显示设置" + 设置/完成切换按钮）；正文 settings 时显示 `SettingsView`，否则卡片列表（空态"未选择任何渠道商"）+ footer（更新于 HH:mm + 刷新按钮[isRefreshing 转圈] + 退出按钮）；整体深色半透明背景，宽度 320（设置 360）。`onMount` 调 `store.init()`。

- [ ] **Step 1：实现 +page.svelte**

复刻 `PopoverRootView` 布局与文案；刷新按钮调 `refreshNow()`，退出按钮调 `quit()`。删除脚手架自带的 greet demo 代码。

- [ ] **Step 2：端到端手动验证（Windows，需真实凭证）**

前置：确保存在 `%USERPROFILE%\.claude\.credentials.json`（claude 已登录）；可选 `%USERPROFILE%\.codex\auth.json`；可选 `%APPDATA%\usage-bar\phanrouter.json`。

Run: `cd tauri && npm run tauri dev`
Expected（逐项确认并截图）：
1. 左键托盘弹面板：Claude/GPT 卡显示 5 小时/周进度条+百分比+重置倒计时；阈值色正确。
2. PhanRouter 卡显示余额/历史/请求数，点"模型"展开二级分组可逐个展开。
3. 点"设置"进入设置页：切换某服务 enabled、上移/下移、勾掉某展示项 → 面板实时变化，且 `%APPDATA%\usage-bar\config.json` 被写回（`type %APPDATA%\usage-bar\config.json` 确认）。
4. 断网或令牌失效时，卡片回退上次缓存并显示 ⚠；首次无缓存显示红色错误。
5. 把告警阈值调到很低 → 几秒内出现 Windows 通知，托盘图标变红色感叹号。

- [ ] **Step 3：Commit**

```bash
git add tauri/src/routes/+page.svelte
git commit -m "Assemble popover root view with header, cards and footer"
```

---

## Phase 4：打包与文档

### Task 16：Windows 打包与文档

**Files:**
- Modify: `tauri/src-tauri/tauri.conf.json`（bundle 配置）, `tauri/src-tauri/icons/icon.ico`
- Create: `tauri/README.md`
- Modify: 仓库根 `README.md`（加"Windows 版（Tauri）"小节）
- 验证：`npm run tauri build` 产出安装包

**Interfaces:**
- Produces: 可分发的 Windows 安装包（NSIS / MSI）；构建与使用文档。

- [ ] **Step 1：配置 bundle**

`tauri.conf.json` 的 `bundle`：`"active": true`、`"targets": ["nsis"]`、`"icon": ["icons/icon.ico","icons/tray.png"]`、`publisher`、`shortDescription`。生成 `icon.ico`（多尺寸）。

- [ ] **Step 2：构建**

Run: `cd "A:/文档/Projects/UsageDashboard/tauri" && npm run tauri build`
Expected: 在 `src-tauri/target/release/bundle/nsis/` 生成 `*-setup.exe`，退出码 0。

- [ ] **Step 3：写文档**

`tauri/README.md`：开发（`npm run tauri dev`）、构建（`npm run tauri build`）、配置/凭证路径（`%APPDATA%\usage-bar\`、`%USERPROFILE%\.claude\.credentials.json` 等）、与 mac 版差异（无 Keychain、托盘形态）。仓库根 `README.md` 加一节链接到 `tauri/README.md`，说明"macOS 用根目录 Swift 版、Windows 用 tauri/"。

- [ ] **Step 4：全量测试回归**

Run: `cargo test --manifest-path tauri/src-tauri/Cargo.toml`
Expected: 全部 PASS。

- [ ] **Step 5：Commit（代码 + 文档同一提交）**

```bash
git add tauri/src-tauri/tauri.conf.json tauri/src-tauri/icons/icon.ico tauri/README.md README.md
git commit -m "Add Windows bundling config and documentation"
```

---

## Self-Review 记录

- **Spec 覆盖**：架构(Task1,10,11) / 三取数器(Task8) / 凭证策略含手动录入(Task6,10 `save_new_api_credentials`) / 配置缓存路径(Task3,4,5) / 告警+通知+托盘红点(Task9,10,11) / 前端 UI 复刻(Task12-15) / 测试(Task2-10) / 打包文档(Task16)。移动端为非目标，未排任务（符合 spec）。
- **占位符**：核心 Rust 任务均含完整测试与实现要点；前端任务给出精确视觉常量与字段映射，无 TBD。
- **类型一致**：Rust 序列化 camelCase 与前端 `types.ts` 对齐；`FetcherKind::NewApi` 序列化为 `newAPI`；`ServiceStatus` 用 `kind` tag 前后端一致。
- **已知取舍**：凭证本期文件存储（非 Windows 凭据管理器），已在 Global Constraints 标注为后续增强。
