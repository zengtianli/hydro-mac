import AppKit
import SwiftUI

/// `HydroMac --ui-self-test <outdir>`：进程内离屏界面自检（验收 native_ui 用）。
/// 不建 WindowGroup 窗口、不激活、不开菜单、不合成任何输入事件：
///   1. ContentView 与 8 个计算器视图用 NSHostingView 离屏布局并渲染 PNG，断言尺寸 > 0 且位图非纯色；
///   2. 经 BackendClient（与视图按钮相同的调用入口）调内嵌 hydro-cli 各计算器 sample 与 run，断言关键条目非空；
///   3. 命令面板用 ContentView 真实条目驱动 PaletteModel 过滤/选择；不存在的后端路径须抛可读错误。
/// stdout 最后一行是单行 JSON；ok=false 时退出码 1。
enum UISelfTest {
    /// 任意 JSON：sample 原样回送 run，保持与 CLI 契约逐字段一致（视图私有模型在各自文件内不可见）。
    indirect enum JSON: Codable {
        case null, bool(Bool), number(Double), string(String), array([JSON]), object([String: JSON])
        init(from decoder: Decoder) throws {
            let c = try decoder.singleValueContainer()
            if c.decodeNil() { self = .null }
            else if let b = try? c.decode(Bool.self) { self = .bool(b) }
            else if let d = try? c.decode(Double.self) { self = .number(d) }
            else if let s = try? c.decode(String.self) { self = .string(s) }
            else if let a = try? c.decode([JSON].self) { self = .array(a) }
            else { self = .object(try c.decode([String: JSON].self)) }
        }
        func encode(to encoder: Encoder) throws {
            var c = encoder.singleValueContainer()
            switch self {
            case .null: try c.encodeNil()
            case .bool(let b): try c.encode(b)
            case .number(let d): try c.encode(d)
            case .string(let s): try c.encode(s)
            case .array(let a): try c.encode(a)
            case .object(let o): try c.encode(o)
            }
        }
        subscript(_ key: String) -> JSON? { if case .object(let o) = self { return o[key] }; return nil }
        var count: Int {
            switch self {
            case .array(let a): return a.count
            case .object(let o): return o.count
            case .string(let s): return s.count
            default: return 0
            }
        }
        var number: Double? { if case .number(let d) = self { return d }; return nil }
    }

    /// 后台任务结果；任务内顺序写入，主线程在 done 置位后才读取。
    private final class Box: @unchecked Sendable {
        private let lock = NSLock()
        private var _results: [String: Bool] = [:], _notes: [String: String] = [:], _bad = "", _done = false
        private func sync<T>(_ f: () -> T) -> T { lock.lock(); defer { lock.unlock() }; return f() }
        var results: [String: Bool] { get { sync { _results } } set { sync { _results = newValue } } }
        var notes: [String: String] { get { sync { _notes } } set { sync { _notes = newValue } } }
        var badPathError: String { get { sync { _bad } } set { sync { _bad = newValue } } }
        var done: Bool { get { sync { _done } } set { sync { _done = newValue } } }
    }

    private struct Wrap: Encodable { let input: JSON }
    private struct IrrigationReq: Encodable { let contents: JSON; let mode: String }
    private struct RainfallReq: Encodable { let input: RainfallPipelineInput; let steps: [UInt32] }
    private struct ConvertReq: Encodable { let lng: Double; let lat: Double }

