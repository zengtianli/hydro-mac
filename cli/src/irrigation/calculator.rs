use chrono::{Datelike, NaiveDate, TimeDelta};

use super::dryland::calculate_dryland_demand;
use super::paddy::*;
use super::types::*;

const WARMUP_DAYS: i64 = 30;

/// 主计算入口
pub fn run_calculation(input: &IrrigationInput, mode: CalcMode) -> IrrigationOutput {
    let mut warnings = Vec::new();

    // 解析起始日期
    let forecast_date = parse_date(&input.time_config.forecast_date);
    let forecast_days = input.time_config.forecast_days as i64;

    // 计算时间范围 (含预热期)
    let start_date = forecast_date - TimeDelta::days(WARMUP_DAYS);
    let total_days = (WARMUP_DAYS + forecast_days) as usize;

    let mut all_dates = Vec::with_capacity(total_days);
    for i in 0..total_days {
        all_dates.push(start_date + TimeDelta::days(i as i64));
    }

    // 只输出预报期的日期 (跳过预热期)
    let output_dates: Vec<NaiveDate> = all_dates[WARMUP_DAYS as usize..].to_vec();
    let date_strings: Vec<String> = output_dates
        .iter()
        .map(|d| d.format("%Y/%m/%d").to_string())
        .collect();

    let zone_names: Vec<String> = input.zones.iter().map(|z| z.name.clone()).collect();
    let base_year = forecast_date.year();

    // 展开 growth stages 为每日参数
    let single_params = expand_growth_stages(&input.single_crop_stages, base_year);
    let double_params = expand_growth_stages(&input.double_crop_stages, base_year);

    let mut daily_results = Vec::new();

    for zone in &input.zones {
        // 查找该区的气象数据
        let zone_weather = input
            .weather
            .iter()
            .find(|w| w.zone_name == zone.name);

        let (rainfall_all, evaporation_all) = match zone_weather {
            Some(w) => build_weather_lookup(w, &all_dates),
            None => {
                warnings.push(format!("区域 {} 缺少气象数据", zone.name));
                (vec![0.0; total_days], vec![0.0; total_days])
            }
        };

        // -- 水稻灌溉 --
        let paddy_results = if mode == CalcMode::Crop {
            vec![(0.0, 0.0, 0.0); total_days]
        } else {
            // 单季稻
            let single_res = simulate_paddy_zone(
                zone.single_rice_area,
                zone.rotation_batches,
                zone.leakage_rate,
                zone.flowering_ratio,
                true,
                &single_params,
                base_year,
                &all_dates,
                &rainfall_all,
                &evaporation_all,
            );
            // 双季稻
            let double_res = simulate_paddy_zone(
                zone.double_rice_area,
                zone.rotation_batches,
                zone.leakage_rate,
                zone.flowering_ratio,
                false,
                &double_params,
                base_year,
                &all_dates,
                &rainfall_all,
                &evaporation_all,
            );
            // 合并
            single_res
                .iter()
                .zip(double_res.iter())
                .map(|(s, d)| (s.0 + d.0, s.1 + d.1, s.2 + d.2))
                .collect::<Vec<_>>()
        };

        // 低洼地 & 水面
        let mut lowland_state = LowlandState::new();
        let mut lowland_drainages = vec![0.0; total_days];
        let mut water_surface_drainages = vec![0.0; total_days];

        if mode != CalcMode::Crop {
            for i in 0..total_days {
                if zone.misc_area > 0.0 {
                    let (d, _) = calculate_lowland(
                        &mut lowland_state,
                        rainfall_all[i],
                        evaporation_all[i],
                        zone.misc_area,
                        all_dates[i],
                    );
                    lowland_drainages[i] = d;
                }
                if zone.water_surface_area > 0.0 {
                    water_surface_drainages[i] = calculate_water_surface(
                        rainfall_all[i],
                        evaporation_all[i],
                        zone.water_surface_area,
                    );
                }
            }
        }

        // -- 旱地作物 --
        let dryland_demand = if mode == CalcMode::Irrigation {
            0.0
        } else {
            // 查找该区的旱地种植面积
            input
                .crop_areas
                .iter()
                .find(|ca| ca.zone_name == zone.name)
                .map(|ca| calculate_dryland_demand(&input.crops, ca))
                .unwrap_or(0.0)
        };

        // 只输出预报期的结果
        for (out_idx, &date) in output_dates.iter().enumerate() {
            let all_idx = out_idx + WARMUP_DAYS as usize;
            let (paddy_irr, paddy_drain, flowering_irr) = paddy_results[all_idx];

            daily_results.push(DailyZoneResult {
                date: date.format("%Y/%m/%d").to_string(),
                zone_name: zone.name.clone(),
                paddy_irrigation: paddy_irr + flowering_irr,
                paddy_drainage: paddy_drain
                    + lowland_drainages[all_idx]
                    + water_surface_drainages[all_idx],
                dryland_irrigation: dryland_demand,
                dryland_drainage: 0.0,
                flowering_irrigation: flowering_irr,
                lowland_drainage: lowland_drainages[all_idx],
                water_surface_drainage: water_surface_drainages[all_idx],
            });
        }
    }

    IrrigationOutput {
        daily_results,
        zone_names,
        dates: date_strings,
        warnings,
    }
}

