//! reservoir CLI 编排 —— 复刻自原 apps/reservoir/src-tauri/src/commands/{calc,io}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
//! calamine 单元格 helper(cell_str/cell_f64/write_header_row)自 hydro-common 内联
//! (不动共享 common.rs,保持本模块自包含;多计算器接入后可由主会话上提)。
use crate::reservoir::interp::create_daily_lookup;
use crate::reservoir::scheduler::run_cascade;
use crate::reservoir::types::*;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};

// ===== hydro-common 内联 helper(与 crates/hydro-common/src/lib.rs 逐字一致) =====

/// 读取单元格为去空白字符串
fn cell_str(range: &Range<Data>, row: usize, col: usize) -> String {
    range
        .get((row, col))
        .map(|c| c.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 读取单元格为 f64
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

// ===== 命令(原 #[tauri::command] 纯函数化) =====

pub fn get_sample_data() -> ReservoirInput {
    crate::reservoir::sample_data::sample_input()
}

pub fn run_schedule(input: ReservoirInput) -> Result<ScheduleOutput, String> {
    std::panic::catch_unwind(move || run_cascade(&input))
        .map_err(|_| "计算过程中出现内部错误".to_string())
}

pub fn read_excel(path: String) -> Result<ReservoirInput, String> {
    let mut wb: Xlsx<_> =
        open_workbook(&path).map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_names: Vec<String> = wb.sheet_names().to_vec();

    // 1. Read 计算参数 sheet
    let params_range = wb
        .worksheet_range("计算参数")
        .map_err(|e| format!("读取'计算参数'失败: {}", e))?;

    let mut up_name = String::new();
    let mut down_name = String::new();
    let mut up_v_special = Vec::new();
    let mut down_v_special = Vec::new();
    let mut need_add_users = Vec::new();
    let mut user_special = Vec::new();
    let mut user_stop_supply = Vec::new();
    let mut up_eco_as_inflow = true;

    for r in 0..params_range.height() as usize {
        let key = cell_str(&params_range, r, 0);
        match key.as_str() {
            "上库" => up_name = cell_str(&params_range, r, 1),
            "下库" => down_name = cell_str(&params_range, r, 1),
            "上库特征库容" => {
                for c in 1..params_range.width() as usize {
                    let v = cell_f64(&params_range, r, c);
                    if v > 0.0 {
                        up_v_special.push(v);
                    }
                }
            }
            "下库特征库容" => {
                for c in 1..params_range.width() as usize {
                    let v = cell_f64(&params_range, r, c);
                    if v > 0.0 {
                        down_v_special.push(v);
                    }
                }
            }
            k if k.contains("需补水的用水户") => {
                for c in 1..params_range.width() as usize {
                    let s = cell_str(&params_range, r, c);
                    if !s.is_empty() {
                        need_add_users.push(s);
                    }
                }
            }
            k if k.contains("额外再补用水户") => {
                let name = cell_str(&params_range, r, 1);
                if !name.is_empty() {
                    // Look for the flow value in the next row
                    let flow_val = if r + 1 < params_range.height() as usize {
                        cell_f64(&params_range, r + 1, 1)
                    } else {
                        0.0
                    };
                    user_special.push((name, flow_val));
                }
            }
            k if k.contains("上游水库生态水是否入") => {
                let v = cell_f64(&params_range, r, 1);
                up_eco_as_inflow = v > 0.5;
            }
            k if k.contains("停止供水的用水户") => {
                for c in 1..params_range.width() as usize {
                    let s = cell_str(&params_range, r, c);
                    if !s.is_empty() {
                        user_stop_supply.push(s);
                    }
                }
            }
            _ => {}
        }
    }

    if up_name.is_empty() || down_name.is_empty() {
        return Err("未找到上库/下库名称".to_string());
    }

    // 2. Read reservoir info sheets
    let upstream = read_reservoir_sheets(&mut wb, &sheet_names, "上游", &up_name)?;
    let downstream = read_reservoir_sheets(&mut wb, &sheet_names, "下游", &down_name)?;

    let params = CalcParams {
        time_step: TimeStep::Daily,
        up_name: up_name.clone(),
        down_name: down_name.clone(),
        hydro_year_start: 4,
        hydro_year_end: 3,
        epsilon_v: 100.0,
        epsilon_w: 10.0,
        max_iterations: 2,
        up_v_special,
        down_v_special,
        need_add_users,
        user_special,
        user_stop_supply,
        up_eco_as_inflow,
    };

    Ok(ReservoirInput {
        params,
        upstream,
        downstream,
    })
}

fn read_reservoir_sheets(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_names: &[String],
    prefix: &str,
    res_name: &str,
) -> Result<Reservoir, String> {
    // Read 水库信息
    let info_sheet = format!("{}_{}", prefix, "水库信息");
    let info_range = wb
        .worksheet_range(&info_sheet)
        .map_err(|e| format!("读取'{}'失败: {}", info_sheet, e))?;

    // Parse reservoir info (key-value pairs in columns 0-1)
    let mut info = std::collections::HashMap::new();
    for r in 0..info_range.height() as usize {
        let key = cell_str(&info_range, r, 0);
        if !key.is_empty() {
            info.insert(key, r);
        }
    }

    let get_val = |key: &str| -> f64 {
        info.get(key)
            .map(|&r| cell_f64(&info_range, r, 1))
            .unwrap_or(0.0)
    };
    let get_str = |key: &str| -> String {
        info.get(key)
            .map(|&r| cell_str(&info_range, r, 1))
            .unwrap_or_default()
    };

    let h_dead = get_val("死水位");
    let h_normal = get_val("正常蓄水位");
    let h_wet = get_val("梅汛限制水位");
    let h_typhoon = get_val("台汛限制水位");
    let k0 = get_val("综合出力系数");
    let k1 = get_val("水头损失系数");
    let k2 = get_val("出力不均匀系数");
    let qm = get_val("最大设计流量");
    let wpv = get_val("装机容量");
    let dh_max = get_val("最大水头损失");
    let dh_min = get_val("最小水头损失");
    let loss_type = get_val("损失计算方法") as i32;
    let loss_val = get_val("损失值");
    let h_dead_power = {
        let v = get_val("发电死水位");
        if v > 0.0 { v } else { h_dead }
    };
    let h_limited_power = {
        let v = get_val("发电限制水位");
        if v > 0.0 { v } else { h_dead }
    };

    let wet_start = get_str("梅汛期开始");
    let wet_end = get_str("梅汛期结束");
    let typhoon_start = get_str("台汛期开始");
    let typhoon_end = get_str("台汛期结束");

    let const_z_down_flag = get_val("尾水位是否恒定") > 0.5;
    let cal_mode = get_str("水库类型");
    let cal_mode = if cal_mode.is_empty() { "年调节".to_string() } else { cal_mode };

    // Read Z-V curve
    let zv_sheet = format!("{}_{}", prefix, "水位库容");
    let zv_curve = if sheet_names.contains(&zv_sheet) {
        let range = wb.worksheet_range(&zv_sheet).map_err(|e| format!("{}: {}", zv_sheet, e))?;
        let mut curve = Vec::new();
        for r in 1..range.height() as usize {
            let wl = cell_f64(&range, r, 0);
            let vol = cell_f64(&range, r, 1);
            if wl > 0.0 || vol > 0.0 {
                curve.push(ZvPoint { water_level: wl, volume: vol });
            }
        }
        curve
    } else {
        vec![]
    };

    // Derive characteristic volumes
    let zv_levels: Vec<f64> = zv_curve.iter().map(|p| p.water_level).collect();
    let zv_vols: Vec<f64> = zv_curve.iter().map(|p| p.volume).collect();
    let v_dead = crate::reservoir::interp::interp(h_dead, &zv_levels, &zv_vols);
    let v_normal = crate::reservoir::interp::interp(h_normal, &zv_levels, &zv_vols);
    let v_wet = crate::reservoir::interp::interp(h_wet, &zv_levels, &zv_vols);
    let v_typhoon = crate::reservoir::interp::interp(h_typhoon, &zv_levels, &zv_vols);
    let v_dead_power = crate::reservoir::interp::interp(h_dead_power, &zv_levels, &zv_vols);
    let v_limited_power = crate::reservoir::interp::interp(h_limited_power, &zv_levels, &zv_vols);

    // Read Q-Z curve (tailwater)
    let qz_sheet = format!("{}_{}", prefix, "水位流量");
    let qz_curve = if !const_z_down_flag && sheet_names.contains(&qz_sheet) {
        let range = wb.worksheet_range(&qz_sheet).map_err(|e| format!("{}: {}", qz_sheet, e))?;
        let mut curve = Vec::new();
        for r in 1..range.height() as usize {
            let q = cell_f64(&range, r, 0);
            let wl = cell_f64(&range, r, 1);
            if q > 0.0 || wl > 0.0 {
                curve.push(QzPoint { q_down: q, water_level: wl });
            }
        }
        curve
    } else {
        vec![]
    };

    // Read tailwater lookup if const
    let z_down_lookup = if const_z_down_flag {
        let zd_sheet = format!("{}_{}", prefix, "下游水位");
        if sheet_names.contains(&zd_sheet) {
            let range = wb.worksheet_range(&zd_sheet).map_err(|e| format!("{}: {}", zd_sheet, e))?;
            let mut points = Vec::new();
            for r in 1..range.height() as usize {
                let mmdd = cell_str(&range, r, 0);
                let val = cell_f64(&range, r, 1);
                if !mmdd.is_empty() {
                    points.push((mmdd, val));
                }
            }
            Some(create_daily_lookup(&points, true))
        } else {
            None
        }
    } else {
        None
    };

    // Read inflow series
    let inflow_sheet = format!("{}_{}", prefix, "来水系列");
    let (dates, q_inflow, q_eco) = if sheet_names.contains(&inflow_sheet) {
        let range = wb.worksheet_range(&inflow_sheet).map_err(|e| format!("{}: {}", inflow_sheet, e))?;
        let mut dates = Vec::new();
        let mut inflow = Vec::new();
        let mut eco = Vec::new();
        for r in 1..range.height() as usize {
            let date_str = cell_str(&range, r, 0);
            if date_str.is_empty() { continue; }
            // Normalize date format
            let normalized = normalize_date(&date_str);
            dates.push(normalized);
            inflow.push(cell_f64(&range, r, 1));
            eco.push(cell_f64(&range, r, 2));
        }
        (dates, inflow, eco)
    } else {
        (vec![], vec![], vec![])
    };

    // Read demand series
    let (user_demand_reservoir, user_names_reservoir) = read_demand_sheet(wb, sheet_names, prefix, "库内需水")?;
    let (user_demand_downstream, user_names_downstream) = read_demand_sheet(wb, sheet_names, prefix, "坝下需水")?;

    // Read supply order from 计算参数 or default to all users
    let mut supply_order = Vec::new();
    supply_order.extend(user_names_reservoir.clone());
    supply_order.extend(user_names_downstream.clone());

    // Build flood control daily lookup
    let wet_start_ref = if wet_start.is_empty() { "06-01".to_string() } else { wet_start.clone() };
    let wet_end_ref = if wet_end.is_empty() { "07-15".to_string() } else { wet_end.clone() };
    let typhoon_start_ref = if typhoon_start.is_empty() { "07-16".to_string() } else { typhoon_start.clone() };
    let typhoon_end_ref = if typhoon_end.is_empty() { "10-15".to_string() } else { typhoon_end.clone() };

    // Build flood volume lookup from limit sheet or default
    let v_flood_day = read_limit_sheet_or_default(wb, sheet_names, prefix, "限制线", v_wet, v_typhoon, v_normal,
        &wet_start_ref, &typhoon_start_ref, &typhoon_end_ref)?;

    // Default dead/limited power day lookups (constant)
    let v_dead_power_day = create_daily_lookup(
        &[("01-01".to_string(), v_dead_power), ("12-31".to_string(), v_dead_power)],
        false,
    );
    let v_limited_power_day = create_daily_lookup(
        &[("01-01".to_string(), v_limited_power), ("12-31".to_string(), v_limited_power)],
        false,
    );
    let v_limited_supply_day = create_daily_lookup(
        &[("01-01".to_string(), v_dead), ("12-31".to_string(), v_dead)],
        false,
    );

    // Read dispatch lines
    let dispatch_line = read_dispatch_sheet(wb, sheet_names, prefix)?;

    // Loss
    let (loss_ratio, loss_value) = if loss_type == 0 {
        (loss_val / 1000.0, 0.0)
    } else {
        (0.0, loss_val)
    };

    Ok(Reservoir {
        name: res_name.to_string(),
        h_dead,
        h_normal,
        h_wet_season_limit: h_wet,
        h_typhoon_limit: h_typhoon,
        v_dead,
        v_normal,
        v_wet_season: v_wet,
        v_typhoon,
        zv_curve,
        zq_curve: qz_curve,
        k0,
        k1,
        k2,
        qm,
        wpv,
        dh_max,
        dh_min,
        loss_type,
        loss_ratio,
        loss_value,
        const_z_down: const_z_down_flag,
        z_down_lookup,
        wet_season_start: wet_start_ref,
        wet_season_end: wet_end_ref,
        typhoon_start: typhoon_start_ref,
        typhoon_end: typhoon_end_ref,
        h_dead_power,
        h_limited_power,
        v_dead_power,
        v_limited_power,
        v_flood_day,
        v_dead_power_day,
        v_limited_power_day,
        v_limited_supply_day,
        dispatch_line,
        dates,
        q_inflow,
        q_upstream: vec![],
        q_eco,
        supply_from_reservoir: !user_demand_reservoir.is_empty(),
        user_demand_reservoir,
        supply_from_downstream: !user_demand_downstream.is_empty(),
        user_demand_downstream,
        supply_order,
        cal_mode,
    })
}

fn read_demand_sheet(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_names: &[String],
    prefix: &str,
    suffix: &str,
) -> Result<(Vec<UserDemand>, Vec<String>), String> {
    let sheet_name = format!("{}_{}", prefix, suffix);
    if !sheet_names.contains(&sheet_name) {
        return Ok((vec![], vec![]));
    }
    let range = wb.worksheet_range(&sheet_name).map_err(|e| format!("{}: {}", sheet_name, e))?;
    let mut demands = Vec::new();
    let mut names = Vec::new();

    // First row is header: date, user1, user2, ...
    let width = range.width() as usize;
    for c in 1..width {
        let name = cell_str(&range, 0, c);
        if name.is_empty() { continue; }
        names.push(name.clone());
        let mut values = Vec::new();
        for r in 1..range.height() as usize {
            values.push(cell_f64(&range, r, c));
        }
        let from_downstream = suffix.contains("坝下");
        demands.push(UserDemand {
            name,
            from_downstream,
            values,
        });
    }

    Ok((demands, names))
}

#[allow(clippy::too_many_arguments)]
fn read_limit_sheet_or_default(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_names: &[String],
    prefix: &str,
    suffix: &str,
    v_wet: f64,
    v_typhoon: f64,
    v_normal: f64,
    wet_start: &str,
    typhoon_start: &str,
    typhoon_end: &str,
) -> Result<DailyLookup, String> {
    let sheet_name = format!("{}_{}", prefix, suffix);
    if sheet_names.contains(&sheet_name) {
        let range = wb.worksheet_range(&sheet_name).map_err(|e| format!("{}: {}", sheet_name, e))?;
        let mut points = Vec::new();
        for r in 1..range.height() as usize {
            let mmdd = cell_str(&range, r, 0);
            let vol = cell_f64(&range, r, 1);
            if !mmdd.is_empty() && vol > 0.0 {
                points.push((mmdd, vol));
            }
        }
        if !points.is_empty() {
            return Ok(create_daily_lookup(&points, false));
        }
    }

    // Default flood control lookup
    // wet_start -> v_wet, typhoon_start -> v_typhoon, typhoon_end+1 -> v_normal
    let typhoon_end_plus1 = increment_mmdd(typhoon_end);
    let points = vec![
        (wet_start.to_string(), v_wet),
        (typhoon_start.to_string(), v_typhoon),
        (typhoon_end_plus1, v_normal),
    ];
    Ok(create_daily_lookup(&points, false))
}

fn read_dispatch_sheet(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    sheet_names: &[String],
    prefix: &str,
) -> Result<DispatchLine, String> {
    let sheet_name = format!("{}_{}", prefix, "调度线");
    if !sheet_names.contains(&sheet_name) {
        return Ok(DispatchLine { monthly_points: vec![] });
    }

    let range = wb.worksheet_range(&sheet_name).map_err(|e| format!("{}: {}", sheet_name, e))?;

    // Format: each row is a month, columns are alternating V,P pairs
    // Row 0: header
    // Row 1+: MM-DD, V1, P1, V2, P2, ...
    let mut monthly_points = Vec::new();
    for r in 1..range.height() as usize {
        let mmdd = cell_str(&range, r, 0);
        if mmdd.is_empty() { continue; }

        let width = range.width() as usize;
        let mut volumes = Vec::new();
        let mut powers = Vec::new();

        let mut c = 1;
        while c + 1 < width {
            let v = cell_f64(&range, r, c);
            let p = cell_f64(&range, r, c + 1);
            if v > 0.0 || p > 0.0 {
                volumes.push(v);
                powers.push(p);
            }
            c += 2;
        }

        monthly_points.push(DispatchMonthEntry {
            mmdd,
            volumes,
            powers,
        });
    }

    Ok(DispatchLine { monthly_points })
}

fn normalize_date(s: &str) -> String {
    // Try to parse various date formats
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.format("%Y-%m-%d").to_string();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return d.format("%Y-%m-%d").to_string();
    }
    // Excel serial number (days since 1900-01-01, with the 1900 bug)
    if let Ok(serial) = s.trim().parse::<f64>() {
        let days = serial as i64 - 2; // Excel 1900 bug correction
        if let Some(d) = chrono::NaiveDate::from_ymd_opt(1900, 1, 1)
            .and_then(|base| base.checked_add_signed(chrono::Duration::days(days)))
        {
            return d.format("%Y-%m-%d").to_string();
        }
    }
    s.to_string()
}

fn increment_mmdd(mmdd: &str) -> String {
    let parts: Vec<&str> = mmdd.split('-').collect();
    if parts.len() != 2 { return mmdd.to_string(); }
    let m: u32 = parts[0].parse().unwrap_or(1);
    let d: u32 = parts[1].parse().unwrap_or(1);

    if let Some(date) = chrono::NaiveDate::from_ymd_opt(2024, m, d) {
        let next = date + chrono::Duration::days(1);
        return next.format("%m-%d").to_string();
    }
    mmdd.to_string()
}

pub fn write_results(path: String, output: ScheduleOutput) -> Result<(), String> {
    use rust_xlsxwriter::{Format, Workbook};

    let mut workbook = Workbook::new();
    let header_fmt = Format::new().set_bold();

    // Write 5 sheets per reservoir
    for (prefix, res_output) in [("上游", &output.upstream), ("下游", &output.downstream)] {
        // Sheet 1: 逐日过程
        write_daily_sheet(&mut workbook, &format!("{}_{}", prefix, "逐日过程"), &res_output.daily, &header_fmt)?;

        // Sheet 2: 逐月过程
        write_agg_sheet(&mut workbook, &format!("{}_{}", prefix, "逐月过程"), &res_output.monthly, &header_fmt)?;

        // Sheet 3: 逐年过程
        write_yearly_sheet(&mut workbook, &format!("{}_{}", prefix, "逐年过程"), &res_output.yearly, &header_fmt)?;

        // Sheet 4: 水文年过程
        write_yearly_sheet(&mut workbook, &format!("{}_{}", prefix, "水文年过程"), &res_output.hydro_yearly, &header_fmt)?;

        // Sheet 5: 汇总
        write_summary_sheet(&mut workbook, &format!("{}_{}", prefix, "汇总"), &res_output.summary, &header_fmt)?;
    }

    workbook
        .save(&path)
        .map_err(|e| format!("保存文件失败: {}", e))?;

    Ok(())
}

fn write_daily_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    daily: &[DailyResult],
    header_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    let headers = [
        "日期", "来水流量", "发电流量", "弃水流量", "损失流量",
        "末库容", "末水位", "Z1", "Z2", "DH", "水头", "出力(kW)",
        "天数", "生态供水", "生态缺口",
        "补水后-发电流量", "补水后-弃水流量", "补水后-末库容", "补水后-末水位",
        "补水后-Z1", "补水后-Z2", "补水后-DH", "补水后-水头", "补水后-出力(kW)",
        "补水1", "补水2", "补水3",
    ];
    write_header_row(ws, 0, &headers, header_fmt).map_err(|e| e.to_string())?;

    for (r, d) in daily.iter().enumerate() {
        let row = r as u32 + 1;
        let _ = ws.write_string(row, 0, &d.date);
        let _ = ws.write_number(row, 1, d.q_in);
        let _ = ws.write_number(row, 2, d.q_gen);
        let _ = ws.write_number(row, 3, d.q_thrown);
        let _ = ws.write_number(row, 4, d.q_loss);
        let _ = ws.write_number(row, 5, d.v_end);
        let _ = ws.write_number(row, 6, d.z_end);
        let _ = ws.write_number(row, 7, d.z_up);
        let _ = ws.write_number(row, 8, d.z_down);
        let _ = ws.write_number(row, 9, d.dh);
        let _ = ws.write_number(row, 10, d.h_net);
        let _ = ws.write_number(row, 11, d.power);
        let _ = ws.write_number(row, 12, d.days);
        let _ = ws.write_number(row, 13, d.eco_supply);
        let _ = ws.write_number(row, 14, d.eco_lack);
        let _ = ws.write_number(row, 15, d.q_gen_after);
        let _ = ws.write_number(row, 16, d.q_thrown_after);
        let _ = ws.write_number(row, 17, d.v_end_after);
        let _ = ws.write_number(row, 18, d.z_end_after);
        let _ = ws.write_number(row, 19, d.z_up_after);
        let _ = ws.write_number(row, 20, d.z_down_after);
        let _ = ws.write_number(row, 21, d.dh_after);
        let _ = ws.write_number(row, 22, d.h_net_after);
        let _ = ws.write_number(row, 23, d.power_after);
        let _ = ws.write_number(row, 24, d.supplement_q1);
        let _ = ws.write_number(row, 25, d.supplement_q2);
        let _ = ws.write_number(row, 26, d.supplement_q3);
    }

    Ok(())
}

