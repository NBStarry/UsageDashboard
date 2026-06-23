<script lang="ts">
  import { config } from '$lib/store';
  import {
    setAlertsEnabled,
    setAlertThreshold,
    setAlertRule,
    setAlertCooldownMinutes,
    setServiceEnabled,
    moveService,
    setDisplayContent,
  } from '$lib/api';
  import type { BillingCategory, ServiceConfig, UsageAlertRule } from '$lib/types';

  // Display option definitions per category, matching DisplayContent.options(for:) in Swift
  interface DisplayOption {
    key: string;
    label: string;
  }

  function displayOptions(category: BillingCategory): DisplayOption[] {
    if (category === 'subscription') {
      return [
        { key: 'plan',           label: '套餐' },
        { key: 'fiveHour',       label: '5 小时' },
        { key: 'weekly',         label: '周额度' },
        { key: 'resetCountdown', label: '重置倒计时' },
        { key: 'updatedAt',      label: '更新时间' },
      ];
    }
    // apiUsage
    return [
      { key: 'balance',      label: '当前余额' },
      { key: 'used',         label: '历史消耗' },
      { key: 'requestCount', label: '请求次数' },
      { key: 'models',       label: '模型列表' },
      { key: 'updatedAt',    label: '更新时间' },
    ];
  }

  // Alert rule label mapping matching UsageAlertRule.title in Swift
  function ruleLabel(rule: UsageAlertRule): string {
    if (rule === 'usageExceedsElapsedWindowPercent') return '跑赢时间进度';
    return '仅超过阈值';
  }

  const ALL_RULES: UsageAlertRule[] = [
    'usageExceedsElapsedWindowPercent',
    'usageExceedsThresholdOnly',
  ];

  // --- Alerts handlers ---
  function onAlertsEnabled(e: Event) {
    setAlertsEnabled((e.target as HTMLInputElement).checked);
  }

  function onThresholdInput(e: Event) {
    setAlertThreshold(Number((e.target as HTMLInputElement).value));
  }

  function onRuleChange(rule: UsageAlertRule) {
    setAlertRule(rule);
  }

  function onCooldownChange(delta: number) {
    if (!$config) return;
    const current = Math.max(1, Math.floor($config.alerts.cooldownSeconds / 60));
    const next = Math.min(240, Math.max(1, current + delta));
    setAlertCooldownMinutes(next);
  }

  // --- Service handlers ---
  function onServiceEnabled(id: string, e: Event) {
    setServiceEnabled(id, (e.target as HTMLInputElement).checked);
  }

  function onMoveService(id: string, delta: number) {
    moveService(id, delta);
  }

  function onDisplayContent(id: string, key: string, e: Event) {
    setDisplayContent(id, key, (e.target as HTMLInputElement).checked);
  }

  function displayIsEnabled(cfg: ServiceConfig, key: string): boolean {
    const d = cfg.display as unknown as Record<string, boolean>;
    return d[key] ?? false;
  }
</script>

