# Windows / 跨平台兼容设计（Tauri 2）

- 日期：2026-06-23
- 状态：已确认，待写实现计划
- 本期交付：Windows 桌面托盘版（移动端仅做架构预留）

## 背景与目标

现有 `TokenUsageDashboard` 是 macOS 原生菜单栏应用（Swift 6 + SwiftUI + AppKit），
实时展示订阅号（Claude / GPT）和 API 网关（New-API 兼容）的用量。其取数逻辑是纯
Swift/Foundation，可移植；但整个 UI 层（NSStatusBar 菜单栏、SwiftUI、UserNotifications、
ServiceManagement 开机自启、Keychain）都是 Apple 专有，无法在 Windows 运行。

目标：用 **Tauri 2** 新建一套跨平台技术栈，实现 Windows 桌面托盘版，并按"将来可扩展到
iOS + Android"来设计架构。**现有 macOS Swift 应用原样保留**，本期不迁移 mac。

### 决策记录

- 路线：跨平台重写（不在 Swift 上硬移植 UI）。
- 技术栈：**Tauri 2（Rust 后端 + Web 前端）**。
- 移动端：iOS + Android 都是长期目标，但**本期不落地**，只做架构预留。
- mac：保留现有 Swift 版，新栈本期只交付 Windows。
- 取数逻辑放 **Rust 后端**（避开 webview CORS、token 不进前端、桌面与移动共用）。

## 总体架构

```
仓库根/
├── (现有 macOS Swift 应用，原样保留)
└── tauri/                    ← 新增跨平台项目
    ├── src/                  ← Web 前端（UI）
    └── src-tauri/            ← Rust 后端（托盘/取数/配置/告警）
```

- **Rust 后端**：托盘图标（普通态 / 告警红色感叹号态）、托盘右键菜单（立即刷新 /
  开机自启切换 / 退出）、点击托盘弹出的无边框 popover 窗口（失焦自动隐藏）、后台定时刷新、
  取数、配置读写、缓存、告警判定 + 通知。
- **Web 前端**：popover 面板里的服务卡片、进度条、设置页——高度复刻现有视觉。
- 通信：前端通过 Tauri `invoke` 调用 Rust 命令；Rust 通过事件（如 `usage-updated`）
  推送刷新结果，前端监听后重渲染。

### 关键 Rust 命令（初稿）

- `fetch_all()` / `fetch_one(id)` — 触发取数，返回各服务用量快照。
- `get_config()` / `save_config(config)` — 读写配置。
- 凭证相关命令（见"凭证策略"）。

## 取数器（Rust 移植现有三个）

用 reqwest 照搬现有逻辑，数据模型与窗口/余额语义保持一致：

- `claudeOAuth`：读凭证 → 调 `api.anthropic.com/api/oauth/usage`，**窗口型**
  （5 小时 / 周百分比 + 重置倒计时）。
- `codexWham`：读 `auth.json`（access_token + account_id）→ 调
  `chatgpt.com/backend-api/wham/usage`，**窗口型**。
- `newAPI`：`<baseUrl>/api/user/self`（余额 + 历史消耗，需
  `Authorization: Bearer <accessToken>` + `New-Api-User: <userId>`）+
  `<baseUrl>/api/pricing`（模型广场，公开），**余额型**（当前余额 / 历史消耗 /
  模型按来源二级分组可展开收起）。`quota`/`used_quota` 除以 `quotaPerUnit`（默认 500000）。

订阅号接口差异大，新增订阅号通常需在 Rust 里加专用取数器；API 网关可复用 `newAPI`。

## 凭证策略（跨平台最大差异点）

- **Windows 桌面自动读取**：`%USERPROFILE%\.claude\.credentials.json`、
  `%USERPROFILE%\.codex\auth.json`（CLI 在 Windows 下也是这些路径）；New-API 凭证放
  配置目录 `%APPDATA%\usage-bar\<credentialFile>`。
