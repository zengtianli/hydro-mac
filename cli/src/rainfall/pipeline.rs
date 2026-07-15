use super::types::*;
use regex::Regex;
use std::collections::HashMap;

/// Run the full 6-step pipeline.
pub fn run_pipeline(input: &PipelineInput, steps: &[u32]) -> PipelineOutput {
    let mut step_results = Vec::new();

    // Step 1 & 2: partition + area — already parsed from input
    let (lake_summary, partition_summary) = build_summaries(&input.partitions);

    if steps.contains(&1) {
        step_results.push(StepResult {
            step: 1,
            name: "分区处理".into(),
            status: "ok".into(),
            message: format!("解析 {} 个分区", input.partitions.len()),
        });
    }
    if steps.contains(&2) {
        step_results.push(StepResult {
            step: 2,
            name: "面积汇总".into(),
            status: "ok".into(),
            message: format!("{} 个湖泊, {} 个分区", lake_summary.len(), partition_summary.len()),
        });
    }

    // Build lake -> partition mapping
    let lake_partition: HashMap<String, String> = input
        .partitions
        .iter()
        .flat_map(|p| p.lake_ids.iter().map(move |id| (id.clone(), p.code.clone())))
        .collect();

    // Build partition code -> display_name mapping (for rainfall lookup)
    let partition_code_to_display: HashMap<String, String> = input
        .partitions
        .iter()
        .map(|p| (p.code.clone(), p.display_name.clone()))
        .collect();

    // Build lake -> area_m2 mapping
    let lake_area: HashMap<String, f64> = input
        .partitions
        .iter()
        .flat_map(|p| {
            p.lake_ids
                .iter()
                .zip(p.areas.iter())
                .map(|(id, &area)| (id.clone(), area))
        })
        .collect();

    // Build partition code -> total_area_m2
    let partition_total_area: HashMap<String, f64> = input
        .partitions
        .iter()
        .map(|p| (p.code.clone(), p.areas.iter().sum::<f64>()))
        .collect();

    // Ordered list of all lake IDs from partitions
    let all_lake_ids: Vec<String> = input
        .partitions
        .iter()
        .flat_map(|p| p.lake_ids.iter().cloned())
        .collect();

    // Build date range: expand daily rainfall dates to hourly
    let hourly_datetimes = expand_daily_to_hourly(&input.rainfall);

    // Step 3: ggxs (rainfall coefficient)
    // For each lake, compute hourly_ggxs = (daily_rainfall / 24) * 10000 / partition_area_m2 * 1000
    let mut lake_ggxs: HashMap<String, Vec<f64>> = HashMap::new();

    if steps.contains(&3) {
        // Build daily rainfall lookup: display_name -> date -> value
        let mut rainfall_lookup: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for dr in &input.rainfall {
            for (name, &val) in &dr.values {
                rainfall_lookup
                    .entry(name.clone())
                    .or_default()
                    .insert(dr.date.clone(), val);
            }
        }

        // Also try matching by partition number format from the CSV step naming
        // The rainfall headers are display names like "青洲平原区"
        // We need to match partition display names to rainfall column names

        // Build partition_name_map for flexible matching
        // The FQNNGXL columns use the names from the 2nd line of static_PYLYSCS.txt

        for lake_id in &all_lake_ids {
            let partition_code = match lake_partition.get(lake_id) {
                Some(c) => c,
                None => continue,
            };

            let display_name = match partition_code_to_display.get(partition_code) {
                Some(n) => n,
                None => continue,
            };

            let total_area = match partition_total_area.get(partition_code) {
                Some(&a) if a > 0.0 => a,
                _ => continue,
            };

            // Find the matching rainfall column
            let daily_map = match find_rainfall_column(&rainfall_lookup, display_name, partition_code) {
                Some(m) => m,
                None => {
                    // No rainfall data for this partition — fill with zeros
                    lake_ggxs.insert(lake_id.clone(), vec![0.0; hourly_datetimes.len()]);
                    continue;
                }
            };

            let mut hourly_vals = Vec::with_capacity(hourly_datetimes.len());
            for dt in &hourly_datetimes {
                let date_part = &dt[..dt.find(' ').unwrap_or(dt.len())];
                let daily_val = daily_map.get(date_part).copied().unwrap_or(0.0);
                let hourly_val = (daily_val / 24.0) * 10000.0 / total_area * 1000.0;
                hourly_vals.push(hourly_val);
            }
            lake_ggxs.insert(lake_id.clone(), hourly_vals);
        }

        step_results.push(StepResult {
            step: 3,
            name: "降雨系数计算".into(),
            status: "ok".into(),
            message: format!("计算 {} 个湖泊的降雨系数", lake_ggxs.len()),
        });
    }

    // Step 4: intake (water user intake)
    let mut lake_intake: HashMap<String, Vec<f64>> = HashMap::new();

    if steps.contains(&4) {
        // Group users by lake
        let mut lake_users: HashMap<String, Vec<&UserIntake>> = HashMap::new();
        for user in &input.users {
            if let Some(lake_id) = input.user_lake_map.get(&user.user_name) {
                lake_users.entry(lake_id.clone()).or_default().push(user);
            }
        }

        for (lake_id, users) in &lake_users {
            let area_m2 = match lake_area.get(lake_id) {
                Some(&a) if a > 0.0 => a,
                _ => continue,
            };

            // Build daily intake sum from all users for this lake
            let mut daily_intake_sum: HashMap<String, f64> = HashMap::new();
            for u in users {
                *daily_intake_sum.entry(u.date.clone()).or_insert(0.0) += u.hourly_intake;
            }

            let mut hourly_vals = Vec::with_capacity(hourly_datetimes.len());
            for dt in &hourly_datetimes {
                let date_part = &dt[..dt.find(' ').unwrap_or(dt.len())];
                let daily_val = daily_intake_sum.get(date_part).copied().unwrap_or(0.0);
                let hourly_val = daily_val / area_m2 * 1000.0;
                hourly_vals.push(hourly_val);
            }
            lake_intake.insert(lake_id.clone(), hourly_vals);
        }

        step_results.push(StepResult {
            step: 4,
            name: "取水处理".into(),
            status: "ok".into(),
            message: format!("处理 {} 个湖泊的取水数据", lake_intake.len()),
        });
    }

    // Step 5: deduct (merge rainfall + intake)
    let mut lake_demand: HashMap<String, Vec<f64>> = HashMap::new();

    if steps.contains(&5) {
        let n_hours = hourly_datetimes.len();
        for lake_id in &all_lake_ids {
            let ggxs = lake_ggxs.get(lake_id);
            let intake = lake_intake.get(lake_id);

            let demand: Vec<f64> = (0..n_hours)
                .map(|i| {
                    let g = ggxs.map(|v| v[i]).unwrap_or(0.0);
                    let k = intake.map(|v| v[i]).unwrap_or(0.0);
                    g + k
                })
                .collect();
            lake_demand.insert(lake_id.clone(), demand);
        }

        step_results.push(StepResult {
            step: 5,
            name: "扣减计算".into(),
            status: "ok".into(),
            message: format!("合并 {} 个湖泊的需水量", lake_demand.len()),
        });
    }

    // Step 6: merge final (baseline - demand)
    let mut final_columns = Vec::new();
    let mut final_rows = Vec::new();

    if steps.contains(&6) {
        // Normalize baseline column names to G{number} format
        let col_re = Regex::new(r"G(\d+)").unwrap();

        // Build normalized baseline column index: "G1" -> column index
        let mut baseline_col_index: HashMap<String, usize> = HashMap::new();
        for (i, col) in input.baseline_columns.iter().enumerate() {
            if let Some(caps) = col_re.captures(col) {
                let num: u32 = caps[1].parse().unwrap_or(0);
                let normalized = format!("G{}", num);
                baseline_col_index.insert(normalized, i);
            }
        }

        // Build sorted column list: G1, G2, ..., G228
        let mut all_col_nums: Vec<u32> = baseline_col_index
            .keys()
            .filter_map(|k| k[1..].parse::<u32>().ok())
            .collect();
        all_col_nums.sort();
        all_col_nums.dedup();

        final_columns = all_col_nums.iter().map(|n| format!("G{}", n)).collect();

        // Build demand lookup: normalized lake_id -> Vec<f64>
        let mut demand_normalized: HashMap<String, &Vec<f64>> = HashMap::new();
        for (lake_id, demand) in &lake_demand {
            if let Some(caps) = col_re.captures(lake_id) {
                let num: u32 = caps[1].parse().unwrap_or(0);
                let normalized = format!("G{}", num);
                demand_normalized.insert(normalized, demand);
            }
        }

        // If we have baseline data (hourly), use it directly
        // If baseline rows count matches hourly_datetimes, align by index
        // Otherwise, build datetime lookup
        if !input.baseline.is_empty() {
            let baseline_by_dt: HashMap<String, &HourlyRow> = input
                .baseline
                .iter()
                .map(|r| {
                    // Normalize datetime format to match
                    let dt = normalize_datetime(&r.datetime);
                    (dt, r)
                })
                .collect();

            for (hi, dt) in hourly_datetimes.iter().enumerate() {
                let dt_normalized = normalize_datetime(dt);
                let mut values = Vec::with_capacity(final_columns.len());

                for col_name in &final_columns {
                    let baseline_val = baseline_by_dt
                        .get(&dt_normalized)
                        .and_then(|row| {
                            baseline_col_index.get(col_name).map(|&ci| {
                                if ci < row.values.len() {
                                    row.values[ci]
                                } else {
                                    0.0
                                }
                            })
                        })
                        .unwrap_or(0.0);

                    let demand_val = demand_normalized
                        .get(col_name)
                        .map(|v| if hi < v.len() { v[hi] } else { 0.0 })
                        .unwrap_or(0.0);

                    values.push(baseline_val - demand_val);
                }

                final_rows.push(HourlyRow {
                    datetime: dt.clone(),
                    values,
                });
            }
        } else {
            // No baseline — just negate demand
            for (hi, dt) in hourly_datetimes.iter().enumerate() {
                let mut values = Vec::with_capacity(final_columns.len());
                for col_name in &final_columns {
                    let demand_val = demand_normalized
                        .get(col_name)
                        .map(|v| if hi < v.len() { v[hi] } else { 0.0 })
                        .unwrap_or(0.0);
                    values.push(-demand_val);
                }
                final_rows.push(HourlyRow {
                    datetime: dt.clone(),
                    values,
                });
            }
        }

        step_results.push(StepResult {
            step: 6,
            name: "合并输出".into(),
            status: "ok".into(),
            message: format!(
                "生成 {} 行 x {} 列",
                final_rows.len(),
                final_columns.len()
            ),
        });
    }

    let row_count = final_rows.len();
    let col_count = final_columns.len();

    PipelineOutput {
        steps: step_results,
        lake_summary,
        partition_summary,
        final_columns,
        final_rows,
        row_count,
        col_count,
    }
}

