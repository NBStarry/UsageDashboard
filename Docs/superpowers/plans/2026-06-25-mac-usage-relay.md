# Mac 用量中转（usage-relay）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mac 的 Swift 菜单栏 app 通过 Tailscale 暴露三家用量快照 JSON；手机 Tauri app 以"中转模式"拉取并用现有卡片渲染，手机端零凭证/零代理/零 OAuth。

**Architecture:** Mac 端 `TokenUsageDashboard`（Swift，Sources/UsageBar/）新增一个 `NWListener` HTTP 服务，`GET /usage` 返回当前快照序列化的 JSON（形状 == 手机 `Vec<ServiceSnapshot>`）；手机端（tauri/，Rust + Svelte）新增 `relay` 配置与 RelayFetcher，刷新时拉 Mac 的 JSON 并替换本地快照，前端渲染逻辑不变。配置经二维码/手动粘贴一次性引导。

**Tech Stack:** Swift + Network.framework(NWListener) + CryptoKit（Mac）；Rust + reqwest + serde（手机后端）；SvelteKit/TypeScript（手机前端）。无新增第三方依赖。

## Global Constraints

- 契约 JSON 形状 == Rust `state.rs::ServiceSnapshot` 的 serde 输出：`[{ "config": {...camelCase...}, "status": { "kind": "ok|loading|stale|error", ... } }]`。详见各任务的 Interfaces。
- camelCase 字段命名（serde `rename_all="camelCase"`）；时间用 RFC3339（`fetchedAt`/`cachedAt`/`resetAt`）。
- Mac secret：32 字节随机，base64url；存 `~/.config/usage-bar/relay.json`（权限 0600）；绝不进日志。
- 端口默认 `8787`，可在 relay.json 配置。
- 手机 Rust 测试用 `USAGE_DASHBOARD_HOME` 临时目录隔离（沿用现有测试模式）；`#[serial]`。
- 提交信息：代码与文档同一 commit（用户全局规则）。
- 不删手机端已有本地凭证录入代码（保留为隐藏次要路径，默认走中转）。

---

## 契约：relay JSON（两端共用，先锁定）

Mac `GET /usage`（带 `Authorization: Bearer <secret>`）返回：

```json
{
  "ts": "2026-06-25T05:56:00Z",
  "services": [
    {
      "config": {
        "id": "claude", "title": "Claude", "accent": "#D97757",
        "category": "subscription", "fetcher": "claudeOauth",
        "credentialFile": null, "enabled": true,
        "display": { "plan": true, "fiveHour": true, "weekly": true,
          "resetCountdown": true, "updatedAt": true, "balance": true,
          "used": true, "requestCount": true, "models": true }
      },
      "status": {
        "kind": "ok",
        "usage": { "plan": null,
          "windows": [ {"label":"5 小时","pct":46,"resetAt":"2026-06-25T07:10:00Z"},
                       {"label":"周","pct":35,"resetAt":"2026-06-29T08:00:00Z"} ],
          "balance": null },
        "fetchedAt": "2026-06-25T05:56:00Z"
      }
    }
  ]
}
```

- `services[i]` 形状 == Rust `ServiceSnapshot`（`config` + `status`）。
- `status.kind` ∈ `ok`(usage+fetchedAt) / `stale`(usage+cachedAt+error) / `error`(message) / `loading`。
- 外层 `ts` = Mac 最后刷新时间（手机 lastUpdated 用）。

---

# Phase 1 — Mac RelayServer（Swift，Sources/UsageBar/）

> Phase 1 全部任务在 `feat/android-tauri` 分支的 `Sources/UsageBar/` 下。Swift 构建用 `swift build`，跑 app 用 `./build-app.sh`。

### Task 1: Relay 配置存储（secret + 端口）

**Files:**
- Create: `Sources/UsageBar/RelayConfigStore.swift`

**Interfaces:**
- Produces:
  - `struct RelaySettings: Codable { var port: Int; var secret: String }`
  - `enum RelayConfigStore { static func loadOrCreate() -> RelaySettings; static var fileURL: URL }`
  - 文件 `~/.config/usage-bar/relay.json`；首次生成 32 字节随机 secret（base64url）、端口默认 8787，写入并 chmod 0600。

- [ ] **Step 1: 写实现**

```swift
import Foundation
import CryptoKit

struct RelaySettings: Codable {
    var port: Int
    var secret: String
}

enum RelayConfigStore {
    static var fileURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/usage-bar/relay.json")
    }

    static func loadOrCreate() -> RelaySettings {
        let url = fileURL
        if let data = try? Data(contentsOf: url),
           let s = try? JSONDecoder().decode(RelaySettings.self, from: data) {
            return s
        }
        let secret = randomSecret()
        let s = RelaySettings(port: 8787, secret: secret)
        write(s)
        return s
    }

    private static func randomSecret() -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        return Data(bytes).base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    private static func write(_ s: RelaySettings) {
        let url = fileURL
        try? FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted]
        if let data = try? enc.encode(s) {
            try? data.write(to: url)
            try? FileManager.default.setAttributes(
                [.posixPermissions: 0o600], ofItemAtPath: url.path)
        }
    }
}
```

