import SwiftUI

// 余额型服务卡片主体:当前余额 + 历史消耗 + 模型广场(按来源分类,可展开滚动)。
struct BalanceCardBody: View {
    let info: BalanceInfo
    let accent: Color
    let display: ServiceDisplayOptions
    @State private var expanded: Bool            // 模型总区是否展开
    @State private var openVendors: Set<String>  // 已展开的厂商组(二级)

    init(info: BalanceInfo, accent: Color, display: ServiceDisplayOptions = .all,
         forceExpand: Bool = false) {
        self.info = info
        self.accent = accent
        self.display = display
        _expanded = State(initialValue: forceExpand)   // 默认收起;forceExpand 仅供 --render 预览
        _openVendors = State(initialValue: [])          // 各厂商组默认收起,点击逐个展开
    }

    private func money(_ v: Double) -> String {
        String(format: "\(info.currency)%.2f", v)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            // 余额 + 历史消耗
            if display.balance || display.used {
                HStack(alignment: .firstTextBaseline) {
                    if display.balance {
                        metric(title: "当前余额", value: money(info.balance), color: accent, big: true)
                    }
                    if display.balance && display.used { Spacer() }
                    if display.used {
                        metric(title: "历史消耗", value: money(info.used), color: Theme.labelGray,
                               big: !display.balance)
                    }
                }
            }
            if display.requestCount, let rc = info.requestCount {
                Text("累计请求 \(rc.formatted())")
                    .font(.system(size: 10)).foregroundColor(Theme.subGray)
            }

            if display.models, !info.models.isEmpty {
                Divider().overlay(Color.white.opacity(0.12))
                modelsSection
            }

            if !hasVisibleContent {
                Text("无可展示内容")
                    .font(.system(size: 12))
                    .foregroundColor(Theme.subGray)
            }
        }
    }

    private var hasVisibleContent: Bool {
        display.balance || display.used ||
        (display.requestCount && info.requestCount != nil) ||
        (display.models && !info.models.isEmpty)
    }

    private func metric(title: String, value: String, color: Color, big: Bool) -> some View {
        VStack(alignment: big ? .leading : .trailing, spacing: 2) {
            Text(title).font(.system(size: 10)).foregroundColor(Theme.subGray)
            Text(value)
                .font(.system(size: big ? 22 : 15, weight: .semibold))
                .foregroundColor(color)
        }
    }

    // ─── 模型广场:可展开,按来源分组,滚动查看 ───
    private var modelsSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Button {
                withAnimation(.easeInOut(duration: 0.18)) { expanded.toggle() }
            } label: {
                HStack(spacing: 6) {
                    Text("模型").font(.system(size: 12, weight: .medium)).foregroundColor(Theme.labelGray)
                    Text("\(info.models.count)")
                        .font(.system(size: 12, weight: .semibold)).foregroundColor(accent)
                    Spacer()
                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 10, weight: .semibold)).foregroundColor(Theme.subGray)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            if expanded {
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(info.groupedModels, id: \.vendor) { group in
                            vendorRow(group)
                            if openVendors.contains(group.vendor) {
                                VStack(alignment: .leading, spacing: 3) {
                                    ForEach(group.models) { m in
                                        Text(m.name)
                                            .font(.system(size: 11))
                                            .foregroundColor(Theme.subGray)
                                            .lineLimit(1)
                                            .truncationMode(.middle)
                                    }
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.leading, 16)
                                .padding(.bottom, 4)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.trailing, 4)
                }
                .frame(height: 260)
            }
        }
    }

    // 二级:厂商组行,可单独展开/收起。
    private func vendorRow(_ group: (vendor: String, models: [ModelEntry])) -> some View {
        let open = openVendors.contains(group.vendor)
        return Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                if open { openVendors.remove(group.vendor) } else { openVendors.insert(group.vendor) }
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: open ? "chevron.down" : "chevron.right")
                    .font(.system(size: 8, weight: .semibold))
                    .foregroundColor(Theme.subGray)
                    .frame(width: 10, alignment: .center)
                Text(group.vendor)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(Theme.labelGray)
                Text("\(group.models.count)")
                    .font(.system(size: 9))
                    .foregroundColor(Theme.footGray)
                Spacer(minLength: 0)
            }
            .contentShape(Rectangle())
            .padding(.vertical, 2)
        }
        .buttonStyle(.plain)
    }
}
