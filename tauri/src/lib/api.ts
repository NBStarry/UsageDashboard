// Wrappers over @tauri-apps/api/core invoke for every command in commands.rs.
// Event listeners via @tauri-apps/api/event listen.
// Command names map 1:1 to Rust #[tauri::command] fn names (snake_case).
// Tauri 2 maps camelCase JS arg keys → snake_case Rust params automatically.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppConfig, ServiceSnapshot, UsageAlertRule } from '$lib/types';

// ---- Getters ----

export function getSnapshots(): Promise<ServiceSnapshot[]> {
  return invoke('get_snapshots');
}

export function getConfig(): Promise<AppConfig> {
  return invoke('get_config');
}

// ---- Actions ----

export function refreshNow(): Promise<void> {
  return invoke('refresh_now');
}

// commands.rs set_service_enabled(id, enabled)
export function setServiceEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke('set_service_enabled', { id, enabled });
}

// commands.rs move_service(id, delta)
export function moveService(id: string, delta: number): Promise<void> {
  return invoke('move_service', { id, delta });
}

// commands.rs set_display_content(id, item, enabled)
export function setDisplayContent(id: string, item: string, enabled: boolean): Promise<void> {
  return invoke('set_display_content', { id, item, enabled });
}

// commands.rs set_alerts_enabled(enabled)
export function setAlertsEnabled(enabled: boolean): Promise<void> {
  return invoke('set_alerts_enabled', { enabled });
}

// commands.rs set_alert_threshold(percent)
export function setAlertThreshold(percent: number): Promise<void> {
  return invoke('set_alert_threshold', { percent });
}

// commands.rs set_alert_rule(rule)
export function setAlertRule(rule: UsageAlertRule): Promise<void> {
  return invoke('set_alert_rule', { rule });
}

// commands.rs set_alert_cooldown_minutes(minutes)
export function setAlertCooldownMinutes(minutes: number): Promise<void> {
  return invoke('set_alert_cooldown_minutes', { minutes });
}

// commands.rs set_relay_config(url, secret)
export function setRelayConfig(url: string, secret: string): Promise<void> {
  return invoke('set_relay_config', { url, secret, enabled: true });
}

// commands.rs request_pin_widget() — Android: prompt to pin the home-screen widget
export function requestPinWidget(): Promise<boolean> {
  return invoke('request_pin_widget');
}

// commands.rs request_pin_double_widget() — Android: prompt to pin the 2x2 home-screen widget
export function requestPinDoubleWidget(): Promise<boolean> {
  return invoke('request_pin_double_widget');
}

// commands.rs is_mobile() — true on Android/iOS, false on desktop.
export function isMobile(): Promise<boolean> {
  return invoke('is_mobile');
}

export function quit(): Promise<void> {
  return invoke('quit');
}

// ---- Event listeners ----

// Rust emits "usage-updated" with unit payload after each refresh.
// Caller receives the raw event; unit payload means frontend re-invokes getSnapshots().
export function onUsageUpdated(cb: () => void): Promise<UnlistenFn> {
  return listen('usage-updated', () => cb());
}

// Rust emits "config-updated" with unit payload after each config mutation.
export function onConfigUpdated(cb: () => void): Promise<UnlistenFn> {
  return listen('config-updated', () => cb());
}
