/// CRITIC 客观赋权
///
/// directions: 1.0 = 正向(越大越好), -1.0 = 负向(越小越好)
pub fn critic_weights(data: &[Vec<f64>], directions: &[f64]) -> Vec<f64> {
    let m = data.len(); // 行数（年度）
    if m == 0 {
        return vec![];
    }
    let n = data[0].len(); // 列数（指标）

    // Min-max 标准化
    let mut normed = vec![vec![0.0; n]; m];
    for j in 0..n {
        let col: Vec<f64> = (0..m).map(|i| data[i][j]).collect();
        let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        for i in 0..m {
            if range == 0.0 {
                normed[i][j] = 1.0;
            } else if directions[j] > 0.0 {
                normed[i][j] = (col[i] - min) / range;
            } else {
                normed[i][j] = (max - col[i]) / range;
            }
        }
    }

    // 标准差 (ddof=1)
    let sigma: Vec<f64> = (0..n)
        .map(|j| {
            let col: Vec<f64> = (0..m).map(|i| normed[i][j]).collect();
            std_ddof1(&col)
        })
        .collect();

    // 相关系数矩阵
    let corr = correlation_matrix(&normed, m, n);

    // 信息量
    let info: Vec<f64> = (0..n)
        .map(|j| {
            let conflict: f64 = (0..n)
                .filter(|&k| k != j)
                .map(|k| 1.0 - corr[j][k].abs())
                .sum();
            sigma[j] * conflict
        })
        .collect();

    let total: f64 = info.iter().sum();
    if total == 0.0 {
        return vec![1.0 / n as f64; n];
    }
    info.iter().map(|v| v / total).collect()
}

/// 样本标准差 (ddof=1)
fn std_ddof1(vals: &[f64]) -> f64 {
    let n = vals.len();
    if n <= 1 {
        return 0.0;
    }
    let mean = vals.iter().sum::<f64>() / n as f64;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
    var.sqrt()
}

/// Pearson 相关系数矩阵
fn correlation_matrix(normed: &[Vec<f64>], m: usize, n: usize) -> Vec<Vec<f64>> {
    let means: Vec<f64> = (0..n)
        .map(|j| (0..m).map(|i| normed[i][j]).sum::<f64>() / m as f64)
        .collect();

    let stds: Vec<f64> = (0..n)
        .map(|j| {
            let s: f64 = (0..m).map(|i| (normed[i][j] - means[j]).powi(2)).sum();
            (s / (m as f64 - 1.0)).sqrt()
        })
        .collect();

    let mut corr = vec![vec![1.0; n]; n];
    for j1 in 0..n {
        for j2 in (j1 + 1)..n {
            if stds[j1] == 0.0 || stds[j2] == 0.0 {
                corr[j1][j2] = 1.0; // NaN → 1.0，与 Python 版一致
                corr[j2][j1] = 1.0;
            } else {
                let cov: f64 = (0..m)
                    .map(|i| (normed[i][j1] - means[j1]) * (normed[i][j2] - means[j2]))
                    .sum::<f64>()
                    / (m as f64 - 1.0);
                let r = cov / (stds[j1] * stds[j2]);
                corr[j1][j2] = r;
                corr[j2][j1] = r;
            }
        }
    }
    corr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_data() {
        // 所有值相同 → 均匀权重
        let data = vec![vec![1.0, 2.0]; 3];
        let dirs = vec![1.0, 1.0];
        let w = critic_weights(&data, &dirs);
        assert_eq!(w.len(), 2);
        assert!((w[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_std_ddof1() {
        let vals = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let s = std_ddof1(&vals);
        assert!((s - 2.1380899352993952).abs() < 1e-10);
    }
}