- [ ] **Step 2: 手测**

Run:
```bash
cd /Users/hanzebei/Documents/AI_Projects/UsageDashboard && swift build 2>&1 | tail -3
```
Expected: 编译通过。临时建个 main 调 `RelayConfigStore.loadOrCreate()` 或下个任务集成后验证文件生成 + 权限 `-rw-------`。

- [ ] **Step 3: Commit**

```bash
git add Sources/UsageBar/RelayConfigStore.swift
git commit -m "Mac relay: RelaySettings 存储(secret+端口,0600)"
```

---

### Task 2: 快照序列化为 relay JSON

**Files:**
- Create: `Sources/UsageBar/RelaySnapshot.swift`

**Interfaces:**
- Consumes: `ServiceRuntime`（`Sources/UsageBar/Models.swift`：`{ config: ServiceConfig, status: ServiceStatus }`），`ServiceStatus` 枚举（`.loading`/`.ok(Usage,fetchedAt:Date)`/`.stale(Usage,cachedAt:Date?,error)`/`.error(String)`），`Usage`/`UsageWindow{label,pct,resetAt:Date?}`/`BalanceInfo{balance,used,currency,requestCount:Int?,models:[ModelEntry{name,vendor}]}`，`ServiceConfig`（Codable，字段 id/title/accent/category/fetcher/credentialFile/enabled/display）。
- Produces: `func relayPayloadJSON(states: [ServiceRuntime], lastUpdated: Date?) -> Data` — 返回契约 JSON 的 `Data`。内部用 Codable 镜像结构 `RelayPayload/RelayService/RelayStatus/RelayUsage/...`，字段名严格 camelCase 对齐契约；`status` 用 `kind` 区分；ISO8601（带 `Z`）。

- [ ] **Step 1: 写实现**

```swift
import Foundation

// 契约镜像结构。字段名即 JSON 键(camelCase),与 Rust ServiceSnapshot 对齐。
private struct RelayPayload: Encodable { let ts: String?; let services: [RelayService] }
private struct RelayService: Encodable { let config: ServiceConfig; let status: RelayStatus }
private struct RelayWindow: Encodable { let label: String; let pct: Double; let resetAt: String? }
private struct RelayModel: Encodable { let name: String; let vendor: String }
private struct RelayBalance: Encodable {
    let balance: Double; let used: Double; let currency: String
    let requestCount: Int?; let models: [RelayModel]
}
private struct RelayUsage: Encodable {
    let plan: String?; let windows: [RelayWindow]; let balance: RelayBalance?
}
// status: { kind, usage?, fetchedAt?, cachedAt?, error?, message? }
private struct RelayStatus: Encodable {
    let kind: String
    let usage: RelayUsage?
    let fetchedAt: String?
    let cachedAt: String?
    let error: String?
    let message: String?
}

private let isoFmt: ISO8601DateFormatter = {
    let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime]; return f
}()
private func iso(_ d: Date?) -> String? { d.map { isoFmt.string(from: $0) } }

private func mapUsage(_ u: Usage) -> RelayUsage {
    RelayUsage(
        plan: u.plan,
        windows: u.windows.map { RelayWindow(label: $0.label, pct: $0.pct, resetAt: iso($0.resetAt)) },
        balance: u.balance.map { b in
            RelayBalance(balance: b.balance, used: b.used, currency: b.currency,
                         requestCount: b.requestCount,
                         models: b.models.map { RelayModel(name: $0.name, vendor: $0.vendor) })
        })
}

private func mapStatus(_ s: ServiceStatus) -> RelayStatus {
    switch s {
    case .loading:
        return RelayStatus(kind: "loading", usage: nil, fetchedAt: nil, cachedAt: nil, error: nil, message: nil)
    case .ok(let u, let at):
        return RelayStatus(kind: "ok", usage: mapUsage(u), fetchedAt: iso(at), cachedAt: nil, error: nil, message: nil)
    case .stale(let u, let cachedAt, let err):
        return RelayStatus(kind: "stale", usage: mapUsage(u), fetchedAt: nil, cachedAt: iso(cachedAt), error: err, message: nil)
    case .error(let msg):
        return RelayStatus(kind: "error", usage: nil, fetchedAt: nil, cachedAt: nil, error: nil, message: msg)
    }
}

func relayPayloadJSON(states: [ServiceRuntime], lastUpdated: Date?) -> Data {
    let payload = RelayPayload(
        ts: iso(lastUpdated),
        services: states.map { RelayService(config: $0.config, status: mapStatus($0.status)) })
    let enc = JSONEncoder()
    enc.outputFormatting = [.withoutEscapingSlashes]
    return (try? enc.encode(payload)) ?? Data("{\"services\":[]}".utf8)
}
```

> 注：若 `ServiceConfig` 的 Codable 输出字段与契约不符（如 category/fetcher 的 rawValue），在本任务内对 `ServiceConfig` 校验编码结果；不符则在此文件内改用显式 `RelayConfig` 镜像结构（字段与契约 JSON 的 `config` 块逐一对齐）而非直接编码 `ServiceConfig`。实现时以 Step 2 的实际 JSON 为准。

