use chrono::{Datelike, NaiveDate, TimeDelta};

use super::types::*;

/// 计算单日单批次水田水量平衡
pub fn calculate_paddy_day(
    h_start: f64,
    rainfall: f64,
    evaporation: f64,
    eva_ratio: f64,
    h_min: f64,
    storage: f64,
    h_max: f64,
    max_leak_h: f64,
) -> PaddyDayResult {
    // 初始水平衡
    let actual_evaporation = evaporation * eva_ratio;
    let mut begin_h = h_start + rainfall - actual_evaporation;

    // 渗漏
    let leakage;
    if begin_h < 0.0 {
        leakage = 0.0;
    } else if begin_h < max_leak_h {
        leakage = begin_h;
        begin_h = 0.0;
    } else {
        leakage = max_leak_h;
        begin_h -= max_leak_h;
    }

    // 灌溉/排水决策
    let (irrigation, drainage, end_h);
    if begin_h > h_max {
        irrigation = 0.0;
        drainage = begin_h - h_max;
        end_h = h_max;
    } else if begin_h < h_min {
        irrigation = storage - begin_h;
        drainage = 0.0;
        end_h = storage;
    } else {
        irrigation = 0.0;
        drainage = 0.0;
        end_h = begin_h;
    }

    PaddyDayResult {
        end_h,
        irrigation,
        drainage,
        leakage,
        actual_evaporation,
    }
}

/// 从 growth stages 展开为每日参数 (全年 366 天)
/// year: 用于确定年份
pub fn expand_growth_stages(stages: &[GrowthStage], year: i32) -> Vec<(NaiveDate, DailyParams)> {
    let mut result = Vec::new();
    let start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();

    let mut current = start;
    for stage in stages {
        let days = stage.days as i64;
        for _ in 0..days {
            result.push((
                current,
                DailyParams {
                    eva_ratio: stage.eva_ratio,
                    h_min: stage.h_min,
                    storage: stage.storage,
                    h_max: stage.h_max,
                },
            ));
            current += TimeDelta::days(1);
        }
    }

    result
}

/// 处理闰年 2/29 -> 2/28
pub fn handle_leap_year(date: NaiveDate, target_year: i32) -> NaiveDate {
    let month = date.month();
    let day = date.day();
    if month == 2 && day == 29 {
        NaiveDate::from_ymd_opt(target_year, 2, 28).unwrap()
    } else {
        NaiveDate::from_ymd_opt(target_year, month, day)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(target_year, month, 28).unwrap())
    }
}

/// 获取某日的灌溉参数
pub fn get_daily_params(
    params_map: &[(NaiveDate, DailyParams)],
    date: NaiveDate,
    base_year: i32,
) -> DailyParams {
    let lookup = handle_leap_year(date, base_year);
    for (d, p) in params_map {
        if *d == lookup {
            return p.clone();
        }
    }
    // 默认参数 (非生长期)
    DailyParams {
        eva_ratio: 0.5,
        h_min: -45.0,
        storage: -25.0,
        h_max: 0.0,
    }
}

/// 计算低洼地水量平衡
pub fn calculate_lowland(
    state: &mut LowlandState,
    rainfall: f64,
    evaporation: f64,
    area_km2: f64,
    date: NaiveDate,
) -> (f64, f64) {
    // drainage, leakage (万m³)
    let mut begin_h = state.water_level + rainfall;

    // 蒸发 (7月前 0.65, 之后 0.8)
    let eva_ratio = if date.month() < 7 { 0.65 } else { 0.8 };
    let max_e = evaporation * eva_ratio;
    let actual_e;
    if begin_h > max_e {
        begin_h -= max_e;
        actual_e = max_e;
    } else {
        actual_e = begin_h;
        begin_h = 0.0;
    }
    let _ = actual_e;

    // 渗漏: 低洼地全渗
    let leak;
    if begin_h > 0.0 {
        leak = begin_h;
        begin_h = 0.0;
    } else {
        leak = 0.0;
    }
    let _ = leak;

    // 排水
    let drainage;
    if begin_h > 80.0 {
        drainage = (begin_h - 80.0) * area_km2 / 10.0;
        state.water_level = 80.0;
    } else {
        drainage = 0.0;
        state.water_level = begin_h;
    }

    (drainage, 0.0) // lowland drainage, no irrigation
}

/// 计算水面排水
pub fn calculate_water_surface(rainfall: f64, evaporation: f64, area_km2: f64) -> f64 {
    if rainfall > evaporation {
        (rainfall - evaporation) * area_km2 / 10.0
    } else {
        0.0
    }
}

