use super::types::*;
use std::collections::BTreeMap;

/// Aggregate daily results to monthly.
pub fn to_monthly(daily: &[DailyResult], supply_order: &[String]) -> Vec<MonthlyResult> {
    // Group by year-month
    let mut groups: BTreeMap<String, Vec<&DailyResult>> = BTreeMap::new();
    for d in daily {
        let ym = &d.date[..7]; // "YYYY-MM"
        groups.entry(ym.to_string()).or_default().push(d);
    }

    groups
        .into_iter()
        .map(|(ym, rows)| {
            let n = rows.len() as f64;
            let mut values = Vec::new();

            // Averages
            values.push(("来水流量".to_string(), rows.iter().map(|r| r.q_in).sum::<f64>() / n));
            values.push(("补水后-发电流量".to_string(), rows.iter().map(|r| r.q_gen_after).sum::<f64>() / n));
            values.push(("补水后-弃水流量".to_string(), rows.iter().map(|r| r.q_thrown_after).sum::<f64>() / n));
            values.push(("损失流量".to_string(), rows.iter().map(|r| r.q_loss).sum::<f64>() / n));
            values.push(("补水后-出力".to_string(), rows.iter().map(|r| r.power_after).sum::<f64>() / n));

            // Last value of month
            if let Some(last) = rows.last() {
                values.push(("补水后-末库容".to_string(), last.v_end_after));
                values.push(("补水后-末水位".to_string(), last.z_end_after));
            }

            // Sum of days
            values.push(("天数".to_string(), rows.iter().map(|r| r.days).sum()));

            // Supply stats per user
            for user_name in supply_order {
                let avg_demand: f64 = rows
                    .iter()
                    .map(|r| {
                        r.supply_detail_after
                            .iter()
                            .find(|s| s.name == *user_name)
                            .map(|s| s.demand)
                            .unwrap_or(0.0)
                    })
                    .sum::<f64>()
                    / n;
                let avg_supply: f64 = rows
                    .iter()
                    .map(|r| {
                        r.supply_detail_after
                            .iter()
                            .find(|s| s.name == *user_name)
                            .map(|s| s.supply)
                            .unwrap_or(0.0)
                    })
                    .sum::<f64>()
                    / n;
                let lack_days: f64 = rows
                    .iter()
                    .map(|r| {
                        r.supply_detail_after
                            .iter()
                            .find(|s| s.name == *user_name)
                            .map(|s| s.lack_day)
                            .unwrap_or(0.0)
                    })
                    .sum();

                values.push((format!("{}_需水", user_name), avg_demand));
                values.push((format!("{}_供水", user_name), avg_supply));
                values.push((format!("{}_缺水天数", user_name), lack_days));
            }

            // Eco stats
            values.push(("生态供水".to_string(), rows.iter().map(|r| r.eco_supply).sum::<f64>() / n));
            values.push(("生态缺水天数".to_string(), rows.iter().map(|r| r.eco_lack_day).sum()));

            MonthlyResult {
                year_month: ym,
                values,
            }
        })
        .collect()
}

/// Aggregate daily results to calendar yearly.
pub fn to_yearly(daily: &[DailyResult], supply_order: &[String]) -> Vec<YearlyResult> {
    let mut groups: BTreeMap<String, Vec<&DailyResult>> = BTreeMap::new();
    for d in daily {
        let year = &d.date[..4];
        groups.entry(year.to_string()).or_default().push(d);
    }

    groups
        .into_iter()
        .map(|(year, rows)| {
            let values = aggregate_yearly_values(&rows, supply_order);
            YearlyResult { year, values }
        })
        .collect()
}

/// Aggregate daily results to hydrological year.
pub fn to_hydro_yearly(
    daily: &[DailyResult],
    supply_order: &[String],
    start_month: u32,
    _end_month: u32,
) -> Vec<YearlyResult> {
    // Assign each day to a hydrological year
    let mut groups: BTreeMap<String, Vec<&DailyResult>> = BTreeMap::new();
    for d in daily {
        let month: u32 = d.date[5..7].parse().unwrap_or(1);
        let year: i32 = d.date[..4].parse().unwrap_or(2000);
        let hydro_year = if month >= start_month { year } else { year - 1 };
        let key = format!("{}.{}~{}.{}", hydro_year, start_month, hydro_year + 1, _end_month);
        groups.entry(key).or_default().push(d);
    }

    groups
        .into_iter()
        .map(|(year, rows)| {
            let values = aggregate_yearly_values(&rows, supply_order);
            YearlyResult { year, values }
        })
        .collect()
}