- [ ] **Step 2: 手测序列化输出**

临时在 `App.swift` 加一个隐藏 CLI 分支 `--relay-sample`（用模拟 states，复用 `renderReadmeScreenshots` 里的 `sampleStates` 构造方式）打印 `String(data: relayPayloadJSON(...), encoding: .utf8)`，跑：
```bash
swift build && .build/debug/TokenUsageDashboard --relay-sample | python3 -m json.tool
```
Expected: 输出含 `services[].config.id/title/accent/category/fetcher/credentialFile/enabled/display` 与 `services[].status.kind` + `usage.windows[].resetAt`(ISO `...Z`)，字段全 camelCase。核对 `category` 是 `"subscription"`/`"apiUsage"`、`fetcher` 是 `"claudeOauth"`/`"codexWham"`/`"newAPI"`（与手机 types.ts 一致）。不一致则按 Step 1 注释改 `RelayConfig` 镜像。

- [ ] **Step 3: Commit**

```bash
git add Sources/UsageBar/RelaySnapshot.swift Sources/UsageBar/App.swift
git commit -m "Mac relay: 快照序列化为契约 JSON(对齐 ServiceSnapshot 形状) + --relay-sample 调试"
```

---

### Task 3: HTTP 服务器（NWListener）

**Files:**
- Create: `Sources/UsageBar/RelayServer.swift`

**Interfaces:**
- Consumes: `relayPayloadJSON(states:lastUpdated:)`（Task 2）、`RelaySettings`（Task 1）。
- Produces: `final class RelayServer { init(settings: RelaySettings, snapshotProvider: @escaping () -> Data); func start(); func stop(); var boundAddressDescription: String { get } }`。`snapshotProvider` 是个闭包，返回当前 relay JSON（由 MenuBarController 注入，内部读 UsageStore 当前 states）。`GET /usage` 校验 `Authorization: Bearer <secret>` → 200 + JSON；缺/错 secret → 401；其它路径 → 404。监听 `0.0.0.0:port`（Tailscale 地址也在其中）。

- [ ] **Step 1: 写实现**

```swift
import Foundation
import Network

final class RelayServer {
    private let settings: RelaySettings
    private let snapshotProvider: () -> Data
    private var listener: NWListener?

    init(settings: RelaySettings, snapshotProvider: @escaping () -> Data) {
        self.settings = settings
        self.snapshotProvider = snapshotProvider
    }

    func start() {
        guard listener == nil else { return }
        let params = NWParameters.tcp
        params.allowLocalEndpointReuse = true
        guard let port = NWEndpoint.Port(rawValue: UInt16(settings.port)),
              let l = try? NWListener(using: params, on: port) else { return }
        l.newConnectionHandler = { [weak self] conn in self?.handle(conn) }
        l.start(queue: .global(qos: .utility))
        listener = l
    }

    func stop() { listener?.cancel(); listener = nil }

    private func handle(_ conn: NWConnection) {
        conn.start(queue: .global(qos: .utility))
        conn.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, _, _ in
            guard let self, let data, let req = String(data: data, encoding: .utf8) else {
                conn.cancel(); return
            }
            let response = self.route(req)
            conn.send(content: response, completion: .contentProcessed { _ in conn.cancel() })
        }
    }

    private func route(_ req: String) -> Data {
        let firstLine = req.split(separator: "\r\n", maxSplits: 1).first.map(String.init) ?? ""
        let parts = firstLine.split(separator: " ")
        let method = parts.count > 0 ? String(parts[0]) : ""
        let path = parts.count > 1 ? String(parts[1]) : ""
        let authed = req.range(of: "Authorization: Bearer \(settings.secret)") != nil
        if method == "GET" && path == "/usage" {
            if !authed { return httpResponse(401, "{\"error\":\"unauthorized\"}".data(using: .utf8)!) }
            return httpResponse(200, snapshotProvider())
        }
        return httpResponse(404, "{\"error\":\"not found\"}".data(using: .utf8)!)
    }

    private func httpResponse(_ code: Int, _ body: Data) -> Data {
        let reason = code == 200 ? "OK" : (code == 401 ? "Unauthorized" : "Not Found")
        let header = "HTTP/1.1 \(code) \(reason)\r\n"
            + "Content-Type: application/json; charset=utf-8\r\n"
            + "Content-Length: \(body.count)\r\n"
            + "Access-Control-Allow-Origin: *\r\n"
            + "Connection: close\r\n\r\n"
        return Data(header.utf8) + body
    }
}
```

- [ ] **Step 2: 手测（curl）**

临时在 `App.swift` 加 `--serve` CLI：`let s = RelayConfigStore.loadOrCreate(); let srv = RelayServer(settings: s, snapshotProvider: { relayPayloadJSON(states: sampleStates(), lastUpdated: Date()) }); srv.start(); print("listening \(s.port), secret \(s.secret)"); RunLoop.main.run()`。跑：
```bash
swift build && .build/debug/TokenUsageDashboard --serve &
SECRET=$(python3 -c "import json;print(json.load(open('$HOME/.config/usage-bar/relay.json'))['secret'])")
curl -s -H "Authorization: Bearer $SECRET" http://127.0.0.1:8787/usage | python3 -m json.tool | head
curl -s -o /dev/null -w "no-auth=%{http_code}\n" http://127.0.0.1:8787/usage
```
Expected: 带 secret → 200 + 契约 JSON；无 secret → 401。结束后 `kill %1`。

