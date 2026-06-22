# Repository Guidelines

## Project Structure & Module Organization

TokenUsageDashboard is a Swift 6 macOS menu bar app built with SwiftPM. `Package.swift` defines one executable target at `Sources/UsageBar`. Core app entry and coordination live in `App.swift`, `MenuBarController.swift`, and `UsageStore.swift`; network, credential, cache, and configuration logic live in `Http.swift`, `CredentialStore.swift`, `UsageCache.swift`, `UsageFetcher.swift`, and `AppConfigStore.swift`. SwiftUI views are grouped under `Sources/UsageBar/Views`. `AppIcon.icns` and `make_icon.swift` support app icon generation. Treat `.build/` and `*.app/` as generated artifacts.

## Build, Test, and Development Commands

- `swift build`: compile the debug executable.
- `swift build -c release`: compile the optimized release binary.
- `./build-app.sh`: build release, assemble `TokenUsageDashboard.app`, embed the icon, and ad-hoc sign the bundle.
- `open TokenUsageDashboard.app`: run the bundled menu bar app after packaging.
- `.build/release/TokenUsageDashboard --fetch claude` or `.build/release/TokenUsageDashboard --fetch codex`: validate fetcher behavior without launching the GUI.
- `.build/release/TokenUsageDashboard --render-readme`: regenerate README interface screenshots.
- `swift test`: run SwiftPM tests once a `Tests/` target is added; no test suite is present currently.

## Coding Style & Naming Conventions

Use idiomatic Swift with 4-space indentation and descriptive type names in `UpperCamelCase`. Keep methods, properties, and local variables in `lowerCamelCase`. Prefer small, focused SwiftUI views and keep shared styling in `Theme.swift`. Use structured APIs such as `URLSession`, `Codable`, and `FileManager` rather than ad hoc parsing. Keep comments brief and only where they clarify non-obvious behavior.

## Testing Guidelines

There is no dedicated test target yet. When adding tests, create `Tests/TokenUsageDashboardTests` and prefer focused unit tests around parsing, caching, configuration defaults, and fetcher error handling. Name test files after the unit under test, for example `UsageCacheTests.swift`, and test methods with behavior-focused names such as `testFallsBackToCachedUsageOnFetchFailure`.

## Commit & Pull Request Guidelines

Git history is not available from this checkout, so no existing commit convention can be inferred. Use short imperative commit subjects such as `Add usage cache fallback test`. Pull requests should include a clear summary, manual verification steps, linked issues when applicable, and screenshots or screen recordings for UI changes.

## Security & Configuration Tips

Do not commit local credentials or generated config from `~/.config/usage-bar/`. Keep token handling inside `CredentialStore.swift` and avoid logging bearer tokens, account IDs, or raw API responses that may contain private usage data.
