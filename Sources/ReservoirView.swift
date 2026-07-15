import SwiftUI
import AppKit
import UniformTypeIdentifiers

// MARK: - 本视图自包含类型(全 private,禁塞 Models.swift)

/// 任意 JSON 树 —— 无损往返:CLI 给的 ReservoirInput/ScheduleOutput 原样持有、原样回传,
/// Swift 不镜像 Rust 的 40+ 字段结构(部分解码再编码会静默丢字段)。
/// 整数与浮点分 case 保存,保证 Rust usize 字段(max_iterations 等)回传仍是整数字面量。
private enum ResJSON: Codable {
    case null
    case bool(Bool)
    case int(Int)
    case double(Double)
    case string(String)
    case array([ResJSON])
    case object([String: ResJSON])

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() {
            self = .null
        } else if let b = try? c.decode(Bool.self) {
            self = .bool(b)
        } else if let i = try? c.decode(Int.self) {
            self = .int(i)
        } else if let d = try? c.decode(Double.self) {
            self = .double(d)
        } else if let s = try? c.decode(String.self) {
            self = .string(s)
        } else if let a = try? c.decode([ResJSON].self) {
            self = .array(a)
        } else if let o = try? c.decode([String: ResJSON].self) {
            self = .object(o)
        } else {
            throw DecodingError.dataCorruptedError(in: c, debugDescription: "未知 JSON 形态")
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .null: try c.encodeNil()
        case .bool(let b): try c.encode(b)
        case .int(let i): try c.encode(i)
        case .double(let d): try c.encode(d)
        case .string(let s): try c.encode(s)
        case .array(let a): try c.encode(a)
        case .object(let o): try c.encode(o)
        }
    }

    subscript(key: String) -> ResJSON? {
        if case .object(let o) = self { return o[key] }
        return nil
    }
    var stringValue: String? { if case .string(let s) = self { return s }; return nil }
    var intValue: Int? {
        switch self {
        case .int(let i): return i
        case .double(let d) where d == d.rounded(): return Int(d)
        default: return nil
        }
    }
    var doubleValue: Double? {
        switch self {
        case .double(let d): return d
        case .int(let i): return Double(i)
        default: return nil
        }
    }
    var boolValue: Bool? { if case .bool(let b) = self { return b }; return nil }
    var arrayValue: [ResJSON]? { if case .array(let a) = self { return a }; return nil }

    mutating func set(_ key: String, _ value: ResJSON) {
        if case .object(var o) = self { o[key] = value; self = .object(o) }
    }

    /// 转 CellValue 给共享 DataTableView 渲染。
    var cell: CellValue {
        switch self {
        case .string(let s): return .string(s)
        case .int(let i): return .number(Double(i))
        case .double(let d): return .number(d)
        case .bool(let b): return .bool(b)
        default: return .null
        }
    }
}

private struct ResPathRequest: Encodable { let path: String }
private struct ResRunRequest: Encodable { let input: ResJSON }
private struct ResWriteRequest: Encodable {
    let path: String
    let output: ResJSON
}
private struct ResWriteResponse: Decodable { let path: String }

/// 上/下游选择
private enum ResSide: String, CaseIterable {
    case upstream, downstream
    var label: String { self == .upstream ? "上游" : "下游" }
}

/// 结果粒度
private enum ResTab: String, CaseIterable {
    case summary, daily, monthly, yearly, hydroYearly
    var label: String {
        switch self {
        case .summary: return "汇总"
        case .daily: return "逐日"
        case .monthly: return "逐月"
        case .yearly: return "逐年"
        case .hydroYearly: return "水文年"
        }
    }
    var key: String {
        switch self {
        case .summary: return "summary"
        case .daily: return "daily"
        case .monthly: return "monthly"
        case .yearly: return "yearly"
        case .hydroYearly: return "hydro_yearly"
        }
    }
}

/// 逐日表列(表头, JSON 字段) —— 与 Rust DailyResult 字段名对齐,取核心列。
private let RES_DAILY_COLS: [(String, String)] = [
    ("日期", "date"), ("来水流量", "q_in"), ("发电流量", "q_gen"), ("弃水流量", "q_thrown"),
    ("损失流量", "q_loss"), ("末库容", "v_end"), ("末水位", "z_end"), ("水头", "h_net"),
    ("出力(kW)", "power"), ("生态供水", "eco_supply"), ("生态缺口", "eco_lack"),
    ("补水后-发电流量", "q_gen_after"), ("补水后-末库容", "v_end_after"),
    ("补水后-末水位", "z_end_after"), ("补水后-出力(kW)", "power_after"),
    ("补水1", "supplement_q1"), ("补水2", "supplement_q2"), ("补水3", "supplement_q3"),
]

