# Mac 用量中转（usage-relay over Tailscale）设计

日期：2026-06-25
分支：`feat/android-tauri`
状态：已与用户确认，待写实现计划

## 背景与动机

手机版（Tauri Android）要显示 Claude / GPT / PhanRouter 的订阅用量。手机直连官方接口（方案 A）有两大拦路虎，经实测确认：

1. **token 全会过期**：Claude/Codex 是 OAuth 轮换式 token，手机端没有可行的自动刷新——「手机独立 OAuth 登录（A1）」的桌面 spike 失败（consent 提交稳定报 `Invalid request format`，且 `platform.claude.com` token 端点被 GFW 按 SNI 阻断）。详见 memory `android-tauri-effort`。
2. **GFW**：Claude/GPT 接口在国内需代理。

而 **Mac 的 Swift 菜单栏 app（`TokenUsageDashboard`）本来就在正常工作**：它读 macOS 钥匙串里 Claude Code 自动维护的新鲜订阅 token、读 `~/.codex/auth.json`、读 `~/.config/usage-bar/phanrouter.json`，每 5 分钟取好三家用量并缓存。token 刷新、GFW 代理、取数全在 Mac 端解决。

用户有 **Tailscale**：Mac 和手机在同一 tailnet，私有加密网络，**任何地方都可达**（不限同一 WiFi）。

**核心思路**：让 Mac 把算好的用量快照通过 Tailscale 暴露，手机变成纯显示端拉取渲染。手机零凭证、零代理、零 OAuth。

## 目标 / 非目标

**目标（v1）**
- Mac Swift app 暴露一个 HTTP 端点，返回当前三家用量快照 JSON。
- 手机 Tauri app 新增「中转模式」：从 Mac 的 Tailscale 地址拉快照并用现有卡片渲染。
- 二维码一次性引导手机配置中转地址 + 密钥。

**非目标（v1，YAGNI）**
- 不做服务端 push（手机按现有刷新周期 pull 即可）。
- 不删除手机端已有的本地凭证录入代码（保留为隐藏/次要路径，默认走中转）。
- 不做 Windows 端的 server（Mac 是唯一有钥匙串新鲜 token 的宿主；Windows 版另议）。
- 不做多用户/多 Mac、不做公网暴露（仅 tailnet 内）。

## 架构

```
┌─────────────── Mac (Swift TokenUsageDashboard) ───────────────┐
│  现有: Fetcher(claude/codex/newAPI) → UsageStore 缓存快照       │
│  新增: RelayServer  —— GET /usage  返回快照 JSON (Bearer 校验)  │
│         绑定 Tailscale 接口, 固定端口 (默认 8787)               │
└───────────────────────────┬───────────────────────────────────┘
                            │ Tailscale (100.x / MagicDNS, 端到端加密)
                            ▼
┌─────────────── 手机 (Tauri Android) ──────────────────────────┐
│  新增: RelayFetcher —— GET https://<mac-ts>:8787/usage         │
│        + Bearer secret → 解析 → 复用现有 Svelte 卡片渲染        │
│  配置: relay { url, secret } 存 config(沙盒)                    │
│  引导: 扫 Mac 生成的二维码一次性写入 relay 配置                 │
└───────────────────────────────────────────────────────────────┘
```

## 组件（单一职责）

### 1. Mac 端 `RelayServer`（Swift，新增）
- **做什么**：起一个最小 HTTP 服务器，`GET /usage` 返回 `UsageStore` 当前快照序列化的 JSON；校验 `Authorization: Bearer <secret>`，不匹配返回 401。
- **怎么用**：app 启动时随 `UsageStore` 一起拉起；可在应用内设置里开关「中转服务」并显示当前 Tailscale 地址 + 二维码。
- **依赖**：`UsageStore`（取现成快照）、`Network`/`NWListener`（Swift 原生 HTTP，无三方依赖）、一个持久化的 secret。
- **绑定**：优先绑定 Tailscale 接口地址（`100.64.0.0/10` 网段的本机 IP）；取不到则绑 `0.0.0.0` 但仅靠 secret + tailnet 防护。端口默认 `8787`，可配置。
- **secret**：首次启动随机生成（32 字节 base64），存 `~/.config/usage-bar/relay.json`，写进二维码。