- [ ] **Step 3: Commit**

```bash
git add Sources/UsageBar/RelayServer.swift Sources/UsageBar/App.swift
git commit -m "Mac relay: NWListener HTTP 服务(GET /usage, Bearer 校验)"
```

---

### Task 4: 接入 app 生命周期 + 设置页显示地址/二维码

**Files:**
- Modify: `Sources/UsageBar/MenuBarController.swift`（持有 `RelayServer`，注入 `snapshotProvider` 读 `store` 当前 states + lastUpdated）
- Modify: `Sources/UsageBar/Views/DisplaySettingsView.swift`（新增"手机中转"面板：显示 `http://<tailscale-ip>:<port>` + secret + 二维码图）
- Create: `Sources/UsageBar/TailscaleAddress.swift`（取本机 100.64/10 地址）
- Create: `Sources/UsageBar/QRCode.swift`（`func qrImage(_ text: String) -> NSImage?` 用 `CIQRCodeGenerator`）

**Interfaces:**
- Consumes: `RelayServer`(Task 3)、`RelayConfigStore`(Task 1)、`UsageStore`（`store.states`、`store.lastUpdated`，见 UsageStore.swift）。
- Produces:
  - `enum TailscaleAddress { static func current() -> String? }` — 遍历 `getifaddrs` 找 `100.64.0.0/10` 的 IPv4；无则 nil。
  - `func qrImage(_ text: String) -> NSImage?`
  - 引导用 JSON 文本（二维码内容）：`{"url":"http://<ip>:<port>","secret":"<secret>"}`。

- [ ] **Step 1: TailscaleAddress 实现**

```swift
import Foundation

enum TailscaleAddress {
    // 取本机 Tailscale IPv4(100.64.0.0/10);取不到返回 nil。
    static func current() -> String? {
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddr) == 0, let first = ifaddr else { return nil }
        defer { freeifaddrs(ifaddr) }
        var ptr: UnsafeMutablePointer<ifaddrs>? = first
        while let p = ptr {
            let a = p.pointee.ifa_addr
            if a?.pointee.sa_family == UInt8(AF_INET) {
                var addr = sockaddr_in()
                memcpy(&addr, a, MemoryLayout<sockaddr_in>.size)
                let ip = String(cString: inet_ntoa(addr.sin_addr))
                if ip.hasPrefix("100.") {
                    let second = Int(ip.split(separator: ".")[1]) ?? 0
                    if (64...127).contains(second) { return ip }  // 100.64/10
                }
            }
            ptr = p.pointee.ifa_next
        }
        return nil
    }
}
```

- [ ] **Step 2: QRCode 实现**

```swift
import AppKit
import CoreImage

func qrImage(_ text: String) -> NSImage? {
    guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
    filter.setValue(Data(text.utf8), forKey: "inputMessage")
    filter.setValue("M", forKey: "inputCorrectionLevel")
    guard let ci = filter.outputImage?.transformed(by: .init(scaleX: 8, y: 8)) else { return nil }
    let rep = NSCIImageRep(ciImage: ci)
    let img = NSImage(size: rep.size); img.addRepresentation(rep); return img
}
```

- [ ] **Step 3: MenuBarController 起停服务器**

在 `MenuBarController.init` 末尾（`store` 已就绪后）注入并启动：

```swift
// 手机中转服务:GET /usage 返回当前快照。secret 见 ~/.config/usage-bar/relay.json。
let relaySettings = RelayConfigStore.loadOrCreate()
self.relayServer = RelayServer(settings: relaySettings, snapshotProvider: { [weak store] in
    let states = store?.states ?? []
    return relayPayloadJSON(states: states, lastUpdated: store?.lastUpdated)
})
self.relayServer?.start()
```
（在类里加 `private var relayServer: RelayServer?`。`store.states`/`store.lastUpdated` 已是 `@Published private(set)`，可读。）

- [ ] **Step 4: 设置页"手机中转"面板**

在 `DisplaySettingsView` 顶部加一段（仅显示，不需交互）：

```swift
// 手机中转:显示地址 + secret + 二维码,供手机扫码配置。
private var relayPanel: some View {
    let s = RelayConfigStore.loadOrCreate()
    let ip = TailscaleAddress.current() ?? "（未检测到 Tailscale）"
    let url = "http://\(ip):\(s.port)"
    let payload = "{\"url\":\"\(url)\",\"secret\":\"\(s.secret)\"}"
    return VStack(alignment: .leading, spacing: 8) {
        Text("手机中转").font(.system(size: 12, weight: .semibold)).foregroundColor(.white.opacity(0.92))
        Text(url).font(.system(size: 11)).foregroundColor(.white.opacity(0.8)).textSelection(.enabled)
        if let img = qrImage(payload) {
            Image(nsImage: img).resizable().interpolation(.none).frame(width: 140, height: 140)
        }
        Text("手机端扫码配置（需同一 Tailscale 网络）").font(.system(size: 10)).foregroundColor(.white.opacity(0.55))
    }
    .padding(10)
    .background(RoundedRectangle(cornerRadius: 8).fill(Color.black.opacity(0.24)))
}
```
并在 `body` 顶部插入 `relayPanel`。

