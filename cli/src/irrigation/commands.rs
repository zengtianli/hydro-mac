//! irrigation CLI 编排 —— 复刻自原 apps/irrigation/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 纯函数,返回 serde 可序列化结果;main.rs 负责 JSON 解码/编码与 stdout。
use crate::irrigation::calculator::run_calculation;
use crate::irrigation::sample_data::{self, SampleData};
use crate::irrigation::types::*;

pub fn get_sample_data() -> SampleData {
    sample_data::get_sample_data()
}

/// 从数据目录读 8 个标准命名输入文件(同原 Tauri 前端经 fs 插件从盘上读的文件集)。
pub fn load_data_dir(path: String) -> Result<SampleData, String> {
    let read = |name: &str| -> Result<String, String> {
        let p = std::path::Path::new(&path).join(name);
        std::fs::read_to_string(&p).map_err(|e| format!("读取 {} 失败: {}", p.display(), e))
    };
    Ok(SampleData {
        time_config: read("in_TIME.txt")?,
        zones: read("static_fenqu.txt")?,
        single_crop_stages: read("static_single_crop.txt")?,
        double_crop_stages: read("static_double_crop.txt")?,
        crops: read("static_crops.txt")?,
        rainfall: read("in_JYGC.txt")?,
        evaporation: read("in_ZFGC.txt")?,
        crop_areas: read("in_dry_crop_area.txt")?,
    })
}

/// 解析所有输入文件内容并运行灌溉计算(原 irrigation_run_irrigation,8 个内容参数收进 SampleData)。
pub fn run_irrigation(contents: SampleData, mode: String) -> Result<IrrigationOutput, String> {
    // 解析所有输入
    let time_config = parse_time_config(&contents.time_config)?;
    let zones = parse_zones(&contents.zones)?;
    let single_crop_stages = parse_growth_stages(&contents.single_crop_stages)?;
    let double_crop_stages = parse_growth_stages(&contents.double_crop_stages)?;
    let crops = parse_crops(&contents.crops)?;
    let crop_areas = parse_crop_areas(&contents.crop_areas)?;

    // 解析气象数据
    let zone_names: Vec<String> = zones.iter().map(|z| z.name.clone()).collect();
    let rainfall_data = parse_weather_tsv(&contents.rainfall, &zone_names);
    let evaporation_data = parse_weather_tsv(&contents.evaporation, &zone_names);
    let weather = merge_weather(&rainfall_data, &evaporation_data);

    // 解析计算模式
    let calc_mode = match mode.as_str() {
        "crop" => CalcMode::Crop,
        "irrigation" => CalcMode::Irrigation,
        _ => CalcMode::Both,
    };

    let input = IrrigationInput {
        time_config,
        zones,
        single_crop_stages,
        double_crop_stages,
        crops,
        weather,
        crop_areas,
    };

    Ok(run_calculation(&input, calc_mode))
}

/// 格式化灌溉量输出为 TSV(原 irrigation_format_irrigation_tsv)。
pub fn format_irrigation_tsv(output: IrrigationOutput) -> Result<String, String> {
    Ok(format_output_tsv(&output, |r| {
        r.paddy_irrigation + r.dryland_irrigation
    }))
}

/// 格式化排水量输出为 TSV(原 irrigation_format_drainage_tsv)。
pub fn format_drainage_tsv(output: IrrigationOutput) -> Result<String, String> {
    Ok(format_output_tsv(&output, |r| {
        r.paddy_drainage + r.dryland_drainage
    }))
}

// ============================================================================
// 以下 = 原 commands/io.rs 输入解析/输出格式化(逐字复刻,纯函数无 tauri 概念)
// ============================================================================

/// 解析 in_TIME.txt (TSV)
/// 格式:
/// ForcastDate\t2025/07/15
/// ForcastDays\t16
pub fn parse_time_config(content: &str) -> Result<TimeConfig, String> {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return Err("时间配置文件至少需要2行".into());
    }

    let date_parts: Vec<&str> = lines[0].split('\t').collect();
    let days_parts: Vec<&str> = lines[1].split('\t').collect();

    if date_parts.len() < 2 || days_parts.len() < 2 {
        return Err("时间配置文件格式错误".into());
    }

    Ok(TimeConfig {
        forecast_date: date_parts[1].trim().to_string(),
        forecast_days: days_parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("解析预报天数失败: {}", e))?,
    })
}

