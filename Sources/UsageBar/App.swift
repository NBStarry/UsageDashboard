import AppKit
import SwiftUI
import UserNotifications

@main
struct UsageBarApp {
    @MainActor
    static func main() {
        // 隐藏 CLI 模式:--fetch <claude|codex>,跑一次取数并打印 JSON 后退出。
        // 用于无头验证取数层,与 GUI 共用同一套 Fetcher。
        let args = CommandLine.arguments
        if args.count >= 3, args[1] == "--fetch" {
            runFetchCLI(service: args[2])
            return
        }
        // 无头开/关开机自启(注册的是当前运行二进制所属的 .app)。
        if args.count >= 2, args[1] == "--register-login" {
            let ok = LaunchAtLogin.set(true)
            print(ok ? "开机自启:已开启 (\(LaunchAtLogin.isEnabled))" : "开机自启:注册失败")
            return
        }
        if args.count >= 2, args[1] == "--unregister-login" {
            let ok = LaunchAtLogin.set(false)
            print(ok ? "开机自启:已关闭" : "开机自启:取消失败")
            return
        }
        // 调试:离屏渲染 PhanRouter 卡(模型列表展开)到 /tmp/usagebar_preview.png。
        if args.count >= 2, args[1] == "--render" {
            renderPreview()
            return
        }

        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.setActivationPolicy(.accessory)   // 无 Dock 图标的菜单栏 App
        app.run()
    }

    @MainActor
    static func renderPreview() {
        let sem = DispatchSemaphore(value: 0)
        var info: BalanceInfo?
        let config = AppConfig.default.services.first { $0.id == "phanrouter" }
        Task.detached {
            if let config, let fetcher = makeFetcher(for: config),
               case .success(let u) = await fetcher.fetch() {
                info = u.balance
            }
            sem.signal()
        }
        sem.wait()
        guard let info else { print("无 PhanRouter 数据"); return }

        let card = VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Circle().fill(Color(hex: "#7C5CFC")).frame(width: 9, height: 9)
                Text("PhanRouter").font(.system(size: 14, weight: .semibold)).foregroundColor(.white)
                Spacer()
            }.padding(.bottom, 12)
            BalanceCardBody(info: info, accent: Color(hex: "#7C5CFC"), forceExpand: true)
        }
        .padding(.horizontal, 16).padding(.vertical, 14)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.cardBg))
        .frame(width: 320)
        .padding(20)
        .background(Color(white: 0.12))

        let renderer = ImageRenderer(content: card)
        renderer.scale = 2
        guard let img = renderer.nsImage,
              let tiff = img.tiffRepresentation,
              let rep = NSBitmapImageRep(data: tiff),
              let png = rep.representation(using: .png, properties: [:]) else {
            print("渲染失败"); return
        }
        let url = URL(fileURLWithPath: "/tmp/usagebar_preview.png")
        try? png.write(to: url)
        print("已渲染 \(url.path)")
    }

    @MainActor
    static func runFetchCLI(service: String) {
        guard let fetcher = makeFetcher(for: service) else {
            print("{\"ok\":false,\"error\":\"未知服务: \(service)\"}")
            return
        }
        let sem = DispatchSemaphore(value: 0)
        Task.detached {
            let outcome = await fetcher.fetch()
            switch outcome {
            case .success(let u):
                if let b = u.balance {
                    let vendors = b.groupedModels.map { "\($0.vendor):\($0.models.count)" }.joined(separator: ", ")
                    print("{\"ok\":true,\"service\":\"\(service)\",\"balance\":\(b.balance),\"used\":\(b.used),\"requestCount\":\(b.requestCount ?? 0),\"modelCount\":\(b.models.count),\"vendors\":\"\(vendors)\"}")
                } else {
                    let wins = u.windows.map {
                        "{\"label\":\"\($0.label)\",\"pct\":\($0.pct),\"resetAt\":\"\($0.resetAt?.description ?? "")\"}"
                    }.joined(separator: ",")
                    print("{\"ok\":true,\"service\":\"\(service)\",\"plan\":\"\(u.plan ?? "")\",\"windows\":[\(wins)]}")
                }
            case .failure(let msg):
                print("{\"ok\":false,\"service\":\"\(service)\",\"error\":\"\(msg)\"}")
            }
            sem.signal()
        }
        sem.wait()
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var store: UsageStore!
    private var menuBar: MenuBarController!

    func applicationDidFinishLaunching(_ notification: Notification) {
        UNUserNotificationCenter.current().delegate = self
        let config = AppConfigStore.load()
        store = UsageStore(config: config)
        menuBar = MenuBarController(store: store)
        store.start()
    }
}

extension AppDelegate: UNUserNotificationCenterDelegate {
    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            willPresent notification: UNNotification) async
    -> UNNotificationPresentationOptions {
        [.banner, .list, .sound]
    }
}
