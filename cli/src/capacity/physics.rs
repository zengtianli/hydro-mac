/// 单位换算系数 (秒→年, mg→t)
const UNIT_FACTOR: f64 = 31.536;

/// 计算流速: u = a * Q^beta
pub fn velocity(q: f64, a: f64, beta: f64) -> f64 {
    if q <= 0.0 {
        return 0.0;
    }
    a * q.powf(beta)
}

/// 计算出流浓度 (链式传递): C_out = C0 * exp(-K*L/u)
pub fn outflow_concentration(c0: f64, k: f64, l: f64, u: f64) -> f64 {
    if u <= 0.0 || l <= 0.0 {
        return c0;
    }
    c0 * (-k * l / u).exp()
}

/// 河道纳污能力
/// W = 31.536 * b * (Cs - C0*exp(-KL/u)) * (Q*K*L/u) / (1 - exp(-KL/u))
pub fn capacity_value(cs: f64, c0: f64, q: f64, u: f64, k: f64, l: f64, b: f64) -> f64 {
    if u <= 0.0 || q <= 0.0 {
        return 0.0;
    }

    let decay = (-k * l / u).exp();

    // 避免除零
    if decay >= 1.0 - 1e-10 {
        return 0.0;
    }

    let concentration_term = cs - c0 * decay;
    let flow_term = (q * k * l / u) / (1.0 - decay);
    let w = UNIT_FACTOR * b * concentration_term * flow_term;

    w.max(0.0)
}

/// 水库纳污能力: W = 31.536 * K * V * Cs * b
pub fn reservoir_capacity(k: f64, cs: f64, v: f64, b: f64) -> f64 {
    if v <= 0.0 || cs <= 0.0 {
        return 0.0;
    }
    UNIT_FACTOR * k * v * cs * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_positive() {
        let u = velocity(10.0, 0.05, 0.6);
        assert!(u > 0.0);
        // u = 0.05 * 10^0.6 ≈ 0.05 * 3.981 ≈ 0.199
        assert!((u - 0.199).abs() < 0.01);
    }

    #[test]
    fn test_velocity_zero_q() {
        assert_eq!(velocity(0.0, 0.05, 0.6), 0.0);
        assert_eq!(velocity(-1.0, 0.05, 0.6), 0.0);
    }

    #[test]
    fn test_outflow_concentration() {
        let c_out = outflow_concentration(0.5, 1e-6, 5000.0, 0.2);
        // decay = exp(-1e-6 * 5000 / 0.2) = exp(-0.025) ≈ 0.9753
        assert!((c_out - 0.5 * 0.9753).abs() < 0.01);
    }

    #[test]
    fn test_outflow_concentration_zero_u() {
        assert_eq!(outflow_concentration(0.5, 1e-6, 5000.0, 0.0), 0.5);
    }

    #[test]
    fn test_capacity_value_basic() {
        let w = capacity_value(0.2, 0.1, 10.0, 0.2, 1e-6, 5000.0, 0.9);
        assert!(w > 0.0);
    }

    #[test]
    fn test_capacity_value_zero_u() {
        assert_eq!(capacity_value(0.2, 0.1, 10.0, 0.0, 1e-6, 5000.0, 0.9), 0.0);
    }

    #[test]
    fn test_capacity_value_nonneg() {
        // When C0 >> Cs, result should clamp to 0
        let w = capacity_value(0.01, 100.0, 10.0, 0.2, 1e-6, 5000.0, 0.9);
        assert!(w >= 0.0);
    }

    #[test]
    fn test_reservoir_capacity() {
        let w = reservoir_capacity(1e-6, 0.2, 1e8, 0.9);
        // W = 31.536 * 1e-6 * 1e8 * 0.2 * 0.9 = 31.536 * 100 * 0.2 * 0.9
        //   = 31.536 * 18 = 567.648
        assert!((w - 567.648).abs() < 0.01);
    }

    #[test]
    fn test_reservoir_capacity_zero_volume() {
        assert_eq!(reservoir_capacity(1e-6, 0.2, 0.0, 0.9), 0.0);
    }
}
