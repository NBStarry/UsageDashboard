// TS interfaces mirroring Rust models/state serialization (camelCase via serde rename_all).
// ServiceStatus mirrors state.rs ServiceStatus with internal `kind` tag (serde tag = "kind").

// models.rs: BillingCategory
export type BillingCategory = 'subscription' | 'apiUsage';

// models.rs: FetcherKind — note NewApi serializes as "newAPI" (serde rename)
export type FetcherKind = 'claudeOauth' | 'codexWham' | 'newAPI' | 'unsupported';

// models.rs: UsageAlertRule
export type UsageAlertRule =
  | 'usageExceedsElapsedWindowPercent'
  | 'usageExceedsThresholdOnly';

// models.rs: UsageWindow — reset_at → resetAt (RFC3339 string | null)
export interface UsageWindow {
  label: string;
  pct: number;
  resetAt: string | null;
}

// models.rs: ModelEntry
export interface ModelEntry {
  name: string;
  vendor: string;
}

// models.rs: BalanceInfo — request_count → requestCount
export interface BalanceInfo {
  balance: number;
  used: number;
  currency: string;
  requestCount: number | null;
  models: ModelEntry[];
}

// models.rs: Usage
export interface Usage {
  plan: string | null;
  windows: UsageWindow[];
  balance: BalanceInfo | null;
}

// models.rs: ServiceDisplayOptions — all five_hour→fiveHour, reset_countdown→resetCountdown, etc.
export interface ServiceDisplayOptions {
  plan: boolean;
  fiveHour: boolean;
  weekly: boolean;
  resetCountdown: boolean;
  updatedAt: boolean;
  balance: boolean;
  used: boolean;
  requestCount: boolean;
  models: boolean;
}

// models.rs: ServiceConfig
export interface ServiceConfig {
  id: string;
  title: string;
  accent: string;
  category: BillingCategory;
  fetcher: FetcherKind;
  credentialFile: string | null;
  enabled: boolean;
  display: ServiceDisplayOptions;
}

// models.rs: UsageAlertConfig — minimum_usage_percent→minimumUsagePercent, etc.
export interface UsageAlertConfig {
  enabled: boolean;
  minimumUsagePercent: number;
  rule: UsageAlertRule;
  paceMultiplier: number;
  cooldownSeconds: number;
  serviceIds: string[] | null;
  windows: string[] | null;
}

// models.rs: AppConfig — refresh_seconds→refreshSeconds
export interface AppConfig {
  refreshSeconds: number;
  alerts: UsageAlertConfig;
  services: ServiceConfig[];
  proxyUrl?: string | null;
}

// state.rs: ServiceStatus — discriminated union on `kind` (serde tag = "kind", rename_all = "camelCase")
// Loading: { kind: "loading" }
// Ok: { kind: "ok", usage: Usage, fetchedAt: string }
// Stale: { kind: "stale", usage: Usage, cachedAt: string | null, error: string }
// Error: { kind: "error", message: string }
export type ServiceStatus =
  | { kind: 'loading' }
  | { kind: 'ok'; usage: Usage; fetchedAt: string }
  | { kind: 'stale'; usage: Usage; cachedAt: string | null; error: string }
  | { kind: 'error'; message: string };

// state.rs: ServiceSnapshot
export interface ServiceSnapshot {
  config: ServiceConfig;
  status: ServiceStatus;
}
