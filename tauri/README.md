# TokenDeck — Windows (Tauri)

Windows system-tray app for monitoring Claude, GPT, and New-API subscription/API usage.
Built with Tauri 2 + SvelteKit + Rust backend. Mirrors the macOS Swift version in functionality.

## Development

Prerequisites: [Node.js](https://nodejs.org/), [Rust](https://rustup.rs/), and the [Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/) (WebView2 is already bundled on Windows 11).

```powershell
# From the tauri/ directory
npm install
npm run tauri dev
```

The SvelteKit dev server starts on `http://localhost:1420` and the Tauri window opens automatically.

## Build (Release)

```powershell
# From the tauri/ directory
npm run tauri build
```

Produces an NSIS installer at:

```
src-tauri\target\release\bundle\nsis\TokenDeck_1.0.0_x64-setup.exe
```

If Cargo fails to download dependencies due to SSL certificate revocation checks (corporate proxies / Windows Defender), prefix the command:

```powershell
$env:CARGO_HTTP_CHECK_REVOKE = "false"; npm run tauri build
```

## Configuration and Credential Paths (Windows)

### App config

`%APPDATA%\usage-bar\config.json`

Controls which services are shown, their display order, refresh interval, and alert settings.
Generated automatically on first launch with sensible defaults.

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
    {"id": "codex",  "title": "GPT",    "accent": "#10A37F", "category": "subscription", "fetcher": "codexWham",  "enabled": true}
  ]
}
```

### Credential files (read-only at startup)

| Service | Path |
|---------|------|
| Claude OAuth token | `%USERPROFILE%\.claude\.credentials.json` |
| GPT / Codex WHAM  | `%USERPROFILE%\.codex\auth.json` |
| New-API gateway   | `%APPDATA%\usage-bar\<credentialFile>.json` (one file per gateway, named by `credentialFile` in config) |

**Claude** credentials are written by Claude Code CLI (`claude login`). The app reads them directly — no extra setup needed.

**GPT** credentials are written by the Codex CLI (`codex login`). Same pattern.

**New-API** gateways require a manually created JSON file:

```json
{
  "baseUrl": "https://your-gateway.example.com",
  "accessToken": "<system-access-token>",
  "userId": 0,
  "quotaPerUnit": 500000,
  "currency": "$"
}
```

The `accessToken` is the **system access token** from your New-API panel (Personal Settings → Generate), not the `sk-` relay token from Token Management.

To add a New-API gateway, append a service entry to `config.json` and create the credential file:

```json
{"id": "mygateway", "title": "MyGateway", "accent": "#2F80ED", "category": "apiUsage", "fetcher": "newAPI", "credentialFile": "mygateway.json", "enabled": true}
```

### Cache directory

`%LOCALAPPDATA%\usage-dashboard\` — last-known-good API responses (`<service-id>.json`), used as fallback when a fetch fails.

## Differences vs the macOS Swift Version

| Feature | macOS (Swift) | Windows (Tauri) |
|---------|---------------|-----------------|
| UI host | Native SwiftUI popover | Tauri WebView (SvelteKit) |
| Entry point | Menu bar icon (top menu bar) | System tray icon (bottom-right) |
| Credential storage | macOS Keychain (`Claude Code-credentials`) with file fallback | File-based only (no Windows Credential Manager integration in v1.0) |
| Claude creds path | Keychain → `~/.claude/.credentials.json` | `%USERPROFILE%\.claude\.credentials.json` |
| Config path | `~/.config/usage-bar/config.json` | `%APPDATA%\usage-bar\config.json` |
| New-API creds | `~/.config/usage-bar/<file>.json` | `%APPDATA%\usage-bar\<file>.json` |
| Auto-start | `SMAppService` (macOS Login Items) | `tauri-plugin-autostart` (Windows registry) |
| Notifications | macOS `UNUserNotificationCenter` | `tauri-plugin-notification` (Windows toast) |
| Build output | `TokenDeck.app` | NSIS installer `.exe` |

Functional parity: same three-counter fetch logic, same alert rules (`usageExceedsElapsedWindowPercent` / `usageExceedsThresholdOnly`), same tray red-dot on alert, same settings panel for toggling/reordering service cards.

## Tests

```powershell
# From the tauri/ directory
cargo test --manifest-path src-tauri/Cargo.toml
```

All unit and integration tests run headlessly (no window required).
