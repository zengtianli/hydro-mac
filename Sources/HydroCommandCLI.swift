import Foundation

enum HydroCommandCLI {
    static let verbs = ["status", "version", "list", "calc"]
    static func handles(_ word: String?) -> Bool { word.map(verbs.contains) ?? false }
    static let help = """
    usage: hydromac <command> [--json]
    Read:
      status                       Version, calculator list, preferences and account shape (read only)
      version                      Bundle version/build/identity
      list [--search text]         Same eight calculator catalogue as the window
      calc <calculator> <action> [--input file.json|-] [--out file.json] [--force]
        Same BackendClient and embedded Rust engine as the window; sample/load-dir/read-excel/run/query,
        aggregate/compare/stats/indicators/convert/detect-columns/run-batch/format-irrigation-tsv/
        format-drainage-tsv/run-pipeline are calculator-specific actions.
    Write:
      calc <calculator> export-excel|write-results|write-output|export-dir --input file.json [--force]
        Payload path/dir is explicit; an existing output requires --force. --out saves returned JSON.
      preferences status | set --calculator <key> --yes
      shortcuts status | set --key <letter> [--modifiers command,shift] --yes | enable|disable --yes
      config status | export -o <file> | import <file> --yes | sync on|off --yes
      update check | options | install --yes
    --json: {ok:true,data:...}; failures {ok:false,error:{code,message}}.
    exit codes: 0 success, 1 operation failed, 2 usage error. Calculations never open a window.
    仅在窗口中：打开「配置与更新」窗口、实际文件选择对话框、键盘焦点与窗口布局。
    无自建账号；可选系统 iCloud 和高德 API key 不构成本产品账号。
    """
    static func run(_ arguments: [String]) async -> Int32 {
        var words = arguments.filter { $0 != "--json" }
        let verb = words.removeFirst()
        func emit(_ value: [String: Any]) { print(String(decoding: try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), as: UTF8.self)) }
        do {
            let info: [String: Any] = ["version": Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") ?? "",
                "build": Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") ?? "", "bundle_id": Bundle.main.bundleIdentifier ?? ""]
            let catalogue = CALCULATORS.map { ["key": $0.key, "name": $0.name] }
            if verb == "status", words.isEmpty {
                emit(["ok": true, "app": info, "calculators": catalogue, "startup_calculator": HydroPreferences.calculator,
                      "account": ["mode": "none"], "map_key_from_environment": ProcessInfo.processInfo.environment["AMAP_KEY"]?.isEmpty == false,
                      "background_service": false]); return 0
            }
            if verb == "version", words.isEmpty { emit(["ok": true, "data": info]); return 0 }
            if verb == "list" {
                var result = catalogue
                if words.count == 2, words[0] == "--search" { result = catalogue.filter { $0.values.contains { $0.localizedCaseInsensitiveContains(words[1]) } } }
                else if !words.isEmpty { throw AppUpdateError("list 只接受 --search <text>。") }
                emit(["ok": true, "data": result]); return 0
            }
            guard verb == "calc", words.count >= 2, CALCULATORS.contains(where: { $0.key == words[0] }) else { throw AppUpdateError("用 hydromac --help 查看命令与计算器。") }
            let calculator = words.removeFirst(), action = words.removeFirst()
            var input: Data?, out: URL?, force = false
            while !words.isEmpty {
                let flag = words.removeFirst()
                if flag == "--force" { force = true; continue }
                guard ["--input", "--out"].contains(flag), !words.isEmpty else { throw AppUpdateError("未知参数或缺少值。") }
                let value = words.removeFirst()
                if flag == "--input" { input = value == "-" ? FileHandle.standardInput.readDataToEndOfFile() : try Data(contentsOf: URL(fileURLWithPath: value)) }
                else { out = URL(fileURLWithPath: value) }
            }
            if let out, FileManager.default.fileExists(atPath: out.path), !force { throw AppUpdateError("输出已存在，须显式 --force。") }
            if let input {
                let object = try JSONSerialization.jsonObject(with: input)
                if ["write-results", "write-output", "export-excel", "export-dir"].contains(action), let d = object as? [String: Any],
                   let path = (d["path"] ?? d["dir"]) as? String, FileManager.default.fileExists(atPath: path), !force {
                    throw AppUpdateError("导出目标已存在，须显式 --force。")
                }
            }
            let data = try await BackendClient.shared.runJSON(calculator, action, inputData: input)
            let value: [String: Any] = ["ok": true, "data": try JSONSerialization.jsonObject(with: JSONEncoder().encode(data))]
            if let out { try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys]).write(to: out, options: .atomic) }
            emit(value); return 0
        } catch {
            emit(["ok": false, "error": ["code": "operation_failed", "message": error.localizedDescription]])
            return verb == "calc" ? 1 : 2
        }
    }
}