- [ ] **Step 5: 构建运行验证**

Run:
```bash
cd /Users/hanzebei/Documents/AI_Projects/UsageDashboard && ./build-app.sh 2>&1 | tail -3
open TokenUsageDashboard.app
# 等几秒让取数完成,然后从另一终端:
SECRET=$(python3 -c "import json;print(json.load(open('$HOME/.config/usage-bar/relay.json'))['secret'])")
IP=$(ifconfig | grep -o '100\.[0-9]*\.[0-9]*\.[0-9]*' | head -1)
curl -s -H "Authorization: Bearer $SECRET" "http://$IP:8787/usage" | python3 -m json.tool | head -30
```
Expected: 返回真实三家用量快照（与菜单栏面板一致）。设置页能看到地址+二维码。

- [ ] **Step 6: Commit**

```bash
git add Sources/UsageBar/TailscaleAddress.swift Sources/UsageBar/QRCode.swift Sources/UsageBar/MenuBarController.swift Sources/UsageBar/Views/DisplaySettingsView.swift
git commit -m "Mac relay: 接入生命周期 + 设置页显示地址/二维码"
```

---

# Phase 2 — 手机 RelayFetcher（tauri/，Rust + Svelte）

> Phase 2 全部任务在 `tauri/` 下。Rust 测试 `cd tauri/src-tauri && cargo test`；前端 `cd tauri && npm run check`。真机构建见 README-android（需内联 export ANDROID_HOME/NDK_HOME，见 memory）。

### Task 5: AppConfig 增加 relay 字段（Rust + TS）

**Files:**
- Modify: `tauri/src-tauri/src/models.rs`（加 `RelayConfig` 结构 + `AppConfig.relay`）
- Modify: `tauri/src/lib/types.ts`（加 `RelayConfig` + `AppConfig.relay`）

**Interfaces:**
- Produces:
  - Rust: `pub struct RelayConfig { pub url: String, pub secret: String, #[serde(default)] pub enabled: bool }`（`#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)] #[serde(rename_all="camelCase")]`）；`AppConfig.relay: Option<RelayConfig>`（`#[serde(skip_serializing_if="Option::is_none")]`，`RawAppConfig` 加 `#[serde(default)] relay: Option<RelayConfig>`，`From` 里透传）。
  - TS: `export interface RelayConfig { url: string; secret: string; enabled: boolean }`；`AppConfig.relay?: RelayConfig | null`。

- [ ] **Step 1: 写失败测试**（models roundtrip）

`tauri/src-tauri/src/models.rs` 的 `#[cfg(test)]`（若无则新增）加：

```rust
#[test]
fn relay_config_roundtrips() {
    let json = r#"{"refreshSeconds":300,"alerts":{},"services":[{"id":"claude","title":"Claude"}],"relay":{"url":"http://100.64.0.1:8787","secret":"abc","enabled":true}}"#;
    let cfg: AppConfig = serde_json::from_str(json).unwrap();
    let r = cfg.relay.clone().unwrap();
    assert_eq!(r.url, "http://100.64.0.1:8787");
    assert!(r.enabled);
    let back = serde_json::to_string(&cfg).unwrap();
    assert!(back.contains("\"relay\""));
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd tauri/src-tauri && cargo test relay_config_roundtrips 2>&1 | tail -5`
Expected: 编译失败（`relay` 字段不存在）。

- [ ] **Step 3: 实现**

models.rs 加结构（放在 AppConfig 定义附近）：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayConfig {
    pub url: String,
    pub secret: String,
    #[serde(default)]
    pub enabled: bool,
}
```
`AppConfig` 加字段 `#[serde(skip_serializing_if = "Option::is_none")] pub relay: Option<RelayConfig>,`；`RawAppConfig` 加 `#[serde(default)] relay: Option<RelayConfig>,`；`From<RawAppConfig>` 里加 `relay: raw.relay,`；`Default` 里加 `relay: None,`。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd tauri/src-tauri && cargo test relay_config_roundtrips 2>&1 | tail -5`
Expected: PASS。

- [ ] **Step 5: 改 types.ts**

加 `export interface RelayConfig { url: string; secret: string; enabled: boolean }`，`AppConfig` 加 `relay?: RelayConfig | null;`。

- [ ] **Step 6: Commit**

```bash
git add tauri/src-tauri/src/models.rs tauri/src/lib/types.ts
git commit -m "手机 relay: AppConfig 增加 relay 配置(url/secret/enabled)"
```

---

### Task 6: RelayFetcher + 应用中转快照

**Files:**
- Modify: `tauri/src-tauri/src/fetchers.rs`（加 `fetch_relay`）
- Modify: `tauri/src-tauri/src/state.rs`（加 `relay_snapshots` 字段 + `apply_relay` + `snapshots()` 优先返回 relay）

