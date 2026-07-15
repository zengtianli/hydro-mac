use super::interpolation::level_to_volume;
use super::types::WaterBalanceRow;

/// 逐日水平衡计算 (单个河区)
///
/// 参数:
/// - dates: 日期列表
/// - total_inflow: 合计来水 (逐日)
/// - total_demand: 需水量 (逐日)
/// - eco_demand: 其他生态需水 (逐日)
/// - initial_level: 初始水位
/// - drainage_level: 排水水位
/// - levels: 库容曲线水位数组 [5]
/// - volumes: 库容曲线容积数组 [5]
/// - high_level_volume: 高水位容积
/// - low_level_volume: 低水位容积
///
/// 返回: 逐日水平衡结果
pub fn calculate_water_balance(
    dates: &[String],
    total_inflow: &[f64],
    total_demand_basic: &[f64],
    eco_demand: &[f64],
    initial_level: f64,
    drainage_level: f64,
    target_level: f64,
    levels: &[f64],
    vols: &[f64],
    high_level_volume: f64,
    low_level_volume: f64,
) -> Vec<WaterBalanceRow> {
    let n = dates.len();
    let first_day_storage = level_to_volume(levels, vols, initial_level);
    let drain_storage = level_to_volume(levels, vols, drainage_level);
    let target_storage = level_to_volume(levels, vols, target_level);

    let mut results: Vec<WaterBalanceRow> = Vec::with_capacity(n);

    for i in 0..n {
        let inflow = total_inflow[i];
        let demand = total_demand_basic[i];
        let eco = eco_demand[i];
        let total_demand = demand + eco;
        let net_flow = inflow - total_demand;

        // 日初容积
        let day_initial = if i == 0 {
            first_day_storage
        } else {
            // 取上一天的排末容积
            results[i - 1]
                .fields
                .iter()
                .find(|(k, _)| k == "排末容积")
                .map(|(_, v)| *v)
                .unwrap_or(first_day_storage)
        };

        // 日中容积
        let day_middle = day_initial + net_flow;

        // 缺水(浙东需供)
        let external_supply = f64::max(total_demand - inflow, 0.0);

        // 日末容积
        let day_end = day_middle + external_supply;

        // 排水
        let drainage = f64::max(0.0, day_end - drain_storage);
        let end_after_drain = day_end - drainage;

        // 纳蓄能力
        let storage_capacity = high_level_volume - end_after_drain;

        // 低水位以上蓄水量
        let above_low = f64::max(0.0, end_after_drain - low_level_volume);

        // 本地可供水量
        let init_storage = if day_initial > 0.0 { day_initial } else { 0.0 };
        let local_supply = inflow + init_storage;

        let fields = vec![
            ("合计来水".into(), inflow),
            ("需水量".into(), demand),
            ("其他生态需水".into(), eco),
            ("水位生态需水".into(), 0.0),
            ("生态需水".into(), eco),
            ("总需水量".into(), total_demand),
            ("净流量".into(), net_flow),
            ("目标容积".into(), target_storage),
            ("日初容积".into(), day_initial),
            ("日中容积".into(), day_middle),
            ("缺水(浙东需供)".into(), external_supply),
            ("日末容积".into(), day_end),
            ("排水容积".into(), drain_storage),
            ("河区排水".into(), drainage),
            ("排末容积".into(), end_after_drain),
            ("容积变化".into(), net_flow),
            ("排后变化".into(), net_flow - drainage),
            ("纳蓄能力".into(), storage_capacity),
            ("低水位以上蓄水量".into(), above_low),
            ("总蓄水量".into(), end_after_drain),
            ("初河蓄水".into(), 0.0),
            ("蓄水消后".into(), 0.0),
            ("末河蓄水".into(), 0.0),
            ("排河蓄水".into(), 0.0),
            ("本地可供水量".into(), local_supply),
        ];

        results.push(WaterBalanceRow {
            date: dates[i].clone(),
            fields,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_balance_basic() {
        let dates = vec!["2025/09/05".to_string(), "2025/09/06".to_string()];
        let inflow = vec![100.0, 80.0];
        let demand = vec![50.0, 60.0];
        let eco = vec![5.0, 5.0];
        let levels = vec![0.0, 3.4, 3.7, 4.0, 4.2];
        let volumes = vec![0.0, 1760.0, 2100.0, 2450.0, 2683.333];
        let initial_level = 4.2;
        let drainage_level = 4.2;
        let target_level = 4.0;

        let high_vol = 2450.0;
        let low_vol = 1760.0;

        let result = calculate_water_balance(
            &dates,
            &inflow,
            &demand,
            &eco,
            initial_level,
            drainage_level,
            target_level,
            &levels,
            &volumes,
            high_vol,
            low_vol,
        );

        assert_eq!(result.len(), 2);

        // Day 0: initial storage = level_to_volume(4.2) = 2683.333
        // net_flow = 100 - 55 = 45
        // day_middle = 2683.333 + 45 = 2728.333
        // external_supply = max(55 - 100, 0) = 0
        // day_end = 2728.333
        // drainage = max(0, 2728.333 - 2683.333) = 45.0
        // end_after_drain = 2728.333 - 45.0 = 2683.333
        let day0 = &result[0];
        let get = |name: &str| {
            day0.fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| *v)
                .unwrap_or(f64::NAN)
        };
        assert!((get("净流量") - 45.0).abs() < 0.01);
        assert!((get("缺水(浙东需供)") - 0.0).abs() < 0.01);
        assert!((get("河区排水") - 45.0).abs() < 0.1);
    }
}
