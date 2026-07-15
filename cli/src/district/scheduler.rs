use std::collections::HashMap;

use super::balance::calculate_water_balance;
use super::config::{
    district_code, is_balanced, LEVEL_COLUMNS, SUMMARY_COLUMNS, VOLUME_COLUMNS,
};
use super::interpolation::level_to_volume;
use super::types::*;

/// 从 SchedulerInput 的 files 中查找 TSV
fn find_table<'a>(files: &'a [(String, TsvTable)], key: &str) -> Option<&'a TsvTable> {
    // 先按 key 精确匹配
    files.iter().find(|(k, _)| k == key).map(|(_, t)| t)
}

/// 从 TSV 表中提取某列的所有数值 (跳过表头, 按行)
fn extract_column(table: &TsvTable, col_name: &str) -> Vec<f64> {
    let col_idx = table.headers.iter().position(|h| h == col_name);
    match col_idx {
        Some(idx) => table
            .rows
            .iter()
            .map(|row| {
                row.get(idx)
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0)
            })
            .collect(),
        None => vec![0.0; table.rows.len()],
    }
}

/// 从日期列提取日期列表
fn extract_dates(table: &TsvTable) -> Vec<String> {
    let col_idx = table
        .headers
        .iter()
        .position(|h| h == "日期")
        .unwrap_or(0);
    table
        .rows
        .iter()
        .map(|row| row.get(col_idx).cloned().unwrap_or_default())
        .collect()
}

/// 获取单行单列值 (用于 SW_CS, SW_PS, SW_MB 等只有 1 数据行的表)
fn get_single_row_value(table: &TsvTable, col_name: &str) -> f64 {
    let col_idx = table.headers.iter().position(|h| h == col_name);
    match col_idx {
        Some(idx) => table
            .rows
            .first()
            .and_then(|row| row.get(idx))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0),
        None => 0.0,
    }
}

/// 解析库容曲线
fn parse_storage_curves(table: &TsvTable) -> HashMap<String, (Vec<f64>, Vec<f64>)> {
    let mut curves = HashMap::new();
    let name_idx = table
        .headers
        .iter()
        .position(|h| h == "分区名称")
        .unwrap_or(0);

    let vol_indices: Vec<usize> = VOLUME_COLUMNS
        .iter()
        .filter_map(|c| table.headers.iter().position(|h| h == *c))
        .collect();
    let lvl_indices: Vec<usize> = LEVEL_COLUMNS
        .iter()
        .filter_map(|c| table.headers.iter().position(|h| h == *c))
        .collect();

    for row in &table.rows {
        let name = row.get(name_idx).cloned().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let volumes: Vec<f64> = vol_indices
            .iter()
            .map(|&i| row.get(i).and_then(|v| v.parse().ok()).unwrap_or(0.0))
            .collect();
        let levels: Vec<f64> = lvl_indices
            .iter()
            .map(|&i| row.get(i).and_then(|v| v.parse().ok()).unwrap_or(0.0))
            .collect();
        curves.insert(name, (levels, volumes));
    }
    curves
}

/// 解析河区-水库映射
fn parse_reservoir_mapping(table: &TsvTable) -> HashMap<String, Vec<String>> {
    let mut mapping = HashMap::new();
    let name_idx = table
        .headers
        .iter()
        .position(|h| h == "分区名称")
        .unwrap_or(0);
    let count_idx = table
        .headers
        .iter()
        .position(|h| h == "包含水库数量")
        .unwrap_or(1);

    for row in &table.rows {
        let name = row.get(name_idx).cloned().unwrap_or_default();
        let count: usize = row
            .get(count_idx)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if count == 0 || name.is_empty() {
            continue;
        }
        let mut reservoirs = Vec::new();
        for i in 0..count {
            let col = count_idx + 1 + i;
            if let Some(val) = row.get(col) {
                let v = val.trim();
                if !v.is_empty() {
                    reservoirs.push(v.to_string());
                }
            }
        }
        mapping.insert(name, reservoirs);
    }
    mapping
}

/// Step 3: 生成各河区水库来水
fn generate_reservoir_inflow(
    sk_table: &TsvTable,
    hq_sk_table: &TsvTable,
) -> HashMap<String, Vec<f64>> {
    let mapping = parse_reservoir_mapping(hq_sk_table);
    let nrows = sk_table.rows.len();
    let mut result: HashMap<String, Vec<f64>> = HashMap::new();

    for (district, reservoirs) in &mapping {
        let mut total = vec![0.0; nrows];
        for reservoir in reservoirs {
            let col_data = extract_column(sk_table, reservoir);
            for (i, v) in col_data.iter().enumerate() {
                if i < nrows {
                    total[i] += v;
                }
            }
        }
        result.insert(district.clone(), total);
    }
    result
}

