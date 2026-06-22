import Foundation

// 极薄的 GET JSON 封装。返回 (解析后的 JSON, HTTP 状态码)。
// 网络层失败抛错;HTTP 状态码不在此判定,交给调用方按业务区分(401 等)。
enum Http {
    static let timeout: TimeInterval = 12

    enum HttpError: Error { case network(String); case badPayload }

    static func getJSON(_ urlString: String, headers: [String: String]) async throws -> (Any, Int) {
        guard let url = URL(string: urlString) else { throw HttpError.network("URL 非法") }
        var req = URLRequest(url: url, timeoutInterval: timeout)
        req.httpMethod = "GET"
        for (k, v) in headers { req.setValue(v, forHTTPHeaderField: k) }

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(for: req)
        } catch {
            throw HttpError.network(error.localizedDescription)
        }
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard let json = try? JSONSerialization.jsonObject(with: data) else {
            throw HttpError.badPayload
        }
        return (json, status)
    }
}
