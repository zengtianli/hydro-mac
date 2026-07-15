/// 线性插值: 给定 x 数组和 y 数组 (已排序), 对 target_x 求 y
/// 超出范围则 clamp 到边界值
pub fn linear_interp(xs: &[f64], ys: &[f64], target: f64) -> f64 {
    assert_eq!(xs.len(), ys.len());
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 || target <= xs[0] {
        return ys[0];
    }
    if target >= xs[n - 1] {
        return ys[n - 1];
    }
    // 找到区间 [i, i+1]
    for i in 0..n - 1 {
        if target >= xs[i] && target <= xs[i + 1] {
            let dx = xs[i + 1] - xs[i];
            if dx.abs() < 1e-15 {
                return ys[i];
            }
            let t = (target - xs[i]) / dx;
            return ys[i] + t * (ys[i + 1] - ys[i]);
        }
    }
    ys[n - 1]
}

/// 水位 → 容积
pub fn level_to_volume(levels: &[f64], volumes: &[f64], level: f64) -> f64 {
    linear_interp(levels, volumes, level)
}

/// 容积 → 水位
#[allow(dead_code)]
pub fn volume_to_level(levels: &[f64], volumes: &[f64], volume: f64) -> f64 {
    linear_interp(volumes, levels, volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_interp_basic() {
        let xs = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = vec![0.0, 10.0, 30.0, 60.0, 100.0];

        assert!((linear_interp(&xs, &ys, 0.5) - 5.0).abs() < 1e-10);
        assert!((linear_interp(&xs, &ys, 1.5) - 20.0).abs() < 1e-10);
        assert!((linear_interp(&xs, &ys, 2.5) - 45.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_bounds() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![10.0, 20.0, 30.0];

        assert!((linear_interp(&xs, &ys, 0.0) - 10.0).abs() < 1e-10);
        assert!((linear_interp(&xs, &ys, 5.0) - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_level_to_volume_real_data() {
        // 黛山平原区 data
        let levels = vec![0.0, 3.4, 3.7, 4.0, 4.2];
        let volumes = vec![0.0, 1760.0, 2100.0, 2450.0, 2683.333333333333];

        let v = level_to_volume(&levels, &volumes, 4.2);
        assert!((v - 2683.333333333333).abs() < 0.01);

        let v = level_to_volume(&levels, &volumes, 3.55);
        // Between 3.4→1760 and 3.7→2100: t = (3.55-3.4)/(3.7-3.4) = 0.5
        // v = 1760 + 0.5 * 340 = 1930
        assert!((v - 1930.0).abs() < 0.01);
    }

    #[test]
    fn test_volume_to_level() {
        let levels = vec![0.0, 3.4, 3.7, 4.0, 4.2];
        let volumes = vec![0.0, 1760.0, 2100.0, 2450.0, 2683.333333333333];

        let l = volume_to_level(&levels, &volumes, 1930.0);
        assert!((l - 3.55).abs() < 0.01);
    }
}