**Interfaces:**
- Consumes: `RelayConfig`(Task 5)、契约 JSON、现有 `ServiceSnapshot`/`ServiceStatus`（state.rs，已是 `Deserialize`？→ 见 Step）。
- Produces:
  - `pub async fn fetchers::fetch_relay(relay: &RelayConfig) -> Result<RelayPayload, String>`，其中 `#[derive(Deserialize)] pub struct RelayPayload { pub ts: Option<String>, pub services: Vec<ServiceSnapshot> }`（放 state.rs，紧挨 ServiceSnapshot）。
  - `AppState.relay_snapshots: Mutex<Option<Vec<ServiceSnapshot>>>`。
  - `AppState::apply_relay(&self, services: Vec<ServiceSnapshot>)` — 存入 relay_snapshots。
  - `snapshots()` 改：若 relay 模式开启且 `relay_snapshots` 有值 → 直接返回它；否则走原逻辑。

> 关键：`ServiceSnapshot`/`ServiceStatus`/`Usage`/`UsageWindow`/`BalanceInfo`/`ServiceConfig` 目前是 `Serialize`（部分仅 Serialize）。本任务需给 `ServiceStatus` 及 state.rs 内仅 Serialize 的运行态结构补 `Deserialize`（`UsageWindow`/`BalanceInfo`/`ModelEntry`/`Usage` 在 models.rs 仅 `Serialize` → 补 `Deserialize`；`ServiceConfig` 已 `Deserialize`）。`ServiceStatus` 的 `#[serde(tag="kind", rename_all="camelCase")]` 同时支持反序列化。

- [ ] **Step 1: 写失败测试**（反序列化契约 JSON）

state.rs 测试模块加：

```rust
#[test]
#[serial]
fn deserialize_relay_payload() {
    let json = r#"{"ts":"2026-06-25T05:56:00Z","services":[
      {"config":{"id":"claude","title":"Claude","accent":"#D97757","category":"subscription","fetcher":"claudeOauth","credentialFile":null,"enabled":true,
        "display":{"plan":true,"fiveHour":true,"weekly":true,"resetCountdown":true,"updatedAt":true,"balance":true,"used":true,"requestCount":true,"models":true}},
       "status":{"kind":"ok","usage":{"plan":null,"windows":[{"label":"周","pct":35,"resetAt":null}],"balance":null},"fetchedAt":"2026-06-25T05:56:00Z"}}]}"#;
    let p: crate::fetchers::RelayPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.services.len(), 1);
    match &p.services[0].status {
        ServiceStatus::Ok { usage, .. } => assert_eq!(usage.windows[0].pct, 35.0),
        _ => panic!("expected ok"),
    }
}
```
（`RelayPayload` 实际放 state.rs；测试里路径按最终位置调整。）

- [ ] **Step 2: 跑测试确认失败**

Run: `cd tauri/src-tauri && cargo test deserialize_relay_payload 2>&1 | tail -8`
Expected: 编译失败（`RelayPayload` 不存在 / 缺 `Deserialize`）。

- [ ] **Step 3: 实现**

1) models.rs：给 `UsageWindow`/`ModelEntry`/`BalanceInfo`/`Usage` 的 derive 从 `Serialize` 改成 `Serialize, Deserialize`。
2) state.rs：`ServiceStatus` 的 derive 加 `Deserialize`；新增

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RelayPayload {
    pub ts: Option<String>,
    pub services: Vec<ServiceSnapshot>,
}
```
并给 `ServiceSnapshot` derive 加 `Deserialize`。
3) `AppState` 加字段 `pub relay_snapshots: Mutex<Option<Vec<ServiceSnapshot>>>,`（`new` 里初始化 `Mutex::new(None)`）。
4) 加方法：

```rust
pub fn apply_relay(&self, services: Vec<ServiceSnapshot>) {
    *self.relay_snapshots.lock().unwrap() = Some(services);
}

fn relay_enabled(&self) -> bool {
    self.config.lock().unwrap().relay.as_ref().map(|r| r.enabled).unwrap_or(false)
}
```
5) `snapshots()` 开头加：

```rust
if self.relay_enabled() {
    if let Some(s) = self.relay_snapshots.lock().unwrap().clone() {
        return s;
    }
}
```
6) fetchers.rs 加：

```rust
pub async fn fetch_relay(relay: &crate::models::RelayConfig) -> Result<crate::state::RelayPayload, String> {
    let url = format!("{}/usage", relay.url.trim_end_matches('/'));
    let client = build_client();  // 复用现有 reqwest client 构造(含 proxy);中转走局域网通常不需代理,但保持一致
    let resp = client.get(&url)
        .header("Authorization", format!("Bearer {}", relay.secret))
        .send().await.map_err(|e| format!("连接 Mac 中转失败:{e}"))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("中转密钥无效,请重新扫码".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("中转返回 HTTP {}", resp.status()));
    }
    resp.json::<crate::state::RelayPayload>().await.map_err(|e| format!("中转数据格式异常:{e}"))
}
```
> `build_client()`：若 fetchers.rs 现无此公共构造，则参照现有 `fetch_*` 里构造 reqwest client 的方式（带 `http::proxy()`）内联同样代码。中转默认不强制走代理——可直接 `reqwest::Client::new()`，但若 config 有 proxy 仍尊重（与现有一致即可）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd tauri/src-tauri && cargo test 2>&1 | tail -10`
Expected: 新测试 PASS，旧测试不破。

