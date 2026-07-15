import SwiftUI
import AppKit

// MARK: - irrigation 契约类型(自包含本文件,与 cli/src/irrigation 对齐, snake_case;
//         不进 Models.swift —— 各计算器视图类型自包含,防并发接入冲突)

/// 8 个输入文件的内容集合(= Rust 侧 SampleData:既是示例数据载体也是 run 输入)。
private struct IrrigationContents: Codable {
    var time_config: String
    var zones: String
    var single_crop_stages: String
    var double_crop_stages: String
    var crops: String
    var rainfall: String
    var evaporation: String
    var crop_areas: String
}

private struct IrrigationPathRequest: Encodable { let path: String }

private struct IrrigationRunRequest: Encodable {
    let contents: IrrigationContents
    let mode: String
}

private struct IrrigationDailyResult: Decodable {
    let date: String
    let zone_name: String
    let paddy_irrigation: Double
    let paddy_drainage: Double
    let dryland_irrigation: Double
    let dryland_drainage: Double
    let flowering_irrigation: Double
    let lowland_drainage: Double
    let water_surface_drainage: Double
}

private struct IrrigationRunOutput: Decodable {
    let daily_results: [IrrigationDailyResult]
    let zone_names: [String]
    let dates: [String]
    let warnings: [String]
}

/// 灌溉需水日计算 —— irrigation 计算器视图(已接入 hydro-cli)。
/// 流程:加载示例数据 / 打开数据目录(8 个标准命名 TSV/TXT) → 选计算模式 → 运行 → 宽表结果。
struct IrrigationView: View {
    @State private var contents: IrrigationContents?
    @State private var sourceLabel = "未加载"
    @State private var mode = "both"
    @State private var outputKind: OutputKind = .irrigation
    @State private var output: IrrigationRunOutput?
    @State private var loading = false
    @State private var errorMessage: String?

    private enum OutputKind: String, CaseIterable, Identifiable {
        case irrigation, drainage
        var id: String { rawValue }
        var label: String { self == .irrigation ? "灌溉量" : "排水量" }
    }

    private static let modes: [(key: String, label: String)] = [
        ("both", "综合"), ("irrigation", "仅水稻灌溉"), ("crop", "仅旱地作物"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("灌溉需水计算")
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
            if contents != nil {
                HStack(spacing: 10) {
                    Picker("计算模式", selection: $mode) {
                        ForEach(Self.modes, id: \.key) { Text($0.label).tag($0.key) }
                    }
                    .frame(maxWidth: 260)
                    Button { Task { await runCalc() } } label: {
                        Label("运行计算", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading)
                    if output != nil {
                        Picker("结果", selection: $outputKind) {
                            ForEach(OutputKind.allCases) { Text($0.label).tag($0) }
                        }
                        .pickerStyle(.segmented)
                        .frame(maxWidth: 200)
                    }
                    Spacer()
                }
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
        } else if let table = tableResult {
            VStack(alignment: .leading, spacing: 8) {
                if let output, !output.warnings.isEmpty {
                    Label(output.warnings.joined(separator: " · "), systemImage: "exclamationmark.triangle")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                DataTableView(result: table)
            }
            .padding(16)
        } else {
            centered {
                Image(systemName: "leaf").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或数据目录后运行计算").foregroundStyle(.secondary)
                Text("数据目录需含 8 个标准命名文件:in_TIME / static_fenqu / static_single_crop / static_double_crop / static_crops / in_JYGC / in_ZFGC / in_dry_crop_area")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                    .multilineTextAlignment(.center)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    /// 结果透视成宽表:日期 × 分区,值 = 灌溉量(水田+旱地)或排水量(同 Rust 侧 TSV 口径),单位 万m³。
    private var tableResult: TableResult? {
        guard let output else { return nil }
        var lookup: [String: IrrigationDailyResult] = [:]
        for r in output.daily_results { lookup["\(r.date)|\(r.zone_name)"] = r }
        let headers = ["日期"] + output.zone_names
        let rows: [[CellValue]] = output.dates.map { date in
            var row: [CellValue] = [.string(date)]
            for zone in output.zone_names {
                let v: Double
                if let r = lookup["\(date)|\(zone)"] {
                    v = outputKind == .irrigation
                        ? r.paddy_irrigation + r.dryland_irrigation
                        : r.paddy_drainage + r.dryland_drainage
                } else {
                    v = 0
                }
                row.append(.number(v))
            }
            return row
        }
        var summary = "\(output.dates.count) 天 × \(output.zone_names.count) 区 · \(outputKind.label)(万m³)"
        if !output.warnings.isEmpty { summary += " · \(output.warnings.count) 条预警" }
        return TableResult(headers: headers, rows: rows, summary: summary)
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("irrigation", "sample", as: IrrigationContents.self) }
        sourceLabel = "示例数据"
    }

    private func loadDir() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load {
            try await BackendClient.shared.run(
                "irrigation", "load-dir",
                input: IrrigationPathRequest(path: url.path),
                as: IrrigationContents.self
            )
        }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> IrrigationContents) async {
        loading = true; errorMessage = nil; output = nil
        defer { loading = false }
        do {
            contents = try await op()
        } catch {
            contents = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runCalc() async {
        guard let contents else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let req = IrrigationRunRequest(contents: contents, mode: mode)
            output = try await BackendClient.shared.run("irrigation", "run", input: req, as: IrrigationRunOutput.self)
        } catch {
            output = nil
            errorMessage = error.localizedDescription
        }
    }
}
