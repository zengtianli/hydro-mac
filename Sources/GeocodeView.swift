import SwiftUI
import AppKit

// MARK: - geocode 专属 Codable 类型(与 vendored Rust geocode/types.rs 对齐, snake_case)
// 自包含在本文件(舰队规则:计算器类型不塞 Models.swift,防并发冲突)。

/// 功能类型 —— rawValue 与 Rust serde 枚举串一致("Reverse"/"Forward"/"CompanySearch")
private enum GeoFunction: String, CaseIterable, Identifiable, Codable {
    case reverse = "Reverse"
    case forward = "Forward"
    case company = "CompanySearch"
    var id: String { rawValue }
    var label: String {
        switch self {
        case .reverse: return "逆地理(坐标→地址)"
        case .forward: return "正向(地址→坐标)"
        case .company: return "企业搜索(名称→位置)"
        }
    }
    /// read-excel 动作用的小写关键词
    var cliKeyword: String {
        switch self {
        case .reverse: return "reverse"
        case .forward: return "forward"
        case .company: return "company"
        }
    }
}

private struct GeoInputRow: Codable {
    let index: Int
    let fields: [String: String]
}

private struct GeoInput: Codable {
    var rows: [GeoInputRow]
    var function_type: GeoFunction
    var coordinate_system: String
    var api_key: String
}

private struct GeoResult: Codable {
    let index: Int
    let address: String
    let province: String
    let city: String
    let district: String
    let adcode: String
    let lng: Double?
    let lat: Double?
    let error: String?
}

private struct GeoBatchOutput: Codable {
    let results: [GeoResult]
    let success_count: Int
    let total: Int
}

private struct GeoConvertOutput: Decodable {
    let wgs_lng: Double
    let wgs_lat: Double
    let gcj_lng: Double
    let gcj_lat: Double
    let out_of_china: Bool
}

// 请求包装(与 main.rs geocode_dispatch 的字段名对齐)
private struct GeoReadExcelRequest: Encodable { let path: String; let function_type: String }
private struct GeoBatchRequest: Encodable { let input: GeoInput }
private struct GeoConvertRequest: Encodable { let lng: Double; let lat: Double }
private struct GeoWriteRequest: Encodable { let path: String; let input: GeoInput; let results: GeoBatchOutput }

// 列名候选(与 Rust commands.rs 一致,仅用于表格展示的输入回显列)
private let GEO_LNG_NAMES = ["经度", "JD", "lng", "longitude", "Lng", "LNG", "Longitude"]
private let GEO_LAT_NAMES = ["纬度", "WD", "lat", "latitude", "Lat", "LAT", "Latitude"]
private let GEO_ADDR_NAMES = ["地址", "详细地址", "address", "Address"]
private let GEO_COMPANY_NAMES = ["公司名称", "企业名称", "名称", "单位名称", "用水户名称", "QYMC", "company", "Company"]

private func geoFindField(_ fields: [String: String], _ candidates: [String]) -> String {
    for name in candidates {
        if let v = fields[name], !v.isEmpty { return v }
    }
    return ""
}

/// 高德地理编码 —— geocode 计算器视图(批量 Excel 编码 + 离线坐标转换)。
struct GeocodeView: View {
    @State private var input: GeoInput?
    @State private var sourceLabel = "未加载"
    @State private var function: GeoFunction = .reverse
    @State private var coordSystem = "WGS-84"
    @AppStorage("geocode.amapKey") private var apiKey = ""
    @State private var batchOutput: GeoBatchOutput?
    @State private var result: TableResult?
    @State private var loading = false
    @State private var errorMessage: String?