/// 水库群调度 —— reservoir 计算器视图(上下游梯级水库逐日调节计算,已接入 hydro-cli)。
struct ReservoirView: View {
    @State private var inputRaw: ResJSON?
    @State private var outputRaw: ResJSON?
    @State private var sourceLabel = "未加载"
    @State private var maxIterations = 2
    @State private var upEcoAsInflow = true
    @State private var loading = false
    @State private var errorMessage: String?
    @State private var exportMessage: String?
    @State private var side: ResSide = .upstream
    @State private var tab: ResTab = .summary

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("水库群调度")
    }

    // MARK: - 头部:数据源 + 参数 + 运行

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Button { Task { await loadSample() } } label: {
                    Label("加载示例数据", systemImage: "sparkles")
                }
                Button { Task { await loadExcel() } } label: {
                    Label("打开 Excel 数据", systemImage: "doc.badge.arrow.up")
                }
                Spacer()
                Text(sourceLabel)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            if inputRaw != nil {
                HStack(spacing: 14) {
                    Text(inputSummary)
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    Stepper("迭代 \(maxIterations) 次", value: $maxIterations, in: 1...10)
                        .font(.callout)
                        .fixedSize()
                    Toggle("上库生态水入下库", isOn: $upEcoAsInflow)
                        .font(.callout)
                        .toggleStyle(.checkbox)
                    Button { Task { await runSchedule() } } label: {
                        Label("运行调度", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading)
                    if outputRaw != nil {
                        Button { Task { await exportExcel() } } label: {
                            Label("导出 Excel", systemImage: "square.and.arrow.down")
                        }
                        .disabled(loading)
                    }
                    Spacer()
                }
            }
            if outputRaw != nil {
                HStack(spacing: 12) {
                    Picker("水库", selection: $side) {
                        ForEach(ResSide.allCases, id: \.self) { s in
                            Text("\(s.label) · \(reservoirName(s))").tag(s)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: 340)
                    Picker("视图", selection: $tab) {
                        ForEach(ResTab.allCases, id: \.self) { Text($0.label).tag($0) }
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: 340)
                    Spacer()
                    if let exportMessage {
                        Text(exportMessage)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .textSelection(.enabled)
                    }
                }
            }
        }
        .padding(16)
    }

    private var inputSummary: String {
        guard let input = inputRaw else { return "" }
        let up = input["params"]?["up_name"]?.stringValue ?? "上库"
        let down = input["params"]?["down_name"]?.stringValue ?? "下库"
        let days = input["upstream"]?["dates"]?.arrayValue?.count ?? 0
        return "\(up) → \(down) · \(days) 天系列"
    }

    private func reservoirName(_ s: ResSide) -> String {
        outputRaw?[s.rawValue]?["name"]?.stringValue ?? (s == .upstream ? "上库" : "下库")
    }

    // MARK: - 主体

    @ViewBuilder private var content: some View {
        if loading {
            centered { ProgressView().controlSize(.large); Text("计算中…").foregroundStyle(.secondary) }
        } else if let errorMessage {
            centered {
                Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 30)).foregroundStyle(.orange)
                Text(errorMessage).foregroundStyle(.secondary).multilineTextAlignment(.center).textSelection(.enabled)
            }
        } else if outputRaw != nil, let table = buildTable() {
            DataTableView(result: table).padding(16)
        } else if inputRaw != nil {
            centered {
                Image(systemName: "play.circle").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("数据已加载,点「运行调度」计算").foregroundStyle(.secondary)
            }
        } else {
            centered {
                Image(systemName: "water.waves").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或 Excel 数据后运行调度").foregroundStyle(.secondary)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    // MARK: - 结果表构建(直接遍历 JSON 树,列序固定)

    private func buildTable() -> TableResult? {
        guard let res = outputRaw?[side.rawValue] else { return nil }
        switch tab {
        case .summary:
            guard let pairs = res["summary"]?.arrayValue else { return nil }
            let rows: [[CellValue]] = pairs.compactMap { p in
                guard let a = p.arrayValue, a.count == 2 else { return nil }
                return [a[0].cell, a[1].cell]
            }
            return TableResult(headers: ["指标", "值"], rows: rows, summary: "\(reservoirName(side)) · 全期汇总")
        case .daily:
            guard let daily = res["daily"]?.arrayValue else { return nil }
            let rows: [[CellValue]] = daily.map { d in
                RES_DAILY_COLS.map { (_, key) in d[key]?.cell ?? .null }
            }
            return TableResult(
                headers: RES_DAILY_COLS.map { $0.0 },
                rows: rows,
                summary: "\(reservoirName(side)) · 逐日过程 · 共 \(rows.count) 天"
            )
        case .monthly:
            return namedValuesTable(res["monthly"]?.arrayValue, rowKey: "year_month", rowHeader: "年-月", label: "逐月过程")
        case .yearly:
            return namedValuesTable(res["yearly"]?.arrayValue, rowKey: "year", rowHeader: "年", label: "逐年过程")
        case .hydroYearly:
            return namedValuesTable(res["hydro_yearly"]?.arrayValue, rowKey: "year", rowHeader: "水文年", label: "水文年过程")
        }
    }

    /// monthly/yearly/hydro_yearly 共形:{ rowKey, values: [[列名, 值], …] }
    private func namedValuesTable(_ items: [ResJSON]?, rowKey: String, rowHeader: String, label: String) -> TableResult? {
        guard let items, let first = items.first, let firstVals = first["values"]?.arrayValue else { return nil }
        let colNames: [String] = firstVals.compactMap { $0.arrayValue?.first?.stringValue }
        let rows: [[CellValue]] = items.map { item in
            var row: [CellValue] = [item[rowKey]?.cell ?? .null]
            let vals = item["values"]?.arrayValue ?? []
            row += vals.compactMap { pair -> CellValue? in
                guard let a = pair.arrayValue, a.count == 2 else { return nil }
                return a[1].cell
            }
            return row
        }
        return TableResult(
            headers: [rowHeader] + colNames,
            rows: rows,
            summary: "\(reservoirName(side)) · \(label) · 共 \(rows.count) 行"
        )
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("reservoir", "sample", as: ResJSON.self) }
        sourceLabel = "示例数据(程序生成 366 天)"
    }

    private func loadExcel() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        if let xlsx = UTType(filenameExtension: "xlsx") { panel.allowedContentTypes = [xlsx] }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load {
            try await BackendClient.shared.run(
                "reservoir", "read-excel",
                input: ResPathRequest(path: url.path), as: ResJSON.self
            )
        }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> ResJSON) async {
        loading = true; errorMessage = nil; outputRaw = nil; exportMessage = nil
        defer { loading = false }
        do {
            let input = try await op()
            inputRaw = input
            maxIterations = input["params"]?["max_iterations"]?.intValue ?? 2
            upEcoAsInflow = input["params"]?["up_eco_as_inflow"]?.boolValue ?? true
        } catch {
            inputRaw = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runSchedule() async {
        guard var input = inputRaw else { return }
        // 参数区的编辑写回 JSON 树后原样送 Rust
        if var params = input["params"] {
            params.set("max_iterations", .int(maxIterations))
            params.set("up_eco_as_inflow", .bool(upEcoAsInflow))
            input.set("params", params)
            inputRaw = input
        }
        loading = true; errorMessage = nil; exportMessage = nil
        defer { loading = false }
        do {
            outputRaw = try await BackendClient.shared.run(
                "reservoir", "run",
                input: ResRunRequest(input: input), as: ResJSON.self
            )
        } catch {
            outputRaw = nil
            errorMessage = error.localizedDescription
        }
    }

    private func exportExcel() async {
        guard let output = outputRaw else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "水库群调度结果.xlsx"
        if let xlsx = UTType(filenameExtension: "xlsx") { panel.allowedContentTypes = [xlsx] }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let resp = try await BackendClient.shared.run(
                "reservoir", "write-results",
                input: ResWriteRequest(path: url.path, output: output), as: ResWriteResponse.self
            )
            exportMessage = "已导出 → \(resp.path)"
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
