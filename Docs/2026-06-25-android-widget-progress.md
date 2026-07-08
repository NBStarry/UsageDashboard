# Android Widget Progress - 2026-06-25

## Current State

- Mac PhanRouter token has been regenerated from the Mac Chrome logged-in session and written back to `~/.config/usage-bar/phanrouter.json`.
- Mac CLI check passed:
  - `TokenUsageDashboard --fetch phanrouter` returns `ok`.
  - Mac relay returns PhanRouter `ok` with fresh balance/request count.
- Android widget layout has been tightened:
  - The card fills its allocated widget area instead of leaving large transparent padding.
  - Claude/GPT progress bars are vertical, one above the other.
  - Refresh icon is visible.
  - Existing widget renders cleanly after resize/re-add; verified on the Samsung device.
- Widget configuration UI now has size choices: compact, standard, large.

## Important Debug Findings

- Samsung Launcher may keep old widget sizing and old `RemoteViews` pending intents until the app/widget renders again. Re-add or resize can be required after changing `usage_widget_info.xml`.
- A direct Kotlin `HttpURLConnection` refresh to Claude returned HTTP 403 on device. The fallback/direct path exists, but the robust path should prefer Mac relay for widget refresh.
- The latest code change makes `UsageWidgetProvider` prefer relay data from `usage-bar/config.json` and only fall back to direct local credentials if relay is not configured.
- Relay-priority code has been fully rebuilt, installed, and verified on the Samsung device at `10.110.10.31`.

## Last Verified Behavior

- Earlier build verified on real device:
  - Widget no longer has large transparent blank area.
  - Two progress bars fit vertically.
  - Mac token/relay/PhanRouter data are fresh.
- Direct widget refresh behavior before the final relay-priority patch:
  - Launcher stayed foreground when tapping widget/root.
  - Snapshot updated, but Claude direct fetch became HTTP 403.

## 2026-06-29 Verification

- Device: `10.110.10.31:5555`, model `SM-S9180`.
- Installed the latest debug APK built with:
  - `npm run tauri -- android build --debug --target aarch64`
- Android relay config was written from the Mac relay:
  - `relay.url = http://10.110.8.155:8787`
  - `relay.enabled = true`
  - secret present, not recorded here.
- Phone snapshot verification passed:
  - Claude `ok`
  - GPT `ok`
  - PhanRouter `ok`
- Widget refresh verification passed:
  - Tapping the widget refresh/root kept Samsung Launcher in the foreground.
  - `widget.json.updatedAtMs` changed from `1782720213000` to `1782720322656`.
  - Claude/GPT/PhanRouter remained `ok`.
- Default widget size was changed and verified through Samsung's Add widget prompt:
  - `UsageDashboard 2 x 1`
- Existing widgets keep their old Launcher-assigned size. Re-add or resize an existing widget to see the new `2 x 1` default.

## Next Steps

1. Re-add or resize the existing home-screen widget if the Launcher is still holding the older `3 x 2` allocation.
2. Inspect the actual `2 x 1` rendered widget after adding a fresh instance, and tune width-specific text visibility if it feels cramped.

## Files Changed In This Work

- `tauri/src-tauri/gen/android/app/src/main/res/layout/usage_widget.xml`
- `tauri/src-tauri/gen/android/app/src/main/res/layout/activity_widget_config.xml`
- `tauri/src-tauri/gen/android/app/src/main/res/xml/usage_widget_info.xml`
- `tauri/src-tauri/gen/android/app/src/main/res/drawable/widget_action_bg.xml`
- `tauri/src-tauri/gen/android/app/src/main/res/drawable/widget_refresh_icon.xml`
- `tauri/src-tauri/gen/android/app/src/main/java/app/usagedashboard/UsageWidgetProvider.kt`
- `tauri/src-tauri/gen/android/app/src/main/java/app/usagedashboard/WidgetConfigActivity.kt`
- `tauri/src-tauri/src/widget.rs`
