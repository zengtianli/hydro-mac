/// TOPSIS 评价 + 等级划分

/// 等级阈值
const GRADES: [(f64, &str, &str); 4] = [
    (80.0, "水效领跑", "#1890ff"),
    (60.0, "水效先进", "#52c41a"),
    (40.0, "水效达标", "#faad14"),
    (0.0, "水效待改进", "#f5222d"),
];

pub struct TopsisRawResult {
    pub scores: Vec<f64>,
    pub closeness: Vec<f64>,
}

/// TOPSIS 评价
///
/// data: m×n (m 个对象, n 个指标)
/// weights: 权重 (n,)
/// directions: 1.0=正向, -1.0=负向
pub fn topsis_evaluate(
    data: &[Vec<f64>],
    weights: &[f64],
    directions: &[f64],
) -> TopsisRawResult {
    let m = data.len();
    let n = if m > 0 { data[0].len() } else { 0 };

    // 向量归一化（按列 Euclidean norm）
    let col_norms: Vec<f64> = (0..n)
        .map(|j| {
            let s: f64 = (0..m).map(|i| data[i][j].powi(2)).sum();
            let norm = s.sqrt();
            if norm == 0.0 { 1.0 } else { norm }
        })
        .collect();

    let normed: Vec<Vec<f64>> = (0..m)
        .map(|i| (0..n).map(|j| data[i][j] / col_norms[j]).collect())
        .collect();

    // 加权
    let weighted: Vec<Vec<f64>> = normed
        .iter()
        .map(|row| row.iter().zip(weights).map(|(v, w)| v * w).collect())
        .collect();

    // 正理想解 / 负理想解
    let mut ideal_best = vec![0.0; n];
    let mut ideal_worst = vec![0.0; n];
    for j in 0..n {
        let col: Vec<f64> = (0..m).map(|i| weighted[i][j]).collect();
        let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if directions[j] > 0.0 {
            ideal_best[j] = max;
            ideal_worst[j] = min;
        } else {
            ideal_best[j] = min;
            ideal_worst[j] = max;
        }
    }

    // 距离
    let d_best: Vec<f64> = (0..m)
        .map(|i| {
            (0..n)
                .map(|j| (weighted[i][j] - ideal_best[j]).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect();

    let d_worst: Vec<f64> = (0..m)
        .map(|i| {
            (0..n)
                .map(|j| (weighted[i][j] - ideal_worst[j]).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect();

    // 贴近度
    let closeness: Vec<f64> = d_best
        .iter()
        .zip(&d_worst)
        .map(|(db, dw)| {
            let denom = db + dw;
            if denom == 0.0 { 1.0 } else { round4(dw / denom) }
        })
        .collect();

    let scores: Vec<f64> = closeness.iter().map(|c| round2(c * 100.0)).collect();

    TopsisRawResult { scores, closeness }
}

/// 根据得分分级
pub fn classify(score: f64) -> (&'static str, &'static str) {
    for &(threshold, name, color) in &GRADES {
        if score >= threshold {
            return (name, color);
        }
    }
    (GRADES[3].1, GRADES[3].2)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify() {
        assert_eq!(classify(85.0).0, "水效领跑");
        assert_eq!(classify(65.0).0, "水效先进");
        assert_eq!(classify(45.0).0, "水效达标");
        assert_eq!(classify(30.0).0, "水效待改进");
    }

    #[test]
    fn test_topsis_identical() {
        // 所有对象相同 → 贴近度 1.0（分母为 0 的边界情况）
        let data = vec![vec![1.0, 2.0]; 3];
        let w = vec![0.5, 0.5];
        let d = vec![1.0, 1.0];
        let result = topsis_evaluate(&data, &w, &d);
        for s in &result.scores {
            assert_eq!(*s, 100.0);
        }
    }
}
