use crate::efficiency::types::*;

/// 大循环指标 C1-C4
pub fn calc_macro_indicators(data: &[MacroRawRow]) -> Vec<IndicatorRow> {
    let n = data.len();
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        let r = &data[i];
        let c1 = r.recycled_usage / r.sewage_treated * 100.0;
        let c2 = r.recycled_usage / r.industrial_gdp * 10.0;
        let c3 = (r.supply - r.sales) / r.supply * 100.0;
        let c4 = if i == 0 {
            None
        } else {
            let prev = data[i - 1].recycled_usage;
            Some(round2((r.recycled_usage - prev) / prev * 100.0))
        };

        results.push(IndicatorRow {
            label: r.year.clone(),
            values: vec![Some(round2(c1)), Some(round2(c2)), Some(round2(c3)), c4],
        });
    }
    results
}

/// 小循环指标 C5-C6
pub fn calc_meso_indicators(data: &[MesoRawRow]) -> Vec<IndicatorRow> {
    let n = data.len();
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        let r = &data[i];
        let c5 = r.connected_enterprises / r.total_enterprises * 100.0;
        let c6 = if i == 0 {
            None
        } else {
            let prev = data[i - 1].park_recycled_usage;
            Some(round2((r.park_recycled_usage - prev) / prev * 100.0))
        };

        results.push(IndicatorRow {
            label: r.year.clone(),
            values: vec![Some(round2(c5)), c6],
        });
    }
    results
}

/// 点循环指标 C7-C10（单个年度的企业数据）
pub fn calc_micro_indicators(data: &[MicroRawRow]) -> Vec<IndicatorRow> {
    data.iter()
        .map(|r| {
            let c7 = r.reuse_amount / (r.water_intake + r.reuse_amount) * 100.0;
            let c8 = r.cooling_circulation
                / (r.cooling_intake + r.cooling_circulation)
                * 100.0;
            let c9 = r.process_reuse / r.process_total * 100.0;
            let c10 = r.prior_recycled_usage.map(|prev| {
                round2((r.recycled_usage - prev) / prev * 100.0)
            });

            IndicatorRow {
                label: r.enterprise.clone(),
                values: vec![Some(round2(c7)), Some(round2(c8)), Some(round2(c9)), c10],
            }
        })
        .collect()
}

/// 按年度聚合点循环指标均值
pub fn aggregate_micro_by_year(
    micro_data: &std::collections::HashMap<String, Vec<MicroRawRow>>,
) -> Vec<IndicatorRow> {
    let mut years: Vec<&String> = micro_data.keys().collect();
    years.sort();

    years
        .iter()
        .map(|year| {
            let indicators = calc_micro_indicators(&micro_data[*year]);
            let num_indicators = if indicators.is_empty() {
                4
            } else {
                indicators[0].values.len()
            };
            let means: Vec<Option<f64>> = (0..num_indicators)
                .map(|j| {
                    let vals: Vec<f64> = indicators
                        .iter()
                        .filter_map(|row| row.values[j])
                        .collect();
                    if vals.is_empty() {
                        None
                    } else {
                        Some(round2(vals.iter().sum::<f64>() / vals.len() as f64))
                    }
                })
                .collect();

            IndicatorRow {
                label: year.to_string(),
                values: means,
            }
        })
        .collect()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_macro() -> Vec<MacroRawRow> {
        vec![
            MacroRawRow { year: "2023年".into(), recycled_usage: 220.0, sewage_treated: 1100.0, industrial_gdp: 78.0, supply: 235.0, sales: 220.0 },
            MacroRawRow { year: "2024年".into(), recycled_usage: 280.0, sewage_treated: 1200.0, industrial_gdp: 85.0, supply: 295.0, sales: 280.0 },
        ]
    }

    #[test]
    fn test_macro_c1() {
        let result = calc_macro_indicators(&sample_macro());
        assert_eq!(result[0].values[0], Some(20.0)); // 220/1100*100
    }

    #[test]
    fn test_macro_c4_first_row_none() {
        let result = calc_macro_indicators(&sample_macro());
        assert_eq!(result[0].values[3], None);
    }

    #[test]
    fn test_macro_c4_second_row() {
        let result = calc_macro_indicators(&sample_macro());
        // (280-220)/220*100 = 27.27
        assert_eq!(result[1].values[3], Some(27.27));
    }
}
