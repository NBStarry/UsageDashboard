# TokenUsageDashboard — 订阅用量菜单栏 App

macOS 原生菜单栏 App(Swift + SwiftUI,无 Dock 图标),实时显示订阅号和 API 渠道用量。
取代了旧的 Übersicht 桌面看板(已移除)。

## 界面预览

| 用量面板 | 显示设置 |
| --- | --- |
| ![TokenUsageDashboard 用量面板](Docs/Images/token-usage-dashboard-popover.png) | ![TokenUsageDashboard 显示设置](Docs/Images/token-usage-dashboard-settings.png) |

## 功能
- 菜单栏图标,点击弹出面板:每个服务一张卡,显示 5 小时 / 周窗口的进度条 + 百分比 + 重置倒计时。
- 进度条阈值色:<75% 绿 / 75–90% 琥珀 / ≥90% 红。
- 后台每 5 分钟自动刷新;面板内有手动刷新按钮。
- 取数失败时回退上次缓存并标 ⚠;从未成功过才显示纯错误。
- 弹窗齿轮设置:选择展示哪些渠道商卡片、调整卡片顺序、勾选每张卡展示内容。
- 订阅号告警:**5 小时**与**周额度**两个窗口各自独立设置阈值与规则(“跑赢时间进度”/“仅超过阈值”),超阈值触发 macOS 通知。菜单栏角标按严重度着色——5 小时触发为**红色**、仅周额度触发为**橙色**,角标取当前最高严重度。
- 右键菜单:立即刷新 / 开机自启(`SMAppService`)/ 退出。

## 取数
纯 Swift + URLSession,凭证只读不外泄。当前抽象为两类卡片:
- **订阅号**:Claude / GPT 这类官方订阅窗口,展示 5 小时 / 周用量窗口。
- **API 用量**:New-API 兼容网关,展示余额、历史消耗、请求次数和模型列表。

内置取数器:
- **Claude**:钥匙串 `Claude Code-credentials` → 回退 `~/.claude/.credentials.json`,调 `api.anthropic.com/api/oauth/usage`。窗口型(5 小时 / 周百分比)。
- **GPT**:`~/.codex/auth.json`(access_token + account_id),调 `chatgpt.com/backend-api/wham/usage`。窗口型。
- **PhanRouter**(New-API 网关):凭证 `~/.config/usage-bar/phanrouter.json`,调 `<baseUrl>/api/user/self`(余额 + 历史消耗,需 `Authorization: Bearer <accessToken>` + `New-Api-User: <userId>`)和 `<baseUrl>/api/pricing`(模型广场,公开)。余额型(当前余额 / 历史消耗 / 模型按来源分类,二级分组可逐个展开收起)。

PhanRouter 的 `accessToken` 是 **系统访问令牌**(个人设置→生成,不是「令牌管理」里的 `sk-` 中转令牌)。`quota`/`used_quota` 除以 `quotaPerUnit`(默认 500000)得到货币金额。

缓存目录 `~/.cache/usage-dashboard/`(与旧 Python 版兼容)。

## 配置
首次启动自动生成 `~/.config/usage-bar/config.json`,可改显示哪些卡片、顺序、颜色、刷新间隔:
```json
{
  "refreshSeconds": 300,
  "alerts": {
    "enabled": true,
    "cooldownSeconds": 1800,
    "fiveHour": {"enabled": true, "threshold": 60, "rule": "usageExceedsElapsedWindowPercent", "paceMultiplier": 1},
    "weekly":   {"enabled": true, "threshold": 80, "rule": "usageExceedsThresholdOnly", "paceMultiplier": 1}
  },
  "services": [
    {"id": "claude", "title": "Claude", "accent": "#D97757", "category": "subscription", "fetcher": "claudeOAuth", "enabled": true},
    {"id": "codex", "title": "GPT", "accent": "#10A37F", "category": "subscription", "fetcher": "codexWham", "enabled": true},
    {"id": "phanrouter", "title": "PhanRouter", "accent": "#7C5CFC", "category": "apiUsage", "fetcher": "newAPI", "credentialFile": "phanrouter.json", "enabled": true}
  ]
}
```
应用内设置会把告警开关、各窗口阈值与规则、冷却时间、卡片开关、顺序和 `display` 展示项写回该文件。

告警只作用于 `category: "subscription"` 的服务。`alerts.fiveHour` 与 `alerts.weekly` 分别配置 5 小时窗口和周窗口,各自独立的 `enabled` / `threshold`(阈值百分比)/ `rule` / `paceMultiplier`:`usageExceedsElapsedWindowPercent` 表示用量超过阈值后,再判断用量百分比是否高于当前窗口已流逝时间百分比(乘 `paceMultiplier`);`usageExceedsThresholdOnly` 表示只要超过阈值就报警。严重度固定为 5 小时=红、周=橙,菜单栏角标取当前最高严重度。`cooldownSeconds` 用于避免同一窗口反复提醒。

> 兼容旧配置:旧的单一 `minimumUsagePercent` / `rule` / `paceMultiplier` / `windows` 字段在加载时自动迁移——5 小时窗口沿用旧阈值与旧规则,周窗口套用新默认(80% / 仅超过阈值);下次保存后写回为上面的新结构。

New-API 兼容渠道的凭证单独放 `~/.config/usage-bar/<credentialFile>`:
```json
{"baseUrl": "https://example.com/new-api", "accessToken": "<系统访问令牌>", "userId": 0, "quotaPerUnit": 500000, "currency": "$"}
```

新增一个 New-API 兼容 API 渠道时,在 `services` 里追加:
```json
{"id": "mygateway", "title": "MyGateway", "accent": "#2F80ED", "category": "apiUsage", "fetcher": "newAPI", "credentialFile": "mygateway.json", "enabled": true}
```
再创建 `~/.config/usage-bar/mygateway.json`。订阅号接口差异较大,新增订阅号通常需要在代码里添加一个专用 `UsageFetcher`,但 UI 卡片可复用 `subscription` 类型。

## Windows 版（Tauri）

Windows 系统托盘版使用 Tauri 2 + SvelteKit + Rust 实现，与 macOS Swift 版功能对等。
源码位于 [`tauri/`](tauri/) 目录，构建产物为 NSIS 安装包（`.exe`）。

详见 [tauri/README.md](tauri/README.md)，涵盖：开发启动、发布构建、配置/凭证路径（`%APPDATA%\usage-bar\`、`%USERPROFILE%\.claude\.credentials.json` 等）以及与本 macOS 版的差异说明。

## 图标
`AppIcon.icns` 由项目根的源图生成,`build-app.sh` 会自动嵌入 bundle。
重做图标:改 `make_icon.swift` 风格脚本 → 生成 1024 PNG → `sips` 切尺寸 → `iconutil -c icns`。

## 构建与安装
```bash
./build-app.sh                              # 编译 + 组装 TokenUsageDashboard.app(含图标) + ad-hoc 签名
open TokenUsageDashboard.app                # 运行
cp -r TokenUsageDashboard.app /Applications/  # 建议:开机自启需 App 在稳定路径
```

## 调试
无头验证取数层(不启动 GUI):
```bash
.build/release/TokenUsageDashboard --fetch claude
.build/release/TokenUsageDashboard --fetch codex
.build/release/TokenUsageDashboard --fetch phanrouter
.build/release/TokenUsageDashboard --fetch mygateway  # 配置中的自定义服务 id
.build/release/TokenUsageDashboard --render-readme    # 重新生成 README 界面图
.build/release/TokenUsageDashboard --dump-alerts      # 打印解析后的告警配置(含旧格式迁移,只读)
```
