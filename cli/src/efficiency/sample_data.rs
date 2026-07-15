//! 复刻自 上游 hydro 项目(私有)efficiency 的 sample_data.rs(仅 use 路径改写)。
//! efficiency 的示例数据在 SSOT 里就是硬编码 Rust(无外部数据文件),故无 include_str!;
//! cli/data/sample/efficiency/ 下另附同数据的 xlsx(供「打开数据文件」链路与 golden 用例)。
use crate::efficiency::types::*;
use std::collections::HashMap;

pub fn sample_macro() -> Vec<MacroRawRow> {
    vec![
        MacroRawRow { year: "2023年".into(), recycled_usage: 220.0, sewage_treated: 1100.0, industrial_gdp: 78.0, supply: 235.0, sales: 220.0 },
        MacroRawRow { year: "2024年".into(), recycled_usage: 280.0, sewage_treated: 1200.0, industrial_gdp: 85.0, supply: 295.0, sales: 280.0 },
        MacroRawRow { year: "2025年".into(), recycled_usage: 350.0, sewage_treated: 1280.0, industrial_gdp: 92.0, supply: 365.0, sales: 350.0 },
        MacroRawRow { year: "2026年".into(), recycled_usage: 410.0, sewage_treated: 1350.0, industrial_gdp: 98.0, supply: 425.0, sales: 410.0 },
    ]
}

pub fn sample_meso() -> Vec<MesoRawRow> {
    vec![
        MesoRawRow { year: "2023年".into(), connected_enterprises: 20.0, total_enterprises: 62.0, park_recycled_usage: 140.0 },
        MesoRawRow { year: "2024年".into(), connected_enterprises: 28.0, total_enterprises: 65.0, park_recycled_usage: 180.0 },
        MesoRawRow { year: "2025年".into(), connected_enterprises: 38.0, total_enterprises: 68.0, park_recycled_usage: 230.0 },
        MesoRawRow { year: "2026年".into(), connected_enterprises: 45.0, total_enterprises: 70.0, park_recycled_usage: 280.0 },
    ]
}

fn micro_2024() -> Vec<MicroRawRow> {
    vec![
        MicroRawRow { enterprise: "示例企业甲".into(), water_intake: 42.0, reuse_amount: 150.0, cooling_intake: 7.5, cooling_circulation: 60.0, process_total: 22.0, process_reuse: 12.0, recycled_usage: 12.0, prior_recycled_usage: Some(8.0) },
        MicroRawRow { enterprise: "示例企业乙".into(), water_intake: 35.0, reuse_amount: 120.0, cooling_intake: 5.5, cooling_circulation: 38.0, process_total: 18.0, process_reuse: 8.0, recycled_usage: 10.0, prior_recycled_usage: Some(7.0) },
        MicroRawRow { enterprise: "示例企业丙".into(), water_intake: 11.0, reuse_amount: 28.0, cooling_intake: 1.8, cooling_circulation: 10.0, process_total: 5.0, process_reuse: 1.8, recycled_usage: 3.0, prior_recycled_usage: Some(2.0) },
        MicroRawRow { enterprise: "示例企业丁".into(), water_intake: 25.0, reuse_amount: 65.0, cooling_intake: 4.5, cooling_circulation: 27.0, process_total: 13.0, process_reuse: 5.0, recycled_usage: 7.0, prior_recycled_usage: Some(5.0) },
        MicroRawRow { enterprise: "示例企业戊".into(), water_intake: 14.0, reuse_amount: 42.0, cooling_intake: 2.8, cooling_circulation: 19.0, process_total: 7.0, process_reuse: 3.0, recycled_usage: 4.0, prior_recycled_usage: Some(2.5) },
    ]
}

fn micro_2025() -> Vec<MicroRawRow> {
    vec![
        MicroRawRow { enterprise: "示例企业甲".into(), water_intake: 45.0, reuse_amount: 180.0, cooling_intake: 8.0, cooling_circulation: 72.0, process_total: 25.0, process_reuse: 15.0, recycled_usage: 18.0, prior_recycled_usage: Some(12.0) },
        MicroRawRow { enterprise: "示例企业乙".into(), water_intake: 38.0, reuse_amount: 152.0, cooling_intake: 6.0, cooling_circulation: 48.0, process_total: 20.0, process_reuse: 11.0, recycled_usage: 15.0, prior_recycled_usage: Some(10.0) },
        MicroRawRow { enterprise: "示例企业丙".into(), water_intake: 12.0, reuse_amount: 36.0, cooling_intake: 2.0, cooling_circulation: 14.0, process_total: 6.0, process_reuse: 2.5, recycled_usage: 5.0, prior_recycled_usage: Some(3.0) },
        MicroRawRow { enterprise: "示例企业丁".into(), water_intake: 28.0, reuse_amount: 84.0, cooling_intake: 5.0, cooling_circulation: 35.0, process_total: 15.0, process_reuse: 7.0, recycled_usage: 10.0, prior_recycled_usage: Some(7.0) },
        MicroRawRow { enterprise: "示例企业戊".into(), water_intake: 15.0, reuse_amount: 52.5, cooling_intake: 3.0, cooling_circulation: 24.0, process_total: 8.0, process_reuse: 4.0, recycled_usage: 6.0, prior_recycled_usage: Some(4.0) },
    ]
}

