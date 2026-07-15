/// Calculate head loss: dh = clamp(k1 * (k2 * q)^2, dh_min, dh_max)
/// Also considers k1 * qm^2 as a floor alongside dh_min.
pub fn calc_head_loss(k1: f64, k2: f64, q: f64, qm: f64, dh_min: f64, dh_max: f64) -> f64 {
    let dh_flow = k1 * (k2 * q).powi(2);
    let dh_qm = k1 * qm.powi(2);
    let dh = dh_flow.max(dh_qm).max(dh_min).min(dh_max);
    dh
}

/// Calculate hydropower generation (kW):
/// power = k0 * q_gen * h_net
pub fn calc_power(k0: f64, q_gen: f64, h_net: f64) -> f64 {
    k0 * q_gen * h_net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_loss_clamped() {
        // k1=0.0001, k2=1.0, q=100, qm=150, dh_min=0.5, dh_max=5.0
        let dh = calc_head_loss(0.0001, 1.0, 100.0, 150.0, 0.5, 5.0);
        // k1*(k2*q)^2 = 0.0001 * 10000 = 1.0
        // k1*qm^2 = 0.0001 * 22500 = 2.25
        // max(1.0, 2.25, 0.5) = 2.25, min(2.25, 5.0) = 2.25
        assert!((dh - 2.25).abs() < 1e-10);
    }

    #[test]
    fn test_head_loss_min_clamp() {
        let dh = calc_head_loss(0.00001, 1.0, 10.0, 10.0, 2.0, 5.0);
        // k1*(k2*q)^2 = 0.00001 * 100 = 0.001
        // k1*qm^2 = 0.00001 * 100 = 0.001
        // max(0.001, 0.001, 2.0) = 2.0
        assert!((dh - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_head_loss_max_clamp() {
        let dh = calc_head_loss(1.0, 1.0, 100.0, 100.0, 0.5, 5.0);
        // k1*(k2*q)^2 = 1.0 * 10000 = 10000 -> clamped to 5.0
        assert!((dh - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_power_calculation() {
        let p = calc_power(8.5, 50.0, 80.0);
        // 8.5 * 50 * 80 = 34000
        assert!((p - 34000.0).abs() < 1e-10);
    }
}
