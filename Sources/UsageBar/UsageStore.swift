import Foundation
import Combine

@MainActor
final class UsageStore: ObservableObject {
    @Published private(set) var states: [ServiceRuntime]
    @Published private(set) var lastUpdated: Date?
    @Published private(set) var isRefreshing = false
    @Published private(set) var config: AppConfig
    @Published private(set) var configSaveError: String?
    @Published private(set) var activeAlertCount = 0
    @Published private(set) var activeAlertSummary: String?

    private var timer: Timer?
    private var lastAlertAtByKey: [String: Date] = [:]
    private var activeAlertKeysByService: [String: Set<String>] = [:]

    init(config: AppConfig) {
        self.config = config
        let enabled = config.services.filter { $0.enabled }
        // 启动时先用缓存填充,避免空白。
        self.states = enabled.map { cfg in
            if let cached = UsageCache.read(cfg.id) {
                return ServiceRuntime(config: cfg, status: .stale(cached.usage, cachedAt: cached.ts, error: "加载中…"))
            }
            return ServiceRuntime(config: cfg, status: .loading)
        }
    }

    func start() {
        if config.alerts.enabled { AlertNotifier.requestAuthorizationIfNeeded() }
        Task { await refresh() }
        scheduleTimer()
    }

    func setServiceEnabled(_ id: String, enabled: Bool) {
        var next = config
        guard let idx = next.services.firstIndex(where: { $0.id == id }) else { return }
        next.services[idx].enabled = enabled
        applyConfig(next)
        if enabled { Task { await refresh() } }
    }

    func moveService(_ id: String, by delta: Int) {
        var next = config
        guard let idx = next.services.firstIndex(where: { $0.id == id }) else { return }
        let newIndex = max(0, min(next.services.count - 1, idx + delta))
        guard newIndex != idx else { return }
        let item = next.services.remove(at: idx)
        next.services.insert(item, at: newIndex)
        applyConfig(next)
    }

    func setDisplayContent(_ item: DisplayContent, for id: String, enabled: Bool) {
        var next = config
        guard let idx = next.services.firstIndex(where: { $0.id == id }) else { return }
        next.services[idx].display.set(item, enabled: enabled)
        applyConfig(next)
    }

    func displayContentIsEnabled(_ item: DisplayContent, for id: String) -> Bool {
        config.services.first(where: { $0.id == id })?.display.isEnabled(item) ?? true
    }

    func setAlertsEnabled(_ enabled: Bool) {
        var next = config
        next.alerts.enabled = enabled
        applyConfig(next)
        if !enabled { clearActiveAlerts() }
        if enabled {
            AlertNotifier.requestAuthorizationIfNeeded()
            refreshActiveAlertsFromCurrentStates()
        }
    }

    func setAlertMinimumUsagePercent(_ pct: Double) {
        var next = config
        next.alerts.minimumUsagePercent = min(100, max(0, pct))
        applyConfig(next)
        refreshActiveAlertsFromCurrentStates()
    }

    func setAlertRule(_ rule: UsageAlertRule) {
        var next = config
        next.alerts.rule = rule
        applyConfig(next)
        refreshActiveAlertsFromCurrentStates()
    }

    func setAlertCooldownMinutes(_ minutes: Int) {
        var next = config
        next.alerts.cooldownSeconds = max(60, minutes * 60)
        applyConfig(next)
    }

    private func scheduleTimer() {
        timer?.invalidate()
        let interval = TimeInterval(max(60, config.refreshSeconds))
        let t = Timer(timeInterval: interval, repeats: true) { [weak self] _ in
            guard let self else { return }
            Task { @MainActor in await self.refresh() }
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    private func applyConfig(_ next: AppConfig) {
        config = next
        rebuildStates()
        scheduleTimer()
        do {
            try AppConfigStore.save(next)
            configSaveError = nil
        } catch {
            configSaveError = "配置保存失败:\(error.localizedDescription)"
        }
    }

    private func rebuildStates() {
        var existing: [String: ServiceStatus] = [:]
        for rt in states { existing[rt.id] = rt.status }
        states = config.services.filter { $0.enabled }.map { cfg in
            if let status = existing[cfg.id] {
                return ServiceRuntime(config: cfg, status: status)
            }
            if let cached = UsageCache.read(cfg.id) {
                return ServiceRuntime(config: cfg, status: .stale(cached.usage, cachedAt: cached.ts, error: "加载中…"))
            }
            return ServiceRuntime(config: cfg, status: .loading)
        }
        activeAlertKeysByService = activeAlertKeysByService.filter { serviceID, _ in
            states.contains { $0.id == serviceID }
        }
        publishActiveAlerts()
    }

    func refresh() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }

        // 各服务独立取数,并发执行,一个失败不影响另一个。
        await withTaskGroup(of: (String, FetchOutcome).self) { group in
            for rt in states {
                guard let fetcher = makeFetcher(for: rt.config) else {
                    apply(.failure("未支持的取数类型:\(rt.config.fetcher.rawValue)"), to: rt.config.id)
                    continue
                }
                group.addTask {
                    let outcome = await fetcher.fetch()
                    return (rt.config.id, outcome)
                }
            }
            for await (id, outcome) in group {
                apply(outcome, to: id)
            }
        }
        lastUpdated = Date()
    }