fn micro_2026() -> Vec<MicroRawRow> {
    vec![
        MicroRawRow { enterprise: "示例企业甲".into(), water_intake: 48.0, reuse_amount: 210.0, cooling_intake: 8.5, cooling_circulation: 85.0, process_total: 28.0, process_reuse: 18.0, recycled_usage: 22.0, prior_recycled_usage: Some(18.0) },
        MicroRawRow { enterprise: "示例企业乙".into(), water_intake: 40.0, reuse_amount: 176.0, cooling_intake: 6.5, cooling_circulation: 58.0, process_total: 22.0, process_reuse: 14.0, recycled_usage: 18.0, prior_recycled_usage: Some(15.0) },
        MicroRawRow { enterprise: "示例企业丙".into(), water_intake: 13.0, reuse_amount: 45.0, cooling_intake: 2.2, cooling_circulation: 18.0, process_total: 7.0, process_reuse: 3.5, recycled_usage: 7.0, prior_recycled_usage: Some(5.0) },
        MicroRawRow { enterprise: "示例企业丁".into(), water_intake: 30.0, reuse_amount: 102.0, cooling_intake: 5.5, cooling_circulation: 44.0, process_total: 17.0, process_reuse: 9.0, recycled_usage: 13.0, prior_recycled_usage: Some(10.0) },
        MicroRawRow { enterprise: "示例企业戊".into(), water_intake: 16.0, reuse_amount: 64.0, cooling_intake: 3.2, cooling_circulation: 29.0, process_total: 9.0, process_reuse: 5.5, recycled_usage: 8.0, prior_recycled_usage: Some(6.0) },
    ]
}

pub fn sample_micro() -> HashMap<String, Vec<MicroRawRow>> {
    let mut m = HashMap::new();
    m.insert("2024年".into(), micro_2024());
    m.insert("2025年".into(), micro_2025());
    m.insert("2026年".into(), micro_2026());
    m
}

#[rustfmt::skip]
pub fn default_ahp_matrix() -> Vec<Vec<f64>> {
    vec![
        vec![  1.0,   2.0,   2.0,   3.0,   3.0,   3.0,   4.0,   4.0,   4.0,   5.0],
        vec![1.0/2.0, 1.0,   1.0,   2.0,   2.0,   2.0,   3.0,   3.0,   3.0,   4.0],
        vec![1.0/2.0, 1.0,   1.0,   2.0,   2.0,   2.0,   3.0,   3.0,   3.0,   4.0],
        vec![1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0,   1.0,   2.0,   2.0,   2.0,   3.0],
        vec![1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0,   1.0,   2.0,   2.0,   2.0,   3.0],
        vec![1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0,   1.0,   2.0,   2.0,   2.0,   3.0],
        vec![1.0/4.0, 1.0/3.0, 1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0, 1.0, 2.0],
        vec![1.0/4.0, 1.0/3.0, 1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0, 1.0, 2.0],
        vec![1.0/4.0, 1.0/3.0, 1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0, 1.0, 1.0, 2.0],
        vec![1.0/5.0, 1.0/4.0, 1.0/4.0, 1.0/3.0, 1.0/3.0, 1.0/3.0, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0],
    ]
}

pub fn indicator_labels() -> Vec<String> {
    vec![
        "C1-再生水利用率(%)".into(),
        "C2-万元工业增加值再生水利用量(m³/万元)".into(),
        "C3-再生水管网漏损率(%)".into(),
        "C4-再生水利用量增长率(%)".into(),
        "C5-企业再生水管网覆盖率(%)".into(),
        "C6-再生水利用量增长率(%)".into(),
        "C7-工业用水重复利用率(%)".into(),
        "C8-间接冷却水循环利用率(%)".into(),
        "C9-工艺水回用率(%)".into(),
        "C10-再生水利用量增长率(%)".into(),
    ]
}

/// C1-C10 的方向: 1=正向(越大越好), -1=负向(越小越好)
/// C3(漏损率) 是负向，其余均为正向
pub fn indicator_directions() -> Vec<f64> {
    vec![1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
}

pub fn sample_input() -> AssessmentInput {
    AssessmentInput {
        macro_data: sample_macro(),
        meso_data: sample_meso(),
        micro_data: sample_micro(),
        ahp_matrix: default_ahp_matrix(),
        alpha: 0.5,
    }
}
