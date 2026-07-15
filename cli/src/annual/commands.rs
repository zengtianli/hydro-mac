//! annual CLI 编排 —— 复刻自原 apps/annual/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
use crate::annual::data_loader;
use crate::annual::query;
use crate::annual::sample_data;
use crate::annual::types::*;

pub fn get_sample_data() -> DataIndex {
    sample_data::sample_index()
}

pub fn load_data_dir(path: String) -> Result<DataIndex, String> {
    data_loader::scan_data_dir(&path)
}

pub fn get_indicators(index: DataIndex, table: String) -> Result<Vec<String>, String> {
    for entry in &index.files {
        if entry.table_name == table {
            if entry.path.starts_with("(内嵌)") {
                return Ok(sample_data::sample_indicators(&table));
            }
            let (headers, _) = data_loader::load_csv_file(entry)?;
            let indicators: Vec<String> = headers
                .iter()
                .skip(9)
                .filter(|h| !h.is_empty())
                .cloned()
                .collect();
            return Ok(indicators);
        }
    }
    Err(format!("未找到表: {}", table))
}

pub fn export_excel(
    path: String,
    headers: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
) -> Result<(), String> {
    use rust_xlsxwriter::*;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let header_fmt = Format::new().set_bold();

    for (c, h) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, c as u16, h, &header_fmt)
            .map_err(|e| format!("写入表头失败: {}", e))?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let row_idx = (r + 1) as u32;
            let col_idx = c as u16;
            match cell {
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        worksheet
                            .write_number(row_idx, col_idx, f)
                            .map_err(|e| format!("写入数字失败: {}", e))?;
                    }
                }
                serde_json::Value::String(s) => {
                    worksheet
                        .write_string(row_idx, col_idx, s)
                        .map_err(|e| format!("写入字符串失败: {}", e))?;
                }
                _ => {}
            }
        }
    }
    workbook.save(&path).map_err(|e| format!("保存文件失败: {}", e))?;
    Ok(())
}

fn load_data(
    index: &DataIndex,
    cities: &[String],
    years: &[i32],
    table: &str,
) -> Result<(Vec<String>, Vec<DataRow>), String> {
    let is_embedded = index.files.iter().any(|f| f.path.starts_with("(内嵌)"));
    if is_embedded {
        sample_data::load_sample_data(table)
    } else {
        data_loader::load_filtered_data(index, cities, years, table)
    }
}

pub fn run_query(index: DataIndex, input: QueryInput) -> Result<QueryOutput, String> {
    let (headers, rows) = load_data(&index, &input.cities, &input.years, &input.table)?;
    Ok(query::query_data(&headers, &rows, &input))
}

pub fn run_aggregate(index: DataIndex, params: AggregateParams) -> Result<QueryOutput, String> {
    let (_, rows) = load_data(&index, &params.cities, &params.years, &params.table)?;
    match params.mode.as_str() {
        "city" => Ok(query::aggregate_by_city(&rows, &params.indicators)),
        "year" => Ok(query::aggregate_by_year(&rows, &params.indicators)),
        _ => Err(format!("未知聚合模式: {}", params.mode)),
    }
}

pub fn run_compare(index: DataIndex, params: CompareParams) -> Result<YearComparison, String> {
    let years = vec![params.year1, params.year2];
    let (_, rows) = load_data(&index, &params.cities, &years, &params.table)?;
    Ok(query::compare_years(
        &rows,
        params.year1,
        params.year2,
        &params.indicators,
    ))
}

#[allow(dead_code)]
pub fn run_stats(
    index: DataIndex,
    cities: Vec<String>,
    years: Vec<i32>,
    table: String,
    indicator: String,
) -> Result<StatsResult, String> {
    let (_, rows) = load_data(&index, &cities, &years, &table)?;
    Ok(query::calculate_stats(&rows, &indicator))
}
