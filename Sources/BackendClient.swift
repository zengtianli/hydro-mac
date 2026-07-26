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

    /// hydro-cli 二进制定位,按序:
    ///   1) .app/Contents/Resources/hydro-cli(发布形态,自包含)
    ///   2) 环境变量 HYDRO_CLI_PATH(显式覆盖)
    ///   3) 从可执行文件向上找本仓的 cli/target/release/hydro-cli(开发期:`cargo build --release` 后即可用)
    private func cliPath() throws -> String {
        let fm = FileManager.default
        if let res = Bundle.main.resourceURL?.appendingPathComponent("hydro-cli").path,
           fm.isExecutableFile(atPath: res) {
            return res
        }
        if let env = ProcessInfo.processInfo.environment["HYDRO_CLI_PATH"],
           fm.isExecutableFile(atPath: env) {
            return env
        }
        // 向上回溯找仓根(含 cli/ 的那层),避免写死任何机器相关的绝对路径
        var dir = URL(fileURLWithPath: Bundle.main.executablePath ?? CommandLine.arguments[0])
            .resolvingSymlinksInPath()
            .deletingLastPathComponent()
        for _ in 0..<8 {
            let candidate = dir.appendingPathComponent("cli/target/release/hydro-cli").path
            if fm.isExecutableFile(atPath: candidate) { return candidate }
            let parent = dir.deletingLastPathComponent()
            if parent.path == dir.path { break }
            dir = parent
        }
        throw BackendError.notFound("Resources/hydro-cli · $HYDRO_CLI_PATH · <repo>/cli/target/release/hydro-cli")
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