    @MainActor
    static func runAndExit(outDir: String) -> Never {
        guard !outDir.isEmpty, Bundle.main.bundleIdentifier?.hasPrefix("test.hydromac.ui.") == true,
              ProcessInfo.processInfo.environment["AMAP_KEY"] == nil, ProcessInfo.processInfo.environment["AMAP_API_KEY"] == nil else { exit(2) }
        let started = Date()
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let out = URL(fileURLWithPath: outDir, isDirectory: true)
        try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

        var checks: [String: Bool] = [:]
        var screenshots: [String] = []
        var notes: [String: String] = [:]

        // ── 1. 离屏渲染 ────────────────────────────────────────────────
        @discardableResult
        func render<V: View>(_ name: String, _ view: V, width: CGFloat = 1180, height: CGFloat = 760) -> Int {
            let host = NSHostingView(rootView: view.frame(width: width, height: height)
                // cacheDisplay 只绘制视图，不包含 NSWindow 的背景；透明黑字 PNG 无法直接审阅。
                .background(Color(nsColor: .windowBackgroundColor))
                .environment(\.colorScheme, .light))
            host.appearance = NSAppearance(named: .aqua)
            host.frame = NSRect(x: 0, y: 0, width: width, height: height)
            // 离屏无边框窗口：只作为 AppKit 视图树宿主（NavigationSplitView 侧栏的 NSTableView 需要窗口才绘制），
            // 从不 orderFront / makeKey，不出现在屏幕上，也不抢焦点。
            let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.appearance = NSAppearance(named: .aqua)
            window.contentView = host
            defer { window.contentView = nil; window.close() }
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.15))
            host.layoutSubtreeIfNeeded()
            guard host.bounds.width > 0, host.bounds.height > 0,
                  let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else {
                checks["render_\(name)"] = false; return 0
            }
            host.cacheDisplay(in: host.bounds, to: rep)
            var colors = Set<UInt32>(), leftColors = Set<UInt32>()
            var sampled = 0, opaque = 0
            let leftEdge = Int(Double(rep.pixelsWide) * 200 / Double(width))
            for x in stride(from: 0, to: rep.pixelsWide, by: 3) {
                for y in stride(from: 0, to: rep.pixelsHigh, by: 3) {
                    guard let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
                    sampled += 1
                    if c.alphaComponent >= 0.99 { opaque += 1 }
                    let q = UInt32(c.redComponent * 31) << 15 | UInt32(c.greenComponent * 31) << 10
                        | UInt32(c.blueComponent * 31) << 5 | UInt32(c.alphaComponent * 31)
                    colors.insert(q)
                    if x < leftEdge { leftColors.insert(q) }
                }
            }
            let file = "native-ui-\(name).png"
            let written = (try? rep.representation(using: .png, properties: [:])?.write(to: out.appendingPathComponent(file))) != nil
            if written { screenshots.append(file) }
            notes["render_\(name)"] = "\(rep.pixelsWide)x\(rep.pixelsHigh) colors=\(colors.count)"
            checks["render_\(name)"] = written && rep.pixelsWide > 0 && rep.pixelsHigh > 0 && colors.count >= 3
            checks["render_\(name)_opaque_background"] = sampled > 0 && Double(opaque) / Double(sampled) >= 0.99
            return leftColors.count
        }
        let sidebarColors = render("content", ContentView())
        checks["render_content_sidebar_nonblank"] = sidebarColors >= 3   // 侧栏计算器列表确实绘制出来
        notes["render_content_sidebar"] = "colors=\(sidebarColors)"
        render("annual", AnnualView(), width: 960)
        render("capacity", CapacityView(), width: 960)
        render("efficiency", EfficiencyView(), width: 960)
        render("reservoir", ReservoirView(), width: 960)
        render("district", DistrictView(), width: 960)
        render("irrigation", IrrigationView(), width: 960)
        render("rainfall", RainfallView(), width: 960)
        render("geocode", GeocodeView(), width: 960)

        // ── 2. 命令面板（ContentView 真实条目 + 共享 PaletteModel 过滤/选择）──────
        let navigation = HydroNavigation(readPreferences: false)
        let items = ContentView(navigation: navigation).paletteItems
        let model = PaletteModel(items)
        var palette = items.count == CALCULATORS.count
        model.query = "水库"
        palette = palette && model.current?.id == "calc-reservoir"
        model.query = "geocode"   // 只在 keywords 里，验证隐藏搜索词参与匹配
        palette = palette && model.current?.id == "calc-geocode"
        model.query = "降雨"
        palette = palette && model.results.first?.id == "calc-rainfall"
        model.query = "zzqqxx"
        palette = palette && model.results.isEmpty && model.current == nil
        model.query = ""
        palette = palette && model.results.count == items.count
        model.move(-1)
        palette = palette && model.sel == items.count - 1   // ↑ 从首项回绕到末项
        model.move(1)
        palette = palette && model.sel == 0
        model.query = "水库"
        model.current?.run()
        checks["palette_action_navigates_real_content"] = navigation.selection?.key == "reservoir"
        navigation.openDefaultCalculator()
        checks["default_calculator_action"] = navigation.selection?.key == HydroPreferences.calculator
        checks["palette_filter_select"] = palette
        let closeWindow = NSWindow(contentRect: NSRect(x: -20000, y: -20000, width: 1180, height: 760), styleMask: [.titled, .closable], backing: .buffered, defer: false)
        closeWindow.isReleasedWhenClosed = false
        closeWindow.contentView = NSHostingView(rootView: ContentView(navigation: navigation))
        closeWindow.contentView?.layoutSubtreeIfNeeded()
        closeWindow.performClose(nil)
        checks["close_action_without_focus"] = !closeWindow.isVisible && !closeWindow.isKeyWindow && app.activationPolicy() == .prohibited

        // ── 3. 后端动作（BackendClient → 内嵌 hydro-cli）────────────────────
        let box = Box()
        Task.detached {
            let client = BackendClient.shared
            func sample(_ calc: String) async -> JSON? {
                do { return try await client.run(calc, "sample", as: JSON.self) }
                catch { box.notes["\(calc)_sample"] = error.localizedDescription; return nil }
            }
            func record(_ key: String, _ body: () async throws -> (Bool, String)) async {
                do { let (ok, note) = try await body(); box.results[key] = ok; box.notes[key] = note }
                catch { box.results[key] = false; box.notes[key] = error.localizedDescription }
            }
            // annual：真实 Swift 模型（Models.swift）解码 + 查询
            await record("annual_sample_query") {
                let idx = try await client.run("annual", "sample", as: DataIndex.self)
                let table = idx.available_tables.first ?? ""
                let q = try await client.run("annual", "query",
                                             input: QueryRequest(index: idx, input: QueryInput(table: table)), as: QueryOutput.self)
                return (idx.total_files > 0 && !table.isEmpty && !q.headers.isEmpty && !q.rows.isEmpty,
                        "files=\(idx.total_files) headers=\(q.headers.count) rows=\(q.rows.count)")
            }
            for calc in ["capacity", "efficiency", "reservoir", "district"] {
                await record("\(calc)_sample_run") {
                    guard let s = await sample(calc), s.count > 0 else { return (false, "sample 为空") }
                    let r = try await client.run(calc, "run", input: Wrap(input: s), as: JSON.self)
                    let ok: Bool
                    switch calc {
                    case "capacity": ok = (r["result_table"]?.count ?? 0) > 0 && (r["process_table"]?.count ?? 0) > 0
                    case "efficiency": ok = (r["topsis_results"]?.count ?? 0) > 0 && (r["combined_weights"]?.count ?? 0) > 0
                    case "reservoir": ok = (r["upstream"]?.count ?? 0) > 0 && (r["downstream"]?.count ?? 0) > 0
                    default: ok = (r["districts"]?.count ?? 0) > 0 && (r["total_water_demand"]?.number ?? 0) > 0
                    }
                    return (ok, "sample_keys=\(s.count) result_keys=\(r.count)")
                }
            }
            await record("irrigation_sample_run") {
                guard let s = await sample("irrigation"), s.count > 0 else { return (false, "sample 为空") }
                let r = try await client.run("irrigation", "run", input: IrrigationReq(contents: s, mode: "both"), as: JSON.self)
                let n = r["daily_results"]?.count ?? 0
                return (n > 0 && (r["zone_names"]?.count ?? 0) > 0, "daily_results=\(n)")
            }
            // rainfall：真实 Swift 模型（RainfallView.swift 内 internal 类型）解码 + 6 步管线
            await record("rainfall_sample_run") {
                let s = try await client.run("rainfall", "sample", as: RainfallPipelineInput.self)
                let r = try await client.run("rainfall", "run-pipeline", input: RainfallReq(input: s, steps: [1, 2, 3, 4, 5, 6]),
                                             as: RainfallPipelineOutput.self)
                try JSONEncoder().encode(s).write(to: out.appendingPathComponent("rainfall-input.json"))
                try JSONEncoder().encode(r).write(to: out.appendingPathComponent("rainfall-output.json"))
                return (r.steps.count == 6 && r.row_count > 0, "steps=\(r.steps.count) rows=\(r.row_count)")
            }
            // geocode：sample（离线）+ 坐标转换（离线）；run-batch 需高德 key 与联网，不在此覆盖
            await record("geocode_sample_convert") {
                guard let s = await sample("geocode"), (s["rows"]?.count ?? 0) > 0 else { return (false, "sample 为空") }
                let r = try await client.run("geocode", "convert", input: ConvertReq(lng: 120.1551, lat: 30.2741), as: JSON.self)
                let lng = r["gcj_lng"]?.number ?? 0, lat = r["gcj_lat"]?.number ?? 0
                return (abs(lng - 120.1551) > 1e-4 && abs(lng - 120.1551) < 0.02 && abs(lat - 30.2741) < 0.02,
                        String(format: "rows=%d gcj=%.6f,%.6f", s["rows"]?.count ?? 0, lng, lat))
            }
            // 后端缺失：须抛出非空、可读错误
            do { _ = try await BackendClient(executable: "/nonexistent").run("annual", "sample", as: JSON.self) }
            catch { box.badPathError = error.localizedDescription }
            box.done = true
        }
        let deadline = Date().addingTimeInterval(45)
        while !box.done && Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.02)) }
        checks["backend_finished_in_time"] = box.done
        for (k, v) in box.results { checks["backend_\(k)"] = v }
        for (k, v) in box.notes { notes["backend_\(k)"] = v }
        checks["backend_missing_executable_error"] = !box.badPathError.isEmpty
        notes["backend_missing_executable_error"] = box.badPathError

        if let data = try? Data(contentsOf: out.appendingPathComponent("rainfall-input.json")),
           let input = try? JSONDecoder().decode(RainfallPipelineInput.self, from: data),
           let resultData = try? Data(contentsOf: out.appendingPathComponent("rainfall-output.json")),
           let output = try? JSONDecoder().decode(RainfallPipelineOutput.self, from: resultData) {
            let nav = HydroNavigation(readPreferences: false)
            nav.selection = CALCULATORS.first { $0.key == "rainfall" }
            render("rainfall-results", ContentView(navigation: nav, rainfallSnapshot: (input, output)))
        } else { checks["rainfall_result_capture"] = false }

        let elapsed = Date().timeIntervalSince(started)
        checks["elapsed_under_60s"] = elapsed < 60
        let ok = checks.values.allSatisfy { $0 } && box.results.count == 8
        let result: [String: Any] = [
            "ok": ok, "checks": checks, "screenshots": screenshots.sorted(), "notes": notes,
            "elapsed_s": (elapsed * 100).rounded() / 100,
            "not_covered": [
                "物理点击/键盘交互与按钮 → @State 更新（按钮动作为视图私有方法，自检直调同一 BackendClient 入口与请求结构）",
                "视图私有 Codable 模型解码（capacity/efficiency/reservoir/district/irrigation/geocode 以通用 JSON 回送；annual/rainfall 用真实 Swift 模型）",
                "NSOpenPanel/NSSavePanel 打开/导出文件与 Excel 写出",
                "geocode run-batch（需高德 key 与联网）",
                "物理 ⌘K 输入；已直接执行生产命令面板条目动作并验证导航",
                "真实窗口、NavigationSplitView 侧栏选择切换",
            ],
        ]
        if let data = try? JSONSerialization.data(withJSONObject: result, options: .sortedKeys) {
            FileHandle.standardOutput.write(data + Data("\n".utf8))
        }
        exit(ok ? 0 : 1)
    }
}
