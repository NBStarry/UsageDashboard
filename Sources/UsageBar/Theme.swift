import SwiftUI

extension Color {
    // 从 "#RRGGBB" 解析。失败回退灰色。
    init(hex: String) {
        let s = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#")).uppercased()
        var v: UInt64 = 0
        guard s.count == 6, Scanner(string: s).scanHexInt64(&v) else {
            self = Color(white: 0.56); return
        }
        self.init(.sRGB,
                  red: Double((v >> 16) & 0xFF) / 255,
                  green: Double((v >> 8) & 0xFF) / 255,
                  blue: Double(v & 0xFF) / 255)
    }
}

enum Theme {
    // 进度条阈值色:<75 绿 / 75–90 琥珀 / ≥90 红。
    static func barColor(_ pct: Double) -> Color {
        if pct >= 90 { return Color(hex: "#F85149") }
        if pct >= 75 { return Color(hex: "#D29922") }
        return Color(hex: "#3FB950")
    }

    // 告警严重度色:critical(5h)=红,warning(周)=橙。
    static func severityColor(_ severity: AlertSeverity) -> Color {
        switch severity {
        case .critical: return red
        case .warning:  return Color(hex: "#FF9F0A")
        }
    }

    static let cardBg = Color(.sRGB, red: 28/255, green: 28/255, blue: 30/255, opacity: 0.9)
    static let labelGray = Color(hex: "#D8D8DA")
    static let subGray = Color(hex: "#8E8E93")
    static let footGray = Color(hex: "#6E6E73")
    static let red = Color(hex: "#F85149")
    static let amber = Color(hex: "#D29922")
}

// 时间格式辅助。
enum TimeFmt {
    static func hm(_ date: Date?) -> String? {
        guard let date else { return nil }
        let f = DateFormatter()
        f.dateFormat = "HH:mm"
        return f.string(from: date)
    }

    // "重置 Xh Ym 后" / "重置 Ym 后"。
    static func resetCountdown(_ date: Date?) -> String? {
        guard let date else { return nil }
        let secs = max(0, Int(date.timeIntervalSinceNow))
        let h = secs / 3600, m = (secs % 3600) / 60
        return h > 0 ? "重置 \(h)h \(m)m 后" : "重置 \(m)m 后"
    }
}
