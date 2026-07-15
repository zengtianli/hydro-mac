use super::types::{DailyRow, MonthlyRow, ZoneMonthlyAvgRow};
use std::collections::HashMap;

/// 解析日期字符串为 (year, month)
fn parse_ym(date_str: &str) -> Option<(i32, u32)> {
    // Try "YYYY-MM-DD" or "YYYY/MM/DD"
    let parts: Vec<&str> = date_str.splitn(3, |c| c == '-' || c == '/').collect();
    if parts.len() >= 2 {
        let y: i32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        if (1..=12).contains(&m) {
            return Some((y, m));
        }
    }
    None
}

/// 逐日 → 逐月平均
pub fn daily_to_monthly(daily: &[DailyRow], columns: &[String]) -> Vec<MonthlyRow> {
    // Accumulate: (year, month) -> { col -> (sum, count) }
    let mut acc: HashMap<(i32, u32), HashMap<String, (f64, u32)>> = HashMap::new();

    for row in daily {
        let (year, month) = match parse_ym(&row.date) {
            Some(ym) => ym,
            None => continue,
        };

        let entry = acc.entry((year, month)).or_default();
        for (col, val) in &row.values {
            if columns.contains(col) {
                let e = entry.entry(col.clone()).or_insert((0.0, 0));
                e.0 += val;
                e.1 += 1;
            }
        }
    }

    let mut keys: Vec<(i32, u32)> = acc.keys().cloned().collect();
    keys.sort();

    keys.into_iter()
        .map(|(year, month)| {
            let month_acc = &acc[&(year, month)];
            let values = columns
                .iter()
                .map(|col| {
                    let (sum, count) = month_acc.get(col).copied().unwrap_or((0.0, 0));
                    let avg = if count > 0 { sum / count as f64 } else { 0.0 };
                    (col.clone(), avg)
                })
                .collect();
            MonthlyRow { year, month, values }
        })
        .collect()
}

/// 逐月 → 多年月平均 (每月跨年取平均)
pub fn monthly_to_yearly_avg(monthly: &[MonthlyRow], columns: &[String], is_capacity: bool) -> Vec<ZoneMonthlyAvgRow> {
    // For each column, group by month
    columns
        .iter()
        .map(|col| {
            let mut month_sums = [0.0_f64; 12];
            let mut month_counts = [0u32; 12];

            for row in monthly {
                let idx = (row.month - 1) as usize;
                if idx < 12 {
                    let val = row.values.iter().find(|(c, _)| c == col).map(|(_, v)| *v).unwrap_or(0.0);
                    month_sums[idx] += val;
                    month_counts[idx] += 1;
                }
            }

            let months: Vec<f64> = (0..12)
                .map(|i| {
                    if month_counts[i] > 0 {
                        month_sums[i] / month_counts[i] as f64
                    } else {
                        0.0
                    }
                })
                .collect();

            let summary = if is_capacity {
                months.iter().sum()
            } else {
                let non_zero: Vec<f64> = months.iter().copied().filter(|v| *v != 0.0).collect();
                if non_zero.is_empty() { 0.0 } else { non_zero.iter().sum::<f64>() / non_zero.len() as f64 }
            };

            ZoneMonthlyAvgRow {
                zone_id: col.clone(),
                months,
                summary,
            }
        })
        .collect()
}

/// 水文年月份重排序: 给定 start_month，返回 [start, start+1, ..., 12, 1, ..., start-1]
pub fn hydro_year_reorder(start_month: u32) -> Vec<u32> {
    (0..12).map(|i| (start_month + i - 1) % 12 + 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hydro_year_reorder_april() {
        let order = hydro_year_reorder(4);
        assert_eq!(order, vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2, 3]);
    }

    #[test]
    fn test_hydro_year_reorder_january() {
        let order = hydro_year_reorder(1);
        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn test_daily_to_monthly() {
        let cols = vec!["Z1".to_string()];
        let daily: Vec<DailyRow> = (1..=31)
            .map(|d| DailyRow {
                date: format!("2024-01-{:02}", d),
                values: vec![("Z1".into(), 10.0)],
            })
            .collect();

        let monthly = daily_to_monthly(&daily, &cols);
        assert_eq!(monthly.len(), 1);
        assert_eq!(monthly[0].year, 2024);
        assert_eq!(monthly[0].month, 1);
        let v = monthly[0].values.iter().find(|(c, _)| c == "Z1").unwrap().1;
        assert!((v - 10.0).abs() < 1e-10);
    }
}
