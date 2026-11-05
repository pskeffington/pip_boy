import Foundation

enum DashboardEndpoint {
    static func rootURL(from outputLine: String) -> URL? {
        let prefix = "Open "
        guard outputLine.hasPrefix(prefix),
              let url = URL(string: String(outputLine.dropFirst(prefix.count))),
              url.scheme == "http",
              url.host == "127.0.0.1",
              url.user == nil,
              url.password == nil,
              let port = url.port,
              (1...65_535).contains(port),
              url.path == "/",
              url.query == nil,
              url.fragment == nil,
              url.absoluteString == "http://127.0.0.1:\(port)/" else {
            return nil
        }
        return url
    }
}
