// Svelte writable stores for app state.
// init() fetches initial values from Tauri commands and registers event listeners
// that refresh the stores whenever the backend emits usage-updated / config-updated.

import { writable } from 'svelte/store';
import type { AppConfig, ServiceSnapshot } from '$lib/types';
import { getSnapshots, getConfig, isMobile, onUsageUpdated, onConfigUpdated } from '$lib/api';

export const snapshots = writable<ServiceSnapshot[]>([]);
export const config = writable<AppConfig | null>(null);
export const lastUpdated = writable<Date | null>(null);
export const isRefreshing = writable<boolean>(false);

// 移动端(Android/iOS):隐藏 quit、安全区 padding。init() 时从后端一次性读取。
export const mobile = writable<boolean>(false);

// Fetch snapshots and update stores.
async function refreshSnapshots(): Promise<void> {
  const data = await getSnapshots();
  snapshots.set(data);
  lastUpdated.set(new Date());
}

// Fetch config and update store.
async function refreshConfig(): Promise<void> {
  const data = await getConfig();
  config.set(data);
}

// init() should be called once from the root layout on mount.
// Fetches initial state then registers event listeners for live updates.
export async function init(): Promise<void> {
  // 平台标志一次性读取(运行期不变)。失败时保守按桌面端处理。
  isMobile().then((m) => mobile.set(m)).catch(() => {});

  // Register listeners FIRST so the backend's startup-refresh emit isn't missed
  // (the Rust first refresh can complete before the initial fetch below resolves).
  await onUsageUpdated(async () => {
    await refreshSnapshots();
  });
  await onConfigUpdated(async () => {
    await refreshConfig();
  });

  isRefreshing.set(true);
  try {
    await Promise.all([refreshSnapshots(), refreshConfig()]);
  } finally {
    isRefreshing.set(false);
  }
}
