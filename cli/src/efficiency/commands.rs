//! efficiency CLI 编排 —— 复刻自原 apps/efficiency/src-tauri/src/commands/{calc,io}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
use crate::efficiency::sample_data;
use crate::efficiency::types::*;
use crate::efficiency::{ahp, critic, indicators, topsis};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::HashMap;

// ── calamine 单元格助手(镜像 hydro-common 的 cell_str/cell_f64;
//    留在本模块内保证 worker 产物自包含,主会话后续可上提进 common.rs 收敛) ──

fn cell_str(range: &Range<Data>, row: usize, col: usize) -> String {
    range
        .get((row, col))
        .map(|c| c.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn cell_f64(range: &Range<Data>, row: usize, col: usize) -> f64 {
    range
        .get((row, col))
        .and_then(|c| match c {
            Data::Float(f) => Some(*f),
            Data::Int(i) => Some(*i as f64),
            Data::String(s) => s.trim().parse().ok(),
            _ => None,
        })
        .unwrap_or(0.0)
}

// ── io(原 commands/io.rs) ──

pub fn get_sample_data() -> AssessmentInput {
    sample_data::sample_input()
}

pub fn read_excel(path: String) -> Result<AssessmentInput, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(&path).map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();

    // 大循环
    let macro_data = parse_macro_sheet(&mut workbook)?;

    // 小循环
    let meso_data = parse_meso_sheet(&mut workbook)?;

    // 点循环 - 找所有 "点循环-" 开头的 sheet
    let mut micro_data: HashMap<String, Vec<MicroRawRow>> = HashMap::new();
    let micro_sheets: Vec<String> = sheet_names
        .iter()
        .filter(|name| name.starts_with("点循环-"))
        .cloned()
        .collect();

    for sheet_name in &micro_sheets {
        let year = sheet_name.replace("点循环-", "");
        let rows = parse_micro_sheet(&mut workbook, sheet_name)?;
        micro_data.insert(year, rows);
    }

    // AHP 判断矩阵（可选）
    let ahp_matrix = if sheet_names.iter().any(|n| n.contains("AHP")) {
        parse_ahp_sheet(&mut workbook)?
    } else {
        sample_data::default_ahp_matrix()
    };

    Ok(AssessmentInput {
        macro_data,
        meso_data,
        micro_data,
        ahp_matrix,
        alpha: 0.5,
    })
}

pub fn write_results(path: String, output: AssessmentOutput) -> Result<(), String> {
    use rust_xlsxwriter::*;

    let mut wb = Workbook::new();

    // Sheet 1: 综合评价结果
    let ws = wb.add_worksheet();
    ws.set_name("综合评价结果").map_err(|e| e.to_string())?;

    let bold = Format::new().set_bold();

    // 层面评分表
    let headers1 = ["年度", "大循环评分", "小循环评分", "点循环评分", "汇总评分"];
    for (c, h) in headers1.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }
    for (r, ls) in output.layer_scores.iter().enumerate() {
        let row = (r + 1) as u32;
        ws.write_string(row, 0, &ls.year).map_err(|e| e.to_string())?;
        ws.write_number(row, 1, ls.macro_score).map_err(|e| e.to_string())?;
        ws.write_number(row, 2, ls.meso_score).map_err(|e| e.to_string())?;
        ws.write_number(row, 3, ls.micro_score).map_err(|e| e.to_string())?;
        ws.write_number(row, 4, ls.total_score).map_err(|e| e.to_string())?;
    }

    // TOPSIS 企业评价
    let offset = output.layer_scores.len() as u32 + 3;
    let headers2 = ["企业名称", "相对贴近度", "水效评分", "水效等级", "预警颜色"];
    for (c, h) in headers2.iter().enumerate() {
        ws.write_string_with_format(offset, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }

    let mut row = offset + 1;
    // 合并所有年度的 TOPSIS 结果
    let mut all_years: Vec<&String> = output.topsis_results.keys().collect();
    all_years.sort();
    for year in all_years {
        if let Some(entries) = output.topsis_results.get(year.as_str()) {
            ws.write_string_with_format(row, 0, &format!("── {} ──", year), &bold)
                .map_err(|e| e.to_string())?;
            row += 1;
            for e in entries {
                ws.write_string(row, 0, &e.name).map_err(|e| e.to_string())?;
                ws.write_number(row, 1, e.closeness).map_err(|e| e.to_string())?;
                ws.write_number(row, 2, e.score).map_err(|e| e.to_string())?;
                ws.write_string(row, 3, &e.grade).map_err(|e| e.to_string())?;
                ws.write_string(row, 4, &e.color).map_err(|e| e.to_string())?;
                row += 1;
            }
        }
    }

    // Sheet 2: 权重明细
    let ws2 = wb.add_worksheet();
    ws2.set_name("权重明细").map_err(|e| e.to_string())?;

    let w_headers = ["指标", "AHP权重", "CRITIC权重", "组合权重"];
    for (c, h) in w_headers.iter().enumerate() {
        ws2.write_string_with_format(0, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }
    for (i, label) in output.indicator_labels.iter().enumerate() {
        let row = (i + 1) as u32;
        ws2.write_string(row, 0, label).map_err(|e| e.to_string())?;
        ws2.write_number(row, 1, crate::common::round4(output.ahp_result.weights[i]))
            .map_err(|e| e.to_string())?;
        ws2.write_number(row, 2, crate::common::round4(output.critic_weights[i]))
            .map_err(|e| e.to_string())?;
        ws2.write_number(row, 3, crate::common::round4(output.combined_weights[i]))
            .map_err(|e| e.to_string())?;
    }

    // Sheet 3: 指标明细
    let ws3 = wb.add_worksheet();
    ws3.set_name("指标明细").map_err(|e| e.to_string())?;

    let mut row: u32 = 0;

    // 大循环指标
    ws3.write_string_with_format(row, 0, "大循环指标 (C1-C4)", &bold)
        .map_err(|e| e.to_string())?;
    row += 1;
    let macro_labels = ["年度", "C1", "C2", "C3", "C4"];
    for (c, h) in macro_labels.iter().enumerate() {
        ws3.write_string_with_format(row, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }
    row += 1;
    for ind in &output.indicators_macro {
        ws3.write_string(row, 0, &ind.label).map_err(|e| e.to_string())?;
        for (j, v) in ind.values.iter().enumerate() {
            if let Some(val) = v {
                ws3.write_number(row, (j + 1) as u16, *val)
                    .map_err(|e| e.to_string())?;
            }
        }
        row += 1;
    }
    row += 1;

    // 小循环指标
    ws3.write_string_with_format(row, 0, "小循环指标 (C5-C6)", &bold)
        .map_err(|e| e.to_string())?;
    row += 1;
    let meso_labels = ["年度", "C5", "C6"];
    for (c, h) in meso_labels.iter().enumerate() {
        ws3.write_string_with_format(row, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }
    row += 1;
    for ind in &output.indicators_meso {
        ws3.write_string(row, 0, &ind.label).map_err(|e| e.to_string())?;
        for (j, v) in ind.values.iter().enumerate() {
            if let Some(val) = v {
                ws3.write_number(row, (j + 1) as u16, *val)
                    .map_err(|e| e.to_string())?;
            }
        }
        row += 1;
    }

    wb.save(&path).map_err(|e| format!("保存失败: {}", e))?;

    Ok(())
}

// ── Excel 解析辅助函数(原 commands/io.rs) ──

fn parse_macro_sheet(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<Vec<MacroRawRow>, String> {
    let range = wb
        .worksheet_range("大循环")
        .map_err(|e| format!("找不到[大循环]工作表: {}", e))?;

    let mut rows = Vec::new();
    for r in 1..range.height() {
        let year = cell_str(&range, r, 0);
        if year.is_empty() { continue; }
        rows.push(MacroRawRow {
            year,
            recycled_usage: cell_f64(&range, r, 1),
            sewage_treated: cell_f64(&range, r, 2),
            industrial_gdp: cell_f64(&range, r, 3),
            supply: cell_f64(&range, r, 4),
            sales: cell_f64(&range, r, 5),
        });
    }
    Ok(rows)
}

fn parse_meso_sheet(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<Vec<MesoRawRow>, String> {
    let range = wb
        .worksheet_range("小循环")
        .map_err(|e| format!("找不到[小循环]工作表: {}", e))?;

    let mut rows = Vec::new();
    for r in 1..range.height() {
        let year = cell_str(&range, r, 0);
        if year.is_empty() { continue; }
        rows.push(MesoRawRow {
            year,
            connected_enterprises: cell_f64(&range, r, 1),
            total_enterprises: cell_f64(&range, r, 2),
            park_recycled_usage: cell_f64(&range, r, 3),
        });
    }
    Ok(rows)
}

fn parse_micro_sheet(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_name: &str,
) -> Result<Vec<MicroRawRow>, String> {
    let range = wb
        .worksheet_range(sheet_name)
        .map_err(|e| format!("找不到[{}]工作表: {}", sheet_name, e))?;

    let mut rows = Vec::new();
    let ncols = range.width();

    for r in 1..range.height() {
        let enterprise = cell_str(&range, r, 0);
        if enterprise.is_empty() { continue; }

        rows.push(MicroRawRow {
            enterprise,
            water_intake: cell_f64(&range, r, 1),
            reuse_amount: cell_f64(&range, r, 2),
            cooling_intake: cell_f64(&range, r, 3),
            cooling_circulation: cell_f64(&range, r, 4),
            process_total: cell_f64(&range, r, 5),
            process_reuse: cell_f64(&range, r, 6),
            recycled_usage: if ncols > 7 { cell_f64(&range, r, 7) } else { cell_f64(&range, r, 6) },
            prior_recycled_usage: if ncols > 8 {
                let v = cell_f64(&range, r, 8);
                if v > 0.0 { Some(v) } else { None }
            } else {
                None
            },
        });
    }
    Ok(rows)
}

fn parse_ahp_sheet(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<Vec<Vec<f64>>, String> {
    let range = wb
        .worksheet_range("AHP判断矩阵")
        .map_err(|e| format!("找不到AHP工作表: {}", e))?;

    let n = (range.height() - 1) as usize; // 第一行是表头
    let mut matrix = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = cell_f64(&range, i + 1, j + 1);
        }
    }
    Ok(matrix)
}

// ── calc 编排(原 commands/calc.rs) ──

/// 核心计算编排：输入 → 全部指标 + 权重 + 评分
pub fn run_assessment(input: AssessmentInput) -> Result<AssessmentOutput, String> {
    // 1. 计算各层指标
    let ind_macro = indicators::calc_macro_indicators(&input.macro_data);
    let ind_meso = indicators::calc_meso_indicators(&input.meso_data);

    let mut ind_micro: HashMap<String, Vec<IndicatorRow>> = HashMap::new();
    for (year, rows) in &input.micro_data {
        ind_micro.insert(year.clone(), indicators::calc_micro_indicators(rows));
    }

    let micro_agg = indicators::aggregate_micro_by_year(&input.micro_data);

    // 2. 合并年度 × C1-C10 矩阵
    let labels = sample_data::indicator_labels();
    let directions = sample_data::indicator_directions();

    let year_matrix = build_year_indicator_matrix(&ind_macro, &ind_meso, &micro_agg);

    // 过滤掉含 None 的行（增长率首年）
    let complete_rows: Vec<&IndicatorRow> = year_matrix
        .iter()
        .filter(|r| r.values.iter().all(|v| v.is_some()))
        .collect();

    if complete_rows.len() < 2 {
        return Err("有效数据不足 2 年，无法进行权重计算".into());
    }

    // 提取纯数值矩阵
    let data_matrix: Vec<Vec<f64>> = complete_rows
        .iter()
        .map(|r| r.values.iter().map(|v| v.unwrap()).collect())
        .collect();

    // 3. AHP 权重
    let ahp_result = ahp::ahp_weights(&input.ahp_matrix);

    // 4. CRITIC 权重
    let critic_w = critic::critic_weights(&data_matrix, &directions);

    // 5. 组合权重
    let combined_w = ahp::combined_weights(&ahp_result.weights, &critic_w, input.alpha);

    // 6. 层面评分
    let layer_scores = calc_layer_scores(&complete_rows, &combined_w);

    // 7. TOPSIS 企业评价（用 C7-C10 子集）
    let micro_weight_indices = [6, 7, 8, 9]; // C7-C10 在 combined_w 中的位置
    let micro_sub_weights: Vec<f64> = micro_weight_indices
        .iter()
        .map(|&i| combined_w[i])
        .collect();
    let micro_sub_sum: f64 = micro_sub_weights.iter().sum();
    let micro_norm_weights: Vec<f64> = if micro_sub_sum > 0.0 {
        micro_sub_weights.iter().map(|w| w / micro_sub_sum).collect()
    } else {
        vec![0.25; 4]
    };
    let micro_sub_dirs: Vec<f64> = micro_weight_indices
        .iter()
        .map(|&i| directions[i])
        .collect();

    let mut topsis_results: HashMap<String, Vec<TopsisEntry>> = HashMap::new();
    for (year, enterprise_indicators) in &ind_micro {
        let data: Vec<Vec<f64>> = enterprise_indicators
            .iter()
            .map(|r| r.values.iter().map(|v| v.unwrap_or(0.0)).collect())
            .collect();

        if data.is_empty() {
            continue;
        }

        let result = topsis::topsis_evaluate(&data, &micro_norm_weights, &micro_sub_dirs);

        let entries: Vec<TopsisEntry> = enterprise_indicators
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let (grade, color) = topsis::classify(result.scores[i]);
                TopsisEntry {
                    name: row.label.clone(),
                    closeness: result.closeness[i],
                    score: result.scores[i],
                    grade: grade.to_string(),
                    color: color.to_string(),
                }
            })
            .collect();

        // 按分数降序排列
        let mut entries = entries;
        entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        topsis_results.insert(year.clone(), entries);
    }

    Ok(AssessmentOutput {
        indicators_macro: ind_macro,
        indicators_meso: ind_meso,
        indicators_micro: ind_micro,
        micro_aggregated: micro_agg,
        ahp_result,
        critic_weights: critic_w,
        combined_weights: combined_w,
        layer_scores,
        topsis_results,
        indicator_labels: labels,
        year_indicator_matrix: year_matrix,
    })
}

/// 合并 macro(C1-C4) + meso(C5-C6) + micro_agg(C7-C10) 为年度矩阵
fn build_year_indicator_matrix(
    macro_ind: &[IndicatorRow],
    meso_ind: &[IndicatorRow],
    micro_agg: &[IndicatorRow],
) -> Vec<IndicatorRow> {
    // 以 macro 的年度为基准
    macro_ind
        .iter()
        .map(|macro_row| {
            let year = &macro_row.label;

            let meso_vals = meso_ind
                .iter()
                .find(|r| r.label == *year)
                .map(|r| r.values.clone())
                .unwrap_or_else(|| vec![None, None]);

            let micro_vals = micro_agg
                .iter()
                .find(|r| r.label == *year)
                .map(|r| r.values.clone())
                .unwrap_or_else(|| vec![None; 4]);

            let mut values = macro_row.values.clone();
            values.extend(meso_vals);
            values.extend(micro_vals);

            IndicatorRow {
                label: year.clone(),
                values,
            }
        })
        .collect()
}

/// 计算各层面评分
fn calc_layer_scores(rows: &[&IndicatorRow], combined_w: &[f64]) -> Vec<LayerScore> {
    // 层面划分: 大循环 C1-C4 (idx 0-3), 小循环 C5-C6 (idx 4-5), 点循环 C7-C10 (idx 6-9)
    let layer_ranges = [(0usize, 4usize), (4, 6), (6, 10)];

    rows.iter()
        .map(|row| {
            let mut layer_scores_raw = Vec::new();

            for &(start, end) in &layer_ranges {
                let sub_w: Vec<f64> = combined_w[start..end].to_vec();
                let sub_sum: f64 = sub_w.iter().sum();
                let norm_w: Vec<f64> = if sub_sum > 0.0 {
                    sub_w.iter().map(|w| w / sub_sum).collect()
                } else {
                    vec![1.0 / (end - start) as f64; end - start]
                };

                let vals: Vec<f64> = row.values[start..end]
                    .iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect();

                // 百分制加权评分
                let score: f64 = vals
                    .iter()
                    .zip(&norm_w)
                    .map(|(v, w)| v * w)
                    .sum();

                layer_scores_raw.push(round2(score));
            }

            // 层面权重：大:小:点 = sum(C1-C4权重) : sum(C5-C6权重) : sum(C7-C10权重)
            let macro_w: f64 = combined_w[0..4].iter().sum();
            let meso_w: f64 = combined_w[4..6].iter().sum();
            let micro_w: f64 = combined_w[6..10].iter().sum();
            let total_w = macro_w + meso_w + micro_w;

            let total_score = if total_w > 0.0 {
                round2(
                    (layer_scores_raw[0] * macro_w
                        + layer_scores_raw[1] * meso_w
                        + layer_scores_raw[2] * micro_w)
                        / total_w,
                )
            } else {
                0.0
            };

            LayerScore {
                year: row.label.clone(),
                macro_score: layer_scores_raw[0],
                meso_score: layer_scores_raw[1],
                micro_score: layer_scores_raw[2],
                total_score,
            }
        })
        .collect()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
