import SwiftUI
import AppKit
import UniformTypeIdentifiers

// MARK: - efficiency 类型(与 vendored Rust efficiency/types.rs 对齐, snake_case)
// 自包含本文件(舰队并发规矩:各计算器视图不塞 Models.swift),fileprivate 防跨文件撞名。

private struct EffMacroRow: Codable {
    var year: String
    var recycled_usage: Double
    var sewage_treated: Double
    var industrial_gdp: Double
    var supply: Double
    var sales: Double
}

private struct EffMesoRow: Codable {
    var year: String
    var connected_enterprises: Double
    var total_enterprises: Double
    var park_recycled_usage: Double
}

private struct EffMicroRow: Codable {
    var enterprise: String
    var water_intake: Double
    var reuse_amount: Double
    var cooling_intake: Double
    var cooling_circulation: Double
    var process_total: Double
    var process_reuse: Double
    var recycled_usage: Double
    var prior_recycled_usage: Double?
}

private struct EffAssessmentInput: Codable {
    var macro_data: [EffMacroRow]
    var meso_data: [EffMesoRow]
    var micro_data: [String: [EffMicroRow]]
    var ahp_matrix: [[Double]]
    var alpha: Double
}

private struct EffIndicatorRow: Codable {
    var label: String
    var values: [Double?]
}

private struct EffAhpResult: Codable {
    var weights: [Double]
    var cr: Double
    var consistent: Bool
}

private struct EffTopsisEntry: Codable {
    var name: String
    var closeness: Double
    var score: Double
    var grade: String
    var color: String
}

private struct EffLayerScore: Codable {
    var year: String
    var macro_score: Double
    var meso_score: Double
    var micro_score: Double
    var total_score: Double
}

private struct EffAssessmentOutput: Codable {
    var indicators_macro: [EffIndicatorRow]
    var indicators_meso: [EffIndicatorRow]
    var indicators_micro: [String: [EffIndicatorRow]]
    var micro_aggregated: [EffIndicatorRow]
    var ahp_result: EffAhpResult
    var critic_weights: [Double]
    var combined_weights: [Double]
    var layer_scores: [EffLayerScore]
    var topsis_results: [String: [EffTopsisEntry]]
    var indicator_labels: [String]
    var year_indicator_matrix: [EffIndicatorRow]
}

private struct EffRunRequest: Encodable {
    let input: EffAssessmentInput
}

private struct EffPathRequest: Encodable { let path: String }

/// 结果页签
private enum EffResultTab: String, CaseIterable, Identifiable {
    case layers = "层面评分"
    case weights = "指标权重"
    case topsis = "企业评价"
    case matrix = "指标矩阵"
    var id: String { rawValue }
}