/// 解析日期字符串 "YYYY/MM/DD"
fn parse_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y/%m/%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2025, 7, 15).unwrap())
}

/// 从 ZoneWeather 构建日期对齐的 rainfall / evaporation 向量
fn build_weather_lookup(
    weather: &ZoneWeather,
    dates: &[NaiveDate],
) -> (Vec<f64>, Vec<f64>) {
    use std::collections::HashMap;

    let mut rain_map: HashMap<NaiveDate, f64> = HashMap::new();
    let mut evap_map: HashMap<NaiveDate, f64> = HashMap::new();

    for r in &weather.records {
        if let Ok(d) = NaiveDate::parse_from_str(&r.date, "%Y/%m/%d") {
            rain_map.insert(d, r.rainfall);
            evap_map.insert(d, r.evaporation);
        }
    }

    let rainfall: Vec<f64> = dates.iter().map(|d| *rain_map.get(d).unwrap_or(&0.0)).collect();
    let evaporation: Vec<f64> = dates.iter().map(|d| *evap_map.get(d).unwrap_or(&0.0)).collect();

    (rainfall, evaporation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_input() -> IrrigationInput {
        IrrigationInput {
            time_config: TimeConfig {
                forecast_date: "2025/07/15".into(),
                forecast_days: 3,
            },
            zones: vec![IrrigationZone {
                name: "测试区".into(),
                single_rice_area: 10.0,
                double_rice_area: 5.0,
                dryland_area: 20.0,
                misc_area: 5.0,
                water_surface_area: 3.0,
                plain_area: 50.0,
                leakage_rate: 2.0,
                dryland_leakage: 2.0,
                flowering_ratio: 0.0,
                rotation_batches: 10,
            }],
            single_crop_stages: vec![GrowthStage {
                start: "01/01".into(),
                end: "12/31".into(),
                days: 366,
                eva_ratio: 1.0,
                h_min: 0.0,
                storage: 30.0,
                h_max: 50.0,
            }],
            double_crop_stages: vec![GrowthStage {
                start: "01/01".into(),
                end: "12/31".into(),
                days: 366,
                eva_ratio: 1.0,
                h_min: 0.0,
                storage: 30.0,
                h_max: 50.0,
            }],
            crops: vec![Crop {
                name: "小麦".into(),
                water_75: 0.25,
                water_90: 0.32,
            }],
            weather: vec![ZoneWeather {
                zone_name: "测试区".into(),
                records: (0..50)
                    .map(|i| {
                        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
                            + TimeDelta::days(i);
                        WeatherRecord {
                            date: date.format("%Y/%m/%d").to_string(),
                            rainfall: 3.0,
                            evaporation: 4.0,
                        }
                    })
                    .collect(),
            }],
            crop_areas: vec![CropAreaEntry {
                zone_name: "测试区".into(),
                hydro_year: 90,
                crop_areas: vec![("小麦".into(), 5.0)],
            }],
        }
    }

    #[test]
    fn test_run_calculation_both() {
        let input = make_test_input();
        let output = run_calculation(&input, CalcMode::Both);
        assert_eq!(output.dates.len(), 3);
        assert_eq!(output.zone_names.len(), 1);
        assert_eq!(output.daily_results.len(), 3); // 3 days * 1 zone
        // Dryland demand should be > 0
        assert!(output.daily_results[0].dryland_irrigation > 0.0);
    }

    #[test]
    fn test_run_calculation_crop_only() {
        let input = make_test_input();
        let output = run_calculation(&input, CalcMode::Crop);
        // Paddy should be 0
        for r in &output.daily_results {
            assert_eq!(r.paddy_irrigation, 0.0);
            assert_eq!(r.paddy_drainage, 0.0);
        }
        assert!(output.daily_results[0].dryland_irrigation > 0.0);
    }

    #[test]
    fn test_run_calculation_irrigation_only() {
        let input = make_test_input();
        let output = run_calculation(&input, CalcMode::Irrigation);
        for r in &output.daily_results {
            assert_eq!(r.dryland_irrigation, 0.0);
        }
    }
}