### 2. 手机端 `RelayFetcher`（Rust/Tauri，新增）
- **做什么**：实现一个新的取数源——按现有刷新周期 `GET <relayUrl>/usage` 带 Bearer，解析返回的快照 JSON 成手机的 `ServiceSnapshot` 列表。
- **怎么用**：当 config 里存在 `relay { url, secret }` 且「中转模式」开启时，`refresh` 走 RelayFetcher 取代本地各 Fetcher。
- **依赖**：`reqwest`（已有）、config 里的 relay 字段、现有 `ServiceSnapshot/Usage` 模型与 Svelte 卡片（**几乎不改渲染**）。
- **失败回退**：拉取失败 → 复用现有 stale 缓存机制，显示上次快照 + ⚠。

### 3. 二维码引导
- **Mac**：设置页展示二维码，编码 JSON `{ "url": "https://<mac-ts-host>:8787", "secret": "<secret>" }`（用 Tailscale MagicDNS 主机名优先，回退 100.x IP）。
- **手机**：app 内「扫码配置中转」→ 调系统相机/扫码 → 解析写入 config 的 `relay` 字段 → 立即 `refresh` 验证。

## 数据流

1. Mac：定时器每 `refreshSeconds`（默认 300）取三家用量 → 写缓存（**现有行为，不改**）。
2. 手机：刷新时（定时器或手动）→ `GET /usage` → 收到快照 → 渲染。
3. **快照 JSON schema**（Mac 序列化，手机反序列化，对齐手机现有模型）：

```json
{
  "ts": "2026-06-25T13:56:00Z",
  "services": [
    {
      "id": "claude", "title": "Claude", "accent": "#D97757",
      "category": "subscription",
      "status": "ok",                       // ok | stale | error
      "error": null,
      "usage": {
        "plan": null,
        "windows": [
          {"label": "5 小时", "pct": 46, "resetAt": "2026-06-25T07:10:00Z"},
          {"label": "周",     "pct": 35, "resetAt": "2026-06-29T08:00:00Z"}
        ],
        "balance": null
      }
    },
    {
      "id": "phanrouter", "title": "PhanRouter", "accent": "#7C5CFC",
      "category": "apiUsage", "status": "error",
      "error": "PhanRouter 访问令牌无效,请重新生成",
      "usage": null
    }
  ]
}
```

字段命名用 camelCase（与手机前端一致）。`resetAt` 用 ISO8601。每服务带独立 `status`/`error`，单个失败不影响其他卡片。

## 错误处理

- **Mac 不可达**（关机 / Tailscale 断 / 端口未开）：手机 `reqwest` 超时 → 回退上次缓存快照 + ⚠ stale（app 已有机制）；从未成功过则显示「未连接到 Mac 中转」。
- **401**（secret 不符）：手机显示「中转密钥无效，请重新扫码」。
- **Mac 端某服务取数失败**：该服务在快照里 `status:"error"`，手机对应卡片显示错误，其余正常。
- **JSON 解析失败**（schema 不匹配）：手机显示「中转数据格式异常」并保留缓存。

## 安全

- **传输**：Tailscale 端到端加密（WireGuard），仅 tailnet 成员可达。
- **鉴权**：随机 secret 作 Bearer；Mac 端对所有路径强制校验。
- **绑定**：尽量绑 Tailscale 接口，避免监听公网/其它局域网；即便绑 0.0.0.0，secret + tailnet 双重防护。
- **secret 存储**：Mac `~/.config/usage-bar/relay.json`（600 权限）；手机 app 沙盒 config。
- 不在日志里打印 secret 或 token。

## 测试策略

- **Mac RelayServer 单测/手测**：`curl -H "Authorization: Bearer <secret>" http://<ts-ip>:8787/usage` 返回合法快照 JSON；错误 secret 返回 401；服务关闭时连接被拒。
- **手机 RelayFetcher**：用 mock/真 Mac 端点验证解析；断网回退 stale；401 提示。
- **端到端真机验证**：手机扫码配置 → 拉到三家真实用量并渲染（对照 Mac 菜单栏显示一致）。
- **复用既有 CLI**：Swift 侧可加 `--serve` 调试模式打印监听地址；Tauri 侧用 WebView DevTools `invoke` 直接测 RelayFetcher（见 memory 的调试手法）。

## 开放问题（实现时定）

- Tailscale 接口 IP 的获取方式（遍历 `getifaddr` 找 `100.64/10`，或读 `tailscale ip -4`）。
- 端口冲突时的处理（8787 被占则提示或自增）。
- 二维码在 Swift 端的生成（`CIQRCodeGenerator`）与手机端扫码（系统相机 intent / WebView 里的扫码库）。
- 手机端「中转模式」与「本地凭证模式」的切换 UI（默认中转）。