// ── Helper functions ──

fn build_summaries(partitions: &[Partition]) -> (Vec<LakeInfo>, Vec<PartitionSummary>) {
    let mut lakes = Vec::new();
    let mut parts = Vec::new();

    for p in partitions {
        let total: f64 = p.areas.iter().sum();
        parts.push(PartitionSummary {
            code: p.code.clone(),
            display_name: p.display_name.clone(),
            total_area_m2: total,
            lake_count: p.lake_ids.len(),
        });

        for (id, &area) in p.lake_ids.iter().zip(p.areas.iter()) {
            lakes.push(LakeInfo {
                id: id.clone(),
                area_m2: area,
                partition_code: p.code.clone(),
            });
        }
    }

    (lakes, parts)
}

/// Expand daily dates to hourly datetimes (24 hours per day).
fn expand_daily_to_hourly(rainfall: &[DailyRainfall]) -> Vec<String> {
    let mut datetimes = Vec::new();
    for dr in rainfall {
        for h in 0..24 {
            datetimes.push(format!("{} {:02}:00", dr.date, h));
        }
    }
    datetimes
}

/// Try to find a matching rainfall column by display name or partition code.
fn find_rainfall_column<'a>(
    rainfall_lookup: &'a HashMap<String, HashMap<String, f64>>,
    display_name: &str,
    _partition_code: &str,
) -> Option<&'a HashMap<String, f64>> {
    // Try exact display name match first
    if let Some(m) = rainfall_lookup.get(display_name) {
        return Some(m);
    }
    // Try partial match
    for (key, map) in rainfall_lookup {
        if key.contains(display_name) || display_name.contains(key.as_str()) {
            return Some(map);
        }
    }
    None
}