- **手动录入兜底**（移动端唯一方式）：设置里提供凭证录入页。桌面存 Windows 凭据管理器，
  移动端用安全存储插件。
- 用一层 `CredentialProvider` 抽象隔离平台差异，移动端将来只需换实现。
- 本期 Windows：以自动读取为主，手动录入作为缺省兜底实现（至少覆盖 New-API，
  Claude/GPT 在文件缺失时给出提示）。

## 配置与缓存（Windows 路径）

- 配置：`%APPDATA%\usage-bar\config.json`，结构与现有完全一致：

```json
{
  "refreshSeconds": 300,
  "alerts": {
    "enabled": true,
    "minimumUsagePercent": 60,
    "rule": "usageExceedsElapsedWindowPercent",
    "paceMultiplier": 1,
    "cooldownSeconds": 1800
  },
  "services": [
    {"id": "claude", "title": "Claude", "accent": "#D97757", "category": "subscription", "fetcher": "claudeOAuth", "enabled": true},
    {"id": "codex", "title": "GPT", "accent": "#10A37F", "category": "subscription", "fetcher": "codexWham", "enabled": true},
    {"id": "phanrouter", "title": "PhanRouter", "accent": "#7C5CFC", "category": "apiUsage", "fetcher": "newAPI", "credentialFile": "phanrouter.json", "enabled": true}
  ]
}
```

- 缓存：`%LOCALAPPDATA%\usage-dashboard\`。沿用回退策略：取数失败回退上次缓存并标 ⚠；
  从未成功过才显示纯错误。
- 路径用一层抽象封装（desktop 用 OS 目录，移动端用 app 数据目录）。

## 告警与系统集成

- 告警判定在 Rust，沿用规则：`usageExceedsElapsedWindowPercent`（超阈值后再比用量百分比
  是否高于窗口已流逝时间百分比）/ `usageExceedsThresholdOnly`（超阈值即报）+
  `cooldownSeconds` 冷却。告警只作用于 `category: "subscription"`。
- 触发 **Tauri 通知插件**；同时托盘图标切红色感叹号态。
- 开机自启：`tauri-plugin-autostart`。

## 前端 UI（复刻现有）

- 订阅卡：5h / 周进度条 + 百分比 + 重置倒计时。
- API 卡：当前余额 / 历史消耗 / 请求次数 / 模型按来源二级分组（可逐个展开收起）。
- 进度条阈值色：<75% 绿 / 75–90% 琥珀 / ≥90% 红。
- 齿轮设置：选择展示哪些卡片、调整顺序、每卡展示项、告警开关/阈值/规则/冷却、刷新间隔，
  全部写回 `config.json`。
- 框架：建议 **Svelte 或纯 TS**（包小、够用），具体在实现计划阶段定。
- 主题/配色沿用现有 accent 色与阈值色。

## 移动端预留（本期不实现）

手机端无托盘形态：

- popover 面板将来直接作为 App 主屏。
- 告警走本地 / 推送通知。
- 凭证只能手动录入。
- 因此本期把 UI 组件、取数器、数据模型都做成平台无关；移动端将来主要是加 iOS/Android
  target + 一个引导/凭证录入页 + 凭证/路径抽象的移动实现。

## 错误处理

- 取数失败 → 回退上次缓存并标 ⚠；从未成功 → 纯错误态。
- 凭证文件缺失 → 卡片给出可读提示，引导手动录入。
- 网络/解析错误分别可读化，不泄露 token / account_id / 原始响应。

## 测试

Rust 单测覆盖：

- 三个取数器的响应解析（含窗口型/余额型边界）。
- 配置默认值与读写往返。
- 缓存回退逻辑（失败回退、从未成功）。
- 告警判定逻辑（两种规则 + 冷却）。

## 非目标（本期）

- 不迁移 macOS（Swift 版保留）。
- 不实现 iOS / Android 落地（仅架构预留）。
- 不做应用商店上架 / 签名分发流程。
- 不做无关重构。
