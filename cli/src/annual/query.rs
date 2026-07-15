use crate::annual::types::*;
use crate::common::round2;
use std::collections::HashMap;

/// 基础查询：按城市/年份过滤，选择指标列
pub fn query_data(
    headers: &[String],
    rows: &[DataRow],
    input: &QueryInput,
) -> QueryOutput {
    // 构建输出表头
    let mut out_headers = vec!["年份".to_string(), "市".to_string(), "县区名称".to_string()];
    let indicator_cols: Vec<String> = if input.indicators.is_empty() {
        // 所有数值列（跳过前几个基础列）
        headers
            .iter()
            .skip(9) // 跳过 水系..面积 等基础列
            .filter(|h| !h.is_empty())
            .cloned()
            .collect()
    } else {
        input.indicators.clone()
    };
    out_headers.extend(indicator_cols.iter().cloned());

    // 过滤数据行
    let filtered: Vec<&DataRow> = rows
        .iter()
        .filter(|r| {
            (input.years.is_empty() || input.years.contains(&r.year))
                && (input.cities.is_empty() || input.cities.contains(&r.city))
        })
        .collect();

    let city_count = filtered
        .iter()
        .map(|r| &r.city)
        .collect::<std::collections::HashSet<_>>()
        .len();

    let year_range = if filtered.is_empty() {
        String::new()
    } else {
        let min_y = filtered.iter().map(|r| r.year).min().unwrap();
        let max_y = filtered.iter().map(|r| r.year).max().unwrap();
        if min_y == max_y {
            format!("{}", min_y)
        } else {
            format!("{}-{}", min_y, max_y)
        }
    };

    let out_rows: Vec<Vec<serde_json::Value>> = filtered
        .iter()
        .map(|r| {
            let mut row = vec![
                serde_json::Value::Number(serde_json::Number::from(r.year)),
                serde_json::Value::String(r.city.clone()),
                serde_json::Value::String(r.district.clone()),
            ];
            for col in &indicator_cols {
                row.push(
                    r.values
                        .get(col)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            row
        })
        .collect();

    let total_rows = out_rows.len();
    QueryOutput {
        headers: out_headers,
        rows: out_rows,
        total_rows,
        year_range,
        city_count,
    }
}

/// 按城市聚合：group by (年份, 城市), 对指标列求和
pub fn aggregate_by_city(
    rows: &[DataRow],
    indicators: &[String],
) -> QueryOutput {
    let mut groups: HashMap<(i32, String), Vec<f64>> = HashMap::new();

    for row in rows {
        let key = (row.year, row.city.clone());
        let entry = groups
            .entry(key)
            .or_insert_with(|| vec![0.0; indicators.len()]);
        for (i, col) in indicators.iter().enumerate() {
            if let Some(serde_json::Value::Number(n)) = row.values.get(col) {
                entry[i] += n.as_f64().unwrap_or(0.0);
            }
        }
    }

    let mut out_headers = vec!["年份".to_string(), "市".to_string()];
    out_headers.extend(indicators.iter().cloned());

    let mut sorted_keys: Vec<_> = groups.keys().cloned().collect();
    sorted_keys.sort();

    let out_rows: Vec<Vec<serde_json::Value>> = sorted_keys
        .iter()
        .map(|(year, city)| {
            let vals = &groups[&(*year, city.clone())];
            let mut row: Vec<serde_json::Value> = vec![
                serde_json::Value::Number(serde_json::Number::from(*year)),
                serde_json::Value::String(city.clone()),
            ];
            for v in vals {
                row.push(f64_to_json(round2(*v)));
            }
            row
        })
        .collect();

    let total_rows = out_rows.len();
    let city_count = sorted_keys
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let year_range = year_range_str(&sorted_keys.iter().map(|(y, _)| *y).collect::<Vec<_>>());

    QueryOutput {
        headers: out_headers,
        rows: out_rows,
        total_rows,
        year_range,
        city_count,
    }
}

/// 按年份聚合：group by 年份，生成合计/平均行
pub fn aggregate_by_year(
    rows: &[DataRow],
    indicators: &[String],
) -> QueryOutput {
    let mut year_sums: HashMap<i32, Vec<f64>> = HashMap::new();
    let mut year_counts: HashMap<i32, usize> = HashMap::new();

    for row in rows {
        let entry = year_sums
            .entry(row.year)
            .or_insert_with(|| vec![0.0; indicators.len()]);
        for (i, col) in indicators.iter().enumerate() {
            if let Some(serde_json::Value::Number(n)) = row.values.get(col) {
                entry[i] += n.as_f64().unwrap_or(0.0);
            }
        }
        *year_counts.entry(row.year).or_insert(0) += 1;
    }

    let mut out_headers = vec!["年份".to_string(), "统计".to_string()];
    out_headers.extend(indicators.iter().cloned());

    let mut sorted_years: Vec<i32> = year_sums.keys().cloned().collect();
    sorted_years.sort();

    let mut out_rows: Vec<Vec<serde_json::Value>> = Vec::new();
    for year in &sorted_years {
        let sums = &year_sums[year];
        let count = year_counts[year];

        // 合计行
        let mut sum_row: Vec<serde_json::Value> = vec![
            serde_json::Value::Number(serde_json::Number::from(*year)),
            serde_json::Value::String("合计".to_string()),
        ];
        for v in sums {
            sum_row.push(f64_to_json(round2(*v)));
        }
        out_rows.push(sum_row);

        // 平均行
        let mut avg_row: Vec<serde_json::Value> = vec![
            serde_json::Value::Number(serde_json::Number::from(*year)),
            serde_json::Value::String("平均".to_string()),
        ];
        for v in sums {
            avg_row.push(f64_to_json(round2(*v / count as f64)));
        }
        out_rows.push(avg_row);
    }

    let total_rows = out_rows.len();
    let year_range = year_range_str(&sorted_years);

    QueryOutput {
        headers: out_headers,
        rows: out_rows,
        total_rows,
        year_range,
        city_count: 0,
    }
}

/// 年度对比：计算两年间的变化量和变化率
pub fn compare_years(
    rows: &[DataRow],
    year1: i32,
    year2: i32,
    indicators: &[String],
) -> YearComparison {
    // 按城市分组，每年取一条汇总
    let mut city_year: HashMap<(String, i32), Vec<f64>> = HashMap::new();
    for row in rows {
        if row.year != year1 && row.year != year2 {
            continue;
        }
        let key = (row.city.clone(), row.year);
        let entry = city_year
            .entry(key)
            .or_insert_with(|| vec![0.0; indicators.len()]);
        for (i, col) in indicators.iter().enumerate() {
            if let Some(serde_json::Value::Number(n)) = row.values.get(col) {
                entry[i] += n.as_f64().unwrap_or(0.0);
            }
        }
    }

    // 构建表头: 市, 指标_year1, 指标_year2, 变化量, 变化率%
    let mut headers = vec!["市".to_string()];
    for ind in indicators {
        headers.push(format!("{}_{}", ind, year1));
        headers.push(format!("{}_{}", ind, year2));
        headers.push(format!("{}_变化量", ind));
        headers.push(format!("{}_变化率%", ind));
    }

    let mut cities: Vec<String> = city_year
        .keys()
        .map(|(c, _)| c.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    cities.sort();

    let mut out_rows = Vec::new();
    for city in &cities {
        let vals1 = city_year
            .get(&(city.clone(), year1))
            .cloned()
            .unwrap_or_else(|| vec![0.0; indicators.len()]);
        let vals2 = city_year
            .get(&(city.clone(), year2))
            .cloned()
            .unwrap_or_else(|| vec![0.0; indicators.len()]);

        let mut row: Vec<serde_json::Value> = vec![serde_json::Value::String(city.clone())];
        for i in 0..indicators.len() {
            let v1 = vals1[i];
            let v2 = vals2[i];
            let diff = v2 - v1;
            let pct = if v1.abs() > 1e-10 {
                round2(diff / v1 * 100.0)
            } else {
                0.0
            };
            row.push(f64_to_json(round2(v1)));
            row.push(f64_to_json(round2(v2)));
            row.push(f64_to_json(round2(diff)));
            row.push(f64_to_json(pct));
        }
        out_rows.push(row);
    }

    YearComparison {
        headers,
        rows: out_rows,
    }
}

/// 计算单个指标的统计值
pub fn calculate_stats(rows: &[DataRow], indicator: &str) -> StatsResult {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| {
            r.values.get(indicator).and_then(|v| match v {
                serde_json::Value::Number(n) => n.as_f64(),
                _ => None,
            })
        })
        .collect();

    if values.is_empty() {
        return StatsResult {
            total: 0.0,
            average: 0.0,
            max: 0.0,
            min: 0.0,
            count: 0,
        };
    }

    let total: f64 = values.iter().sum();
    let count = values.len();
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);

    StatsResult {
        total: round2(total),
        average: round2(total / count as f64),
        max: round2(max),
        min: round2(min),
        count,
    }
}

/// f64 转 serde_json::Value（处理 NaN/Inf）
fn f64_to_json(v: f64) -> serde_json::Value {
    serde_json::Number::from_f64(v)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn year_range_str(years: &[i32]) -> String {
    if years.is_empty() {
        return String::new();
    }
    let min_y = *years.iter().min().unwrap();
    let max_y = *years.iter().max().unwrap();
    if min_y == max_y {
        format!("{}", min_y)
    } else {
        format!("{}-{}", min_y, max_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(year: i32, city: &str, val: f64) -> DataRow {
        let mut values = HashMap::new();
        values.insert(
            "总用水量".to_string(),
            f64_to_json(val),
        );
        DataRow {
            year,
            city: city.to_string(),
            table_name: "用水量".to_string(),
            district: "".to_string(),
            values,
        }
    }

    #[test]
    fn test_calculate_stats() {
        let rows = vec![
            make_row(2023, "汀州市", 100.0),
            make_row(2023, "云港市", 200.0),
            make_row(2024, "汀州市", 150.0),
        ];
        let stats = calculate_stats(&rows, "总用水量");
        assert_eq!(stats.count, 3);
        assert_eq!(stats.total, 450.0);
        assert_eq!(stats.max, 200.0);
        assert_eq!(stats.min, 100.0);
    }

    #[test]
    fn test_compare_years() {
        let rows = vec![
            make_row(2023, "汀州市", 100.0),
            make_row(2024, "汀州市", 120.0),
        ];
        let result = compare_years(&rows, 2023, 2024, &["总用水量".to_string()]);
        assert_eq!(result.rows.len(), 1);
        // 变化量 = 20, 变化率 = 20%
        assert_eq!(result.rows[0].len(), 5); // city + 4 cols per indicator
    }
}
