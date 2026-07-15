import SwiftUI
import AppKit

// MARK: - rainfall Codable 类型(与 vendored Rust rainfall/types.rs 对齐, snake_case)
// 自包含在本文件(舰队并发规矩:计算器私有类型不进 Models.swift)。

struct RainfallPartition: Codable, Hashable {
    let number: UInt32
    let code: String
    let display_name: String
    let full_label: String
    let lake_ids: [String]
    let areas: [Double]
}

struct RainfallDailyRainfall: Codable, Hashable {
    let date: String
    let values: [String: Double]
}

struct RainfallHourlyRow: Codable, Hashable {
    let datetime: String
    let values: [Double]
}

struct RainfallUserIntake: Codable, Hashable {
    let user_name: String
    let date: String
    let daily_intake: Double
    let hourly_intake: Double
}

struct RainfallPipelineInput: Codable, Hashable {
    let partitions: [RainfallPartition]
    let rainfall: [RainfallDailyRainfall]
    let baseline_columns: [String]
    let baseline: [RainfallHourlyRow]
    let users: [RainfallUserIntake]
    let user_lake_map: [String: String]
}

struct RainfallStepResult: Codable, Hashable {
    let step: UInt32
    let name: String
    let status: String // "ok" | "warn" | "error"
    let message: String
}

struct RainfallLakeInfo: Codable, Hashable {
    let id: String
    let area_m2: Double
    let partition_code: String
}

struct RainfallPartitionSummary: Codable, Hashable {
    let code: String
    let display_name: String
    let total_area_m2: Double
    let lake_count: Int
}

struct RainfallPipelineOutput: Codable, Hashable {
    let steps: [RainfallStepResult]
    let lake_summary: [RainfallLakeInfo]
    let partition_summary: [RainfallPartitionSummary]
    let final_columns: [String]
    let final_rows: [RainfallHourlyRow]
    let row_count: Int
    let col_count: Int
}

private struct RainfallPathRequest: Encodable { let path: String }
private struct RainfallRunRequest: Encodable {
    let input: RainfallPipelineInput
    let steps: [UInt32]
}
private struct RainfallWriteRequest: Encodable {
    let path: String
    let output: RainfallPipelineOutput
}

// MARK: - 视图

/// 降雨数据分析 —— rainfall 计算器视图(平原河网降雨-径流 6 步处理管线,已接入 hydro-cli)。
struct RainfallView: View {
    private static let stepNames: [(UInt32, String)] = [
        (1, "分区处理"), (2, "面积汇总"), (3, "降雨系数"),
        (4, "取水处理"), (5, "扣减计算"), (6, "合并输出"),
    ]
    /// 结果数据 tab 只预览前 N 行(312 行 x 228 列全量渲非懒表格会卡;完整结果走「导出 TSV」)。
    private static let previewRowLimit = 48

    private enum ResultTab: String, CaseIterable {
        case steps = "处理步骤"
        case partitions = "分区汇总"
        case lakes = "湖泊汇总"
        case data = "结果数据"
    }

