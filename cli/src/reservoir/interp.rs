use super::types::{DailyLookup, DispatchLine, ZvPoint, QzPoint};

/// Linear interpolation: given sorted (xs, ys), find y at x.
/// Clamps to boundary values if x is outside range.
pub fn interp(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    if xs.is_empty() || ys.is_empty() {
        return 0.0;
    }
    let n = xs.len().min(ys.len());
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    for i in 1..n {
        if x <= xs[i] {
            let t = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
            return ys[i - 1] + t * (ys[i] - ys[i - 1]);
        }
    }
    ys[n - 1]
}

/// Interpolate Z-V curve: volume -> water level
pub fn zv_to_level(v: f64, curve: &[ZvPoint]) -> f64 {
    let vols: Vec<f64> = curve.iter().map(|p| p.volume).collect();
    let lvls: Vec<f64> = curve.iter().map(|p| p.water_level).collect();
    interp(v, &vols, &lvls)
}

/// Interpolate Z-V curve: water level -> volume
pub fn zv_to_volume(z: f64, curve: &[ZvPoint]) -> f64 {
    let lvls: Vec<f64> = curve.iter().map(|p| p.water_level).collect();
    let vols: Vec<f64> = curve.iter().map(|p| p.volume).collect();
    interp(z, &lvls, &vols)
}

/// Interpolate Q-Z curve: flow -> tailwater level
pub fn qz_to_level(q: f64, curve: &[QzPoint]) -> f64 {
    let flows: Vec<f64> = curve.iter().map(|p| p.q_down).collect();
    let lvls: Vec<f64> = curve.iter().map(|p| p.water_level).collect();
    interp(q, &flows, &lvls)
}

/// Look up a daily value from the DailyLookup table.
/// `mmdd` is "MM-DD", `is_leap` selects which column.
pub fn lookup_daily(lookup: &DailyLookup, mmdd: &str, is_leap: bool) -> f64 {
    for (key, normal, leap) in &lookup.entries {
        if key == mmdd {
            return if is_leap { *leap } else { *normal };
        }
    }
    // Fallback: if "02-29" requested but not leap, use "02-28"
    if mmdd == "02-29" && !is_leap {
        return lookup_daily(lookup, "02-28", false);
    }
    0.0
}

/// Get dispatch line volumes and powers for a given date.
/// Returns (volumes, powers) with flood volume prepended and limited_power appended.
pub fn get_dispatch_for_date(
    dispatch: &DispatchLine,
    mmdd: &str,
    _is_leap: bool,
    v_flood: f64,
    wpv: f64,
    v_limited_power: f64,
) -> (Vec<f64>, Vec<f64>) {
    // Find the matching month entry
    // The dispatch line has monthly points; we pick the one matching mmdd's month
    // In the Python code, operate_line_dict is keyed by mmdd and contains
    // per-day interpolated values. For MVP, we match by month.
    let target_month = &mmdd[0..2];

    let mut best_entry = None;
    for entry in &dispatch.monthly_points {
        let entry_month = &entry.mmdd[0..2];
        if entry_month == target_month {
            best_entry = Some(entry);
            break;
        }
    }

    // Fallback to first entry if nothing matches
    let entry = best_entry.or(dispatch.monthly_points.first());

    let (mut volumes, mut powers) = match entry {
        Some(e) => {
            // Select normal/leap values — for MVP the dispatch line is the same
            (e.volumes.clone(), e.powers.clone())
        }
        None => (vec![], vec![]),
    };

    // Prepend flood volume + wpv, append limited_power + 0
    volumes.insert(0, v_flood);
    powers.insert(0, wpv);
    volumes.push(v_limited_power);
    powers.push(0.0);

    (volumes, powers)
}

