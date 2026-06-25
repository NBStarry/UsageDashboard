import Foundation
import Darwin

enum TailscaleAddress {
    // 取本机 Tailscale IPv4 (100.64.0.0/10)；取不到返回 nil。
    static func current() -> String? {
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddr) == 0, let first = ifaddr else { return nil }
        defer { freeifaddrs(ifaddr) }
        var ptr: UnsafeMutablePointer<ifaddrs>? = first
        while let p = ptr {
            let a = p.pointee.ifa_addr
            if a?.pointee.sa_family == UInt8(AF_INET) {
                var addr = sockaddr_in()
                memcpy(&addr, a, MemoryLayout<sockaddr_in>.size)
                let ip = String(cString: inet_ntoa(addr.sin_addr))
                if ip.hasPrefix("100.") {
                    let second = Int(ip.split(separator: ".")[1]) ?? 0
                    if (64...127).contains(second) { return ip }  // 100.64/10
                }
            }
            ptr = p.pointee.ifa_next
        }
        return nil
    }
}
