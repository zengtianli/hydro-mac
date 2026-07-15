import SwiftUI

/// 通用表格渲染器 —— 所有计算器结果都是 {headers, rows},共用这一个视图。
/// VStack/HStack 结构在 ScrollView 内天然顶对齐(不像 Grid 会垂直居中欠尺寸内容)。
struct DataTableView: View {
    let result: TableResult
    private let colWidth: CGFloat = 124

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if !result.summary.isEmpty {
                Text(result.summary)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            // 外竖直(天然顶对齐) + 内水平(管宽表) —— 避开双轴 ScrollView 垂直居中欠尺寸内容的坑。
            ScrollView(.vertical) {
                ScrollView(.horizontal, showsIndicators: true) {
                    VStack(alignment: .leading, spacing: 0) {
                        headerRow
                        Divider()
                        ForEach(Array(result.rows.enumerated()), id: \.offset) { rIdx, row in
                            dataRow(row, striped: rIdx % 2 == 1)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .border(Color.primary.opacity(0.08))
        }
    }

    private var headerRow: some View {
        HStack(spacing: 0) {
            ForEach(Array(result.headers.enumerated()), id: \.offset) { _, h in
                Text(h)
                    .font(.caption.bold())
                    .frame(width: colWidth, alignment: .leading)
                    .padding(.vertical, 7)
                    .padding(.horizontal, 10)
            }
        }
        .background(Color(nsColor: .underPageBackgroundColor))
    }

    private func dataRow(_ row: [CellValue], striped: Bool) -> some View {
        HStack(spacing: 0) {
            ForEach(Array(row.enumerated()), id: \.offset) { _, cell in
                Text(cell.display)
                    .font(.callout.monospacedDigit())
                    .lineLimit(1)
                    .frame(width: colWidth, alignment: cell.isNumeric ? .trailing : .leading)
                    .padding(.vertical, 5)
                    .padding(.horizontal, 10)
            }
        }
        .background(striped ? Color.primary.opacity(0.04) : Color.clear)
    }
}
