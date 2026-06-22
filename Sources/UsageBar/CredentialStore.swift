import Foundation

// 只负责读凭证,不打印 token。
enum CredentialStore {

    // ─── Claude ───────────────────────────────────────────────
    // 1) 钥匙串 "Claude Code-credentials" → claudeAiOauth.accessToken
    // 2) 回退 ~/.claude/.credentials.json
    static func claudeToken() -> String? {
        if let raw = runSecurity(service: "Claude Code-credentials"),
           let tok = parseClaudeOAuth(raw) {
            return tok
        }
        let path = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".claude/.credentials.json")
        if let data = try? Data(contentsOf: path),
           let str = String(data: data, encoding: .utf8),
           let tok = parseClaudeOAuth(str) {
            return tok
        }
        return nil
    }

    private static func parseClaudeOAuth(_ raw: String) -> String? {
        guard let data = raw.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let oauth = obj["claudeAiOauth"] as? [String: Any],
              let tok = oauth["accessToken"] as? String, !tok.isEmpty else { return nil }
        return tok
    }

    private static func runSecurity(service: String) -> String? {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/security")
        proc.arguments = ["find-generic-password", "-s", service, "-w"]
        let pipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = Pipe()
        do {
            try proc.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            proc.waitUntilExit()
            guard proc.terminationStatus == 0 else { return nil }
            let s = String(data: data, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            return (s?.isEmpty == false) ? s : nil
        } catch {
            return nil
        }
    }

    // ─── Codex / GPT ──────────────────────────────────────────
    struct CodexCreds { let accessToken: String; let accountId: String }

    enum CodexCredResult {
        case ok(CodexCreds)
        case missingFile          // 未找到 auth.json
        case parseError           // 解析失败
        case incomplete           // 字段不全
    }

    static func codexCreds() -> CodexCredResult {
        let path = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".codex/auth.json")
        guard FileManager.default.fileExists(atPath: path.path) else { return .missingFile }
        guard let data = try? Data(contentsOf: path),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return .parseError }

        let tokens = obj["tokens"] as? [String: Any]
        let access = (tokens?["access_token"] as? String) ?? (obj["access_token"] as? String)
        let account = (tokens?["account_id"] as? String) ?? (obj["account_id"] as? String)
        guard let a = access, let acc = account, !a.isEmpty, !acc.isEmpty else { return .incomplete }
        return .ok(CodexCreds(accessToken: a, accountId: acc))
    }

    // ─── New-API 兼容网关 ─────────────────────────────────────
    // 凭证文件默认放在 ~/.config/usage-bar/<service-id>.json:
    // {"baseUrl","accessToken","userId","quotaPerUnit","currency"}
    struct NewAPICreds {
        let baseURL: String
        let accessToken: String
        let userId: Int
        let quotaPerUnit: Double
        let currency: String
    }

    enum NewAPICredResult {
        case ok(NewAPICreds)
        case missingFile(String)
        case invalidPath
        case incomplete
    }

    static func newAPICreds(fileName: String) -> NewAPICredResult {
        guard let path = newAPICredURL(fileName: fileName) else { return .invalidPath }
        guard FileManager.default.fileExists(atPath: path.path) else { return .missingFile(path.path) }
        guard let data = try? Data(contentsOf: path),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return .incomplete }

        guard let base = (obj["baseUrl"] as? String)?.trimmingCharacters(in: .whitespaces), !base.isEmpty,
              let token = (obj["accessToken"] as? String), !token.isEmpty
        else { return .incomplete }
        let userId = (obj["userId"] as? Int) ?? Int(obj["userId"] as? String ?? "") ?? 0
        let qpu = (obj["quotaPerUnit"] as? Double)
            ?? (obj["quotaPerUnit"] as? Int).map(Double.init)
            ?? 500000
        let currency = (obj["currency"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "$"
        return .ok(NewAPICreds(baseURL: base, accessToken: token, userId: userId,
                               quotaPerUnit: qpu, currency: currency))
    }

    private static func newAPICredURL(fileName: String) -> URL? {
        let clean = fileName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !clean.isEmpty, !clean.hasPrefix("/"), !clean.contains("..") else { return nil }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/usage-bar", isDirectory: true)
            .appendingPathComponent(clean)
    }
}
