import SwiftUI
import AppKit
import UniformTypeIdentifiers

// MARK: - capacity 类型(与 vendored Rust capacity/types.rs 对齐,snake_case;自包含本文件,不进 Models.swift)

/// Rust `(String, f64)` 元组 → JSON `[名, 值]` 对。
private struct CapPair: Codable, Hashable {
    let name: String
    let value: Double
    init(from decoder: Decoder) throws {
        var c = try decoder.unkeyedContainer()
        name = try c.decode(String.self)
        value = try c.decode(Double.self)
    }
    func encode(to encoder: Encoder) throws {
        var c = encoder.unkeyedContainer()
        try c.encode(name)
        try c.encode(value)
    }
}

private struct CapBranch: Codable, Hashable {
    let name: String
    let length: Double
    let join_position: Double
    let c0: Double
}

private struct CapZone: Codable, Hashable {
    let zone_id: String
    let name: String
    let water_class: String
    let length: Double
    let k: Double
    let b: Double
    let a: Double
    let beta: Double
    let cs: Double
    let c0: Double
    let main_name: String
    let branches: [CapBranch]
}

private struct CapReservoirZone: Codable, Hashable {
    let zone_id: String
    let name: String
    let k: Double
    let b: Double
    let cs: Double
    let c0: Double
}

private struct CapFlowColumnMap: Codable, Hashable {
    let main: String
    let branches: [String]
}

/// Rust `(String, FlowColumnMap)` 元组 → JSON `[功能区, 映射]` 对。
private struct CapFlowMapEntry: Codable, Hashable {
    let zoneId: String
    let map: CapFlowColumnMap
    init(from decoder: Decoder) throws {
        var c = try decoder.unkeyedContainer()
        zoneId = try c.decode(String.self)
        map = try c.decode(CapFlowColumnMap.self)
    }
    func encode(to encoder: Encoder) throws {
        var c = encoder.unkeyedContainer()
        try c.encode(zoneId)
        try c.encode(map)
    }
}

private struct CapDailyRow: Codable, Hashable {
    let date: String
    let values: [CapPair]
}

private struct CapacityInputModel: Codable, Hashable {
    let zones: [CapZone]
    let flow_col_map: [CapFlowMapEntry]
    let daily_flow: [CapDailyRow]
    let reservoir_zones: [CapReservoirZone]
    let daily_volume: [CapDailyRow]
}

private struct CapMonthlyRow: Codable, Hashable {
    let year: Int
    let month: Int
    let values: [CapPair]
}

private struct CapZoneAvgRow: Codable, Hashable {
    let zone_id: String
    let months: [Double]
    let summary: Double
}

private struct CapProcessRow: Codable, Hashable {
    let zone_id: String
    let seg_name: String
    let seg_type: String
    let length: Double
    let avg_q: Double
    let avg_c0: Double
    let avg_c_out: Double
    let avg_w: Double
    let remark: String
}

private struct CapacityOutputModel: Codable, Hashable {
    let monthly_flow: [CapMonthlyRow]
    let monthly_velocity: [CapMonthlyRow]
    let monthly_capacity: [CapMonthlyRow]
    let zone_avg_velocity: [CapZoneAvgRow]
    let zone_avg_capacity: [CapZoneAvgRow]
    let process_table: [CapProcessRow]
    let result_table: [CapProcessRow]
    let reservoir_monthly_volume: [CapMonthlyRow]
    let reservoir_zone_avg_capacity: [CapZoneAvgRow]
}

// MARK: - 请求体

private struct CapPathRequest: Encodable { let path: String }
private struct CapRunRequest: Encodable { let input: CapacityInputModel }
private struct CapWriteRequest: Encodable {
    let path: String
    let output: CapacityOutputModel
    let start_month: Int
}
private struct CapWrittenPath: Decodable { let path: String }

// MARK: - 结果表选择

