import SwiftUI
import AppKit

private struct PathRequest: Encodable { let path: String }

/// 水资源年报查询 —— annual 计算器视图(已接入 hydro-cli)。
struct AnnualView: View {
    @State private var index: DataIndex?
    @State private var sourceLabel = "未加载"
    @State private var selectedTable = ""
    @State private var result: TableResult?
    @State private var loading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            content
        }
        .navigationTitle("水资源年报查询")
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
            if let index {
                HStack(spacing: 10) {
                    Picker("表类型", selection: $selectedTable) {
                        ForEach(index.available_tables, id: \.self) { Text($0).tag($0) }
                    }
                    .frame(maxWidth: 240)
                    Button { Task { await runQuery() } } label: {
                        Label("查询", systemImage: "magnifyingglass")
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .disabled(selectedTable.isEmpty || loading)
                    Spacer()
                    Text("\(index.total_files) 文件 · \(index.available_years.count) 年 · \(index.available_cities.count) 市")
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
        } else if let result {
            DataTableView(result: result).padding(16)
        } else {
            centered {
                Image(systemName: "doc.text.magnifyingglass").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("加载示例数据或数据目录后查询").foregroundStyle(.secondary)
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
        await load { try await BackendClient.shared.run("annual", "sample", as: DataIndex.self) }
        sourceLabel = "示例数据"
    }

    private func loadDir() async {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await load { try await BackendClient.shared.run("annual", "load-dir", input: PathRequest(path: url.path), as: DataIndex.self) }
        sourceLabel = url.lastPathComponent
    }

    private func load(_ op: @escaping () async throws -> DataIndex) async {
        loading = true; errorMessage = nil; result = nil
        defer { loading = false }
        do {
            let idx = try await op()
            index = idx
            selectedTable = idx.available_tables.first ?? ""
        } catch {
            index = nil
            errorMessage = error.localizedDescription
        }
    }

    private func runQuery() async {
        guard let index else { return }
        loading = true; errorMessage = nil
        defer { loading = false }
        do {
            let req = QueryRequest(index: index, input: QueryInput(table: selectedTable))
            let out = try await BackendClient.shared.run("annual", "query", input: req, as: QueryOutput.self)
            result = TableResult(
                headers: out.headers,
                rows: out.rows,
                summary: "共 \(out.total_rows) 条 · \(out.city_count) 个城市 · \(out.year_range)"
            )
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