/// 水效评估(AHP/CRITIC/TOPSIS) —— efficiency 计算器视图。
/// 数据形态 = 单个 xlsx(sheet:大循环/小循环/点循环-<年>/AHP判断矩阵),经 hydro-cli read-excel 解析。
struct EfficiencyView: View {
    @State private var assessmentInput: EffAssessmentInput?
    @State private var output: EffAssessmentOutput?
    @State private var sourceLabel = "未加载"
    @State private var alpha = 0.5
    @State private var tab: EffResultTab = .layers
    @State private var loading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("水效评估")
    }

    // MARK: - 头部(加载 + 参数 + 运行)

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Button { Task { await loadSample() } } label: {
                    Label("加载示例数据", systemImage: "sparkles")
                }
                Button { Task { await loadFile() } } label: {
                    Label("打开数据文件(xlsx)", systemImage: "doc.badge.arrow.up")
                }
                Spacer()
                Text(sourceLabel)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            if let input = assessmentInput {
                HStack(spacing: 12) {
                    Text("组合系数 α")
                        .font(.callout)
                    Slider(value: $alpha, in: 0...1, step: 0.05)
                        .frame(maxWidth: 220)
                    Text(String(format: "%.2f", alpha))
                        .font(.callout.monospacedDigit())
                        .frame(width: 40, alignment: .leading)
                    Button { Task { await runAssessment() } } label: {
                        Label("运行评估", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading)
                    Spacer()
                    Text("\(input.macro_data.count) 年度 · \(input.micro_data.count) 期企业数据")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
        }
        .padding(16)
    }

    // MARK: - 内容

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
                Picker("结果", selection: $tab) {
                    ForEach(EffResultTab.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(maxWidth: 420)
                DataTableView(result: tableResult)
            }
            .padding(16)
        } else {
            centered {
                Image(systemName: "gauge.with.dots.needle.67percent").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或 xlsx 数据文件后运行评估").foregroundStyle(.secondary)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    // MARK: - 结果 → 表格

    private var tableResult: TableResult {
        guard let out = output else { return TableResult(headers: [], rows: []) }
        switch tab {
        case .layers:
            return TableResult(
                headers: ["年度", "大循环评分", "小循环评分", "点循环评分", "综合评分"],
                rows: out.layer_scores.map { ls in
                    [.string(ls.year), .number(ls.macro_score), .number(ls.meso_score),
                     .number(ls.micro_score), .number(ls.total_score)]
                },
                summary: summaryLine(out)
            )
        case .weights:
            return TableResult(
                headers: ["指标", "AHP权重", "CRITIC权重", "组合权重"],
                rows: out.indicator_labels.indices.map { i in
                    [.string(out.indicator_labels[i]),
                     .string(fmt4(out.ahp_result.weights[i])),
                     .string(fmt4(out.critic_weights[i])),
                     .string(fmt4(out.combined_weights[i]))]
                },
                summary: summaryLine(out)
            )
        case .topsis:
            var rows: [[CellValue]] = []
            for year in out.topsis_results.keys.sorted() {
                for e in out.topsis_results[year] ?? [] {
                    rows.append([.string(year), .string(e.name), .string(fmt4(e.closeness)),
                                 .number(e.score), .string(e.grade)])
                }
            }
            return TableResult(
                headers: ["年度", "企业名称", "相对贴近度", "水效评分", "水效等级"],
                rows: rows,
                summary: "TOPSIS 企业评价(C7-C10 子集权重) · 每年度按评分降序"
            )
        case .matrix:
            return TableResult(
                headers: ["年度"] + out.indicator_labels,
                rows: out.year_indicator_matrix.map { row in
                    [.string(row.label)] + row.values.map { v in v.map { CellValue.number($0) } ?? .null }
                },
                summary: "年度 × C1-C10 指标矩阵(空 = 首年增长率无法计算)"
            )
        }
    }

    private func summaryLine(_ out: EffAssessmentOutput) -> String {
        String(format: "α=%.2f · AHP CR=%.4f · 一致性%@",
               alpha, out.ahp_result.cr, out.ahp_result.consistent ? "通过" : "未通过")
    }

    private func fmt4(_ v: Double) -> String { String(format: "%.4f", v) }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("efficiency", "sample", as: EffAssessmentInput.self) }
        sourceLabel = "示例数据"
    }

    private func loadFile() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        if let xlsx = UTType(filenameExtension: "xlsx") {
            panel.allowedContentTypes = [xlsx]
        }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load { try await BackendClient.shared.run("efficiency", "read-excel", input: EffPathRequest(path: url.path), as: EffAssessmentInput.self) }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> EffAssessmentInput) async {
        loading = true; errorMessage = nil; output = nil
        defer { loading = false }
        do {
            let input = try await op()
            assessmentInput = input
            alpha = input.alpha
        } catch {
            assessmentInput = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runAssessment() async {
        guard var input = assessmentInput else { return }
        input.alpha = alpha
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            output = try await BackendClient.shared.run("efficiency", "run", input: EffRunRequest(input: input), as: EffAssessmentOutput.self)
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
