// Svelte writable stores for app state.
// init() fetches initial values from Tauri commands and registers event listeners
// that refresh the stores whenever the backend emits usage-updated / config-updated.

import { writable } from 'svelte/store';
import type { AppConfig, ServiceSnapshot } from '$lib/types';
import { getSnapshots, getConfig, onUsageUpdated, onConfigUpdated } from '$lib/api';

export const snapshots = writable<ServiceSnapshot[]>([]);
export const config = writable<AppConfig | null>(null);
export const lastUpdated = writable<Date | null>(null);
export const isRefreshing = writable<boolean>(false);

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
  isRefreshing.set(true);
  try {
    await Promise.all([refreshSnapshots(), refreshConfig()]);
  } finally {
    isRefreshing.set(false);
  }

  // Re-fetch snapshots whenever Rust emits "usage-updated".
  await onUsageUpdated(async () => {
    await refreshSnapshots();
  });

  // Re-fetch config whenever Rust emits "config-updated".
  await onConfigUpdated(async () => {
    await refreshConfig();
  });
}
