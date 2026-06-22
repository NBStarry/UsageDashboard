import SwiftUI

struct ProgressBarView: View {
    let pct: Double   // 0–100

    var body: some View {
        let clamped = min(100, max(0, pct))
        let color = Theme.barColor(clamped)
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.white.opacity(0.12))
                RoundedRectangle(cornerRadius: 4)
                    .fill(color)
                    .frame(width: geo.size.width * clamped / 100)
                    .animation(.easeOut(duration: 0.4), value: clamped)
            }
        }
        .frame(height: 7)
    }
}
