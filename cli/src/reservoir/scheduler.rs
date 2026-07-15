use super::balance::calc_supply_and_power;
use super::dispatch::calc_power_sub;
use super::interp;
use super::types::*;

/// Run the cascade scheduling for upstream + downstream reservoirs.
/// This is the Rust equivalent of Python's `power_operate_year_up_down`.
pub fn run_cascade(input: &ReservoirInput) -> ScheduleOutput {
    let params = &input.params;
    let up = &input.upstream;
    let down = &input.downstream;

    let n = up.dates.len();
    assert_eq!(n, down.dates.len(), "Date series length mismatch");

    // Parse dates
    let dates: Vec<chrono::NaiveDate> = up
        .dates
        .iter()
        .map(|d| {
            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
        })
        .collect();

    // Compute days per period
    let days_arr: Vec<f64> = (0..n)
        .map(|i| {
            if i + 1 < n {
                (dates[i + 1] - dates[i]).num_days() as f64
            } else if i > 0 {
                (dates[i] - dates[i - 1]).num_days() as f64
            } else {
                1.0
            }
        })
        .collect();

    let mmdd_arr: Vec<String> = dates.iter().map(|d| d.format("%m-%d").to_string()).collect();
    let is_leap_arr: Vec<bool> = dates
        .iter()
        .map(|d| {
            let y = d.year();
            (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
        })
        .collect();

    // Initial volumes (70% between dead_power and normal)
    let mut up_v_start =
        up.v_dead_power + 0.7 * (up.v_normal - up.v_dead_power);
    let mut down_v_start =
        down.v_dead_power + 0.7 * (down.v_normal - down.v_dead_power);

    // Iteration for convergence (start/end volume matching)
    let max_count = params.max_iterations.max(1);

    let mut up_daily = Vec::with_capacity(n);
    let mut down_daily = Vec::with_capacity(n);

    for _iteration in 0..max_count {
        up_daily.clear();
        down_daily.clear();

        let mut up_v_begin = up_v_start;
        let mut down_v_begin = down_v_start;

        for i in 0..n {
            let mmdd = &mmdd_arr[i];
            let is_leap = is_leap_arr[i];
            let dt = days_arr[i] * 8.64;

            // --- Look up daily parameters for upstream ---
            let up_v_flood = interp::lookup_daily(&up.v_flood_day, mmdd, is_leap);
            let up_v_dead_power_val = interp::lookup_daily(&up.v_dead_power_day, mmdd, is_leap);
            let up_v_limited_power_val = interp::lookup_daily(&up.v_limited_power_day, mmdd, is_leap);
            let up_const_z_down = if up.const_z_down {
                up.z_down_lookup
                    .as_ref()
                    .map(|l| interp::lookup_daily(l, mmdd, is_leap))
                    .unwrap_or(0.0)
            } else {
                -1.0
            };

            // Get dispatch line for upstream
            let (up_x_vols, up_x_pows) = interp::get_dispatch_for_date(
                &up.dispatch_line, mmdd, is_leap, up_v_flood, up.wpv, up_v_limited_power_val,
            );

            // Upstream inflow
            let up_q_in = up.q_inflow.get(i).copied().unwrap_or(0.0)
                + up.q_upstream.get(i).copied().unwrap_or(0.0);
            let up_q_eco = up.q_eco.get(i).copied().unwrap_or(0.0);

            // --- Calculate upstream ---
            let up_result = calc_supply_and_power(
                up, i, mmdd, is_leap, dt,
                up_q_in, up_q_eco,
                0.0,  // no upstream eco for the upstream reservoir
                params.up_eco_as_inflow,
                up_const_z_down,
                up_v_begin,
                &up_x_vols, &up_x_pows,
                up_v_flood, up_v_limited_power_val, up_v_dead_power_val, up.v_dead,
                params.epsilon_w,
            );

            // --- Look up daily parameters for downstream ---
            let down_v_flood = interp::lookup_daily(&down.v_flood_day, mmdd, is_leap);
            let down_v_dead_power_val = interp::lookup_daily(&down.v_dead_power_day, mmdd, is_leap);
            let down_v_limited_power_val = interp::lookup_daily(&down.v_limited_power_day, mmdd, is_leap);
            let down_const_z_down = if down.const_z_down {
                down.z_down_lookup
                    .as_ref()
                    .map(|l| interp::lookup_daily(l, mmdd, is_leap))
                    .unwrap_or(0.0)
            } else {
                -1.0
            };

            let (down_x_vols, down_x_pows) = interp::get_dispatch_for_date(
                &down.dispatch_line, mmdd, is_leap, down_v_flood, down.wpv, down_v_limited_power_val,
            );

            // Downstream inflow = own inflow + upstream outflow (gen + spill)
            let down_q_in = down.q_inflow.get(i).copied().unwrap_or(0.0)
                + down.q_upstream.get(i).copied().unwrap_or(0.0)
                + up_result.q_gen
                + up_result.q_thrown;
            let down_q_eco = down.q_eco.get(i).copied().unwrap_or(0.0);

            // Upstream eco supply that enters downstream
            let up_eco_supply = up_result.eco_supply;

            let down_result = calc_supply_and_power(
                down, i, mmdd, is_leap, dt,
                down_q_in, down_q_eco,
                up_eco_supply,
                params.up_eco_as_inflow,
                down_const_z_down,
                down_v_begin,
                &down_x_vols, &down_x_pows,
                down_v_flood, down_v_limited_power_val, down_v_dead_power_val, down.v_dead,
                params.epsilon_w,
            );

            // --- Supplement logic: upstream supplements downstream ---
            let (
                up_q_gen_after, up_q_thrown_after, up_v_end_after,
                down_q_gen_after, down_q_thrown_after, down_v_end_after,
                sup_q1, sup_q2, sup_q3,
                up_supply_after, down_supply_after,
            ) = apply_supplement(
                params, up, down, i, dt, mmdd, is_leap,
                &up_result, &down_result,
                up_v_begin, down_v_begin,
                up_const_z_down, down_const_z_down,
                up_v_dead_power_val, down_v_dead_power_val,
            );

            // Recalculate power after supplement if needed
            let (up_z_up_a, up_z_down_a, up_dh_a, up_h_net_a, up_power_a) = if sup_q1 + sup_q2 + sup_q3 > 0.0 {
                calc_power_sub(up, up_v_begin, up_v_end_after, up_const_z_down, up_q_gen_after, up_q_thrown_after)
            } else {
                (up_result.z_up, up_result.z_down, up_result.dh, up_result.h_net, up_result.power)
            };
            let (down_z_up_a, down_z_down_a, down_dh_a, down_h_net_a, down_power_a) = if sup_q1 + sup_q2 + sup_q3 > 0.0 {
                calc_power_sub(down, down_v_begin, down_v_end_after, down_const_z_down, down_q_gen_after, down_q_thrown_after)
            } else {
                (down_result.z_up, down_result.z_down, down_result.dh, down_result.h_net, down_result.power)
            };

            let up_z_end = interp::zv_to_level(up_result.v_end, &up.zv_curve);
            let up_z_end_after = interp::zv_to_level(up_v_end_after, &up.zv_curve);
            let down_z_end = interp::zv_to_level(down_result.v_end, &down.zv_curve);
            let down_z_end_after = interp::zv_to_level(down_v_end_after, &down.zv_curve);

            // Build daily results
            up_daily.push(DailyResult {
                date: up.dates[i].clone(),
                q_in: up_q_in,
                q_gen: up_result.q_gen,
                q_thrown: up_result.q_thrown,
                q_loss: up_result.q_loss_real,
                v_end: up_result.v_end,
                z_end: up_z_end,
                z_up: up_result.z_up,
                z_down: up_result.z_down,
                dh: up_result.dh,
                h_net: up_result.h_net,
                power: up_result.power,
                days: days_arr[i],
                supply_detail: up_result.supply_details.clone(),
                eco_supply: up_result.eco_supply,
                eco_lack: up_result.eco_lack,
                eco_lack_day: up_result.eco_lack_day,
                q_gen_after: up_q_gen_after,
                q_thrown_after: up_q_thrown_after,
                v_end_after: up_v_end_after,
                z_end_after: up_z_end_after,
                z_up_after: up_z_up_a,
                z_down_after: up_z_down_a,
                dh_after: up_dh_a,
                h_net_after: up_h_net_a,
                power_after: up_power_a,
                supply_detail_after: up_supply_after,
                supplement_q1: sup_q1,
                supplement_q2: sup_q2,
                supplement_q3: sup_q3,
            });

            down_daily.push(DailyResult {
                date: down.dates[i].clone(),
                q_in: down_q_in,
                q_gen: down_result.q_gen,
                q_thrown: down_result.q_thrown,
                q_loss: down_result.q_loss_real,
                v_end: down_result.v_end,
                z_end: down_z_end,
                z_up: down_result.z_up,
                z_down: down_result.z_down,
                dh: down_result.dh,
                h_net: down_result.h_net,
                power: down_result.power,
                days: days_arr[i],
                supply_detail: down_result.supply_details.clone(),
                eco_supply: down_result.eco_supply,
                eco_lack: down_result.eco_lack,
                eco_lack_day: down_result.eco_lack_day,
                q_gen_after: down_q_gen_after,
                q_thrown_after: down_q_thrown_after,
                v_end_after: down_v_end_after,
                z_end_after: down_z_end_after,
                z_up_after: down_z_up_a,
                z_down_after: down_z_down_a,
                dh_after: down_dh_a,
                h_net_after: down_h_net_a,
                power_after: down_power_a,
                supply_detail_after: down_supply_after,
                supplement_q1: sup_q1,
                supplement_q2: sup_q2,
                supplement_q3: sup_q3,
            });

            // Advance to next period
            up_v_begin = up_v_end_after;
            down_v_begin = down_v_end_after;
        }

        // Check convergence: start volume vs end volume
        let up_v_end_final = up_daily.last().map(|d| d.v_end_after).unwrap_or(0.0);
        let down_v_end_final = down_daily.last().map(|d| d.v_end_after).unwrap_or(0.0);

        if (up_v_start - up_v_end_final).abs() < params.epsilon_v {
            break;
        }
        // Update start volumes for next iteration
        up_v_start = up_v_end_final;
        down_v_start = down_v_end_final;
    }

    // Aggregate statistics
    let up_output = build_reservoir_output(
        &up.name, &up_daily, &up.supply_order, params.hydro_year_start, params.hydro_year_end,
    );
    let down_output = build_reservoir_output(
        &down.name, &down_daily, &down.supply_order, params.hydro_year_start, params.hydro_year_end,
    );

    ScheduleOutput {
        upstream: up_output,
        downstream: down_output,
    }
}

use chrono::Datelike;

/// Apply upstream-to-downstream supplement logic.
/// Returns (up_q_gen_after, up_q_thrown_after, up_v_end_after,
///          down_q_gen_after, down_q_thrown_after, down_v_end_after,
///          supplement_q1, supplement_q2, supplement_q3,
///          up_supply_after, down_supply_after)
#[allow(clippy::too_many_arguments)]
fn apply_supplement(
    params: &CalcParams,
    up: &Reservoir,
    down: &Reservoir,
    _i: usize,
    dt: f64,
    _mmdd: &str,
    _is_leap: bool,
    up_result: &super::balance::SupplyResult,
    down_result: &super::balance::SupplyResult,
    _up_v_begin: f64,
    _down_v_begin: f64,
    _up_const_z_down: f64,
    _down_const_z_down: f64,
    _up_v_dead_power: f64,
    _down_v_dead_power: f64,
) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, Vec<UserSupplyDetail>, Vec<UserSupplyDetail>) {
    let mut up_v_end_after = up_result.v_end;
    let mut down_v_end_after = down_result.v_end;
    let mut up_q_gen_after = up_result.q_gen;
    let mut up_q_thrown_after = up_result.q_thrown;
    let mut down_q_gen_after = down_result.q_gen;
    let mut down_q_thrown_after = down_result.q_thrown;
    let mut sup_q1 = 0.0;
    let mut sup_q2 = 0.0;
    let mut sup_q3 = 0.0;

    let mut down_supply_after = down_result.supply_details.clone();
    let up_supply_after = up_result.supply_details.clone();

    let need_recalc;

    // Check: upstream volume above special threshold 0 -> normal operation
    if params.up_v_special.len() >= 2 {
        let v_threshold_high = params.up_v_special[0];
        let v_threshold_low = params.up_v_special[1];

        if up_result.v_end > v_threshold_high {
            // No supplement needed
            need_recalc = false;
        } else if up_result.v_end >= v_threshold_low {
            // Supplement from surplus above threshold_low
            let max_add_q = f64::max(0.0, (up_result.v_end - v_threshold_low) / dt);

            // Supplement user deficits
            let mut remaining = max_add_q;
            for user_name in &params.need_add_users {
                if remaining <= 0.0 {
                    break;
                }
                if let Some(detail) = down_supply_after.iter_mut().find(|d| d.name == *user_name) {
                    let add = detail.lack.min(remaining);
                    detail.supply += add;
                    detail.lack -= add;
                    if detail.lack <= 0.0 {
                        detail.lack_day = 0.0;
                    }
                    remaining -= add;
                    sup_q2 += add;
                }
            }

            // Supplement to downstream volume target
            if !params.down_v_special.is_empty() && remaining > 0.0 {
                let need_q1 = f64::max(0.0, (params.down_v_special[0] - down_result.v_end) / dt);
                sup_q1 = need_q1.min(remaining);
            }

            let total_add = sup_q1 + sup_q2;
            down_v_end_after += f64::max(0.0, sup_q1) * dt;
            up_v_end_after -= total_add * dt;
            need_recalc = total_add > 0.0;
        } else {
            need_recalc = false;
        }

        // Below threshold_low: special user supplementation
        if up_v_end_after <= v_threshold_low {
            let mut tmp_up_v = up_v_end_after;
            let mut tmp_down_v = down_v_end_after;

            for (user_name, min_supply) in &params.user_special {
                let max_add_q = f64::max(0.0, (tmp_up_v - up.v_dead) / dt);

                if let Some(detail) = down_supply_after.iter_mut().find(|d| d.name == *user_name) {
                    let need = (*min_supply).min(detail.lack);
                    let actual = need.min(max_add_q);
                    detail.supply += actual;
                    detail.lack -= actual;
                    if detail.lack <= 0.0 {
                        detail.lack_day = 0.0;
                    }
                    tmp_up_v -= actual * dt;
                    sup_q3 += actual;
                }
            }

            // Stop supply for certain users when upstream is low
            for user_name in &params.user_stop_supply {
                if let Some(detail) = down_supply_after.iter_mut().find(|d| d.name == *user_name) {
                    if detail.supply > 0.0 {
                        let returned = detail.supply;
                        detail.lack += returned;
                        detail.supply = 0.0;
                        detail.lack_day = 1.0;
                        tmp_down_v += returned * dt;
                    }
                }
            }

            up_v_end_after = tmp_up_v;
            down_v_end_after = tmp_down_v;
        }
    } else {
        need_recalc = false;
    }

    // Recalculate generation flows after supplement
    if need_recalc || sup_q3 > 0.0 {
        let total_add = sup_q1 + sup_q2 + sup_q3;
        if total_add > 0.0 {
            // Upstream: additional flow through turbines
            let add_q_gen = f64::min(total_add, up.qm - up_result.q_gen)
                .min(f64::max(0.0, (up_result.v_end - up.v_dead_power) / dt));
            up_q_gen_after = f64::max(0.0, add_q_gen + up_result.q_gen);
            up_q_thrown_after = total_add + up_result.q_gen + up_result.q_thrown - up_q_gen_after;

            // Downstream: account for supply changes
            let down_supply_change: f64 = down_supply_after
                .iter()
                .zip(down_result.supply_details.iter())
                .filter(|(a, _b)| {
                    down.user_demand_downstream.iter().any(|u| u.name == a.name)
                })
                .map(|(a, b)| a.supply - b.supply)
                .sum();

            let add_q_gen_down = f64::min(down_supply_change, down.qm - down_result.q_gen)
                .min(f64::max(0.0, (down_result.v_end - down.v_dead_power) / dt));
            down_q_gen_after = f64::max(0.0, add_q_gen_down + down_result.q_gen);
            down_q_thrown_after = down_supply_change + down_result.q_gen + down_result.q_thrown - down_q_gen_after;
        }
    }

    (
        up_q_gen_after, up_q_thrown_after, up_v_end_after,
        down_q_gen_after, down_q_thrown_after, down_v_end_after,
        sup_q1, sup_q2, sup_q3,
        up_supply_after, down_supply_after,
    )
}

/// Build reservoir output with statistics aggregation
fn build_reservoir_output(
    name: &str,
    daily: &[DailyResult],
    supply_order: &[String],
    hydro_year_start: u32,
    hydro_year_end: u32,
) -> ReservoirOutput {
    let monthly = super::statistics::to_monthly(daily, supply_order);
    let yearly = super::statistics::to_yearly(daily, supply_order);
    let hydro_yearly =
        super::statistics::to_hydro_yearly(daily, supply_order, hydro_year_start, hydro_year_end);
    let summary = super::statistics::build_summary(daily, supply_order);

    ReservoirOutput {
        name: name.to_string(),
        daily: daily.to_vec(),
        monthly,
        yearly,
        hydro_yearly,
        summary,
    }
}