/// 从文件集合中获取某个 key 对应某个河区列的数据
fn get_district_column(
    files: &[(String, TsvTable)],
    file_key: &str,
    district: &str,
) -> Vec<f64> {
    match find_table(files, file_key) {
        Some(table) => extract_column(table, district),
        None => Vec::new(),
    }
}

/// 7 步调度流程
pub fn run_scheduler(input: &SchedulerInput) -> Result<SchedulerOutput, String> {
    let files = &input.files;

    // Step 1: 加载基础表
    let hq_zq = find_table(files, "HQ_ZQ").ok_or("缺少 static_HQ_ZQ.txt")?;
    let hq_sk = find_table(files, "HQ_SK").ok_or("缺少 static_HQ_SK.txt")?;
    let sk = find_table(files, "SK").ok_or("缺少 input_SK.txt")?;
    let sw_cs = find_table(files, "SW_CS").ok_or("缺少 input_SW_CS.txt")?;
    let sw_ps = find_table(files, "SW_PS").ok_or("缺少 static_SW_PS.txt")?;
    let sw_mb = find_table(files, "SW_MB").ok_or("缺少 input_SW_MB.txt")?;
    let xs_st = find_table(files, "XS_ST").ok_or("缺少 input_XS_ST.txt")?;

    // 解析库容曲线
    let storage_curves = parse_storage_curves(hq_zq);

    // 获取日期序列 (从 XS_ST)
    let dates = extract_dates(xs_st);
    let ndays = dates.len();

    // 获取河区列表 (从 SW_CS 的列名, 排除 "日期")
    let districts: Vec<String> = sw_cs
        .headers
        .iter()
        .filter(|h| *h != "日期")
        .cloned()
        .collect();

    // Step 3: 水库来水
    let reservoir_inflow = generate_reservoir_inflow(sk, hq_sk);

    // Step 4: 处理各河区
    let mut district_results: Vec<DistrictData> = Vec::new();

    for district in &districts {
        let code = district_code(district).to_string();

        // 需水数据
        let agri_demand = get_district_column(files, "GPS_GGXS", district);
        let eco_demand_raw = get_district_column(files, "XS_ST", district);
        let non_agri_demand = get_district_column(files, "XS_FN", district);

        // 来水数据
        let plain_water = get_district_column(files, "GPS_PYCS", district);
        let external_supply_raw = get_district_column(files, "LS_QT", district);
        let river_supply = get_district_column(files, "FQJL", district);
        let reservoir_supply_raw = reservoir_inflow.get(district.as_str()).cloned();

        // 补齐长度
        let pad = |v: &[f64]| -> Vec<f64> {
            if v.len() >= ndays {
                v[..ndays].to_vec()
            } else {
                let mut r = v.to_vec();
                r.resize(ndays, 0.0);
                r
            }
        };

        let agri = pad(&agri_demand);
        let eco = pad(&eco_demand_raw);
        let non_agri = pad(&non_agri_demand);
        let plain = pad(&plain_water);
        let mut external = pad(&external_supply_raw);
        let river = pad(&river_supply);
        let reservoir = reservoir_supply_raw
            .map(|v| pad(&v))
            .unwrap_or_else(|| vec![0.0; ndays]);

        // 需水量 = 农业 + 非农
        let basic_demand: Vec<f64> = agri.iter().zip(&non_agri).map(|(a, b)| a + b).collect();

        // 动态平衡河区: 其他外供 = (农业+非农+其他生态需水) - 河网供水 - 水库供水
        if is_balanced(district) {
            for i in 0..ndays {
                external[i] = (agri[i] + non_agri[i] + eco[i]) - river[i] - reservoir[i];
            }
        }

        // 合计来水 = 其他外供 + 河网供水 + 水库供水 (不含平原产水? 按 Python 代码实际上来水合计不含平原产水的sum)
        // Python: inflow_df['合计来水'] = inflow_df[['其他外供', '河网供水', '水库供水']].sum(axis=1)
        let total_inflow: Vec<f64> = (0..ndays)
            .map(|i| external[i] + river[i] + reservoir[i])
            .collect();

        // 构造 inflow DailyRow
        let inflow_rows: Vec<DailyRow> = (0..ndays)
            .map(|i| DailyRow {
                date: dates[i].clone(),
                values: vec![
                    ("平原产水".into(), plain[i]),
                    ("其他外供".into(), external[i]),
                    ("河网供水".into(), river[i]),
                    ("水库供水".into(), reservoir[i]),
                    ("合计来水".into(), total_inflow[i]),
                ],
            })
            .collect();

        // 构造 demand DailyRow
        let demand_rows: Vec<DailyRow> = (0..ndays)
            .map(|i| DailyRow {
                date: dates[i].clone(),
                values: vec![
                    ("农业需水".into(), agri[i]),
                    ("其他生态需水".into(), eco[i]),
                    ("非农需水".into(), non_agri[i]),
                    ("需水量".into(), basic_demand[i]),
                ],
            })
            .collect();

        // Step 5: 水平衡计算
        let initial_level = get_single_row_value(sw_cs, district);
        let drainage_level = get_single_row_value(sw_ps, district);
        let target_level = get_single_row_value(sw_mb, district);

        let (curve_levels, curve_volumes) = storage_curves
            .get(district.as_str())
            .cloned()
            .unwrap_or_else(|| (vec![0.0; 5], vec![0.0; 5]));

        // 获取高/低水位容积
        let high_idx = 3; // 高水位 index
        let low_idx = 1; // 低水位 index
        let high_level = curve_levels.get(high_idx).copied().unwrap_or(0.0);
        let low_level = curve_levels.get(low_idx).copied().unwrap_or(0.0);
        let high_vol = level_to_volume(&curve_levels, &curve_volumes, high_level);
        let low_vol = level_to_volume(&curve_levels, &curve_volumes, low_level);

        let balance = calculate_water_balance(
            &dates,
            &total_inflow,
            &basic_demand,
            &eco,
            initial_level,
            drainage_level,
            target_level,
            &curve_levels,
            &curve_volumes,
            high_vol,
            low_vol,
        );

        district_results.push(DistrictData {
            name: district.clone(),
            code,
            inflow: inflow_rows,
            demand: demand_rows,
            balance,
        });
    }

    // Step 7: 生成汇总
    let summary = generate_summary(&district_results, &dates);

    // 计算统计量
    let districts_processed = district_results.len();
    let mut total_demand = 0.0;
    let mut total_supply = 0.0;
    let mut total_shortage = 0.0;

    for row in &summary {
        for (k, v) in &row.fields {
            match k.as_str() {
                "总需水量" => total_demand += v,
                "合计来水" => total_supply += v,
                "缺水(浙东需供)" => total_shortage += v,
                _ => {}
            }
        }
    }

    Ok(SchedulerOutput {
        districts: district_results,
        summary,
        districts_processed,
        total_water_demand: total_demand,
        total_water_supply: total_supply,
        total_shortage,
    })
}

