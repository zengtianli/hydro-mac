//! capacity CLI 编排 —— 复刻自原 apps/capacity/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
//!
//! 与 SSOT 的两处适配(非数值逻辑):
//! - 原 io.rs 用 chrono 做 Excel 序列日期→日期串;此处用 civil_from_days 纯算法实现,免引 chrono。
//! - 原 hydro_common::{cell_str,cell_f64,write_header_row} 此处内联为本模块私有函数
//!   (不动共享 common.rs,避免多计算器并行接入时的合并冲突;round2 用 crate::common 已有的)。
//! - 过程表/结果表的分段累计原用 HashMap(行序每次运行随机);此处改保序 Vec,
//!   行序 = 分段物理顺序(段1→支流→混合→段2→汇总),数值不变 —— golden test 可字节级比对。
use crate::capacity::aggregation::{daily_to_monthly, hydro_year_reorder, monthly_to_yearly_avg};
use crate::capacity::physics::{reservoir_capacity, velocity};
use crate::capacity::sample_data;
use crate::capacity::segments::calc_zone_segments;
use crate::capacity::types::*;
use crate::common::round2;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::HashMap;

// ── 采样 / IO(复刻自 commands/io.rs) ──

pub fn get_sample_data() -> CapacityInput {
    sample_data::sample_input()
}

pub fn read_excel(path: String) -> Result<CapacityInput, String> {
    let mut wb: Xlsx<_> = open_workbook(&path).map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_names: Vec<String> = wb.sheet_names().to_vec();

    // 找输入 sheet (含 "输入" 和 "功能区")
    let input_sheet = sheet_names
        .iter()
        .find(|n| n.contains("输入") && n.contains("功能区"))
        .cloned()
        .ok_or("未找到包含'功能区'和'输入'的 sheet")?;

    let range = wb
        .worksheet_range(&input_sheet)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    // 行索引 (0-based)
    const ROW_ZONE_COUNT: usize = 0;
    const ROW_SCHEME_COUNT: usize = 1;
    const ROW_CS: usize = 2;
    const ROW_K: usize = 3;
    const ROW_B: usize = 4;
    const ROW_A: usize = 5;
    const ROW_BETA: usize = 6;
    const ROW_MAIN_LENGTH: usize = 7;
    const ROW_MAIN_C0: usize = 8;
    const ROW_BRANCH_COUNT: usize = 9;
    const ROW_MAIN_NAME: usize = 10;
    const ROW_BRANCH_START: usize = 11;
    const ROWS_PER_BRANCH: usize = 5;
    const DATA_COL_START: usize = 4;

    let zone_count = cell_f64(&range, ROW_ZONE_COUNT, 1) as usize;
    let _scheme_count = cell_f64(&range, ROW_SCHEME_COUNT, 1) as usize;

    let mut zones = Vec::new();
    let mut flow_col_map = Vec::new();

    for i in 0..zone_count {
        let col = DATA_COL_START + i;
        let _zone_id = cell_str(&range, ROW_ZONE_COUNT, col);
        let name = cell_str(&range, ROW_SCHEME_COUNT, col);
        let cs = cell_f64(&range, ROW_CS, col);
        let k = cell_f64(&range, ROW_K, col);
        let b = cell_f64(&range, ROW_B, col);
        let a = cell_f64(&range, ROW_A, col);
        let beta = cell_f64(&range, ROW_BETA, col);
        let main_length = cell_f64(&range, ROW_MAIN_LENGTH, col);
        let main_c0 = cell_f64(&range, ROW_MAIN_C0, col);
        let branch_count = cell_f64(&range, ROW_BRANCH_COUNT, col) as usize;
        let main_name = cell_str(&range, ROW_MAIN_NAME, col);

        let mut branches = Vec::new();
        let mut branch_names = Vec::new();
        for j in 0..branch_count {
            let base_row = ROW_BRANCH_START + j * ROWS_PER_BRANCH;
            if base_row + 4 < range.height() as usize {
                let br_name = cell_str(&range, base_row + 1, col);
                let br_length = cell_f64(&range, base_row + 2, col);
                let br_join_pos = cell_f64(&range, base_row + 3, col);
                let br_c0 = cell_f64(&range, base_row + 4, col);
                branch_names.push(br_name.clone());
                branches.push(Branch {
                    name: br_name,
                    length: br_length,
                    join_position: br_join_pos,
                    c0: br_c0,
                });
            }
        }

        let zone = Zone {
            zone_id: name.clone(),
            name: name.clone(),
            water_class: String::new(),
            length: main_length,
            k,
            b,
            a,
            beta,
            cs,
            c0: main_c0,
            main_name: main_name.clone(),
            branches,
        };

        flow_col_map.push((
            name.clone(),
            FlowColumnMap {
                main: main_name,
                branches: branch_names,
            },
        ));

        zones.push(zone);
    }

    // 解析逐日流量 (方案1)
    let flow_sheet = sheet_names
        .iter()
        .find(|n| n.contains("逐日流量"))
        .cloned()
        .ok_or("未找到逐日流量 sheet")?;

    let flow_range = wb
        .worksheet_range(&flow_sheet)
        .map_err(|e| format!("读取流量表失败: {}", e))?;

    let daily_flow = parse_daily_sheet(&flow_range)?;

    // 解析水库 (可选)
    let (reservoir_zones, daily_volume) = parse_reservoir(&mut wb, &sheet_names);

    Ok(CapacityInput {
        zones,
        flow_col_map,
        daily_flow,
        reservoir_zones,
        daily_volume,
    })
}

