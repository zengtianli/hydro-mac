//! rainfall CLI 编排 —— 复刻自原 apps/rainfall/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
//! 与原版差异:TSV 解析拆成「内容级」函数(parse_*_content),文件读取只是薄壳 ——
//! 这样 sample_data 能用 include_str! 内嵌示例(原版运行时读 ~/Dev/hydro-rainfall,路径已不存在)。
use crate::rainfall::parser;
use crate::rainfall::pipeline;
use crate::rainfall::sample_data;
use crate::rainfall::types::*;
use csv::ReaderBuilder;
use regex::Regex;
use std::collections::HashMap;

pub fn get_sample_data() -> Result<PipelineInput, String> {
    sample_data::load_sample_input()
}

/// 从目录读 5 个输入文件(与原 rainfall_read_input_files 同契约)。
/// 需要: static_PYLYSCS.txt, input_FQNNGXL.txt, input_GHJYL.txt, input_YSH.txt, input_YSH_GH.txt
pub fn read_input_files(dir: String) -> Result<PipelineInput, String> {
    let base = std::path::Path::new(&dir);
    let read = |name: &str, what: &str| -> Result<String, String> {
        std::fs::read_to_string(base.join(name)).map_err(|e| format!("读取{}失败({}): {}", what, name, e))
    };

    let partitions = parser::parse_partition_config(&read("static_PYLYSCS.txt", "分区配置")?)?;
    let rainfall = parse_rainfall_content(&read("input_FQNNGXL.txt", "降雨数据")?)?;
    let (baseline_columns, baseline) = parse_baseline_content(&read("input_GHJYL.txt", "基线流量数据")?)?;
    let users = parse_user_intake_content(&read("input_YSH.txt", "取水数据")?)?;
    let user_lake_map = parse_user_lake_map_content(&read("input_YSH_GH.txt", "取水户映射")?)?;

    Ok(PipelineInput {
        partitions,
        rainfall,
        baseline_columns,
        baseline,
        users,
        user_lake_map,
    })
}

/// 跑 6 步管线(与原 rainfall_run_pipeline 同契约,steps = 1..=6 的子集)。
pub fn run_pipeline(input: PipelineInput, steps: Vec<u32>) -> Result<PipelineOutput, String> {
    let valid_steps: Vec<u32> = steps.into_iter().filter(|&s| (1..=6).contains(&s)).collect();
    if valid_steps.is_empty() {
        return Err("请至少选择一个处理步骤".into());
    }
    Ok(pipeline::run_pipeline(&input, &valid_steps))
}

/// 结果写成 TSV(与原 rainfall_write_output 同契约)。
pub fn write_output(path: String, output: PipelineOutput) -> Result<(), String> {
    use std::io::Write;

    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("创建输出文件失败: {}", e))?;

    let mut header = String::from("日期");
    for col in &output.final_columns {
        header.push('\t');
        header.push_str(col);
    }
    writeln!(file, "{}", header).map_err(|e| e.to_string())?;

    for row in &output.final_rows {
        let mut line = row.datetime.clone();
        for val in &row.values {
            line.push('\t');
            line.push_str(&format!("{:.4}", val));
        }
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── TSV 内容级解析(逻辑逐字来自原 commands/io.rs,仅由「读文件」改为「收内容」)──

pub(crate) fn parse_rainfall_content(content: &str) -> Result<Vec<DailyRainfall>, String> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.trim().replace('\u{feff}', "").to_string())
        .collect();

    let mut rainfall = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        let date = record.get(0).unwrap_or("").trim().to_string();
        if date.is_empty() {
            continue;
        }

        let mut values = HashMap::new();
        for (i, header) in headers.iter().enumerate().skip(1) {
            let val: f64 = record
                .get(i)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0.0);
            values.insert(header.clone(), val);
        }

        rainfall.push(DailyRainfall { date, values });
    }

    Ok(rainfall)
}

pub(crate) fn parse_baseline_content(content: &str) -> Result<(Vec<String>, Vec<HourlyRow>), String> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.trim().replace('\u{feff}', "").to_string())
        .collect();

    // 列名(跳过第一列「日期」)
    let col_re = Regex::new(r"G\d+").unwrap();
    let columns: Vec<String> = headers
        .iter()
        .skip(1)
        .filter(|h| col_re.is_match(h))
        .cloned()
        .collect();

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        let datetime = record.get(0).unwrap_or("").trim().to_string();
        if datetime.is_empty() {
            continue;
        }

        let values: Vec<f64> = (1..headers.len())
            .map(|i| {
                record
                    .get(i)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0.0)
            })
            .collect();

        rows.push(HourlyRow { datetime, values });
    }

    Ok((columns, rows))
}

pub(crate) fn parse_user_intake_content(content: &str) -> Result<Vec<UserIntake>, String> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut users = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        let user_name = record.get(0).unwrap_or("").trim().to_string();
        let date = record.get(1).unwrap_or("").trim().to_string();
        let daily_intake: f64 = record
            .get(2)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0.0);
        let hourly_intake: f64 = record
            .get(3)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0.0);

        if !user_name.is_empty() && !date.is_empty() {
            users.push(UserIntake {
                user_name,
                date,
                daily_intake,
                hourly_intake,
            });
        }
    }

    Ok(users)
}

pub(crate) fn parse_user_lake_map_content(content: &str) -> Result<HashMap<String, String>, String> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut map = HashMap::new();
    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        let user = record.get(0).unwrap_or("").trim().to_string();
        let lake = record.get(1).unwrap_or("").trim().to_string();
        if !user.is_empty() && !lake.is_empty() {
            map.insert(user, lake);
        }
    }

    Ok(map)
}
