import SwiftUI

struct ServiceCardView: View {
    let runtime: ServiceRuntime

    private var accent: Color { Color(hex: runtime.config.accent) }
    private var display: ServiceDisplayOptions { runtime.config.display }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            content
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Theme.cardBg)
                .overlay(RoundedRectangle(cornerRadius: 14)
                    .stroke(Color.white.opacity(0.08), lineWidth: 1))
        )
    }

    // ─── 头部:色点 + 名称 + plan 徽章 ───
    private var header: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(accent)
                .frame(width: 9, height: 9)
                .shadow(color: accent, radius: 3)
            Text(runtime.config.title)
                .font(.system(size: 14, weight: .semibold))
                .foregroundColor(.white)
            Spacer(minLength: 0)
            if display.plan, let plan = currentPlan {
                Text(plan)
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundColor(.white)
                    .padding(.horizontal, 7).padding(.vertical, 2)
                    .background(Capsule().fill(Color.black))
            }
        }
        .padding(.bottom, 12)
    }

    @ViewBuilder
    private var content: some View {
        switch runtime.status {
        case .loading:
            HStack(spacing: 6) {
                ProgressView().controlSize(.small)
                Text("加载中…").font(.system(size: 12)).foregroundColor(Theme.subGray)
            }
            .padding(.vertical, 4)

        case .ok(let usage, let fetchedAt):
            usageBody(usage)
            if display.updatedAt { footerOK(fetchedAt) }

        case .stale(let usage, let cachedAt, _):
            usageBody(usage)
            footerStale(cachedAt)

        case .error(let msg):
            Text(msg)
                .font(.system(size: 12))
                .foregroundColor(Theme.red)
                .fixedSize(horizontal: false, vertical: true)
                .lineSpacing(3)
        }
    }

    private var currentPlan: String? {
        switch runtime.status {
        case .ok(let u, _): return u.plan
        case .stale(let u, _, _): return u.plan
        default: return nil
        }
    }

    // 按数据形态选择渲染:余额型(PhanRouter)或用量窗口型(Claude/GPT)。
    @ViewBuilder
    private func usageBody(_ usage: Usage) -> some View {
        if let b = usage.balance {
            BalanceCardBody(info: b, accent: accent, display: display)
        } else {
            windows(usage.windows)
        }
    }

    private func windows(_ ws: [UsageWindow]) -> some View {
        let visible = ws.filter { windowIsVisible($0) }
        return VStack(alignment: .leading, spacing: 11) {
            if visible.isEmpty {
                Text("无可展示内容")
                    .font(.system(size: 12))
                    .foregroundColor(Theme.subGray)
            }
            ForEach(visible) { w in
                let pct = min(100, max(0, w.pct))
                VStack(alignment: .leading, spacing: 5) {
                    HStack {
                        Text(w.label).font(.system(size: 11)).foregroundColor(Theme.labelGray)
                        Spacer()
                        Text("\(Int(pct.rounded()))%")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundColor(Theme.barColor(pct))
                    }
                    ProgressBarView(pct: pct)
                    if display.resetCountdown, let reset = TimeFmt.resetCountdown(w.resetAt) {
                        Text(reset).font(.system(size: 10)).foregroundColor(Theme.subGray)
                    }
                }
            }
        }
    }

    private func windowIsVisible(_ window: UsageWindow) -> Bool {
        switch window.label {
        case "5 小时": return display.fiveHour
        case "周": return display.weekly
        default: return true
        }
    }

    private func footerOK(_ fetchedAt: Date) -> some View {
        HStack {
            Spacer()
            Text("更新于 \(TimeFmt.hm(fetchedAt) ?? "")")
                .font(.system(size: 10)).foregroundColor(Theme.footGray)
        }
        .padding(.top, 8)
    }

    private func footerStale(_ cachedAt: Date?) -> some View {
        HStack {
            Spacer()
            Text("⚠ 刷新失败,显示上次结果" + (TimeFmt.hm(cachedAt).map { " · \($0)" } ?? ""))
                .font(.system(size: 10)).foregroundColor(Theme.amber)
                .multilineTextAlignment(.trailing)
        }
        .padding(.top, 8)
    }
}
