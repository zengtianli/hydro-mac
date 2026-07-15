//! capacity 示例数据 —— 复刻自 SSOT sample_data.rs(程序化生成,SSOT 无外部数据文件,故无 include_str!)。
//! 对应的可加载 xlsx 演示文件见 cli/data/sample/capacity/纳污能力示例输入.xlsx(与本文件数值同源)。
use crate::capacity::types::*;

/// 示例功能区参数 (模拟 2 个功能区, 第一个含 1 条支流)
pub fn sample_zones() -> Vec<Zone> {
    vec![
        Zone {
            zone_id: "功能区A".into(),
            name: "功能区A".into(),
            water_class: "III".into(),
            length: 10000.0,
            k: 1.2e-6,
            b: 0.9,
            a: 0.05,
            beta: 0.6,
            cs: 0.2,
            c0: 0.08,
            main_name: "A干流".into(),
            branches: vec![Branch {
                name: "A支流1".into(),
                length: 2000.0,
                join_position: 5000.0,
                c0: 0.06,
            }],
        },
        Zone {
            zone_id: "功能区B".into(),
            name: "功能区B".into(),
            water_class: "III".into(),
            length: 8000.0,
            k: 1.0e-6,
            b: 0.85,
            a: 0.04,
            beta: 0.55,
            cs: 0.15,
            c0: 0.0, // inherits from upstream
            main_name: "B干流".into(),
            branches: vec![],
        },
    ]
}

pub fn sample_flow_col_map() -> Vec<(String, FlowColumnMap)> {
    vec![
        (
            "功能区A".into(),
            FlowColumnMap {
                main: "A干流".into(),
                branches: vec!["A支流1".into()],
            },
        ),
        (
            "功能区B".into(),
            FlowColumnMap {
                main: "B干流".into(),
                branches: vec![],
            },
        ),
    ]
}

/// 生成 30 天示例逐日流量
pub fn sample_daily_flow() -> Vec<DailyRow> {
    (1..=30)
        .map(|d| {
            let base_q = 15.0 + (d as f64) * 0.3;
            DailyRow {
                date: format!("2024-06-{:02}", d),
                values: vec![
                    ("A干流".into(), base_q),
                    ("A支流1".into(), base_q * 0.25),
                    ("B干流".into(), base_q * 1.1),
                ],
            }
        })
        .collect()
}

pub fn sample_reservoir_zones() -> Vec<ReservoirZone> {
    vec![ReservoirZone {
        zone_id: "水库1".into(),
        name: "示例水库".into(),
        k: 0.8e-6,
        b: 0.85,
        cs: 0.1,
        c0: 0.0,
    }]
}

pub fn sample_daily_volume() -> Vec<DailyRow> {
    (1..=30)
        .map(|d| DailyRow {
            date: format!("2024-06-{:02}", d),
            values: vec![("水库1".into(), 5.0e7 + (d as f64) * 1.0e5)],
        })
        .collect()
}

pub fn sample_input() -> CapacityInput {
    CapacityInput {
        zones: sample_zones(),
        flow_col_map: sample_flow_col_map(),
        daily_flow: sample_daily_flow(),
        reservoir_zones: sample_reservoir_zones(),
        daily_volume: sample_daily_volume(),
    }
}
