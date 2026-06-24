<script lang="ts">
  import { config, mobile } from '$lib/store';
  import {
    setAlertsEnabled,
    setAlertThreshold,
    setAlertRule,
    setAlertCooldownMinutes,
    setServiceEnabled,
    moveService,
    setDisplayContent,
    saveNewApiCredentials,
    saveClaudeCredentials,
    saveCodexCredentials,
    setProxyUrl,
    requestPinWidget,
    refreshNow,
  } from '$lib/api';
  import type { BillingCategory, FetcherKind, ServiceConfig, UsageAlertRule } from '$lib/types';

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

  // --- HTTP 代理 ---
  // GFW 下让 Claude/Codex 经代理取数。输入框预填当前配置,运行期改即生效。
  let proxyInput = $state('');
  let proxySynced = false;
  let proxySaving = $state(false);
  let proxyMsg = $state('');
  $effect(() => {
    // 首次拿到 config 时把已存的 proxyUrl 同步进输入框(之后不覆盖用户编辑)。
    if (!proxySynced && $config) {
      proxyInput = $config.proxyUrl ?? '';
      proxySynced = true;
    }
  });
  async function onSaveProxy() {
    proxySaving = true;
    proxyMsg = '';
    try {
      await setProxyUrl(proxyInput.trim());
      proxyMsg = proxyInput.trim() ? '已设置,正在刷新…' : '已清除,正在刷新…';
    } catch (e) {
      proxyMsg = `失败:${e}`;
    } finally {
      proxySaving = false;
    }
  }

  // --- 主屏小组件(仅移动端)---
  let widgetMsg = $state('');
  async function onAddWidget() {
    widgetMsg = '';
    try {
      const ok = await requestPinWidget();
      widgetMsg = ok ? '已请求,按系统提示确认添加' : '桌面未接受请求';
    } catch (e) {
      widgetMsg = `${e}`;
    }
  }

  // --- 凭证 app 内录入 ---
  // 手机沙盒里没有桌面 CLI 写的凭证文件,只能 app 内录入。表单按服务 id 维护;
  // 不预填(后端不暴露读凭证命令,token 敏感)。支持三种 fetcher:
  //   newAPI(baseUrl/accessToken/userId/quotaPerUnit/currency)— 桌面+移动;
  //   claudeOauth(accessToken)、codexWham(accessToken/accountId)— 仅移动端
  //   (桌面端这两个文件由 Claude Code / Codex CLI 维护,录入会覆盖,故不显示)。
  interface CredForm {
    open: boolean;
    baseUrl: string;
    accessToken: string;
    accountId: string;
    userId: string;
    quotaPerUnit: string;
    currency: string;
    saving: boolean;
    msg: string;
    ok: boolean;
  }

  // 哪些 fetcher 支持 app 内录入,以及是否仅限移动端。
  function credInputKind(fetcher: FetcherKind): 'newAPI' | 'claude' | 'codex' | null {
    if (fetcher === 'newAPI') return 'newAPI';
    if (fetcher === 'claudeOauth') return 'claude';
    if (fetcher === 'codexWham') return 'codex';
    return null;
  }

  // 该服务此刻是否应展示录入表单(claude/codex 仅移动端)。
  function showCredInput(cfg: ServiceConfig): boolean {
    const kind = credInputKind(cfg.fetcher);
    if (!kind) return false;
    if (kind === 'newAPI') return true;
    return $mobile;
  }

  let credForms = $state<Record<string, CredForm>>({});

  // 为可录入服务懒初始化表单,保留用户已输入的值。
  $effect(() => {
    for (const svc of $config?.services ?? []) {
      if (credInputKind(svc.fetcher) && !credForms[svc.id]) {
        credForms[svc.id] = {
          open: false,
          baseUrl: '',
          accessToken: '',
          accountId: '',
          userId: '',
          quotaPerUnit: '',
          currency: '',
          saving: false,
          msg: '',
          ok: false,
        };
      }
    }
  });

  async function onSaveCreds(cfg: ServiceConfig) {
    const f = credForms[cfg.id];
    if (!f) return;
    const kind = credInputKind(cfg.fetcher);

    // 各 fetcher 的校验 + 保存动作。
    let doSave: (() => Promise<void>) | null = null;
    if (kind === 'newAPI') {
      if (!cfg.credentialFile) {
        f.ok = false;
        f.msg = '该服务未配置 credentialFile,无法保存';
        return;
      }
      if (!f.baseUrl.trim() || !f.accessToken.trim()) {
        f.ok = false;
        f.msg = '网关地址和 accessToken 必填';
        return;
      }
      const payload: Record<string, unknown> = {
        baseUrl: f.baseUrl.trim(),
        accessToken: f.accessToken.trim(),
      };
      if (f.userId.trim()) payload.userId = Number(f.userId.trim());
      if (f.quotaPerUnit.trim()) payload.quotaPerUnit = Number(f.quotaPerUnit.trim());
      if (f.currency.trim()) payload.currency = f.currency.trim();
      const file = cfg.credentialFile;
      doSave = () => saveNewApiCredentials(file, JSON.stringify(payload));
    } else if (kind === 'claude') {
      if (!f.accessToken.trim()) {
        f.ok = false;
        f.msg = 'accessToken 必填';
        return;
      }
      const tok = f.accessToken.trim();
      doSave = () => saveClaudeCredentials(tok);
    } else if (kind === 'codex') {
      if (!f.accessToken.trim() || !f.accountId.trim()) {
        f.ok = false;
        f.msg = 'access_token 和 account_id 必填';
        return;
      }
      const tok = f.accessToken.trim();
      const acc = f.accountId.trim();
      doSave = () => saveCodexCredentials(tok, acc);
    }
    if (!doSave) return;

    f.saving = true;
    f.msg = '';
    try {
      await doSave();
      // 清掉敏感的 token,立即拉一次用量验证凭证可用。
      f.accessToken = '';
      f.ok = true;
      f.msg = '已保存,正在刷新…';
      await refreshNow();
      f.msg = '已保存';
    } catch (e) {
      f.ok = false;
      f.msg = `保存失败:${e}`;
    } finally {
      f.saving = false;
    }
  }