- [ ] **Step 5: Commit**

```bash
git add tauri/src-tauri/src/models.rs tauri/src-tauri/src/state.rs tauri/src-tauri/src/fetchers.rs
git commit -m "手机 relay: RelayFetcher + 反序列化契约 + relay 模式快照覆盖"
```

---

### Task 7: refresh 走中转分支 + 设置 relay 的命令

**Files:**
- Modify: `tauri/src-tauri/src/state.rs`（`refresh` 开头判断 relay 模式）
- Modify: `tauri/src-tauri/src/commands.rs`（加 `set_relay_config` 命令）
- Modify: `tauri/src-tauri/src/lib.rs`（注册命令）
- Modify: `tauri/src/lib/api.ts`（`setRelayConfig` 包装）

**Interfaces:**
- Consumes: `fetch_relay`(Task 6)、`apply_relay`(Task 6)、`commit_config`（commands.rs 已有）。
- Produces:
  - `refresh`：若 relay 启用 → `fetch_relay` 成功则 `apply_relay` 并 `emit("usage-updated")`；失败则保留上次（不清空）。relay 启用时**跳过**本地各服务取数。
  - `#[tauri::command] pub async fn set_relay_config(app, state, url: String, secret: String, enabled: bool) -> Result<(), String>` — 写入 `config.relay`，`commit_config`，随即 `refresh`。
  - `api.ts`: `setRelayConfig(url: string, secret: string, enabled: boolean): Promise<void>` → `invoke('set_relay_config', { url, secret, enabled })`。

- [ ] **Step 1: refresh 加 relay 分支**

`state.rs::refresh` 最前面加：

```rust
// 中转模式:从 Mac 拉快照,跳过本地取数。
let relay = self.config.lock().unwrap().relay.clone();
if let Some(r) = relay.filter(|r| r.enabled) {
    match fetchers::fetch_relay(&r).await {
        Ok(payload) => {
            self.apply_relay(payload.services);
            *self.last_updated.lock().unwrap() = Some(Utc::now());
            #[cfg(mobile)]
            {
                let snaps = self.snapshots();
                let lu = *self.last_updated.lock().unwrap();
                crate::widget::write_snapshot(&snaps, lu);
            }
            let _ = app.emit("usage-updated", ());
        }
        Err(_e) => {
            // 保留上次中转快照;仅更新时间不动。前端继续显示旧数据。
            let _ = app.emit("usage-updated", ());
        }
    }
    return;
}
```

- [ ] **Step 2: 加 set_relay_config 命令**

commands.rs：

```rust
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
```
lib.rs 的 `invoke_handler` 加 `commands::set_relay_config,`。

- [ ] **Step 3: api.ts 包装**

```ts
// commands.rs set_relay_config(url, secret, enabled)
export function setRelayConfig(url: string, secret: string, enabled: boolean): Promise<void> {
  return invoke('set_relay_config', { url, secret, enabled });
}
```

- [ ] **Step 4: 编译验证**

Run: `cd tauri/src-tauri && cargo build 2>&1 | tail -5 && cd .. && npm run check 2>&1 | tail -5`
Expected: Rust 编译通过；svelte-check 无新错误。

- [ ] **Step 5: Commit**

```bash
git add tauri/src-tauri/src/state.rs tauri/src-tauri/src/commands.rs tauri/src-tauri/src/lib.rs tauri/src/lib/api.ts
git commit -m "手机 relay: refresh 中转分支 + set_relay_config 命令"
```

---

### Task 8: 前端中转配置 UI（手动粘贴 url+secret）

**Files:**
- Modify: `tauri/src/lib/components/SettingsView.svelte`（新增"手机中转"面板：url + secret 输入 + 启用开关 + 保存）

**Interfaces:**
- Consumes: `setRelayConfig`(Task 7)、`$config.relay`（types.ts）。
- Produces: 设置页一个面板，预填 `$config.relay?.url`（secret 不预填，敏感），点"保存并连接"调 `setRelayConfig` → 成功后卡片显示 Mac 中转来的真实数据。

- [ ] **Step 1: 加面板**

在 `SettingsView.svelte` 的 HTTP 代理面板之后插入（仿其结构）：

```svelte
<!-- 手机中转:从 Mac 拉用量。url+secret 由 Mac 设置页二维码/文本提供 -->
<div class="panel">
  <span class="label-semibold" style="font-size:12px;color:rgba(255,255,255,0.96);">手机中转（从 Mac 取用量）</span>
  <div class="cred-form" style="margin-top:8px;">
    <input class="cred-input" type="text" placeholder="Mac 地址 http://100.x.x.x:8787" bind:value={relayUrl} />
    <input class="cred-input" type="text" placeholder="密钥 secret" bind:value={relaySecret} />
    <label style="display:flex;align-items:center;gap:6px;font-size:11px;color:rgba(255,255,255,0.8);">
      <input type="checkbox" bind:checked={relayEnabled} /> 启用中转模式
    </label>
    <div class="cred-actions">
      <button class="cred-save" disabled={relaySaving} onclick={onSaveRelay}>
        {relaySaving ? '连接中…' : '保存并连接'}
      </button>
      {#if relayMsg}<span class="cred-msg">{relayMsg}</span>{/if}
    </div>
    <span class="hint">需手机与 Mac 在同一 Tailscale 网络。启用后手机直接显示 Mac 算好的用量，不再本地取数。</span>
  </div>
</div>
```
`<script>` 顶部加 state 与 handler：

