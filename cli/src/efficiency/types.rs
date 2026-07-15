use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 输入数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRawRow {
    pub year: String,
    pub recycled_usage: f64,       // 再生水利用量(万m³)
    pub sewage_treated: f64,       // 污水处理量(万m³)
    pub industrial_gdp: f64,       // 工业增加值(亿元)
    pub supply: f64,               // 再生水供水量(万m³)
    pub sales: f64,                // 再生水售水量(万m³)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MesoRawRow {
    pub year: String,
    pub connected_enterprises: f64, // 接入再生水管网企业数(家)
    pub total_enterprises: f64,     // 企业总数(家)
    pub park_recycled_usage: f64,   // 园区再生水利用量(万m³)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroRawRow {
    pub enterprise: String,              // 企业名称
    pub water_intake: f64,               // 取水量(万m³)
    pub reuse_amount: f64,               // 重复利用水量(万m³)
    pub cooling_intake: f64,             // 间接冷却水取水量(万m³)
    pub cooling_circulation: f64,        // 间接冷却水循环量(万m³)
    pub process_total: f64,              // 工艺用水总量(万m³)
    pub process_reuse: f64,              // 工艺水回用量(万m³)
    pub recycled_usage: f64,             // 再生水利用量(万m³)
    pub prior_recycled_usage: Option<f64>, // 上年再生水利用量(万m³)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentInput {
    pub macro_data: Vec<MacroRawRow>,
    pub meso_data: Vec<MesoRawRow>,
    pub micro_data: HashMap<String, Vec<MicroRawRow>>, // year -> enterprises
    pub ahp_matrix: Vec<Vec<f64>>,                     // 10x10
    pub alpha: f64,                                     // 0.0 ~ 1.0
}

// ── 输出数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorRow {
    pub label: String,      // 年度 or 企业名称
    pub values: Vec<Option<f64>>, // C 值，None 表示无法计算
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpResult {
    pub weights: Vec<f64>,
    pub cr: f64,
    pub consistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopsisEntry {
    pub name: String,
    pub closeness: f64,
    pub score: f64,
    pub grade: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerScore {
    pub year: String,
    pub macro_score: f64,
    pub meso_score: f64,
    pub micro_score: f64,
    pub total_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentOutput {
    pub indicators_macro: Vec<IndicatorRow>,
    pub indicators_meso: Vec<IndicatorRow>,
    pub indicators_micro: HashMap<String, Vec<IndicatorRow>>, // year -> enterprise rows
    pub micro_aggregated: Vec<IndicatorRow>,                  // year -> C7-C10 means

    pub ahp_result: AhpResult,
    pub critic_weights: Vec<f64>,
    pub combined_weights: Vec<f64>,

    pub layer_scores: Vec<LayerScore>,
    pub topsis_results: HashMap<String, Vec<TopsisEntry>>, // year -> enterprise results

    pub indicator_labels: Vec<String>, // C1-C10 标签
    pub year_indicator_matrix: Vec<IndicatorRow>, // 完整年度×指标矩阵（用于权重计算）
}