    private func apply(_ outcome: FetchOutcome, to id: String) {
        guard let idx = states.firstIndex(where: { $0.id == id }) else { return }
        switch outcome {
        case .success(let usage):
            UsageCache.write(usage, service: id)
            states[idx].status = .ok(usage, fetchedAt: Date())
            updateAlerts(for: states[idx].config, usage: usage, sendNotifications: true)
        case .failure(let msg):
            if let cached = UsageCache.read(id) {
                states[idx].status = .stale(cached.usage, cachedAt: cached.ts, error: msg)
            } else {
                states[idx].status = .error(msg)
            }
        }
    }

    private func updateAlerts(for service: ServiceConfig, usage: Usage, sendNotifications: Bool) {
        let alerts = config.alerts
        guard alerts.enabled, service.category == .subscription else {
            activeAlertKeysByService[service.id] = []
            publishActiveAlerts()
            return
        }
        if let serviceIDs = alerts.serviceIDs, !serviceIDs.contains(service.id) {
            activeAlertKeysByService[service.id] = []
            publishActiveAlerts()
            return
        }

        let now = Date()
        var activeKeys: Set<String> = []
        for window in usage.windows {
            if let windows = alerts.windows, !windows.contains(window.label) { continue }
            let usagePct = min(100, max(0, window.pct))
            guard usagePct >= alerts.minimumUsagePercent else { continue }

            let elapsedPct = elapsedWindowPercent(for: window, now: now)
            guard alertRuleMatches(alerts.rule, usagePct: usagePct, elapsedPct: elapsedPct,
                                   paceMultiplier: alerts.paceMultiplier) else { continue }

            let key = alertKey(serviceID: service.id, window: window, rule: alerts.rule)
            activeKeys.insert(key)
            guard sendNotifications else { continue }
            if let last = lastAlertAtByKey[key],
               now.timeIntervalSince(last) < TimeInterval(max(60, alerts.cooldownSeconds)) {
                continue
            }
            lastAlertAtByKey[key] = now
            AlertNotifier.send(title: "\(service.title) 用量提醒",
                               body: alertBody(service: service, window: window,
                                               usagePct: usagePct, elapsedPct: elapsedPct,
                                               alerts: alerts))
        }
        activeAlertKeysByService[service.id] = activeKeys
        publishActiveAlerts()
    }

    private func refreshActiveAlertsFromCurrentStates() {
        guard config.alerts.enabled else {
            clearActiveAlerts()
            return
        }
        for rt in states {
            switch rt.status {
            case .ok(let usage, _), .stale(let usage, _, _):
                updateAlerts(for: rt.config, usage: usage, sendNotifications: false)
            case .loading, .error:
                activeAlertKeysByService[rt.id] = []
            }
        }
        publishActiveAlerts()
    }

    private func clearActiveAlerts() {
        activeAlertKeysByService.removeAll()
        publishActiveAlerts()
    }

    private func publishActiveAlerts() {
        let count = activeAlertKeysByService.values.reduce(0) { $0 + $1.count }
        activeAlertCount = count
        activeAlertSummary = count > 0 ? "\(count) 个用量告警" : nil
    }

    private func alertRuleMatches(_ rule: UsageAlertRule, usagePct: Double,
                                  elapsedPct: Double?, paceMultiplier: Double) -> Bool {
        switch rule {
        case .usageExceedsThresholdOnly:
            return true
        case .usageExceedsElapsedWindowPercent:
            guard let elapsedPct else { return false }
            let target = min(100, max(0, elapsedPct * max(0.1, paceMultiplier)))
            return usagePct > target
        }
    }

    private func elapsedWindowPercent(for window: UsageWindow, now: Date) -> Double? {
        guard let resetAt = window.resetAt,
              let duration = windowDurationSeconds(label: window.label) else { return nil }
        let remaining = resetAt.timeIntervalSince(now)
        let elapsed = duration - remaining
        return min(100, max(0, elapsed / duration * 100))
    }

    private func windowDurationSeconds(label: String) -> TimeInterval? {
        if label.contains("5") { return 5 * 60 * 60 }
        if label.contains("周") { return 7 * 24 * 60 * 60 }
        return nil
    }

    private func alertKey(serviceID: String, window: UsageWindow, rule: UsageAlertRule) -> String {
        let resetEpoch = Int(window.resetAt?.timeIntervalSince1970 ?? 0)
        return "\(serviceID):\(window.label):\(resetEpoch):\(rule.rawValue)"
    }

    private func alertBody(service: ServiceConfig, window: UsageWindow, usagePct: Double,
                           elapsedPct: Double?, alerts: UsageAlertConfig) -> String {
        let usageText = "\(Int(usagePct.rounded()))%"
        let thresholdText = "\(Int(alerts.minimumUsagePercent.rounded()))%"
        switch alerts.rule {
        case .usageExceedsThresholdOnly:
            return "\(service.title) \(window.label) 用量 \(usageText),已超过 \(thresholdText) 阈值。"
        case .usageExceedsElapsedWindowPercent:
            let elapsedText = elapsedPct.map { "\(Int($0.rounded()))%" } ?? "未知"
            return "\(service.title) \(window.label) 用量 \(usageText),窗口时间进度 \(elapsedText),已超过 \(thresholdText) 阈值。"
        }
    }
}