/// 解析 static_fenqu.txt (TSV)
/// 表头: 分区名称\t单季稻面积\t双季稻面积\t旱地面积\t杂地面积\t水面面积\t平原面积\t水田渗漏\t旱地渗漏\t春花种植比例\t农田轮灌批次
pub fn parse_zones(content: &str) -> Result<Vec<IrrigationZone>, String> {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return Err("分区配置文件至少需要2行".into());
    }

    let mut zones = Vec::new();
    for line in &lines[1..] {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 11 {
            continue;
        }

        let parse_f64 = |s: &str| -> f64 { s.trim().parse().unwrap_or(0.0) };
        let parse_usize = |s: &str| -> usize { s.trim().parse().unwrap_or(1) };

        zones.push(IrrigationZone {
            name: fields[0].trim().to_string(),
            single_rice_area: parse_f64(fields[1]),
            double_rice_area: parse_f64(fields[2]),
            dryland_area: parse_f64(fields[3]),
            misc_area: parse_f64(fields[4]),
            water_surface_area: parse_f64(fields[5]),
            plain_area: parse_f64(fields[6]),
            leakage_rate: parse_f64(fields[7]),
            dryland_leakage: parse_f64(fields[8]),
            flowering_ratio: parse_f64(fields[9]),
            rotation_batches: parse_usize(fields[10]),
        });
    }

    Ok(zones)
}

/// 解析 static_single_crop.txt / static_double_crop.txt
/// 跳过注释和表头, 格式:
/// 开始日期  结束日期  生长天数  蒸发系数  水位下限  设计蓄水位  水位上限
pub fn parse_growth_stages(content: &str) -> Result<Vec<GrowthStage>, String> {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut stages = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        // 跳过注释, 表头, 分隔线
        if trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.contains("开始日期")
            || trimmed.contains("生长天数")
        {
            continue;
        }

        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        if fields.len() < 7 {
            continue;
        }

        let parse_f64 = |s: &str| -> f64 { s.trim().parse().unwrap_or(0.0) };

        stages.push(GrowthStage {
            start: fields[0].to_string(),
            end: fields[1].to_string(),
            days: parse_f64(fields[2]) as u32,
            eva_ratio: parse_f64(fields[3]),
            h_min: parse_f64(fields[4]),
            storage: parse_f64(fields[5]),
            h_max: parse_f64(fields[6]),
        });
    }

    if stages.is_empty() {
        return Err("未能解析任何生长阶段数据".into());
    }

    Ok(stages)
}

/// 解析 static_crops.txt (TSV)
/// 格式: 作物\t75% mm/d\t90% mm/d
pub fn parse_crops(content: &str) -> Result<Vec<Crop>, String> {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return Err("作物数据文件至少需要2行".into());
    }

    let mut crops = Vec::new();
    for line in &lines[1..] {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            continue;
        }

        let parse_f64 = |s: &str| -> f64 { s.trim().parse().unwrap_or(0.0) };

        crops.push(Crop {
            name: fields[0].trim().to_string(),
            water_75: parse_f64(fields[1]),
            water_90: parse_f64(fields[2]),
        });
    }

    Ok(crops)
}

/// 解析气象数据 (in_JYGC.txt / in_ZFGC.txt) (TSV)
/// 格式: TIME\t区1\t区2\t...
/// 2025/06/15\t0\t0\t...
pub fn parse_weather_tsv(content: &str, zone_names: &[String]) -> Vec<(String, Vec<(String, f64)>)> {
    // Returns: Vec<(zone_name, Vec<(date_str, value)>)>
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return Vec::new();
    }

    let headers: Vec<&str> = lines[0].split('\t').collect();

    // zone name -> column index
    let mut zone_col: Vec<(String, usize)> = Vec::new();
    for name in zone_names {
        for (i, h) in headers.iter().enumerate().skip(1) {
            if h.trim() == name {
                zone_col.push((name.clone(), i));
                break;
            }
        }
    }

    let mut result: Vec<(String, Vec<(String, f64)>)> = zone_col
        .iter()
        .map(|(name, _)| (name.clone(), Vec::new()))
        .collect();

    for line in &lines[1..] {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.is_empty() {
            continue;
        }
        let date = fields[0].trim().to_string();

        for (idx, (_, col)) in zone_col.iter().enumerate() {
            let val = if *col < fields.len() {
                fields[*col].trim().parse::<f64>().unwrap_or(0.0)
            } else {
                0.0
            };
            result[idx].1.push((date.clone(), val));
        }
    }

    result
}