fn write_agg_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    monthly: &[MonthlyResult],
    header_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    if monthly.is_empty() { return Ok(()); }

    // Build headers from first row
    let mut headers = vec!["年-月"];
    let col_names: Vec<&str> = monthly[0].values.iter().map(|(k, _)| k.as_str()).collect();
    headers.extend(col_names.iter());
    write_header_row(ws, 0, &headers, header_fmt).map_err(|e| e.to_string())?;

    for (r, row) in monthly.iter().enumerate() {
        let row_idx = r as u32 + 1;
        let _ = ws.write_string(row_idx, 0, &row.year_month);
        for (c, (_, v)) in row.values.iter().enumerate() {
            let _ = ws.write_number(row_idx, (c + 1) as u16, *v);
        }
    }

    Ok(())
}

fn write_yearly_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    yearly: &[YearlyResult],
    header_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    if yearly.is_empty() { return Ok(()); }

    let mut headers = vec!["年"];
    let col_names: Vec<&str> = yearly[0].values.iter().map(|(k, _)| k.as_str()).collect();
    headers.extend(col_names.iter());
    write_header_row(ws, 0, &headers, header_fmt).map_err(|e| e.to_string())?;

    for (r, row) in yearly.iter().enumerate() {
        let row_idx = r as u32 + 1;
        let _ = ws.write_string(row_idx, 0, &row.year);
        for (c, (_, v)) in row.values.iter().enumerate() {
            let _ = ws.write_number(row_idx, (c + 1) as u16, *v);
        }
    }

    Ok(())
}

fn write_summary_sheet(
    wb: &mut rust_xlsxwriter::Workbook,
    name: &str,
    summary: &[(String, f64)],
    header_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    let ws = wb.add_worksheet();
    ws.set_name(name).map_err(|e| e.to_string())?;

    write_header_row(ws, 0, &["指标", "值"], header_fmt).map_err(|e| e.to_string())?;

    for (r, (key, val)) in summary.iter().enumerate() {
        let row = r as u32 + 1;
        let _ = ws.write_string(row, 0, key);
        let _ = ws.write_number(row, 1, *val);
    }

    Ok(())
}
