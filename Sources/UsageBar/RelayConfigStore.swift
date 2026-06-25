import Foundation
import CryptoKit

struct RelaySettings: Codable {
    var port: Int
    var secret: String
}

enum RelayConfigStore {
    static var fileURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/usage-bar/relay.json")
    }

    static func loadOrCreate() -> RelaySettings {
        let url = fileURL
        if let data = try? Data(contentsOf: url),
           let s = try? JSONDecoder().decode(RelaySettings.self, from: data) {
            return s
        }
        let secret = randomSecret()
        let s = RelaySettings(port: 8787, secret: secret)
        write(s)
        return s
    }

    private static func randomSecret() -> String {
        let key = SymmetricKey(size: .bits256)
        let data = key.withUnsafeBytes { Data($0) }
        return data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    private static func write(_ s: RelaySettings) {
        let url = fileURL
        try? FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted]
        if let data = try? enc.encode(s) {
            try? data.write(to: url)
            try? FileManager.default.setAttributes(
                [.posixPermissions: 0o600], ofItemAtPath: url.path)
        }
    }
}
