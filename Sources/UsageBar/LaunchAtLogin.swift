import Foundation
import ServiceManagement

// 开机自启,基于 SMAppService(macOS 13+ 原生登录项 API)。
enum LaunchAtLogin {
    static var isEnabled: Bool {
        SMAppService.mainApp.status == .enabled
    }

    @discardableResult
    static func set(_ on: Bool) -> Bool {
        do {
            if on {
                if SMAppService.mainApp.status != .enabled {
                    try SMAppService.mainApp.register()
                }
            } else {
                if SMAppService.mainApp.status == .enabled {
                    try SMAppService.mainApp.unregister()
                }
            }
            return true
        } catch {
            NSLog("LaunchAtLogin 设置失败: \(error.localizedDescription)")
            return false
        }
    }
}
