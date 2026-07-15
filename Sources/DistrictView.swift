import SwiftUI
import AppKit

// MARK: - district 专属 Codable(自包含本文件,不进 Models.swift)

private struct DistPathRequest: Encodable { let path: String }

private struct DistTsvTable: Codable {
    let headers: [String]
    let rows: [[String]]
}

/// serde 的 Vec<(String, TsvTable)> 序列化为 [[key, table], …] —— 用 unkeyedContainer 对齐。
private struct DistFilePair: Codable {
    let key: String
    let table: DistTsvTable
    init(from decoder: Decoder) throws {
        var c = try decoder.unkeyedContainer()
        key = try c.decode(String.self)
        table = try c.decode(DistTsvTable.self)
    }
    func encode(to encoder: Encoder) throws {
        var c = encoder.unkeyedContainer()
        try c.encode(key)
        try c.encode(table)
    }
}

private struct DistSchedulerInput: Codable { let files: [DistFilePair] }

/// serde 的 Vec<(String, f64)> 序列化为 [[名, 值], …]。
private struct DistNamedValue: Codable {
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

private struct DistDailyRow: Codable {
    let date: String
    let values: [DistNamedValue]
}

private struct DistBalanceRow: Codable {
    let date: String
    let fields: [DistNamedValue]
}

private struct DistDistrictData: Codable {
    let name: String
    let code: String
    let inflow: [DistDailyRow]
    let demand: [DistDailyRow]
    let balance: [DistBalanceRow]
}

private struct DistSchedulerOutput: Codable {
    let districts: [DistDistrictData]
    let summary: [DistBalanceRow]
    let districts_processed: Int
    let total_water_demand: Double
    let total_water_supply: Double
    let total_shortage: Double
}

private struct DistRunRequest: Encodable {
    let input: DistSchedulerInput
}

private struct DistExportRequest: Encodable {
    let dir: String
    let output: DistSchedulerOutput
}

private struct DistExportResult: Decodable { let dir: String }

private let DIST_SUMMARY_TAG = "__summary__"

/// 河区供需平衡调度 —— district 计算器视图(已接入 hydro-cli)。
struct DistrictView: View {
    @State private var input: DistSchedulerInput?
    @State private var output: DistSchedulerOutput?
    @State private var sourceLabel = "未加载"
    @State private var selectedView = DIST_SUMMARY_TAG
    @State private var loading = false
    @State private var errorMessage: String?
    @State private var exportNote: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("河区调度模型")
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Button { Task { await loadSample() } } label: {
                    Label("加载示例数据", systemImage: "sparkles")
                }
                Button { Task { await loadDir() } } label: {
                    Label("打开数据目录", systemImage: "folder")
                }
                Spacer()
                Text(sourceLabel)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            if let input {
                HStack(spacing: 10) {
                    Button { Task { await runScheduler() } } label: {
                        Label("运行调度", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading)
                    if output != nil {
                        Button { Task { await exportResults() } } label: {
                            Label("导出结果", systemImage: "square.and.arrow.down")
                        }
                        .disabled(loading)
                    }
                    Spacer()
                    Text("\(input.files.count) 个输入表")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
            if let output {
                HStack(spacing: 10) {
                    Picker("查看", selection: $selectedView) {
                        Text("全域汇总").tag(DIST_SUMMARY_TAG)
                        ForEach(output.districts, id: \.code) { d in
                            Text(d.name).tag(d.name)
                        }
                    }
                    .frame(maxWidth: 300)
                    Spacer()
                    Text(String(
                        format: "%d 河区 · 总需水 %.1f · 总来水 %.1f · 缺水 %.1f",
                        output.districts_processed,
                        output.total_water_demand,
                        output.total_water_supply,
                        output.total_shortage
                    ))
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

    @ViewBuilder private var content: some View {
        if loading {
            centered { ProgressView().controlSize(.large); Text("计算中…").foregroundStyle(.secondary) }
        } else if let errorMessage {
            centered {
                Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 30)).foregroundStyle(.orange)
                Text(errorMessage).foregroundStyle(.secondary).multilineTextAlignment(.center).textSelection(.enabled)
            }
        } else if let output {
            DataTableView(result: tableResult(output)).padding(16)
        } else {
            centered {
                Image(systemName: "point.3.connected.trianglepath.dotted").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或数据目录(13 个 TSV)后运行调度").foregroundStyle(.secondary)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    // MARK: - 结果表(纯展示拼装,数值全部来自 Rust)

    private func tableResult(_ output: DistSchedulerOutput) -> TableResult {
        if selectedView == DIST_SUMMARY_TAG {
            return summaryTable(output.summary, caption: "全域汇总 · \(output.summary.count) 日")
        }
        if let d = output.districts.first(where: { $0.name == selectedView }) {
            return districtTable(d)
        }
        return summaryTable(output.summary, caption: "全域汇总 · \(output.summary.count) 日")
    }

    private func summaryTable(_ summary: [DistBalanceRow], caption: String) -> TableResult {
        var headers = ["日期"]
        if let first = summary.first {
            headers += first.fields.map(\.name)
        }
        let rows: [[CellValue]] = summary.map { row in
            [CellValue.string(row.date)] + row.fields.map { CellValue.number($0.value) }
        }
        return TableResult(headers: headers, rows: rows, summary: caption)
    }

    /// 同 Rust export 的合并逻辑:日期 + 来水列 + 需水列 + 平衡列(去重)。
    private func districtTable(_ d: DistDistrictData) -> TableResult {
        var headers = ["日期"]
        if let first = d.inflow.first {
            for v in first.values where !headers.contains(v.name) { headers.append(v.name) }
        }
        if let first = d.demand.first {
            for v in first.values where !headers.contains(v.name) { headers.append(v.name) }
        }
        if let first = d.balance.first {
            for f in first.fields where !headers.contains(f.name) { headers.append(f.name) }
        }

        var rows: [[CellValue]] = []
        for (i, balanceRow) in d.balance.enumerated() {
            var colMap: [String: Double] = [:]
            if i < d.inflow.count {
                for v in d.inflow[i].values { colMap[v.name] = v.value }
            }
            if i < d.demand.count {
                for v in d.demand[i].values { colMap[v.name] = v.value }
            }
            for f in balanceRow.fields { colMap[f.name] = f.value }
            var cells: [CellValue] = [.string(balanceRow.date)]
            for h in headers.dropFirst() {
                cells.append(.number(colMap[h] ?? 0))
            }
            rows.append(cells)
        }
        return TableResult(headers: headers, rows: rows, summary: "\(d.name)(\(d.code))· \(rows.count) 日")
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("district", "sample", as: DistSchedulerInput.self) }
        sourceLabel = "示例数据"
    }

    private func loadDir() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "选择含 13 个 TSV 输入文件的目录(static_HQ_ZQ.txt 等)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load { try await BackendClient.shared.run("district", "load-dir", input: DistPathRequest(path: url.path), as: DistSchedulerInput.self) }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> DistSchedulerInput) async {
        loading = true; errorMessage = nil; output = nil; exportNote = nil
        defer { loading = false }
        do {
            input = try await op()
        } catch {
            input = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runScheduler() async {
        guard let input else { return }
        loading = true; errorMessage = nil; exportNote = nil
        defer { loading = false }
        do {
            let out = try await BackendClient.shared.run("district", "run", input: DistRunRequest(input: input), as: DistSchedulerOutput.self)
            output = out
            selectedView = DIST_SUMMARY_TAG
        } catch {
            output = nil
            errorMessage = error.localizedDescription
        }
    }

    private func exportResults() async {
        guard let output else { return }
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.message = "选择导出目录(逐河区 TSV + 汇总 output_hq_all.txt)"
        panel.prompt = "导出到此"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let res = try await BackendClient.shared.run(
                "district", "export-dir",
                input: DistExportRequest(dir: url.path, output: output),
                as: DistExportResult.self
            )
            exportNote = "已导出 \(output.districts.count + 1) 个 TSV → \(res.dir)"
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
