<script lang="ts">
  import { onMount } from 'svelte';
  import { snapshots, lastUpdated, isRefreshing, init } from '$lib/store';
  import { refreshNow, quit } from '$lib/api';
  import { hm } from '$lib/theme';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';

  let showingSettings = $state(false);

  onMount(() => {
    init();
  });

  function toggleSettings() {
    showingSettings = !showingSettings;
  }

  async function onRefresh() {
    await refreshNow();
  }

  async function onQuit() {
    await quit();
  }
</script>

<div class="root" style="width: {showingSettings ? 360 : 320}px;">
  <!-- Header: gauge icon + title + settings toggle -->
  <div class="header">
    <span class="gauge-icon">◎</span>
    <span class="title">{showingSettings ? '显示设置' : '订阅用量'}</span>
    <button class="settings-btn" onclick={toggleSettings}>
      {#if showingSettings}
        <span class="btn-icon">✓</span>完成
      {:else}
        <span class="btn-icon">⚙</span>设置
      {/if}
    </button>
  </div>

  <!-- Body -->
  {#if showingSettings}
    <SettingsView />
  {:else}
    <div class="card-list">
      {#if $snapshots.length === 0}
        <span class="empty-state">未选择任何渠道商</span>
      {:else}
        {#each $snapshots as snapshot (snapshot.config.id)}
          <ServiceCard {snapshot} />
        {/each}
      {/if}
    </div>

    <!-- Footer: last updated + refresh + quit -->
    <div class="footer">
      {#if $lastUpdated}
        <span class="updated-at">更新于 {hm($lastUpdated)}</span>
      {/if}
      <div class="footer-actions">
        <button
          class="action-btn"
          disabled={$isRefreshing}
          onclick={onRefresh}
        >
          {#if $isRefreshing}
            <span class="spinner"></span>刷新中
          {:else}
            ↻ 刷新
          {/if}
        </button>
        <button class="quit-btn" onclick={onQuit} title="退出">⏻</button>
      </div>
    </div>
  {/if}
</div>

<style>
:global(*, *::before, *::after) {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:global(body) {
  background: transparent;
  font-family: -apple-system, 'Segoe UI', Arial, sans-serif;
  font-size: 13px;
  line-height: 1.4;
  -webkit-font-smoothing: antialiased;
}

.root {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  min-height: 100vh;
  background: rgba(20, 20, 22, 0.92);
  transition: width 0.16s ease-in-out;
}

/* Header */
.header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.gauge-icon {
  font-size: 13px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
  flex-shrink: 0;
  line-height: 1;
}

.title {
  font-size: 13px;
  font-weight: 600;
  color: white;
  flex: 1;
}

.settings-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 7px;
  font-size: 11px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.88);
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 9999px;
  cursor: pointer;
  transition: background 0.12s;
}

.settings-btn:hover {
  background: rgba(255, 255, 255, 0.14);
}

.btn-icon {
  font-size: 11px;
  font-weight: 600;
}

/* Card list */
.card-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.empty-state {
  font-size: 12px;
  color: #8E8E93;
  padding: 8px 0;
}

/* Footer */
.footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-top: 2px;
}

.updated-at {
  font-size: 10px;
  color: #6E6E73;
  flex: 1;
}

.footer-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0;
  font-size: 11px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.9);
  background: none;
  border: none;
  cursor: pointer;
  transition: opacity 0.12s;
}

.action-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.action-btn:not(:disabled):hover {
  opacity: 0.75;
}

.quit-btn {
  padding: 0;
  font-size: 11px;
  font-weight: 600;
  color: #8E8E93;
  background: none;
  border: none;
  cursor: pointer;
  transition: opacity 0.12s;
  line-height: 1;
}

.quit-btn:hover {
  opacity: 0.7;
}

/* Spinner for refresh */
.spinner {
  display: inline-block;
  width: 10px;
  height: 10px;
  border: 1.5px solid rgba(255, 255, 255, 0.2);
  border-top-color: rgba(255, 255, 255, 0.8);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