    // 离线坐标转换小工具
    @State private var convLng = ""
    @State private var convLat = ""
    @State private var convResult = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("高德地理编码")
        .onAppear {
            // GUI .app 不继承 shell 环境;能读到时(终端启动/launchd 注入)作为默认值
            if apiKey.isEmpty {
                let env = ProcessInfo.processInfo.environment
                apiKey = env["AMAP_KEY"] ?? env["AMAP_API_KEY"] ?? ""
            }
        }
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
                    Picker("功能", selection: $function) {
                        ForEach(GeoFunction.allCases) { Text($0.label).tag($0) }
                    }
                    .frame(maxWidth: 260)
                    if function == .reverse {
                        Picker("坐标系", selection: $coordSystem) {
                            Text("WGS-84").tag("WGS-84")
                            Text("GCJ-02").tag("GCJ-02")
                        }
                        .frame(maxWidth: 180)
                    }
                    Button { Task { await runBatch() } } label: {
                        Label("开始编码", systemImage: "play.fill")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(loading || apiKey.trim.isEmpty)
                    if batchOutput != nil {
                        Button { Task { await exportExcel() } } label: {
                            Label("导出 Excel", systemImage: "square.and.arrow.down")
                        }
                        .disabled(loading)
                    }
                    Spacer()
                    Text("\(input.rows.count) 行")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                HStack(spacing: 8) {
                    Image(systemName: "key.fill")
                        .foregroundStyle(apiKey.trim.isEmpty ? Color.secondary : Color.accentColor)
                    SecureField("AMAP key(高德 API 编码动作必需;环境变量 AMAP_KEY 可作默认)", text: $apiKey)
                        .textFieldStyle(.roundedBorder)
                        .frame(maxWidth: 460)
                    if apiKey.trim.isEmpty {
                        Text("需配置 AMAP key(离线坐标转换不受影响)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            convertTool
        }
        .padding(16)
    }

    /// 离线坐标转换小工具(纯计算,无需 key/网络)
    private var convertTool: some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.left.arrow.right")
                .foregroundStyle(.secondary)
            Text("WGS-84 → GCJ-02")
                .font(.caption)
                .foregroundStyle(.secondary)
            TextField("经度", text: $convLng)
                .textFieldStyle(.roundedBorder)
                .frame(width: 110)
            TextField("纬度", text: $convLat)
                .textFieldStyle(.roundedBorder)
                .frame(width: 110)
            Button("转换") { Task { await runConvert() } }
                .disabled(Double(convLng.trim) == nil || Double(convLat.trim) == nil)
            if !convResult.isEmpty {
                Text(convResult)
                    .font(.callout.monospacedDigit())
                    .textSelection(.enabled)
            }
            Spacer()
        }
    }

    // MARK: - 内容区

