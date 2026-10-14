import Foundation
import ObjectiveC

/// Disposable self-test identity intercepts transport; production uses the shared GitHub checker unchanged.
final class ProductBasicsTransport: URLProtocol {
    static func installIfIsolated() {
        let env = ProcessInfo.processInfo.environment
        guard Bundle.main.bundleIdentifier?.hasPrefix("test.hydromac.basics.") == true,
              let support = env["APP_LIFECYCLE_SUPPORT_DIR"], !support.isEmpty,
              let cloud = env["APP_LIFECYCLE_CLOUD_DIR"], !cloud.isEmpty,
              let out = env["HYDROMAC_BASICS_OUT"], !out.isEmpty else { return }
        // Ephemeral sessions do not consume global URLProtocol registration on macOS.
        // Inject the fixture only in this disposable process, retaining the production checker.
        guard let original = class_getClassMethod(URLSessionConfiguration.self, NSSelectorFromString("ephemeralSessionConfiguration")),
              let isolated = class_getClassMethod(URLSessionConfiguration.self, NSSelectorFromString("hydromac_test_ephemeral")) else { fatalError("测试 HTTPS 配置入口缺失") }
        method_exchangeImplementations(original, isolated)
    }
    override class func canInit(with request: URLRequest) -> Bool {
        request.url?.scheme == "https" && request.url?.host == "api.github.com"
    }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        guard let url = request.url, url.path == "/repos/zengtianli/hydro-mac/releases/latest",
              let support = ProcessInfo.processInfo.environment["APP_LIFECYCLE_SUPPORT_DIR"] else {
            client?.urlProtocol(self, didFailWithError: URLError(.unsupportedURL)); return
        }
        let file = URL(fileURLWithPath: support).appendingPathComponent("github-release.json")
        do {
            let data = try Data(contentsOf: file)
            let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: ["Content-Type": "application/json"])!
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch { client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet)) }
    }
    override func stopLoading() {}
}

extension URLSessionConfiguration {
    @objc dynamic class func hydromac_test_ephemeral() -> URLSessionConfiguration {
        // After the exchange this calls Foundation's original ephemeral factory.
        let configuration = hydromac_test_ephemeral()
        configuration.protocolClasses = [ProductBasicsTransport.self]
        return configuration
    }
}
