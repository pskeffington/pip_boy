import Foundation

@main
struct DashboardEndpointTests {
    static func main() {
        let valid = [
            "Open http://127.0.0.1:1/",
            "Open http://127.0.0.1:65535/",
            "Open http://127.0.0.1:58642/",
        ]
        let invalid = [
            "Open https://127.0.0.1:58642/",
            "Open http://127.0.0.1:58642.evil.example/",
            "Open http://127.0.0.1:58642@evil.example/",
            "Open http://user@127.0.0.1:58642/",
            "Open http://127.0.0.1:0/",
            "Open http://127.0.0.1:65536/",
            "Open http://127.0.0.1:58642/micro",
            "Open http://127.0.0.1:58642/?next=https://evil.example/",
            "Open http://127.0.0.1:58642/#fragment",
            "Ready http://127.0.0.1:58642/",
        ]

        for line in valid where DashboardEndpoint.rootURL(from: line) == nil {
            fatalError("Rejected valid loopback endpoint: \(line)")
        }
        for line in invalid where DashboardEndpoint.rootURL(from: line) != nil {
            fatalError("Accepted invalid dashboard endpoint: \(line)")
        }
        print("PASS dashboard endpoint accepts only a plain IPv4 loopback root URL")
    }
}