/// 生成汇总: 按日期合并所有河区数据
fn generate_summary(districts: &[DistrictData], dates: &[String]) -> Vec<WaterBalanceRow> {
    let ndays = dates.len();

    // 收集所有列名 (从来水+需水+平衡)
    let columns: Vec<&str> = SUMMARY_COLUMNS.to_vec();

    let mut summary: Vec<WaterBalanceRow> = (0..ndays)
        .map(|i| {
            let fields: Vec<(String, f64)> = columns.iter().map(|c| (c.to_string(), 0.0)).collect();
            WaterBalanceRow {
                date: dates[i].clone(),
                fields,
            }
        })
        .collect();

    // 累加每个河区
    for district in districts {
        for (day_idx, balance_row) in district.balance.iter().enumerate() {
            if day_idx >= ndays {
                break;
            }
            // 合并来水字段
            if let Some(inflow_row) = district.inflow.get(day_idx) {
                for (col, val) in &inflow_row.values {
                    if let Some(entry) = summary[day_idx]
                        .fields
                        .iter_mut()
                        .find(|(k, _)| k == col)
                    {
                        entry.1 += val;
                    }
                }
            }
            // 合并需水字段
            if let Some(demand_row) = district.demand.get(day_idx) {
                for (col, val) in &demand_row.values {
                    if let Some(entry) = summary[day_idx]
                        .fields
                        .iter_mut()
                        .find(|(k, _)| k == col)
                    {
                        entry.1 += val;
                    }
                }
            }
            // 合并平衡字段
            for (col, val) in &balance_row.fields {
                if let Some(entry) = summary[day_idx]
                    .fields
                    .iter_mut()
                    .find(|(k, _)| k == col)
                {
                    entry.1 += val;
                }
            }
        }
    }

    summary
}