```ts
import { setRelayConfig } from '$lib/api';
let relayUrl = $state(''); let relaySecret = $state(''); let relayEnabled = $state(true);
let relaySaving = $state(false); let relayMsg = $state('');
let relaySynced = false;
$effect(() => {
  if (!relaySynced && $config) {
    relayUrl = $config.relay?.url ?? '';
    relayEnabled = $config.relay?.enabled ?? false;
    relaySynced = true;
  }
});
async function onSaveRelay() {
  relaySaving = true; relayMsg = '';
  try {
    await setRelayConfig(relayUrl.trim(), relaySecret.trim(), relayEnabled);
    relayMsg = relayEnabled ? '已连接,正在拉取…' : '已保存';
  } catch (e) { relayMsg = `失败:${e}`; }
  finally { relaySaving = false; }
}
```

- [ ] **Step 2: 检查**

Run: `cd tauri && npm run check 2>&1 | tail -5`
Expected: 无新 svelte-check 错误。

- [ ] **Step 3: 端到端真机验证**

构建装机（见 README-android），手机设置页填 Mac 的 url+secret（从 Mac 设置页读）→ 启用 → 保存。预期手机三张卡片显示 Mac 算好的真实用量。也可先用 WebView DevTools `invoke('set_relay_config',{...})` 验证（见 memory 调试手法）。

- [ ] **Step 4: Commit**

```bash
git add tauri/src/lib/components/SettingsView.svelte
git commit -m "手机 relay: 设置页中转配置(url/secret/启用) + 保存连接"
```

---

# Phase 3 —（可选增强）手机扫码配置

> v1 核心已可用（手动粘贴）。本阶段把"扫 Mac 二维码"做成原生扫码，免手输。非必须，可后做。

### Task 9: 原生扫码 Activity 写入 relay 配置

**Files:**
- Create: `tauri/src-tauri/gen/android/app/src/main/java/app/usagedashboard/ScanActivity.kt`（CameraX + ML Kit barcode，或复用 `WebLoginActivity` 的 intent 模式）
- Modify: `commands.rs` 加 `#[cfg(target_os="android")] start_relay_scan()`（JNI 启动 ScanActivity，回填 relay 配置，参照 `login_new_api` 模式）
- Modify: `SettingsView.svelte` 中转面板加"📷 扫码配置"按钮

**Interfaces:**
- Consumes: 现有 `login_new_api` 的 JNI 启动模式（commands.rs）、ScanActivity 扫到的 JSON `{url,secret}`。
- Produces: 扫码成功 → 原生侧解析 JSON → 经已注入的 ndk_context 写 `config.relay` 或回前端调 `setRelayConfig`。

- [ ] **Step 1–N:** 参照 `WebLoginActivity.kt` 与 `login_new_api`（commands.rs）的现有 JNI + action-intent 模式实现 ScanActivity，扫到二维码 JSON 后写入 relay 配置并 refresh。AndroidManifest 注册 Activity + CAMERA 权限。依赖：`androidx.camera` + `com.google.mlkit:barcode-scanning`（在 `gen/android/app/build.gradle.kts` 加）。

> 此任务较重且非必须；实现前先确认 v1（手动粘贴）端到端已通。详细 TDD 步骤待 Phase 1/2 完成、确认要做时再展开（届时按本仓 `WebLoginActivity` 的真实代码补全）。

- [ ] **Commit**

```bash
git add tauri/src-tauri/gen/android/app/src/main/java/app/usagedashboard/ScanActivity.kt tauri/src-tauri/src/commands.rs tauri/src/lib/components/SettingsView.svelte tauri/src-tauri/gen/android/app/build.gradle.kts
git commit -m "手机 relay: 原生扫码配置中转(可选增强)"
```

---

## 自检（spec 覆盖）

- Mac HTTP 端点返回快照 JSON → Task 2/3/4 ✅
- 手机中转模式拉取渲染 → Task 5/6/7/8 ✅
- 二维码引导 → Mac 端 Task 4（生成+显示）✅；手机端 Task 8（手动粘贴，v1）+ Task 9（扫码，可选）✅
- 错误处理（Mac 不可达回退 stale / 401 / 解析失败）→ Task 6（fetch_relay 错误信息）+ Task 7（refresh 失败保留上次）✅
- 安全（secret/Bearer/Tailscale 绑定）→ Task 1/3/4 ✅
- 契约 JSON 形状一致 → 顶部契约 + Task 2（Mac 产）/Task 6（手机消费）双向锁定 ✅
- 非目标（不删本地凭证、不做 push、不做 Windows server）→ 计划未触及，符合 ✅
