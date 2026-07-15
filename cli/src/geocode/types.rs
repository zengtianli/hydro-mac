use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 功能类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionType {
    Reverse,
    Forward,
    CompanySearch,
}

/// 输入行: 灵活列名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRow {
    pub index: usize,
    pub fields: HashMap<String, String>,
}

/// 批量输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeInput {
    pub rows: Vec<InputRow>,
    pub function_type: FunctionType,
    pub coordinate_system: String, // "WGS-84" | "GCJ-02"
    pub api_key: String,
}

/// 单条结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResult {
    pub index: usize,
    pub address: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub adcode: String,
    pub lng: Option<f64>,
    pub lat: Option<f64>,
    pub error: Option<String>,
}

/// 批量输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOutput {
    pub results: Vec<GeocodeResult>,
    pub success_count: usize,
    pub total: usize,
}

/// 进度事件 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub message: String,
}