fn aggregate_yearly_values(
    rows: &[&DailyResult],
    supply_order: &[String],
) -> Vec<(String, f64)> {
    let n = rows.len() as f64;
    let mut values = Vec::new();

    values.push(("平均来水流量".to_string(), rows.iter().map(|r| r.q_in).sum::<f64>() / n));
    values.push(("平均发电流量".to_string(), rows.iter().map(|r| r.q_gen_after).sum::<f64>() / n));
    values.push(("平均弃水流量".to_string(), rows.iter().map(|r| r.q_thrown_after).sum::<f64>() / n));
    values.push(("平均出力".to_string(), rows.iter().map(|r| r.power_after).sum::<f64>() / n));

    // Total water volume (sum of q * days * 8.64) in 万m3
    let total_inflow: f64 = rows.iter().map(|r| r.q_in * r.days * 8.64).sum();
    values.push(("来水量_万m3".to_string(), total_inflow));

    let total_power_water: f64 = rows.iter().map(|r| r.q_gen_after * r.days * 8.64).sum();
    values.push(("发电水量_万m3".to_string(), total_power_water));

    // Annual energy (sum of power * days * 24 / 10^4) in 万kWh
    let energy: f64 = rows.iter().map(|r| r.power_after * r.days * 24.0 / 10000.0).sum();
    values.push(("发电量_万kWh".to_string(), energy));

    if let Some(last) = rows.last() {
        values.push(("末库容".to_string(), last.v_end_after));
        values.push(("末水位".to_string(), last.z_end_after));
    }

    values.push(("总天数".to_string(), rows.iter().map(|r| r.days).sum()));

    // Per-user supply stats
    for user_name in supply_order {
        let total_supply: f64 = rows
            .iter()
            .map(|r| {
                r.supply_detail_after
                    .iter()
                    .find(|s| s.name == *user_name)
                    .map(|s| s.supply * r.days * 8.64)
                    .unwrap_or(0.0)
            })
            .sum();
        let lack_days: f64 = rows
            .iter()
            .map(|r| {
                r.supply_detail_after
                    .iter()
                    .find(|s| s.name == *user_name)
                    .map(|s| s.lack_day)
                    .unwrap_or(0.0)
            })
            .sum();
        values.push((format!("{}_供水量_万m3", user_name), total_supply));
        values.push((format!("{}_缺水天数", user_name), lack_days));
    }

    values
}

/// Build summary statistics (multi-year averages, etc.)
pub fn build_summary(daily: &[DailyResult], supply_order: &[String]) -> Vec<(String, f64)> {
    if daily.is_empty() {
        return vec![];
    }

    let n = daily.len() as f64;
    let total_days: f64 = daily.iter().map(|r| r.days).sum();

    // Determine number of years
    let first_year: f64 = daily.first().unwrap().date[..4].parse().unwrap_or(2000.0);
    let last_year: f64 = daily.last().unwrap().date[..4].parse().unwrap_or(2000.0);
    let num_years = (last_year - first_year + 1.0).max(1.0);

    let mut summary = Vec::new();

    summary.push(("多年平均来水流量".to_string(), daily.iter().map(|r| r.q_in).sum::<f64>() / n));
    summary.push(("多年平均发电流量".to_string(), daily.iter().map(|r| r.q_gen_after).sum::<f64>() / n));
    summary.push(("多年平均出力".to_string(), daily.iter().map(|r| r.power_after).sum::<f64>() / n));

    let annual_energy: f64 = daily.iter().map(|r| r.power_after * r.days * 24.0 / 10000.0).sum::<f64>() / num_years;
    summary.push(("年均发电量_万kWh".to_string(), annual_energy));

    let annual_inflow: f64 = daily.iter().map(|r| r.q_in * r.days * 8.64).sum::<f64>() / num_years;
    summary.push(("年均来水量_万m3".to_string(), annual_inflow));

    // Per-user annual supply
    for user_name in supply_order {
        let total_supply: f64 = daily
            .iter()
            .map(|r| {
                r.supply_detail_after
                    .iter()
                    .find(|s| s.name == *user_name)
                    .map(|s| s.supply * r.days * 8.64)
                    .unwrap_or(0.0)
            })
            .sum::<f64>() / num_years;
        let total_lack_days: f64 = daily
            .iter()
            .map(|r| {
                r.supply_detail_after
                    .iter()
                    .find(|s| s.name == *user_name)
                    .map(|s| s.lack_day)
                    .unwrap_or(0.0)
            })
            .sum::<f64>() / num_years;

        summary.push((format!("{}_年均供水量_万m3", user_name), total_supply));
        summary.push((format!("{}_年均缺水天数", user_name), total_lack_days));
    }

    summary.push(("计算总天数".to_string(), total_days));
    summary.push(("计算年数".to_string(), num_years));

    summary
}