/// Create a DailyLookup from monthly control points with linear interpolation.
/// `month_points` is a list of (mmdd_str, value) pairs.
/// This is a simplified version of Python's `ComFun.create_lookup_df`.
pub fn create_daily_lookup(month_points: &[(String, f64)], interpolate: bool) -> DailyLookup {
    if month_points.len() < 2 {
        // Return constant lookup
        let val = month_points.first().map(|p| p.1).unwrap_or(0.0);
        let entries = generate_all_mmdd()
            .into_iter()
            .map(|mmdd| (mmdd, val, val))
            .collect();
        return DailyLookup { entries };
    }

    // Convert mmdd strings to day-of-year (using 2022 as normal year base)
    let mut points: Vec<(u32, f64)> = month_points
        .iter()
        .map(|(mmdd, val)| (mmdd_to_doy(mmdd, false), *val))
        .collect();
    points.sort_by_key(|p| p.0);

    // Add wrap-around point
    let first_val = points[0].1;
    points.push((points[0].0 + 365, first_val));

    let mut lookup_normal = std::collections::HashMap::new();
    let mut lookup_leap = std::collections::HashMap::new();

    // Interpolate segments
    for i in 0..points.len() - 1 {
        let (doy_start, val_start) = points[i];
        let (doy_end, val_end) = points[i + 1];
        let span = doy_end - doy_start;
        if span == 0 {
            continue;
        }

        let delta = if interpolate {
            (val_end - val_start) / span as f64
        } else {
            0.0
        };

        for j in 0..span {
            let doy = (doy_start + j - 1) % 365 + 1;
            let val = val_start + delta * j as f64;
            let mmdd = doy_to_mmdd(doy, false);
            lookup_normal.insert(mmdd.clone(), val);
            lookup_leap.insert(mmdd, val);
        }
    }

    // Build entries in order
    let all_mmdd = generate_all_mmdd();
    let entries = all_mmdd
        .into_iter()
        .map(|mmdd| {
            let normal = lookup_normal.get(&mmdd).copied().unwrap_or(0.0);
            let leap = lookup_leap.get(&mmdd).copied().unwrap_or(normal);
            (mmdd, normal, leap)
        })
        .collect();

    DailyLookup { entries }
}

/// Generate all MM-DD strings for a leap year (366 days)
fn generate_all_mmdd() -> Vec<String> {
    let days_in_month = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut result = Vec::with_capacity(366);
    for (m, &days) in days_in_month.iter().enumerate() {
        for d in 1..=days {
            result.push(format!("{:02}-{:02}", m + 1, d));
        }
    }
    result
}

/// Convert MM-DD to day-of-year (1-based)
fn mmdd_to_doy(mmdd: &str, _is_leap: bool) -> u32 {
    let parts: Vec<&str> = mmdd.split('-').collect();
    if parts.len() != 2 {
        return 1;
    }
    let month: u32 = parts[0].parse().unwrap_or(1);
    let day: u32 = parts[1].parse().unwrap_or(1);
    let days_before = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let m = (month as usize).min(12).max(1) - 1;
    days_before[m] + day
}

/// Convert day-of-year (1-based) to MM-DD string
fn doy_to_mmdd(doy: u32, _is_leap: bool) -> String {
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining = doy;
    for (m, &days) in days_in_month.iter().enumerate() {
        if remaining <= days {
            return format!("{:02}-{:02}", m + 1, remaining);
        }
        remaining -= days;
    }
    "12-31".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interp_basic() {
        let xs = vec![0.0, 10.0, 20.0];
        let ys = vec![100.0, 200.0, 300.0];
        assert!((interp(5.0, &xs, &ys) - 150.0).abs() < 1e-10);
        assert!((interp(0.0, &xs, &ys) - 100.0).abs() < 1e-10);
        assert!((interp(20.0, &xs, &ys) - 300.0).abs() < 1e-10);
        // Extrapolation clamps
        assert!((interp(-5.0, &xs, &ys) - 100.0).abs() < 1e-10);
        assert!((interp(25.0, &xs, &ys) - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_zv_to_level() {
        let curve = vec![
            ZvPoint { water_level: 100.0, volume: 1000.0 },
            ZvPoint { water_level: 110.0, volume: 2000.0 },
            ZvPoint { water_level: 120.0, volume: 4000.0 },
        ];
        let z = zv_to_level(1500.0, &curve);
        assert!((z - 105.0).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_daily() {
        let entries = vec![
            ("01-01".to_string(), 100.0, 100.0),
            ("06-15".to_string(), 200.0, 200.0),
        ];
        let lookup = DailyLookup { entries };
        assert!((lookup_daily(&lookup, "01-01", false) - 100.0).abs() < 1e-10);
        assert!((lookup_daily(&lookup, "06-15", true) - 200.0).abs() < 1e-10);
    }
}
