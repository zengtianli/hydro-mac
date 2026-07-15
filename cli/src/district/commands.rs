//! district CLI 编排 —— 复刻自原 apps/district/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
//! 差异说明:原 district_write_results 打 ZIP(浏览器下载约束);本 CLI 是原生 app 后端,
//! 改为把逐河区 TSV + 汇总直接写进目录(export_dir,纯 std,免 zip 依赖),内容格式逐字节同源。
use crate::district::config::INPUT_FILES;
use crate::district::sample_data;
use crate::district::scheduler::run_scheduler;
use crate::district::types::*;
use std::collections::HashMap;

/// 返回嵌入的示例数据
pub fn get_sample_data() -> SchedulerInput {
    sample_data::sample_input()
}

/// 从目录读取 13 个 TSV 文件, 返回 SchedulerInput
pub fn read_input_files(dir_path: String) -> Result<SchedulerInput, String> {
    let dir = std::path::Path::new(&dir_path);
    if !dir.is_dir() {
        return Err(format!("目录不存在: {}", dir_path));
    }

    let mut files: Vec<(String, TsvTable)> = Vec::new();

    for &(key, filename) in INPUT_FILES {
        let file_path = dir.join(filename);
        if !file_path.exists() {
            // 非必需文件可以跳过
            continue;
        }
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("读取 {} 失败: {}", filename, e))?;
        let table = parse_tsv(&content)?;
        files.push((key.to_string(), table));
    }

    Ok(SchedulerInput { files })
}

/// 运行 7 步调度
pub fn run(input: SchedulerInput) -> Result<SchedulerOutput, String> {
    run_scheduler(&input)
}

/// 解析 TSV 内容
fn parse_tsv(content: &str) -> Result<TsvTable, String> {
    let mut lines = content.lines();
    let header_line = lines.next().ok_or("文件为空")?;
    let headers: Vec<String> = header_line.split('\t').map(|s| s.trim().to_string()).collect();

    let mut rows = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<String> = line.split('\t').map(|s| s.trim().to_string()).collect();
        rows.push(cells);
    }

    Ok(TsvTable { headers, rows })
}

/// 将计算结果写入目录:每河区一个 <code>.txt + 汇总 output_hq_all.txt(TSV 格式同原 ZIP 内容)
pub fn export_dir(dir: String, output: SchedulerOutput) -> Result<(), String> {
    let dir_path = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir_path).map_err(|e| format!("创建目录失败: {}", e))?;

    // 每个河区输出一个文件
    for district in &output.districts {
        let filename = format!("{}.txt", district.code);

        // 合并来水+需水+平衡字段
        let all_headers = build_district_headers(district);
        let all_rows = build_district_rows(district);

        let tsv = format_tsv(&all_headers, &all_rows);
        std::fs::write(dir_path.join(&filename), tsv.as_bytes())
            .map_err(|e| format!("写入 {} 失败: {}", filename, e))?;
    }

    // 汇总文件
    if !output.summary.is_empty() {
        let summary_headers = build_summary_headers(&output.summary);
        let summary_rows = build_summary_rows(&output.summary);
        let tsv = format_tsv(&summary_headers, &summary_rows);
        std::fs::write(dir_path.join("output_hq_all.txt"), tsv.as_bytes())
            .map_err(|e| format!("写入 output_hq_all.txt 失败: {}", e))?;
    }

    Ok(())
}

fn build_district_headers(district: &DistrictData) -> Vec<String> {
    let mut headers = vec!["日期".to_string()];
    // 来水列
    if let Some(first) = district.inflow.first() {
        for (k, _) in &first.values {
            headers.push(k.clone());
        }
    }
    // 需水列
    if let Some(first) = district.demand.first() {
        for (k, _) in &first.values {
            if !headers.contains(k) {
                headers.push(k.clone());
            }
        }
    }
    // 平衡列
    if let Some(first) = district.balance.first() {
        for (k, _) in &first.fields {
            if !headers.contains(k) {
                headers.push(k.clone());
            }
        }
    }
    headers
}

fn build_district_rows(district: &DistrictData) -> Vec<Vec<String>> {
    let ndays = district.balance.len();
    let headers = build_district_headers(district);

    let mut rows = Vec::new();
    for i in 0..ndays {
        let mut col_map: HashMap<String, f64> = HashMap::new();

        if let Some(row) = district.inflow.get(i) {
            for (k, v) in &row.values {
                col_map.insert(k.clone(), *v);
            }
        }
        if let Some(row) = district.demand.get(i) {
            for (k, v) in &row.values {
                col_map.insert(k.clone(), *v);
            }
        }
        let balance_row = &district.balance[i];
        for (k, v) in &balance_row.fields {
            col_map.insert(k.clone(), *v);
        }

        let mut cells = vec![balance_row.date.clone()];
        for h in &headers[1..] {
            let v = col_map.get(h).copied().unwrap_or(0.0);
            cells.push(format!("{:.4}", v));
        }
        rows.push(cells);
    }
    rows
}

fn build_summary_headers(summary: &[WaterBalanceRow]) -> Vec<String> {
    let mut headers = vec!["日期".to_string()];
    if let Some(first) = summary.first() {
        for (k, _) in &first.fields {
            headers.push(k.clone());
        }
    }
    headers
}

fn build_summary_rows(summary: &[WaterBalanceRow]) -> Vec<Vec<String>> {
    summary
        .iter()
        .map(|row| {
            let mut cells = vec![row.date.clone()];
            for (_, v) in &row.fields {
                cells.push(format!("{:.4}", v));
            }
            cells
        })
        .collect()
}

fn format_tsv(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut out = headers.join("\t");
    out.push('\n');
    for row in rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    out
}
