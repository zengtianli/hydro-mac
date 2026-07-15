use crate::efficiency::types::AhpResult;

/// 随机一致性指标 RI
const RI: [f64; 11] = [0.0, 0.0, 0.0, 0.58, 0.90, 1.12, 1.24, 1.32, 1.41, 1.45, 1.49];

/// 计算 AHP 权重和一致性检验
pub fn ahp_weights(matrix: &[Vec<f64>]) -> AhpResult {
    let n = matrix.len();

    // 列求和
    let col_sums: Vec<f64> = (0..n)
        .map(|j| (0..n).map(|i| matrix[i][j]).sum())
        .collect();

    // 列归一化后行均值 = 权重
    let weights: Vec<f64> = (0..n)
        .map(|i| {
            let row_sum: f64 = (0..n).map(|j| matrix[i][j] / col_sums[j]).sum();
            row_sum / n as f64
        })
        .collect();

    if n <= 2 {
        return AhpResult {
            weights,
            cr: 0.0,
            consistent: true,
        };
    }

    // A * w
    let aw: Vec<f64> = (0..n)
        .map(|i| (0..n).map(|j| matrix[i][j] * weights[j]).sum())
        .collect();

    // λ_max = mean(Aw / w)
    let lambda_max: f64 =
        aw.iter().zip(&weights).map(|(a, w)| a / w).sum::<f64>() / n as f64;

    let ci = (lambda_max - n as f64) / (n as f64 - 1.0);
    let ri = if n < RI.len() { RI[n] } else { 1.49 };
    let cr = if ri > 0.0 { ci / ri } else { 0.0 };

    AhpResult {
        weights,
        cr,
        consistent: cr < 0.1,
    }
}

/// 组合权重 W = α * W_AHP + (1-α) * W_CRITIC
pub fn combined_weights(w_ahp: &[f64], w_critic: &[f64], alpha: f64) -> Vec<f64> {
    w_ahp
        .iter()
        .zip(w_critic)
        .map(|(a, c)| alpha * a + (1.0 - alpha) * c)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let m = vec![vec![1.0; 3]; 3];
        let result = ahp_weights(&m);
        for w in &result.weights {
            assert!((w - 1.0 / 3.0).abs() < 1e-10);
        }
        assert!(result.consistent);
        assert!(result.cr.abs() < 1e-10);
    }

    #[test]
    fn test_2x2() {
        let m = vec![vec![1.0, 2.0], vec![0.5, 1.0]];
        let result = ahp_weights(&m);
        assert!(result.consistent);
        assert_eq!(result.cr, 0.0);
    }

    #[test]
    fn test_combined() {
        let ahp = vec![0.6, 0.4];
        let critic = vec![0.3, 0.7];
        let w = combined_weights(&ahp, &critic, 0.5);
        assert!((w[0] - 0.45).abs() < 1e-10);
        assert!((w[1] - 0.55).abs() < 1e-10);
    }
}