fn parse_daily_sheet(range: &calamine::Range<calamine::Data>) -> Result<Vec<DailyRow>, String> {
    let ncols = range.width();
    if ncols < 2 {
        return Err("流量表列数不足".into());
    }

    // 第一行是表头
    let headers: Vec<String> = (1..ncols).map(|c| cell_str(range, 0, c)).collect();

    let mut rows = Vec::new();
    for r in 1..range.height() {
        let date = parse_date_cell(range, r, 0);
        if date.is_empty() {
            continue;
        }

        let values: Vec<(String, f64)> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.clone(), cell_f64(range, r, i + 1)))
            .collect();

        rows.push(DailyRow { date, values });
    }

    Ok(rows)
}

fn parse_date_cell(range: &calamine::Range<calamine::Data>, row: usize, col: usize) -> String {
    use calamine::Data;
    match range.get((row, col)) {
        Some(Data::DateTime(dt)) => excel_serial_to_iso(dt.as_f64()),
        Some(Data::Float(f)) => excel_serial_to_iso(*f),
        Some(Data::String(s)) => {
            // Try to normalize date string
            let s = s.trim().to_string();
            if s.contains('/') {
                s.replace('/', "-")
            } else {
                s
            }
        }
        _ => String::new(),
    }
}

/// Excel 序列号(1899-12-30 起算天数)→ "YYYY-MM-DD"。
/// 原 io.rs 用 chrono;此处等价纯算法(Howard Hinnant civil_from_days),免引依赖。
fn excel_serial_to_iso(serial: f64) -> String {
    let days_since_epoch = serial as i64 - 25569; // 25569 = 1970-01-01 的 Excel 序列号
    let (y, m, d) = civil_from_days(days_since_epoch);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// days since 1970-01-01 → (year, month, day) 公历。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn parse_reservoir(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_names: &[String],
) -> (Vec<ReservoirZone>, Vec<DailyRow>) {
    let zones_sheet = sheet_names.iter().find(|n| n.contains("水库功能区基础信息"));
    let volume_sheet = sheet_names.iter().find(|n| n.contains("水库逐日库容"));

    if zones_sheet.is_none() || volume_sheet.is_none() {
        return (vec![], vec![]);
    }

    let zones_name = zones_sheet.unwrap().clone();
    let volume_name = volume_sheet.unwrap().clone();

    let z_range = match wb.worksheet_range(&zones_name) {
        Ok(r) => r,
        Err(_) => return (vec![], vec![]),
    };

    // 表头: 功能区 | 名称 | K(1/s) | b | Cs | C0
    let mut reservoir_zones = Vec::new();
    for r in 1..z_range.height() {
        let zone_id = cell_str(&z_range, r, 0);
        if zone_id.is_empty() {
            continue;
        }
        reservoir_zones.push(ReservoirZone {
            zone_id,
            name: cell_str(&z_range, r, 1),
            k: cell_f64(&z_range, r, 2),
            b: cell_f64(&z_range, r, 3),
            cs: cell_f64(&z_range, r, 4),
            c0: cell_f64(&z_range, r, 5),
        });
    }

    let v_range = match wb.worksheet_range(&volume_name) {
        Ok(r) => r,
        Err(_) => return (reservoir_zones, vec![]),
    };

    let daily_volume = parse_daily_sheet(&v_range).unwrap_or_default();

    (reservoir_zones, daily_volume)
}

