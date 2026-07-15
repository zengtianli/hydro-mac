use serde::{Deserialize, Serialize};

/// 查询输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInput {
    pub cities: Vec<String>,
    pub years: Vec<i32>,
    pub table: String,
    pub indicators: Vec<String>,
}

/// 查询输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: usize,
    pub year_range: String,
    pub city_count: usize,
}

/// 统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResult {
    pub total: f64,
    pub average: f64,
    pub max: f64,
    pub min: f64,
    pub count: usize,
}

/// 年度对比
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearComparison {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// 单行数据 (CSV 解析后)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRow {
    pub year: i32,
    pub city: String,
    pub table_name: String,
    pub district: String,
    pub values: std::collections::HashMap<String, serde_json::Value>,
}

/// 文件索引条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub year: i32,
    pub city: String,
    pub table_name: String,
}

/// 数据索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataIndex {
    pub files: Vec<FileEntry>,
    pub available_years: Vec<i32>,
    pub available_cities: Vec<String>,
    pub available_tables: Vec<String>,
    pub total_files: usize,
}

/// 聚合参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateParams {
    pub cities: Vec<String>,
    pub years: Vec<i32>,
    pub table: String,
    pub indicators: Vec<String>,
    pub mode: String, // "city" or "year"
}

/// 对比参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareParams {
    pub cities: Vec<String>,
    pub table: String,
    pub indicators: Vec<String>,
    pub year1: i32,
    pub year2: i32,
}
