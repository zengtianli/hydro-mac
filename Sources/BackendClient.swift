import Foundation

/// 经 Foundation.Process 调 vendored Rust `hydro-cli`(JSON 经 stdin 入、JSON 信封经 stdout 出)。
/// Tier-B 瘦视图:Swift 零业务计算,全委托 Rust。
actor BackendClient {
    static let shared = BackendClient()

    enum BackendError: LocalizedError {
        case notFound(String)
        case decode(String)
        case backend(String)
        var errorDescription: String? {
            switch self {
            case .notFound(let p): return "找不到 hydro-cli 后端: \(p)"
            case .decode(let m): return "解析后端输出失败: \(m)"
            case .backend(let m): return m
            }
        }
    }

    /// hydro-cli 二进制定位:优先 .app/Contents/Resources/hydro-cli,开发期 fallback 到 cli/target/release。
    private func cliPath() throws -> String {
        if let res = Bundle.main.resourceURL?.appendingPathComponent("hydro-cli").path,
           FileManager.default.isExecutableFile(atPath: res) {
            return res
        }
        let dev = ("~/Dev/apps/desktop/hydro-mac/cli/target/release/hydro-cli" as NSString).expandingTildeInPath
        if FileManager.default.isExecutableFile(atPath: dev) { return dev }
        throw BackendError.notFound(dev)
    }

    /// 无输入调用。
    func run<O: Decodable>(_ calc: String, _ action: String, as: O.Type) async throws -> O {
        try await runRaw(calc, action, inputData: nil)
    }

    /// 带 Encodable 输入调用。
    func run<I: Encodable, O: Decodable>(_ calc: String, _ action: String, input: I, as: O.Type) async throws -> O {
        let data = try JSONEncoder().encode(input)
        return try await runRaw(calc, action, inputData: data)
    }

    private func runRaw<O: Decodable>(_ calc: String, _ action: String, inputData: Data?) async throws -> O {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: try cliPath())
        process.arguments = [calc, action]

        let stdinPipe = Pipe()
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        try process.run()

        if let inputData {
            stdinPipe.fileHandleForWriting.write(inputData)
        }
        stdinPipe.fileHandleForWriting.closeFile()

        let outData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        _ = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        do {
            let env = try JSONDecoder().decode(Envelope<O>.self, from: outData)
            if env.ok, let payload = env.data {
                return payload
            }
            throw BackendError.backend(env.error ?? "后端返回 ok=false 但无 error")
        } catch let e as BackendError {
            throw e
        } catch {
            let raw = String(data: outData, encoding: .utf8) ?? "(非 UTF-8)"
            throw BackendError.decode("\(error.localizedDescription) — 原始输出: \(raw.prefix(300))")
        }
    }
}