</script>

<div class="settings-root">
  {#if $config}
    <!-- HTTP 代理(GFW 下让 Claude/GPT 经代理取数) -->
    <div class="panel">
      <span class="label-semibold" style="font-size: 12px; color: rgba(255,255,255,0.96);">HTTP 代理</span>
      <div class="cred-form" style="margin-top: 8px;">
        <input
          class="cred-input"
          type="text"
          placeholder="留空=直连;例 http://127.0.0.1:7897"
          bind:value={proxyInput}
        />
        <div class="cred-actions">
          <button class="cred-save" disabled={proxySaving} onclick={onSaveProxy}>
            {proxySaving ? '应用中…' : '应用并刷新'}
          </button>
          {#if proxyMsg}
            <span class="cred-msg" style="color: rgba(255,255,255,0.7);">{proxyMsg}</span>
          {/if}
        </div>
        <span class="hint">国外接口(Claude/GPT)被墙时填代理;由代理按规则分流,国内接口不受影响。</span>
      </div>
    </div>

    {#if $mobile}
      <!-- 主屏小组件 -->
      <div class="panel">
        <div class="cred-actions">
          <button class="cred-save" onclick={onAddWidget}>添加主屏小组件</button>
          {#if widgetMsg}
            <span class="cred-msg" style="color: rgba(255,255,255,0.7);">{widgetMsg}</span>
          {/if}
        </div>
        <span class="hint">把用量小组件固定到桌面;也可长按桌面→微件→UsageDashboard 手动添加。</span>
      </div>
    {/if}

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

          <!-- 凭证录入:newAPI 桌面+移动;claude/codex 仅移动端 -->
          {#if showCredInput(cfg) && credForms[cfg.id]}
            {@const f = credForms[cfg.id]}
            {@const kind = credInputKind(cfg.fetcher)}
            <div class="cred-section">
              <button class="cred-toggle" onclick={() => (f.open = !f.open)}>
                {f.open ? '▾' : '▸'} 凭证录入
              </button>
              {#if f.open}
                <div class="cred-form">
                  {#if kind === 'newAPI'}
                    <input class="cred-input" type="text" placeholder="网关地址 baseUrl" bind:value={f.baseUrl} />
                    <input class="cred-input" type="password" placeholder="accessToken(系统访问令牌)" bind:value={f.accessToken} />
                    <div class="cred-row3">
                      <input class="cred-input" type="number" placeholder="userId(默认 0)" bind:value={f.userId} />
                      <input class="cred-input" type="number" placeholder="quotaPerUnit(默认 500000)" bind:value={f.quotaPerUnit} />
                      <input class="cred-input" type="text" placeholder="货币(默认 $)" bind:value={f.currency} />
                    </div>
                  {:else if kind === 'claude'}
                    <input class="cred-input" type="password" placeholder="Claude accessToken(claudeAiOauth)" bind:value={f.accessToken} />
                  {:else if kind === 'codex'}
                    <input class="cred-input" type="password" placeholder="access_token" bind:value={f.accessToken} />
                    <input class="cred-input" type="text" placeholder="account_id" bind:value={f.accountId} />
                  {/if}
                  <div class="cred-actions">
                    <button class="cred-save" disabled={f.saving} onclick={() => onSaveCreds(cfg)}>
                      {f.saving ? '保存中…' : '保存并刷新'}
                    </button>
                    {#if f.msg}
                      <span class="cred-msg" style="color: {f.ok ? '#34C759' : '#FF6B6B'};">{f.msg}</span>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Bottom hint: 配置路径(桌面端显示文件位置;移动端为应用沙盒,不展示原始路径) -->
    {#if !$mobile}
      <span class="hint">保存到 %APPDATA%\usage-bar\config.json</span>
    {/if}
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

/* New-API 凭证录入 */
.cred-section {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.cred-toggle {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 11px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.78);
  padding: 0;
}

.cred-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 8px;
}

.cred-row3 {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 6px;
}

.cred-input {
  width: 100%;
  padding: 6px 8px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.92);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 5px;
  outline: none;
}

.cred-input::placeholder {
  color: rgba(255, 255, 255, 0.4);
}

.cred-input:focus {
  border-color: rgba(10, 132, 255, 0.7);
}

.cred-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cred-save {
  padding: 5px 12px;
  font-size: 11px;
  font-weight: 600;
  color: white;
  background: #0a84ff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.cred-save:disabled {
  opacity: 0.55;
  cursor: default;
}

.cred-msg {
  font-size: 10px;
  font-weight: 500;
}
</style>
