import Foundation

// MARK: - JSON 信封(同 hydro-cli 契约:{ok, data, error})
struct Envelope<T: Decodable>: Decodable {
    let ok: Bool
    let data: T?
    let error: String?
}

// MARK: - 表格单元(serde_json::Value → 显示串)
enum CellValue: Decodable, Hashable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case null

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() {
            self = .null
        } else if let b = try? c.decode(Bool.self) {
            self = .bool(b)
        } else if let d = try? c.decode(Double.self) {
            self = .number(d)
        } else if let s = try? c.decode(String.self) {
            self = .string(s)
        } else {
            self = .null
        }
    }

    var display: String {
        switch self {
        case .string(let s): return s
        case .number(let d):
            if d == d.rounded() && abs(d) < 1e15 { return String(Int(d)) }
            return String(format: "%.2f", d)
        case .bool(let b): return b ? "是" : "否"
        case .null: return ""
        }
    }

    /// 数值列右对齐用
    var isNumeric: Bool { if case .number = self { return true }; return false }
}

// MARK: - annual 类型(与 vendored Rust types.rs 对齐, snake_case)
struct FileEntry: Codable, Identifiable, Hashable {
    let path: String
    let year: Int
    let city: String
    let table_name: String
    var id: String { path }
}

struct DataIndex: Codable, Hashable {
    let files: [FileEntry]
    let available_years: [Int]
    let available_cities: [String]
    let available_tables: [String]
    let total_files: Int
}

struct QueryInput: Encodable {
    var cities: [String] = []
    var years: [Int] = []
    var table: String
    var indicators: [String] = []
}

struct QueryRequest: Encodable {
    let index: DataIndex
    let input: QueryInput
}

struct QueryOutput: Decodable {
    let headers: [String]
    let rows: [[CellValue]]
    let total_rows: Int
    let year_range: String
    let city_count: Int
}

// MARK: - 通用表格结果(headers + rows,给 TableView)
struct TableResult {
    let headers: [String]
    let rows: [[CellValue]]
    var summary: String = ""
}