private enum CapTable: String, CaseIterable, Identifiable {
    case resultTable = "结果表(干流段+汇总)"
    case processTable = "过程表(全部分段)"
    case zoneAvgCapacity = "功能区月平均纳污能力"
    case zoneAvgVelocity = "功能区月平均流速"
    case monthlyFlow = "逐月流量"
    case monthlyVelocity = "逐月流速"
    case monthlyCapacity = "逐月纳污能力"
    case reservoirMonthlyVolume = "水库逐月库容"
    case reservoirZoneAvgCapacity = "水库功能区月平均纳污能力"
    var id: String { rawValue }
}

/// 纳污能力计算 —— capacity 计算器视图(xlsx 输入经 calamine 由 Rust 后端解析,Swift 零业务计算)。
struct CapacityView: View {
    @State private var input: CapacityInputModel?
    @State private var output: CapacityOutputModel?
    @State private var sourceLabel = "未加载"
    @State private var selectedTable: CapTable = .resultTable
    @State private var startMonth = 4
    @State private var loading = false
    @State private var errorMessage: String?
    @State private var exportNote: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("纳污能力计算")
    }

    // MARK: - 头部

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Button { Task { await loadSample() } } label: {
                    Label("加载示例数据", systemImage: "sparkles")
                }
                Button { Task { await loadExcel() } } label: {
                    Label("打开数据文件", systemImage: "doc.badge.arrow.up")
                }
                Spacer()
                Text(sourceLabel)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            if let input {
                HStack(spacing: 10) {
                    Button { Task { await runCapacity() } } label: {
                        Label("运行计算", systemImage: "function")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading)
                    if output != nil {
                        Picker("结果表", selection: $selectedTable) {
                            ForEach(availableTables) { Text($0.rawValue).tag($0) }
                        }
                        .frame(maxWidth: 320)
                        Picker("起始月", selection: $startMonth) {
                            ForEach(1...12, id: \.self) { Text("\($0)月").tag($0) }
                        }
                        .frame(maxWidth: 130)
                        Button { Task { await exportExcel() } } label: {
                            Label("导出 Excel", systemImage: "square.and.arrow.down")
                        }
                        .disabled(loading)
                    }
                    Spacer()
                    Text(inputSummary(input))
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
            if let exportNote {
                Text(exportNote)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
        .padding(16)
    }

    private func inputSummary(_ input: CapacityInputModel) -> String {
        var parts = ["\(input.zones.count) 功能区", "\(input.daily_flow.count) 天流量"]
        if !input.reservoir_zones.isEmpty {
            parts.append("\(input.reservoir_zones.count) 水库")
        }
        return parts.joined(separator: " · ")
    }

    /// 无水库数据时不列水库两张表。
    private var availableTables: [CapTable] {
        guard let output else { return CapTable.allCases }
        if output.reservoir_monthly_volume.isEmpty {
            return CapTable.allCases.filter {
                $0 != .reservoirMonthlyVolume && $0 != .reservoirZoneAvgCapacity
            }
        }
        return CapTable.allCases
    }

    // MARK: - 内容区

    @ViewBuilder private var content: some View {
        if loading {
            centered { ProgressView().controlSize(.large); Text("计算中…").foregroundStyle(.secondary) }
        } else if let errorMessage {
            centered {
                Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 30)).foregroundStyle(.orange)
                Text(errorMessage).foregroundStyle(.secondary).multilineTextAlignment(.center).textSelection(.enabled)
            }
        } else if output != nil {
            DataTableView(result: tableResult(selectedTable)).padding(16)
        } else if input != nil {
            centered {
                Image(systemName: "function").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("数据已加载,点「运行计算」(↩)").foregroundStyle(.secondary)
            }
        } else {
            centered {
                Image(systemName: "drop.triangle").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据,或打开「功能区参数 + 逐日流量」xlsx 数据文件").foregroundStyle(.secondary)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    // MARK: - 表格装配(CapacityOutput → 通用 TableResult)

    private func tableResult(_ table: CapTable) -> TableResult {
        guard let output else { return TableResult(headers: [], rows: []) }
        switch table {
        case .resultTable:
            return processTable(output.result_table, summary: "干流段与汇总 · W 单位 t/a")
        case .processTable:
            return processTable(output.process_table, summary: "全部分段(支流/混合点不计入汇总) · W 单位 t/a")
        case .zoneAvgCapacity:
            return zoneAvgTable(output.zone_avg_capacity, summaryLabel: "年合计", note: "纳污能力 t/a")
        case .zoneAvgVelocity:
            return zoneAvgTable(output.zone_avg_velocity, summaryLabel: "年平均", note: "流速 m/s")
        case .monthlyFlow:
            return monthlyTable(output.monthly_flow, note: "流量 m³/s")
        case .monthlyVelocity:
            return monthlyTable(output.monthly_velocity, note: "流速 m/s")
        case .monthlyCapacity:
            return monthlyTable(output.monthly_capacity, note: "纳污能力 t/a")
        case .reservoirMonthlyVolume:
            return monthlyTable(output.reservoir_monthly_volume, note: "库容 m³")
        case .reservoirZoneAvgCapacity:
            return zoneAvgTable(output.reservoir_zone_avg_capacity, summaryLabel: "年合计", note: "纳污能力 t/a")
        }
    }

    private func processTable(_ rows: [CapProcessRow], summary: String) -> TableResult {
        TableResult(
            headers: ["功能区", "河段", "类型", "长度(m)", "平均流量", "平均C0", "平均出流C", "纳污能力W", "备注"],
            rows: rows.map { r in
                [.string(r.zone_id), .string(r.seg_name), .string(r.seg_type),
                 .number(r.length), .number(r.avg_q), .number(r.avg_c0),
                 .number(r.avg_c_out), .number(r.avg_w), .string(r.remark)]
            },
            summary: summary
        )
    }

    private func zoneAvgTable(_ rows: [CapZoneAvgRow], summaryLabel: String, note: String) -> TableResult {
        TableResult(
            headers: ["功能区"] + (1...12).map { "\($0)月" } + [summaryLabel],
            rows: rows.map { r in
                [CellValue.string(r.zone_id)] + r.months.map { CellValue.number($0) } + [CellValue.number(r.summary)]
            },
            summary: note
        )
    }

    private func monthlyTable(_ rows: [CapMonthlyRow], note: String) -> TableResult {
        let cols: [String] = rows.first.map { $0.values.map(\.name) } ?? []
        return TableResult(
            headers: ["年", "月"] + cols,
            rows: rows.map { r in
                [CellValue.number(Double(r.year)), CellValue.number(Double(r.month))]
                    + r.values.map { CellValue.number($0.value) }
            },
            summary: note
        )
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("capacity", "sample", as: CapacityInputModel.self) }
        sourceLabel = "示例数据"
    }

    private func loadExcel() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        if let xlsx = UTType(filenameExtension: "xlsx") {
            panel.allowedContentTypes = [xlsx]
        }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load {
            try await BackendClient.shared.run(
                "capacity", "read-excel",
                input: CapPathRequest(path: url.path),
                as: CapacityInputModel.self
            )
        }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> CapacityInputModel) async {
        loading = true; errorMessage = nil; output = nil; exportNote = nil
        defer { loading = false }
        do {
            input = try await op()
        } catch {
            input = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runCapacity() async {
        guard let input else { return }
        loading = true; errorMessage = nil; exportNote = nil
        defer { loading = false }
        do {
            output = try await BackendClient.shared.run(
                "capacity", "run",
                input: CapRunRequest(input: input),
                as: CapacityOutputModel.self
            )
            selectedTable = .resultTable
        } catch {
            output = nil
            errorMessage = error.localizedDescription
        }
    }

    private func exportExcel() async {
        guard let output else { return }
        let panel = NSSavePanel()
        if let xlsx = UTType(filenameExtension: "xlsx") {
            panel.allowedContentTypes = [xlsx]
        }
        panel.nameFieldStringValue = "纳污能力计算结果.xlsx"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let written = try await BackendClient.shared.run(
                "capacity", "write-results",
                input: CapWriteRequest(path: url.path, output: output, start_month: startMonth),
                as: CapWrittenPath.self
            )
            exportNote = "已导出: \(written.path)(水文年从 \(startMonth) 月起)"
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