    @State private var pipelineInput: RainfallPipelineInput?
    @State private var sourceLabel = "未加载"
    @State private var enabledSteps: Set<UInt32> = [1, 2, 3, 4, 5, 6]
    @State private var output: RainfallPipelineOutput?
    @State private var resultTab: ResultTab = .steps
    @State private var loading = false
    @State private var errorMessage: String?
    @State private var exportNote: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("降雨数据分析")
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
            if let input = pipelineInput {
                HStack(spacing: 10) {
                    ForEach(Self.stepNames, id: \.0) { (num, name) in
                        Toggle("\(num) \(name)", isOn: stepBinding(num))
                            .toggleStyle(.checkbox)
                            .font(.caption)
                    }
                }
                HStack(spacing: 10) {
                    Button { Task { await runPipeline() } } label: {
                        Label("运行管线", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(enabledSteps.isEmpty || loading)
                    if output != nil {
                        Button { Task { await exportOutput() } } label: {
                            Label("导出 TSV", systemImage: "square.and.arrow.up")
                        }
                        .disabled(loading)
                    }
                    if let exportNote {
                        Text(exportNote).font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Text("\(input.partitions.count) 分区 · \(input.rainfall.count) 天降雨 · \(input.baseline_columns.count) 概湖列 · \(input.users.count) 条取水记录")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
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
        } else if output != nil {
            VStack(alignment: .leading, spacing: 10) {
                Picker("", selection: $resultTab) {
                    ForEach(ResultTab.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 420)
                DataTableView(result: tabResult)
            }
            .padding(16)
        } else {
            centered {
                Image(systemName: "cloud.rain").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或数据目录(static_PYLYSCS / input_FQNNGXL / input_GHJYL / input_YSH / input_YSH_GH)后运行管线")
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    private func stepBinding(_ num: UInt32) -> Binding<Bool> {
        Binding(
            get: { enabledSteps.contains(num) },
            set: { on in if on { enabledSteps.insert(num) } else { enabledSteps.remove(num) } }
        )
    }

    // MARK: - 结果 tab → TableResult

    private var tabResult: TableResult {
        guard let output else { return TableResult(headers: [], rows: []) }
        switch resultTab {
        case .steps:
            return TableResult(
                headers: ["步骤", "名称", "状态", "说明"],
                rows: output.steps.map { s in
                    [.number(Double(s.step)), .string(s.name), .string(s.status), .string(s.message)]
                },
                summary: "执行 \(output.steps.count) 个步骤"
            )
        case .partitions:
            return TableResult(
                headers: ["分区代码", "分区名称", "总面积(m²)", "概湖数"],
                rows: output.partition_summary.map { p in
                    [.string(p.code), .string(p.display_name), .number(p.total_area_m2), .number(Double(p.lake_count))]
                },
                summary: "共 \(output.partition_summary.count) 个分区"
            )
        case .lakes:
            return TableResult(
                headers: ["概湖", "面积(m²)", "所属分区"],
                rows: output.lake_summary.map { l in
                    [.string(l.id), .number(l.area_m2), .string(l.partition_code)]
                },
                summary: "共 \(output.lake_summary.count) 个概湖"
            )
        case .data:
            let preview = output.final_rows.prefix(Self.previewRowLimit)
            return TableResult(
                headers: ["日期"] + output.final_columns,
                rows: preview.map { r in
                    [CellValue.string(r.datetime)] + r.values.map { CellValue.number($0) }
                },
                summary: "共 \(output.row_count) 行 × \(output.col_count) 列(预览前 \(preview.count) 行,完整结果用「导出 TSV」)"
            )
        }
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("rainfall", "sample", as: RainfallPipelineInput.self) }
        sourceLabel = "示例数据"
    }

    private func loadDir() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load { try await BackendClient.shared.run("rainfall", "load-dir", input: RainfallPathRequest(path: url.path), as: RainfallPipelineInput.self) }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> RainfallPipelineInput) async {
        loading = true; errorMessage = nil; output = nil; exportNote = nil
        defer { loading = false }
        do {
            pipelineInput = try await op()
        } catch {
            pipelineInput = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runPipeline() async {
        guard let pipelineInput else { return }
        loading = true; errorMessage = nil; exportNote = nil
        defer { loading = false }
        do {
            let req = RainfallRunRequest(input: pipelineInput, steps: enabledSteps.sorted())
            output = try await BackendClient.shared.run("rainfall", "run-pipeline", input: req, as: RainfallPipelineOutput.self)
            resultTab = .steps
        } catch {
            output = nil
            errorMessage = error.localizedDescription
        }
    }

    private struct WriteAck: Decodable { let path: String }

    private func exportOutput() async {
        guard let output else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "rainfall_output.txt"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let req = RainfallWriteRequest(path: url.path, output: output)
            let ack = try await BackendClient.shared.run("rainfall", "write-output", input: req, as: WriteAck.self)
            exportNote = "已导出: \(ack.path)"
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