    @ViewBuilder private var content: some View {
        if loading {
            centered { ProgressView().controlSize(.large); Text("编码中…(每行限速 300ms)").foregroundStyle(.secondary) }
        } else if let errorMessage {
            centered {
                Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 30)).foregroundStyle(.orange)
                Text(errorMessage).foregroundStyle(.secondary).multilineTextAlignment(.center).textSelection(.enabled)
            }
        } else if let result {
            DataTableView(result: result).padding(16)
        } else {
            centered {
                Image(systemName: "mappin.and.ellipse").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或 Excel 数据文件(xlsx)后开始编码").foregroundStyle(.secondary)
            }
        }
    }

    private func centered<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 12) { c() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(40)
    }

    // MARK: - 动作

    private func loadSample() async {
        await load { try await BackendClient.shared.run("geocode", "sample", as: GeoInput.self) }
        sourceLabel = "示例数据(5 地标)"
    }

    private func loadExcel() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        panel.allowedContentTypes = [.init(filenameExtension: "xlsx")].compactMap { $0 }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load {
            try await BackendClient.shared.run(
                "geocode", "read-excel",
                input: GeoReadExcelRequest(path: url.path, function_type: function.cliKeyword),
                as: GeoInput.self
            )
        }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> GeoInput) async {
        loading = true; errorMessage = nil; result = nil; batchOutput = nil
        defer { loading = false }
        do {
            let loaded = try await op()
            input = loaded
            function = loaded.function_type
            coordSystem = loaded.coordinate_system
            result = previewTable(loaded)
        } catch {
            input = nil
            errorMessage = error.localizedDescription
        }
    }

    /// 载入后的输入预览表
    private func previewTable(_ input: GeoInput) -> TableResult {
        let keys = (input.rows.first?.fields.keys).map { Array($0).sorted() } ?? []
        let headers = ["序号"] + keys
        let rows: [[CellValue]] = input.rows.map { row in
            [CellValue.string(String(row.index + 1))] + keys.map { CellValue.string(row.fields[$0] ?? "") }
        }
        return TableResult(headers: headers, rows: rows, summary: "输入预览 · \(input.rows.count) 行(点「开始编码」调高德 API)")
    }

    private func runBatch() async {
        guard var req = input else { return }
        req.function_type = function
        req.coordinate_system = coordSystem
        req.api_key = apiKey.trim
        input = req
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let out = try await BackendClient.shared.run(
                "geocode", "run-batch",
                input: GeoBatchRequest(input: req),
                as: GeoBatchOutput.self
            )
            batchOutput = out
            result = batchTable(req, out)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// 批量编码结果表
    private func batchTable(_ input: GeoInput, _ out: GeoBatchOutput) -> TableResult {
        let inputHeader: String
        let inputNames: [String]
        switch input.function_type {
        case .reverse: inputHeader = "输入坐标"; inputNames = []
        case .forward: inputHeader = "输入地址"; inputNames = GEO_ADDR_NAMES
        case .company: inputHeader = "输入名称"; inputNames = GEO_COMPANY_NAMES
        }
        let headers = ["序号", inputHeader, "地址", "省", "市", "区县", "区域编码", "经度", "纬度", "错误"]
        let byIndex = Dictionary(uniqueKeysWithValues: input.rows.map { ($0.index, $0.fields) })
        let rows: [[CellValue]] = out.results.map { r in
            let fields = byIndex[r.index] ?? [:]
            let echo: String
            if input.function_type == .reverse {
                echo = "\(geoFindField(fields, GEO_LNG_NAMES)),\(geoFindField(fields, GEO_LAT_NAMES))"
            } else {
                echo = geoFindField(fields, inputNames)
            }
            return [
                .string(String(r.index + 1)),
                .string(echo),
                .string(r.address),
                .string(r.province),
                .string(r.city),
                .string(r.district),
                .string(r.adcode),
                r.lng.map { CellValue.number($0) } ?? .null,
                r.lat.map { CellValue.number($0) } ?? .null,
                .string(r.error ?? ""),
            ]
        }
        return TableResult(headers: headers, rows: rows, summary: "成功 \(out.success_count)/\(out.total)")
    }

    private func exportExcel() async {
        guard let input, let batchOutput else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "地理编码结果.xlsx"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            struct PathOut: Decodable { let path: String }
            _ = try await BackendClient.shared.run(
                "geocode", "write-results",
                input: GeoWriteRequest(path: url.path, input: input, results: batchOutput),
                as: PathOut.self
            )
            NSWorkspace.shared.activateFileViewerSelecting([url])
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func runConvert() async {
        guard let lng = Double(convLng.trim), let lat = Double(convLat.trim) else { return }
        do {
            let out = try await BackendClient.shared.run(
                "geocode", "convert",
                input: GeoConvertRequest(lng: lng, lat: lat),
                as: GeoConvertOutput.self
            )
            convResult = out.out_of_china
                ? "境外坐标,原样返回"
                : String(format: "GCJ-02: %.6f, %.6f", out.gcj_lng, out.gcj_lat)
        } catch {
            convResult = "转换失败: \(error.localizedDescription)"
        }
    }
}

private extension String {
    var trim: String { trimmingCharacters(in: .whitespacesAndNewlines) }
}
