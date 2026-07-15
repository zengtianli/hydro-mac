use super::dispatch::{calc_power_sub, operate_dispatch};
use super::interp;
use super::types::{Reservoir, UserSupplyDetail};

/// Supply calculation result for a single time step
pub struct SupplyResult {
    pub q_loss_real: f64,
    pub supply_details: Vec<UserSupplyDetail>,
    pub eco_supply: f64,
    pub eco_lack: f64,
    pub eco_lack_day: f64,
    pub q_gen: f64,
    pub q_thrown: f64,
    pub v_end: f64,
    pub z_up: f64,
    pub z_down: f64,
    pub dh: f64,
    pub h_net: f64,
    pub power: f64,
}

/// Calculate supply and electricity for a single time step.
///
/// This is the Rust equivalent of Python's `cal_supply_and_eletricity`.
pub fn calc_supply_and_power(
    res: &Reservoir,
    i: usize,
    mmdd: &str,
    is_leap: bool,
    dt: f64,
    q_in: f64,
    q_eco: f64,
    q_up_eco: f64,          // upstream eco flow that enters this reservoir
    up_eco_as_inflow: bool,  // if true, upstream eco is already in q_in
    const_z_down_val: f64,
    v_begin: f64,
    x_volumes: &[f64],
    x_powers: &[f64],
    v_flood: f64,
    v_limited_power: f64,
    v_dead_power: f64,
    v_dead: f64,
    epsilon_w: f64,
) -> SupplyResult {
    // --- Inflow calculation ---
    let effective_up_eco = if up_eco_as_inflow { 0.0 } else { q_up_eco };
    let q_loss_real = v_begin * res.loss_ratio / dt + res.loss_value / 8.64;
    let q_available_base = q_in - q_loss_real - effective_up_eco;

    // Reserve ecological flow
    let q_available_after_eco = q_available_base - q_eco;

    // --- Supply allocation (priority cascade) ---
    let mut supply_details = Vec::new();
    let mut q_add = q_available_after_eco;
    let mut total_supply_reservoir = 0.0_f64;
    let mut total_supply_downstream = 0.0_f64;

    let v_limited_supply = interp::lookup_daily(&res.v_limited_supply_day, mmdd, is_leap);

    for user_name in &res.supply_order {
        // Find the demand for this user
        let (demand, from_downstream) = find_user_demand(res, user_name, i);

        // Max supply = available flow + (volume above supply limit) / dt
        let max_supply = f64::max(0.0, q_add + (v_begin - v_limited_supply) / dt);
        let actual_supply = max_supply.min(demand);
        q_add -= actual_supply;

        let lack = demand - actual_supply;
        let lack_day = if lack > 0.0 { 1.0 } else { 0.0 };

        if from_downstream {
            total_supply_downstream += actual_supply;
        } else {
            total_supply_reservoir += actual_supply;
        }

        supply_details.push(UserSupplyDetail {
            name: user_name.clone(),
            demand,
            supply: actual_supply,
            lack,
            lack_day,
        });
    }

    // Update available flow (subtract reservoir supply, add back eco)
    let q_available = q_available_after_eco - total_supply_reservoir + q_eco;

    // --- Power dispatch ---
    let mut result = operate_dispatch(
        res,
        const_z_down_val,
        v_begin,
        q_available,
        x_volumes,
        x_powers,
        dt,
        v_flood,
        v_limited_power,
        epsilon_w,
    );

    let mut need_recalc = false;

    // Check if downstream supply requires more outflow
    let q_down = result.q_gen + result.q_thrown;
    if q_down < total_supply_downstream {
        let tmp_q_gen = f64::min(
            f64::max(0.0, q_available + (v_begin - v_dead_power) / dt),
            res.qm,
        );
        if tmp_q_gen < total_supply_downstream {
            result.q_gen = tmp_q_gen;
        } else {
            result.q_gen = total_supply_downstream;
        }
        result.q_thrown = total_supply_downstream - result.q_gen;
        need_recalc = true;
    }

    // Ecological flow check
    let mut eco_supply = q_eco;
    let mut eco_lack = 0.0;
    let mut eco_lack_day = 0.0;
    let q_down2 = result.q_gen + result.q_thrown;
    if q_down2 < total_supply_downstream + q_eco {
        let tmp_q_down = f64::max(0.0, q_available + (v_begin - v_dead) / dt);
        let tmp_q_gen = f64::min(
            f64::max(0.0, q_available + (v_begin - v_dead_power) / dt),
            res.qm,
        );

        if tmp_q_down < total_supply_downstream + q_eco {
            eco_supply = tmp_q_down - total_supply_downstream;
            eco_lack = q_eco - eco_supply;
            eco_lack_day = 1.0;
            let new_q_down = tmp_q_down;
            if tmp_q_gen < new_q_down {
                result.q_gen = tmp_q_gen;
            } else {
                result.q_gen = new_q_down;
            }
            result.q_thrown = new_q_down - result.q_gen;
        } else {
            let new_q_down = total_supply_downstream + q_eco;
            if tmp_q_gen < new_q_down {
                result.q_gen = tmp_q_gen;
            } else {
                result.q_gen = new_q_down;
            }
            result.q_thrown = new_q_down - result.q_gen;
        }
        need_recalc = true;
    }

    // Upstream eco flow can be used for power generation
    if q_up_eco > 0.0 && !up_eco_as_inflow {
        let new_q_down = q_up_eco + result.q_gen + result.q_thrown;
        result.q_gen = f64::min(result.q_gen + q_up_eco, res.qm);
        result.q_thrown = new_q_down - result.q_gen;
        need_recalc = true;
    }

    if need_recalc {
        let q_down_total = result.q_gen + result.q_thrown;
        result.v_end = v_begin + (q_available - q_down_total) * dt;

        let (z_up, z_down, dh, h_net, power) =
            calc_power_sub(res, v_begin, result.v_end, const_z_down_val, result.q_gen, result.q_thrown);
        result.z_up = z_up;
        result.z_down = z_down;
        result.dh = dh;
        result.h_net = h_net;
        result.power = power;
    }

    SupplyResult {
        q_loss_real,
        supply_details,
        eco_supply,
        eco_lack,
        eco_lack_day,
        q_gen: result.q_gen,
        q_thrown: result.q_thrown,
        v_end: result.v_end,
        z_up: result.z_up,
        z_down: result.z_down,
        dh: result.dh,
        h_net: result.h_net,
        power: result.power,
    }
}

/// Find user demand at time step i. Returns (demand_value, is_from_downstream).
fn find_user_demand(res: &Reservoir, user_name: &str, i: usize) -> (f64, bool) {
    // Check in-reservoir users
    for ud in &res.user_demand_reservoir {
        if ud.name == user_name {
            let val = ud.values.get(i).copied().unwrap_or(0.0);
            return (val, false);
        }
    }
    // Check downstream users
    for ud in &res.user_demand_downstream {
        if ud.name == user_name {
            let val = ud.values.get(i).copied().unwrap_or(0.0);
            return (val, true);
        }
    }
    (0.0, false)
}
