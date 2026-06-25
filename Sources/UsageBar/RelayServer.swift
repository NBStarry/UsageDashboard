import Foundation
import Network

final class RelayServer: @unchecked Sendable {
    private let settings: RelaySettings
    private let snapshotProvider: () -> Data
    private var listener: NWListener?

    init(settings: RelaySettings, snapshotProvider: @escaping () -> Data) {
        self.settings = settings
        self.snapshotProvider = snapshotProvider
    }

    func start() {
        guard listener == nil else { return }
        let params = NWParameters.tcp
        params.allowLocalEndpointReuse = true
        guard let port = NWEndpoint.Port(rawValue: UInt16(settings.port)),
              let l = try? NWListener(using: params, on: port) else { return }
        l.newConnectionHandler = { [weak self] conn in self?.handle(conn) }
        l.start(queue: .global(qos: .utility))
        listener = l
    }

    func stop() { listener?.cancel(); listener = nil }

    var boundAddressDescription: String {
        listener?.debugDescription ?? "not started"
    }

    private func handle(_ conn: NWConnection) {
        conn.start(queue: .global(qos: .utility))
        conn.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, _, _ in
            guard let self, let data, let req = String(data: data, encoding: .utf8) else {
                conn.cancel(); return
            }
            let response = self.route(req)
            conn.send(content: response, completion: .contentProcessed { _ in conn.cancel() })
        }
    }

    private func route(_ req: String) -> Data {
        let firstLine = req.split(separator: "\r\n", maxSplits: 1).first.map(String.init) ?? ""
        let parts = firstLine.split(separator: " ")
        let method = parts.count > 0 ? String(parts[0]) : ""
        let path = parts.count > 1 ? String(parts[1]) : ""
        let authed = req.range(of: "Authorization: Bearer \(settings.secret)") != nil
        if method == "GET" && path == "/usage" {
            if !authed { return httpResponse(401, "{\"error\":\"unauthorized\"}".data(using: .utf8)!) }
            return httpResponse(200, snapshotProvider())
        }
        return httpResponse(404, "{\"error\":\"not found\"}".data(using: .utf8)!)
    }

    private func httpResponse(_ code: Int, _ body: Data) -> Data {
        let reason = code == 200 ? "OK" : (code == 401 ? "Unauthorized" : "Not Found")
        let header = "HTTP/1.1 \(code) \(reason)\r\n"
            + "Content-Type: application/json; charset=utf-8\r\n"
            + "Content-Length: \(body.count)\r\n"
            + "Access-Control-Allow-Origin: *\r\n"
            + "Connection: close\r\n\r\n"
        return Data(header.utf8) + body
    }
}