/// Normalize datetime string to "YYYY/MM/DD HH:00" format
fn normalize_datetime(dt: &str) -> String {
    let dt = dt.trim();
    // Handle "YYYY/MM/DD HH:MM" or "YYYY/MM/DD H:00" etc
    if let Some(space_pos) = dt.find(' ') {
        let date_part = &dt[..space_pos];
        let time_part = &dt[space_pos + 1..];
        // Extract hour
        let hour: u32 = time_part
            .split(':')
            .next()
            .and_then(|h| h.trim().parse().ok())
            .unwrap_or(0);
        format!("{} {:02}:00", date_part, hour)
    } else {
        format!("{} 00:00", dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_daily_to_hourly() {
        let rainfall = vec![DailyRainfall {
            date: "2025/05/14".into(),
            values: HashMap::new(),
        }];
        let hours = expand_daily_to_hourly(&rainfall);
        assert_eq!(hours.len(), 24);
        assert_eq!(hours[0], "2025/05/14 00:00");
        assert_eq!(hours[23], "2025/05/14 23:00");
    }

    #[test]
    fn test_normalize_datetime() {
        assert_eq!(normalize_datetime("2025/05/14 8:00"), "2025/05/14 08:00");
        assert_eq!(
            normalize_datetime("2025/05/14 23:00"),
            "2025/05/14 23:00"
        );
        assert_eq!(normalize_datetime("2025/05/14"), "2025/05/14 00:00");
    }
}
