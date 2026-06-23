<script lang="ts">
  import type { BalanceInfo, ModelEntry, ServiceDisplayOptions } from '$lib/types';

  interface Props {
    info: BalanceInfo;
    accent: string;
    display: ServiceDisplayOptions;
  }
  let { info, accent, display }: Props = $props();

  // Models section expanded state
  let expanded = $state(false);
  // Set of vendor names whose sub-lists are expanded
  let openVendors = $state<Set<string>>(new Set());

  function toggleVendor(vendor: string) {
    const next = new Set(openVendors);
    if (next.has(vendor)) {
      next.delete(vendor);
    } else {
      next.add(vendor);
    }
    openVendors = next;
  }

  // groupedModels: mirrors Rust grouped_models — group by vendor; models sorted name asc;
  // groups sorted count desc, then vendor asc on tie.
  interface ModelGroup {
    vendor: string;
    models: ModelEntry[];
  }

  function groupedModels(models: ModelEntry[]): ModelGroup[] {
    const map = new Map<string, ModelEntry[]>();
    for (const m of models) {
      const arr = map.get(m.vendor) ?? [];
      arr.push(m);
      map.set(m.vendor, arr);
    }
    const groups: ModelGroup[] = [];
    for (const [vendor, ms] of map) {
      const sorted = [...ms].sort((a, b) => a.name.localeCompare(b.name));
      groups.push({ vendor, models: sorted });
    }
    // count desc; on tie, vendor asc
    groups.sort((a, b) => {
      const diff = b.models.length - a.models.length;
      if (diff !== 0) return diff;
      return a.vendor.localeCompare(b.vendor);
    });
    return groups;
  }

  const groups = $derived(groupedModels(info.models));

  function money(v: number): string {
    return `${info.currency}${v.toFixed(2)}`;
  }

  const hasVisibleContent = $derived(
    display.balance || display.used ||
    (display.requestCount && info.requestCount !== null) ||
    (display.models && info.models.length > 0)
  );
</script>

<div class="balance-body">
  <!-- 余额 + 历史消耗 -->
  {#if display.balance || display.used}
    <div class="metrics-row">
      {#if display.balance}
        <div class="metric big" style="align-items: flex-start;">
          <span class="sub-gray metric-label">当前余额</span>
          <span class="metric-value" style="font-size: 22px; font-weight: 600; color: {accent};">
            {money(info.balance)}
          </span>
        </div>
      {/if}
      {#if display.balance && display.used}
        <div class="spacer"></div>
      {/if}
      {#if display.used}
        <div class="metric" style="align-items: {display.balance ? 'flex-end' : 'flex-start'};">
          <span class="sub-gray metric-label">历史消耗</span>
          <span class="metric-value label-gray" style="font-size: 15px; font-weight: 600;">
            {money(info.used)}
          </span>
        </div>
      {/if}
    </div>
  {/if}

  <!-- 累计请求 -->
  {#if display.requestCount && info.requestCount !== null}
    <span class="sub-gray" style="font-size: 10px;">累计请求 {info.requestCount.toLocaleString()}</span>
  {/if}

  <!-- 模型区: 可展开, 按来源分组 -->
  {#if display.models && info.models.length > 0}
    <div class="divider"></div>

    <div class="models-section">
      <!-- 总开关 -->
      <button class="models-toggle" onclick={() => (expanded = !expanded)}>
        <span class="label-gray" style="font-size: 12px; font-weight: 500;">模型</span>
        <span style="font-size: 12px; font-weight: 600; color: {accent};">{info.models.length}</span>
        <div class="flex-spacer"></div>
        <span class="chevron sub-gray">{expanded ? '▲' : '▼'}</span>
      </button>

      {#if expanded}
        <div class="scroll-area">
          <div class="groups-list">
            {#each groups as group}
              <!-- 厂商行 -->
              <button class="vendor-row" onclick={() => toggleVendor(group.vendor)}>
                <span class="vendor-chevron sub-gray">{openVendors.has(group.vendor) ? '▾' : '▸'}</span>
                <span class="label-gray" style="font-size: 11px; font-weight: 600;">{group.vendor}</span>
                <span class="foot-gray" style="font-size: 9px;">{group.models.length}</span>
                <div class="flex-spacer"></div>
              </button>

              <!-- 该厂商展开时的模型列表 -->
              {#if openVendors.has(group.vendor)}
                <div class="model-list">
                  {#each group.models as m}
                    <span class="sub-gray model-name">{m.name}</span>
                  {/each}
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if !hasVisibleContent}
    <span class="sub-gray" style="font-size: 12px;">无可展示内容</span>
  {/if}
</div>

<style>
.balance-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.metrics-row {
  display: flex;
  align-items: flex-end;
}

.metric {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.metric-label {
  display: block;
}

.metric-value {
  display: block;
}

.spacer {
  flex: 1;
}

.flex-spacer {
  flex: 1;
}

.divider {
  height: 1px;
  background: rgba(255, 255, 255, 0.12);
}

.models-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.models-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  color: inherit;
}

.chevron {
  font-size: 10px;
  font-weight: 600;
}

/* Scrollable area ~260px */
.scroll-area {
  height: 260px;
  overflow-y: auto;
}

.groups-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  padding-right: 4px;
  box-sizing: border-box;
}

.vendor-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  background: none;
  border: none;
  padding: 2px 0;
  cursor: pointer;
  color: inherit;
  text-align: left;
}

.vendor-chevron {
  font-size: 8px;
  font-weight: 600;
  width: 10px;
  text-align: center;
  flex-shrink: 0;
}

/* Model names indented under vendor */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding-left: 16px;
  padding-bottom: 4px;
  width: 100%;
  box-sizing: border-box;
}

.model-name {
  font-size: 11px;
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.label-gray { color: #D8D8DA; }
.sub-gray   { color: #8E8E93; }
.foot-gray  { color: #6E6E73; }
</style>