/// 合并 rainfall + evaporation 为 ZoneWeather
pub fn merge_weather(
    rainfall_data: &[(String, Vec<(String, f64)>)],
    evaporation_data: &[(String, Vec<(String, f64)>)],
) -> Vec<ZoneWeather> {
    let mut result = Vec::new();

    for (zone_name, rain_records) in rainfall_data {
        let evap_records = evaporation_data
            .iter()
            .find(|(name, _)| name == zone_name);

        let mut records = Vec::new();
        for (i, (date, rain_val)) in rain_records.iter().enumerate() {
            let evap_val = evap_records
                .and_then(|(_, evaps)| evaps.get(i))
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            records.push(WeatherRecord {
                date: date.clone(),
                rainfall: *rain_val,
                evaporation: evap_val,
            });
        }

        result.push(ZoneWeather {
            zone_name: zone_name.clone(),
            records,
        });
    }

    result
}

/// 解析 in_dry_crop_area.txt (TSV)
/// 格式: 平原名称\t灌溉保证率\t小麦\t玉米\t...
pub fn parse_crop_areas(content: &str) -> Result<Vec<CropAreaEntry>, String> {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return Err("旱地作物面积文件至少需要2行".into());
    }

    let headers: Vec<&str> = lines[0].split('\t').collect();
    let mut entries = Vec::new();

    for line in &lines[1..] {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            continue;
        }

        let zone_name = fields[0].trim().to_string();
        let hydro_year: u32 = fields[1].trim().parse().unwrap_or(90);

        let mut crop_areas = Vec::new();
        for (i, header) in headers.iter().enumerate().skip(2) {
            if i < fields.len() {
                let area: f64 = fields[i].trim().parse().unwrap_or(0.0);
                if area > 0.0 {
                    crop_areas.push((header.trim().to_string(), area));
                }
            }
        }

        entries.push(CropAreaEntry {
            zone_name,
            hydro_year,
            crop_areas,
        });
    }

    Ok(entries)
}

/// 生成输出 TSV (灌溉/排水表)
pub fn format_output_tsv(
    output: &IrrigationOutput,
    value_fn: impl Fn(&DailyZoneResult) -> f64,
) -> String {
    let mut buf = String::new();

    // 表头
    buf.push_str("日期");
    for name in &output.zone_names {
        buf.push('\t');
        buf.push_str(name);
    }
    buf.push('\n');

    // 数据行
    for date in &output.dates {
        buf.push_str(date);
        for zone_name in &output.zone_names {
            let val = output
                .daily_results
                .iter()
                .find(|r| r.date == *date && r.zone_name == *zone_name)
                .map(|r| value_fn(r))
                .unwrap_or(0.0);
            buf.push('\t');
            buf.push_str(&format!("{:.2}", val));
        }
        buf.push('\n');
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_config() {
        let content = "ForcastDate\t2025/07/15\nForcastDays\t16";
        let tc = parse_time_config(content).unwrap();
        assert_eq!(tc.forecast_date, "2025/07/15");
        assert_eq!(tc.forecast_days, 16);
    }

    #[test]
    fn test_parse_zones() {
        let content = "分区名称\t单季稻面积\t双季稻面积\t旱地面积\t杂地面积\t水面面积\t平原面积\t水田渗漏\t旱地渗漏\t春花种植比例\t农田轮灌批次\n\
安和平原区\t38.019\t27.414\t55.1\t24.2\t12.4\t118.8\t2\t2\t0\t10";
        let zones = parse_zones(content).unwrap();
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].name, "安和平原区");
        assert!((zones[0].single_rice_area - 38.019).abs() < 1e-3);
        assert_eq!(zones[0].rotation_batches, 10);
    }

    #[test]
    fn test_parse_growth_stages() {
        let content = "# 单季稻灌溉制度表\n\n\
开始日期        结束日期        生长天数  蒸发系数  水位下限(mm)  设计蓄水位(mm)  水位上限(mm)\n\
----------  ----------  ------  ----  --------  ---------  --------\n\
2025/01/01  2025/02/28  59      0.55  -45.0     -25.0      0.0";
        let stages = parse_growth_stages(content).unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].days, 59);
        assert!((stages[0].eva_ratio - 0.55).abs() < 1e-6);
    }

    #[test]
    fn test_parse_crops() {
        let content = "作物\t75% mm/d\t90% mm/d\n小麦\t0.2488\t0.3199";
        let crops = parse_crops(content).unwrap();
        assert_eq!(crops.len(), 1);
        assert_eq!(crops[0].name, "小麦");
    }

    #[test]
    fn test_run_irrigation_sample_roundtrip() {
        // 内嵌示例数据 → run:核心链路冒烟(16 天预报期 × N 区)
        let contents = get_sample_data();
        let out = run_irrigation(contents, "both".into()).unwrap();
        assert_eq!(out.dates.len(), 16);
        assert!(!out.zone_names.is_empty());
        assert_eq!(out.daily_results.len(), out.dates.len() * out.zone_names.len());
    }
}