<div class="settings-root">
  {#if $config}
    <!-- Alerts panel -->
    <div class="panel">
      <div class="alerts-header">
        <label class="checkbox-row">
          <input
            type="checkbox"
            checked={$config.alerts.enabled}
            onchange={onAlertsEnabled}
          />
          <span class="label-semibold" style="font-size: 12px; color: rgba(255,255,255,0.96);">订阅号告警</span>
        </label>
      </div>

      <div class="alerts-body" style="opacity: {$config.alerts.enabled ? 1 : 0.55};">
        <!-- Threshold -->
        <div class="threshold-row">
          <span class="label-medium" style="font-size: 11px; color: rgba(255,255,255,0.86);">阈值</span>
          <span class="label-semibold" style="font-size: 11px; color: rgba(255,255,255,0.9);">
            {Math.round($config.alerts.minimumUsagePercent)}%
          </span>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          step="5"
          value={$config.alerts.minimumUsagePercent}
          oninput={onThresholdInput}
          disabled={!$config.alerts.enabled}
          class="slider"
        />

        <!-- Rule segmented control -->
        <div class="rule-row">
          {#each ALL_RULES as rule}
            <button
              class="rule-btn"
              class:rule-btn-active={$config.alerts.rule === rule}
              disabled={!$config.alerts.enabled}
              onclick={() => onRuleChange(rule)}
            >
              {ruleLabel(rule)}
            </button>
          {/each}
        </div>

        <!-- Cooldown stepper -->
        <div class="stepper-row">
          <span class="label-medium" style="font-size: 11px; color: rgba(255,255,255,0.86);">
            冷却 {Math.max(1, Math.floor($config.alerts.cooldownSeconds / 60))} 分钟
          </span>
          <div class="stepper-btns">
            <button
              class="stepper-btn"
              disabled={!$config.alerts.enabled || Math.max(1, Math.floor($config.alerts.cooldownSeconds / 60)) <= 1}
              onclick={() => onCooldownChange(-5)}
            >−</button>
            <button
              class="stepper-btn"
              disabled={!$config.alerts.enabled || Math.max(1, Math.floor($config.alerts.cooldownSeconds / 60)) >= 240}
              onclick={() => onCooldownChange(5)}
            >+</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Service list label -->
    <span class="label-semibold" style="font-size: 12px; color: rgba(255,255,255,0.92); padding: 0 2px;">渠道商</span>

    <!-- Scrollable service list -->
    <div class="service-list">
      {#each $config.services as cfg, index ($config.services[index]?.id ?? index)}
        {@const isEnabled = cfg.enabled}
        {@const isFirst = index === 0}
        {@const isLast = index === $config.services.length - 1}
        {@const options = displayOptions(cfg.category)}

        <div class="panel service-row">
          <!-- Service header: checkbox + title + category badge + move buttons -->
          <div class="service-header">
            <label class="checkbox-row" style="flex: 1; min-width: 0;">
              <input
                type="checkbox"
                checked={cfg.enabled}
                onchange={(e) => onServiceEnabled(cfg.id, e)}
              />
              <span
                class="accent-dot"
                style="background-color: {cfg.accent};"
              ></span>
              <span
                class="label-semibold"
                style="font-size: 12px; color: rgba(255,255,255,{isEnabled ? 0.96 : 0.58}); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;"
              >{cfg.title}</span>
              <span
                class="category-badge"
                style="color: rgba(255,255,255,{isEnabled ? 0.72 : 0.38});"
              >
                {cfg.category === 'subscription' ? '订阅号' : 'API 用量'}
              </span>
            </label>

            <div class="move-btns">
              <button
                class="move-btn"
                disabled={isFirst}
                style="color: rgba(255,255,255,{isFirst ? 0.28 : 0.86});"
                title="上移"
                onclick={() => onMoveService(cfg.id, -1)}
              >▲</button>
              <button
                class="move-btn"
                disabled={isLast}
                style="color: rgba(255,255,255,{isLast ? 0.28 : 0.86});"
                title="下移"
                onclick={() => onMoveService(cfg.id, 1)}
              >▼</button>
            </div>
          </div>

          <!-- Display options grid -->
          <div class="display-grid" style="opacity: {isEnabled ? 1 : 0.68};">
            {#each options as opt}
              <label class="checkbox-row">
                <input
                  type="checkbox"
                  checked={displayIsEnabled(cfg, opt.key)}
                  disabled={!isEnabled}
                  onchange={(e) => onDisplayContent(cfg.id, opt.key, e)}
                />
                <span
                  class="label-medium"
                  style="font-size: 11px; color: rgba(255,255,255,{isEnabled ? 0.86 : 0.42});"
                >{opt.label}</span>
              </label>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <!-- Bottom hint: Windows config path -->
    <span class="hint">保存到 %APPDATA%\usage-bar\config.json</span>
  {:else}
    <span class="hint">加载中…</span>
  {/if}
</div>

<style>
.settings-root {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 2px 0;
}

/* Panel: bg rgba(0,0,0,0.24) + 1px rgba(255,255,255,0.16) border, radius 8 */
.panel {
  background: rgba(0, 0, 0, 0.24);
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 8px;
  padding: 10px;
}

.alerts-header {
  margin-bottom: 9px;
}

.alerts-body {
  display: flex;
  flex-direction: column;
  gap: 7px;
  transition: opacity 0.15s;
}

.threshold-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.slider {
  width: 100%;
  accent-color: #0A84FF;
}

/* Segmented rule picker */
.rule-row {
  display: flex;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.14);
}

.rule-btn {
  flex: 1;
  padding: 4px 6px;
  font-size: 10px;
  font-weight: 500;
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.7);
  border: none;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.rule-btn:not(:last-child) {
  border-right: 1px solid rgba(255, 255, 255, 0.14);
}

.rule-btn-active {
  background: rgba(255, 255, 255, 0.18);
  color: rgba(255, 255, 255, 0.96);
  font-weight: 600;
}

.rule-btn:disabled {
  cursor: default;
}

/* Cooldown stepper */
.stepper-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.stepper-btns {
  display: flex;
  gap: 4px;
}

.stepper-btn {
  width: 24px;
  height: 24px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  background: rgba(255, 255, 255, 0.07);
  color: rgba(255, 255, 255, 0.86);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.stepper-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

/* Service list */
.service-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 420px;
  overflow-y: auto;
  padding-right: 2px;
}

.service-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.accent-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.category-badge {
  font-size: 9px;
  font-weight: 500;
  padding: 1px 5px;
  background: rgba(255, 255, 255, 0.09);
  border-radius: 9999px;
  white-space: nowrap;
  flex-shrink: 0;
}

.move-btns {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex-shrink: 0;
}

.move-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 10px;
  font-weight: 600;
  padding: 1px 3px;
  line-height: 1;
}

.move-btn:disabled {
  cursor: default;
}

/* Display options grid: adaptive min 86px columns, gap 6px, indent 18px */
.display-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(86px, 1fr));
  gap: 6px;
  padding-left: 18px;
  transition: opacity 0.15s;
}

/* Shared checkbox + label layout */
.checkbox-row {
  display: flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
}

.checkbox-row input[type='checkbox'] {
  accent-color: #0A84FF;
  flex-shrink: 0;
  cursor: pointer;
}

.label-semibold {
  font-weight: 600;
}

.label-medium {
  font-weight: 500;
}

.hint {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.58);
}
</style>