pub fn write_results(path: String, output: CapacityOutput, start_month: u32) -> Result<(), String> {
    use rust_xlsxwriter::*;

    let mut wb = Workbook::new();
    let bold = Format::new().set_bold();
    let num_fmt = Format::new().set_num_format("0.0000");

    let month_order = hydro_year_reorder(start_month);

    // Sheet 1: 逐月流量
    write_monthly_sheet(&mut wb, "逐月流量", &output.monthly_flow, &bold, &num_fmt)?;

    // Sheet 2: 逐月流速
    write_monthly_sheet(&mut wb, "逐月流速", &output.monthly_velocity, &bold, &num_fmt)?;

    // Sheet 3: 功能区月平均流速
    write_zone_avg_sheet(
        &mut wb,
        "功能区月平均流速",
        &output.zone_avg_velocity,
        &month_order,
        "年平均",
        &bold,
        &num_fmt,
    )?;

    // Sheet 4: 功能区月平均纳污能力
    write_zone_avg_sheet(
        &mut wb,
        "功能区月平均纳污能力",
        &output.zone_avg_capacity,
        &month_order,
        "年合计",
        &bold,
        &num_fmt,
    )?;

    // Sheet 5: 水库逐月库容 (optional)
    if !output.reservoir_monthly_volume.is_empty() {
        write_monthly_sheet(
            &mut wb,
            "水库逐月库容",
            &output.reservoir_monthly_volume,
            &bold,
            &num_fmt,
        )?;
    }

    // Sheet 6: 水库功能区月平均纳污能力 (optional)
    if !output.reservoir_zone_avg_capacity.is_empty() {
        write_zone_avg_sheet(
            &mut wb,
            "水库功能区月平均纳污能力",
            &output.reservoir_zone_avg_capacity,
            &month_order,
            "年合计",
            &bold,
            &num_fmt,
        )?;
    }

    wb.save(&path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

fn write_monthly_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    rows: &[MonthlyRow],
    bold: &rust_xlsxwriter::Format,
    num_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    if rows.is_empty() {
        return Ok(());
    }

    // Collect column names from first row
    let col_names: Vec<String> = rows[0].values.iter().map(|(c, _)| c.clone()).collect();

    // Write header
    let mut headers: Vec<&str> = vec!["年", "月"];
    let col_refs: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();
    headers.extend(col_refs);
    write_header_row(ws, 0, &headers, bold).map_err(|e| e.to_string())?;

    for (r, row) in rows.iter().enumerate() {
        let excel_row = (r + 1) as u32;
        ws.write_number(excel_row, 0, row.year as f64)
            .map_err(|e| e.to_string())?;
        ws.write_number(excel_row, 1, row.month as f64)
            .map_err(|e| e.to_string())?;
        for (c, (_, v)) in row.values.iter().enumerate() {
            ws.write_number_with_format(excel_row, (c + 2) as u16, *v, num_fmt)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn write_zone_avg_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    rows: &[ZoneMonthlyAvgRow],
    month_order: &[u32],
    summary_label: &str,
    bold: &rust_xlsxwriter::Format,
    num_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    // Header: 功能区 | 4月 | 5月 | ... | 3月 | 年合计/年平均
    let mut headers: Vec<String> = vec!["功能区".into()];
    for &m in month_order {
        headers.push(format!("{}月", m));
    }
    headers.push(summary_label.into());

    let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
    write_header_row(ws, 0, &header_refs, bold).map_err(|e| e.to_string())?;

    for (r, row) in rows.iter().enumerate() {
        let excel_row = (r + 1) as u32;
        ws.write_string(excel_row, 0, &row.zone_id)
            .map_err(|e| e.to_string())?;

        for (c, &m) in month_order.iter().enumerate() {
            let val = row.months[(m - 1) as usize];
            ws.write_number_with_format(excel_row, (c + 1) as u16, val, num_fmt)
                .map_err(|e| e.to_string())?;
        }

        ws.write_number_with_format(
            excel_row,
            (month_order.len() + 1) as u16,
            round2(row.summary),
            num_fmt,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── 计算编排(复刻自 commands/calc.rs) ──

/// 编排: 输入 → 全部结果
pub fn run_capacity(input: CapacityInput) -> Result<CapacityOutput, String> {
    let zones = &input.zones;
    let zone_ids: Vec<String> = zones.iter().map(|z| z.zone_id.clone()).collect();

    // 建流量列 → zone 映射
    let flow_map: HashMap<String, &FlowColumnMap> = input
        .flow_col_map
        .iter()
        .map(|(k, v)| (k.clone(), v))
        .collect();

    // 收集所有需要聚合的流量列
    let mut all_flow_cols: Vec<String> = Vec::new();
    for (_, m) in &input.flow_col_map {
        if !all_flow_cols.contains(&m.main) {
            all_flow_cols.push(m.main.clone());
        }
        for br in &m.branches {
            if !all_flow_cols.contains(br) {
                all_flow_cols.push(br.clone());
            }
        }
    }

    // 1. 逐月流量
    let monthly_flow = daily_to_monthly(&input.daily_flow, &all_flow_cols);

    // 2. 逐月流速 (基于干流流量)
    let monthly_velocity = calc_monthly_velocity(&monthly_flow, zones, &flow_map);

    // 3. 逐日纳污能力 + 分段累计
    let (daily_capacity, seg_accum) =
        calc_daily_capacity_with_segments(&input.daily_flow, zones, &flow_map);

    // 4. 逐月纳污能力 (从逐日聚合)
    let monthly_capacity = daily_to_monthly(&daily_capacity, &zone_ids);

    // 5. 功能区月平均
    let zone_avg_velocity = monthly_to_yearly_avg(&monthly_velocity, &zone_ids, false);
    let zone_avg_capacity = monthly_to_yearly_avg(&monthly_capacity, &zone_ids, true);

    // 6. 过程表 & 结果表
    let process_table = build_process_table(&seg_accum, zones);
    let result_table = build_result_table(&seg_accum, zones);

    // 水库计算
    let (reservoir_monthly_volume, reservoir_zone_avg_capacity) =
        if !input.reservoir_zones.is_empty() && !input.daily_volume.is_empty() {
            calc_reservoir_all(&input.reservoir_zones, &input.daily_volume)
        } else {
            (vec![], vec![])
        };

    Ok(CapacityOutput {
        monthly_flow,
        monthly_velocity,
        monthly_capacity,
        zone_avg_velocity,
        zone_avg_capacity,
        process_table,
        result_table,
        reservoir_monthly_volume,
        reservoir_zone_avg_capacity,
    })
}

fn calc_monthly_velocity(
    monthly_flow: &[MonthlyRow],
    zones: &[Zone],
    flow_map: &HashMap<String, &FlowColumnMap>,
) -> Vec<MonthlyRow> {
    monthly_flow
        .iter()
        .map(|row| {
            let values = zones
                .iter()
                .map(|zone| {
                    let col_info = flow_map.get(&zone.zone_id);
                    let main_col = col_info.map(|m| m.main.as_str()).unwrap_or(&zone.zone_id);
                    let q = row
                        .values
                        .iter()
                        .find(|(c, _)| c == main_col)
                        .map(|(_, v)| *v)
                        .unwrap_or(0.0);
                    let u = velocity(q, zone.a, zone.beta);
                    (zone.zone_id.clone(), u)
                })
                .collect();
            MonthlyRow {
                year: row.year,
                month: row.month,
                values,
            }
        })
        .collect()
}

/// 每功能区的分段累计表 —— 保序(插入序 = 分段物理顺序),保证输出行序确定。
type ZoneSegAccum = Vec<(String, SegAccum)>;

/// 逐日分段计算
fn calc_daily_capacity_with_segments(
    daily_flow: &[DailyRow],
    zones: &[Zone],
    flow_map: &HashMap<String, &FlowColumnMap>,
) -> (Vec<DailyRow>, HashMap<String, ZoneSegAccum>) {
    let mut seg_accum: HashMap<String, ZoneSegAccum> = zones
        .iter()
        .map(|z| (z.zone_id.clone(), ZoneSegAccum::new()))
        .collect();

    let result: Vec<DailyRow> = daily_flow
        .iter()
        .map(|row| {
            let mut c_current = 0.0_f64;
            let values: Vec<(String, f64)> = zones
                .iter()
                .enumerate()
                .map(|(i, zone)| {
                    let col_info = flow_map.get(&zone.zone_id);
                    let main_col =
                        col_info.map(|m| m.main.as_str()).unwrap_or(&zone.zone_id);
                    let main_q = row
                        .values
                        .iter()
                        .find(|(c, _)| c == main_col)
                        .map(|(_, v)| *v)
                        .unwrap_or(0.0);

                    // 收集支流流量
                    let mut branch_flows = HashMap::new();
                    if let Some(info) = col_info {
                        for br_col in &info.branches {
                            let bq = row
                                .values
                                .iter()
                                .find(|(c, _)| c == br_col)
                                .map(|(_, v)| *v)
                                .unwrap_or(0.0);
                            branch_flows.insert(br_col.clone(), bq);
                        }
                    }

                    // 上游浓度传递
                    let zone_for_calc = if zone.c0 <= 0.0 && i > 0 {
                        let mut z = zone.clone();
                        z.c0 = c_current;
                        z
                    } else {
                        zone.clone()
                    };

                    let (segments, total_w, new_c) =
                        calc_zone_segments(&zone_for_calc, main_q, &branch_flows);
                    c_current = new_c;

                    // 累计分段数据(保序:首日插入序即物理分段序)
                    let zone_acc = seg_accum.get_mut(&zone.zone_id).unwrap();
                    for seg in &segments {
                        let idx = match zone_acc.iter().position(|(n, _)| n == &seg.name) {
                            Some(i) => i,
                            None => {
                                zone_acc.push((
                                    seg.name.clone(),
                                    SegAccum {
                                        seg_type: seg.seg_type.clone(),
                                        length: seg.length,
                                        w_sum: 0.0,
                                        q_sum: 0.0,
                                        c0_sum: 0.0,
                                        c_out_sum: 0.0,
                                        count: 0,
                                        remark: seg.remark.clone(),
                                    },
                                ));
                                zone_acc.len() - 1
                            }
                        };
                        let entry = &mut zone_acc[idx].1;
                        entry.w_sum += seg.w;
                        entry.q_sum += seg.q;
                        entry.c0_sum += seg.c0;
                        entry.c_out_sum += seg.c_out;
                        entry.count += 1;
                    }

                    (zone.zone_id.clone(), total_w)
                })
                .collect();

            DailyRow {
                date: row.date.clone(),
                values,
            }
        })
        .collect();

    (result, seg_accum)
}

struct SegAccum {
    seg_type: String,
    length: f64,
    w_sum: f64,
    q_sum: f64,
    c0_sum: f64,
    c_out_sum: f64,
    count: u32,
    remark: String,
}

fn build_process_table(
    seg_accum: &HashMap<String, ZoneSegAccum>,
    zones: &[Zone],
) -> Vec<ProcessRow> {
    let mut rows = Vec::new();
    for zone in zones {
        if let Some(zone_segs) = seg_accum.get(&zone.zone_id) {
            for (seg_name, acc) in zone_segs {
                let cnt = if acc.count > 0 { acc.count } else { 1 };
                rows.push(ProcessRow {
                    zone_id: zone.zone_id.clone(),
                    seg_name: seg_name.clone(),
                    seg_type: acc.seg_type.clone(),
                    length: acc.length,
                    avg_q: acc.q_sum / cnt as f64,
                    avg_c0: acc.c0_sum / cnt as f64,
                    avg_c_out: acc.c_out_sum / cnt as f64,
                    avg_w: acc.w_sum / cnt as f64,
                    remark: acc.remark.clone(),
                });
            }
        }
    }
    rows
}

fn build_result_table(
    seg_accum: &HashMap<String, ZoneSegAccum>,
    zones: &[Zone],
) -> Vec<ProcessRow> {
    let mut rows = Vec::new();
    for zone in zones {
        if let Some(zone_segs) = seg_accum.get(&zone.zone_id) {
            for (seg_name, acc) in zone_segs {
                if acc.seg_type != "干流段" && acc.seg_type != "汇总" {
                    continue;
                }
                let cnt = if acc.count > 0 { acc.count } else { 1 };
                rows.push(ProcessRow {
                    zone_id: zone.zone_id.clone(),
                    seg_name: seg_name.clone(),
                    seg_type: acc.seg_type.clone(),
                    length: acc.length,
                    avg_q: acc.q_sum / cnt as f64,
                    avg_c0: acc.c0_sum / cnt as f64,
                    avg_c_out: acc.c_out_sum / cnt as f64,
                    avg_w: acc.w_sum / cnt as f64,
                    remark: acc.remark.clone(),
                });
            }
        }
    }
    rows
}

fn calc_reservoir_all(
    reservoir_zones: &[ReservoirZone],
    daily_volume: &[DailyRow],
) -> (Vec<MonthlyRow>, Vec<ZoneMonthlyAvgRow>) {
    let r_zone_ids: Vec<String> = reservoir_zones.iter().map(|z| z.zone_id.clone()).collect();

    // 逐月库容
    let monthly_vol = daily_to_monthly(daily_volume, &r_zone_ids);

    // 逐月纳污能力
    let monthly_cap: Vec<MonthlyRow> = monthly_vol
        .iter()
        .map(|row| {
            let values = reservoir_zones
                .iter()
                .map(|zone| {
                    let v = row
                        .values
                        .iter()
                        .find(|(c, _)| c == &zone.zone_id)
                        .map(|(_, v)| *v)
                        .unwrap_or(0.0);
                    let w = reservoir_capacity(zone.k, zone.cs, v, zone.b);
                    (zone.zone_id.clone(), w)
                })
                .collect();
            MonthlyRow {
                year: row.year,
                month: row.month,
                values,
            }
        })
        .collect();

    // 功能区月平均
    let zone_avg = monthly_to_yearly_avg(&monthly_cap, &r_zone_ids, true);

    (monthly_vol, zone_avg)
}

// ── Excel 单元格辅助(内联自 hydro-common,见文件头注释) ──

/// 读取单元格为字符串
fn cell_str(range: &calamine::Range<calamine::Data>, row: usize, col: usize) -> String {
    range
        .get((row, col))
        .map(|c| c.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 读取单元格为 f64
fn cell_f64(range: &calamine::Range<calamine::Data>, row: usize, col: usize) -> f64 {
    use calamine::Data;
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

/// rust_xlsxwriter 写入辅助: 写一行字符串表头
fn write_header_row(
    ws: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    headers: &[&str],
    fmt: &rust_xlsxwriter::Format,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(row, c as u16, *h, fmt)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_serial_to_iso() {
        assert_eq!(excel_serial_to_iso(25569.0), "1970-01-01");
        assert_eq!(excel_serial_to_iso(45444.0), "2024-06-01"); // 与 chrono 基准核对
        assert_eq!(excel_serial_to_iso(45473.0), "2024-06-30");
    }

    #[test]
    fn test_run_capacity_sample() {
        let out = run_capacity(get_sample_data()).expect("sample 计算失败");
        assert_eq!(out.monthly_flow.len(), 1); // 2024-06 单月
        assert_eq!(out.zone_avg_capacity.len(), 2);
        assert!(!out.result_table.is_empty());
        assert_eq!(out.reservoir_zone_avg_capacity.len(), 1);
    }
}
