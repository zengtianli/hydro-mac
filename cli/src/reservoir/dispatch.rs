use super::interp;
use super::types::Reservoir;

/// Results from a single-step power dispatch calculation.
pub struct PowerResult {
    pub q_gen: f64,
    pub q_thrown: f64,
    pub v_end: f64,
    pub z_up: f64,
    pub z_down: f64,
    pub dh: f64,
    pub h_net: f64,
    pub power: f64,
}

/// Calculate power output for given generation and spill flows.
/// Returns (z_up, z_down, dh, h_net, power).
pub fn calc_power_sub(
    res: &Reservoir,
    v_begin: f64,
    v_end: f64,
    const_z_down_val: f64,
    q_gen: f64,
    q_thrown: f64,
) -> (f64, f64, f64, f64, f64) {
    // Upstream water level at mid-volume
    let v_mid = (v_begin + v_end) / 2.0;
    let z_up = interp::zv_to_level(v_mid, &res.zv_curve);

    // Downstream tailwater level
    let z_down = if res.const_z_down {
        const_z_down_val
    } else {
        interp::qz_to_level(q_gen + q_thrown, &res.zq_curve)
    };

    // Head loss
    let dh = super::physics::calc_head_loss(
        res.k1, res.k2, q_gen + q_thrown, res.qm, res.dh_min, res.dh_max,
    );

    // Net head
    let h_net = z_up - z_down - dh;

    // Power
    let power = super::physics::calc_power(res.k0, q_gen, h_net);

    (z_up, z_down, dh, h_net, power)
}

/// Single-step power calculation: given initial volume, inflow (net after supply),
/// generation flow, compute actual q_gen, q_thrown, v_end, and power metrics.
pub fn calc_power_step(
    res: &Reservoir,
    const_z_down_val: f64,
    v_initial: f64,
    q_in: f64,
    q_gen_input: f64,
    dt: f64,
    v_flood: f64,
    v_limited_power: f64,
) -> PowerResult {
    // Limit generation to not go below limited power volume
    let q_gen_max = f64::max(0.0, q_in + (v_initial - v_limited_power) / dt);
    let q_gen = q_gen_input.min(q_gen_max);

    // Spill and end volume
    let mut q_thrown = 0.0;
    let mut v_end = v_initial + (q_in - q_gen) * dt;
    if v_end > v_flood {
        q_thrown = (v_end - v_flood) / dt;
    }
    v_end = v_initial + (q_in - q_gen - q_thrown) * dt;

    let (z_up, z_down, dh, h_net, power) =
        calc_power_sub(res, v_initial, v_end, const_z_down_val, q_gen, q_thrown);

    PowerResult {
        q_gen,
        q_thrown,
        v_end,
        z_up,
        z_down,
        dh,
        h_net,
        power,
    }
}

/// Dispatch line-based operation: iterate through dispatch line zones to find
/// the appropriate generation level.
/// `x_volumes` and `x_powers` are the dispatch line values (flood + zones + limited).
pub fn operate_dispatch(
    res: &Reservoir,
    const_z_down_val: f64,
    v_initial: f64,
    q_in: f64,
    x_volumes: &[f64],
    x_powers: &[f64],
    dt: f64,
    v_flood: f64,
    v_limited_power: f64,
    epsilon_w: f64,
) -> PowerResult {
    let n_lines = x_volumes.len().saturating_sub(1); // number of dispatch zones
    let qm = res.qm;
    let k0 = res.k0;
    let max_iter = 50;

    let mut current_rank = 1usize; // start from first dispatch zone (index 1)

    while current_rank <= n_lines {
        let v_target = x_volumes[current_rank];
        let p_target = x_powers[current_rank];
        let p_upper = if current_rank > 0 {
            x_powers[current_rank - 1]
        } else {
            f64::INFINITY
        };

        // Initial generation flow: try to reach target volume
        let q_gen_init = f64::min(
            f64::max(0.0, q_in + (v_initial - v_target) / dt),
            qm,
        );

        let mut result = calc_power_step(
            res, const_z_down_val, v_initial, q_in, q_gen_init, dt, v_flood, v_limited_power,
        );

        if result.power >= p_upper {
            // Case 1: exceeds upper target, iterate to match
            let target_power = p_upper;
            let mut iter_count = 0;
            while (result.power - target_power).abs() >= epsilon_w && iter_count < max_iter {
                let q_gen_new = if result.h_net <= 0.0 {
                    0.0
                } else {
                    (target_power / (k0 * result.h_net)).max(0.0).min(qm)
                };

                result = calc_power_step(
                    res, const_z_down_val, v_initial, q_in, q_gen_new, dt, v_flood, v_limited_power,
                );
                iter_count += 1;
            }
            return result;
        } else if result.power >= p_target {
            // Case 2: within current zone
            return result;
        } else {
            // Case 3: below target, try next zone
            if current_rank < n_lines {
                current_rank += 1;
                continue;
            } else {
                return result;
            }
        }
    }

    // Fallback: should not reach here
    calc_power_step(
        res, const_z_down_val, v_initial, q_in, 0.0, dt, v_flood, v_limited_power,
    )
}