/// 模拟一个区的水稻 (单季 or 双季) 水量平衡 —— 全日期范围
/// 返回每天的 (irrigation_万m3, drainage_万m3, flowering_irrigation_万m3)
pub fn simulate_paddy_zone(
    rice_area: f64,
    rotation_batches: usize,
    leakage_rate: f64,
    flowering_ratio: f64,
    is_single: bool,
    params_list: &[(NaiveDate, DailyParams)],
    base_year: i32,
    dates: &[NaiveDate],
    rainfall: &[f64],
    evaporation: &[f64],
) -> Vec<(f64, f64, f64)> {
    if rice_area <= 0.0 || rotation_batches == 0 {
        return vec![(0.0, 0.0, 0.0); dates.len()];
    }

    let batches = rotation_batches;
    let mut levels = vec![-25.0_f64; batches];
    let batch_area = rice_area / batches as f64 / 10.0; // 单批次面积 (用于结果换算)

    // 开花期配置
    let (single_fp, double_fp) = default_flowering_periods();
    let fp = if is_single { &single_fp } else { &double_fp };

    let mut results = Vec::with_capacity(dates.len());

    for (day_idx, &date) in dates.iter().enumerate() {
        let rain = rainfall[day_idx];
        let evap = evaporation[day_idx];

        let mut total_irr = 0.0;
        let mut total_drain = 0.0;

        for i in 0..batches {
            // 轮灌: 每个批次偏移 i 天取参数
            let offset_date = date + TimeDelta::days(i as i64);
            let params = get_daily_params(params_list, offset_date, base_year);

            let result = calculate_paddy_day(
                levels[i],
                rain,
                evap,
                params.eva_ratio,
                params.h_min,
                params.storage,
                params.h_max,
                leakage_rate,
            );

            levels[i] = result.end_h;
            total_irr += result.irrigation * batch_area;
            total_drain += result.drainage * batch_area;
        }

        // 开花期处理: 不在开花期时, 灌溉量转移到 flowering_irrigation
        let flowering_irr;
        if !is_in_flowering_period(date, fp) {
            flowering_irr = total_irr * flowering_ratio;
            total_irr = 0.0;
        } else {
            flowering_irr = 0.0;
        }

        results.push((total_irr, total_drain, flowering_irr));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_paddy_day_needs_irrigation() {
        let r = calculate_paddy_day(
            -25.0, // h_start
            0.0,   // rainfall
            5.0,   // evaporation
            1.0,   // eva_ratio
            0.0,   // h_min
            30.0,  // storage
            50.0,  // h_max
            2.0,   // max_leak
        );
        // begin_h = -25 + 0 - 5 = -30  (< 0 => leak=0)
        // begin_h=-30 < h_min=0 => irr = 30 - (-30) = 60, end_h = 30
        assert_eq!(r.leakage, 0.0);
        assert!((r.irrigation - 60.0).abs() < 1e-9);
        assert_eq!(r.drainage, 0.0);
        assert!((r.end_h - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_calculate_paddy_day_needs_drainage() {
        let r = calculate_paddy_day(
            40.0,  // h_start
            20.0,  // rainfall
            5.0,   // evaporation
            1.0,   // eva_ratio
            0.0,   // h_min
            30.0,  // storage
            50.0,  // h_max
            2.0,   // max_leak
        );
        // begin_h = 40 + 20 - 5 = 55
        // 55 >= max_leak=2 => leak=2, begin_h=53
        // 53 > h_max=50 => drain = 3, end_h = 50
        assert!((r.leakage - 2.0).abs() < 1e-9);
        assert!((r.drainage - 3.0).abs() < 1e-9);
        assert!((r.end_h - 50.0).abs() < 1e-9);
        assert_eq!(r.irrigation, 0.0);
    }

    #[test]
    fn test_calculate_paddy_day_normal() {
        let r = calculate_paddy_day(
            10.0, 5.0, 3.0, 1.0, 0.0, 30.0, 50.0, 2.0,
        );
        // begin_h = 10 + 5 - 3 = 12
        // 12 >= 2 => leak=2, begin_h=10
        // 0 <= 10 <= 50 => no irr, no drain, end_h=10
        assert!((r.leakage - 2.0).abs() < 1e-9);
        assert_eq!(r.irrigation, 0.0);
        assert_eq!(r.drainage, 0.0);
        assert!((r.end_h - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_leakage_partial() {
        let r = calculate_paddy_day(
            0.0, 1.5, 0.0, 1.0, 0.0, 30.0, 50.0, 2.0,
        );
        // begin_h = 0 + 1.5 - 0 = 1.5
        // 0 < 1.5 < 2.0 => leak=1.5, begin_h=0
        // 0 >= h_min=0 => normal
        assert!((r.leakage - 1.5).abs() < 1e-9);
        assert!((r.end_h - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_water_surface() {
        let d = calculate_water_surface(10.0, 3.0, 20.0);
        // (10-3) * 20 / 10 = 14
        assert!((d - 14.0).abs() < 1e-9);

        let d2 = calculate_water_surface(3.0, 10.0, 20.0);
        assert_eq!(d2, 0.0);
    }
}
