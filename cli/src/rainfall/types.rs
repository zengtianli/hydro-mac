use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A partition (分区) containing a subset of lakes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub number: u32,          // 01..15
    pub code: String,         // e.g. "QZPYQ"
    pub display_name: String, // e.g. "青洲平原区"
    pub full_label: String,   // e.g. "QZPYQ(青洲)"
    pub lake_ids: Vec<String>,
    pub areas: Vec<f64>, // m² per lake (parallel to lake_ids)
}

/// Summary info for one lake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LakeInfo {
    pub id: String,
    pub area_m2: f64,
    pub partition_code: String,
}

/// Summary info for one partition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSummary {
    pub code: String,
    pub display_name: String,
    pub total_area_m2: f64,
    pub lake_count: usize,
}

/// One row of daily rainfall for all partitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRainfall {
    pub date: String,                     // "2025/05/14"
    pub values: HashMap<String, f64>,     // partition_display_name -> daily value
}

/// One row of hourly baseline flow (228 columns)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyRow {
    pub datetime: String,     // "2025/05/14 00:00"
    pub values: Vec<f64>,     // parallel to column_names
}

/// Water user intake record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntake {
    pub user_name: String,
    pub date: String,         // "2025/05/14"
    pub daily_intake: f64,
    pub hourly_intake: f64,
}

/// Complete pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInput {
    pub partitions: Vec<Partition>,
    pub rainfall: Vec<DailyRainfall>,
    pub baseline_columns: Vec<String>,    // G1..G228 column names from GHJYL header
    pub baseline: Vec<HourlyRow>,         // hourly baseline flow rows
    pub users: Vec<UserIntake>,
    pub user_lake_map: HashMap<String, String>, // user_name -> lake_id
}

/// Progress update sent step by step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step: u32,
    pub name: String,
    pub status: String, // "ok" | "warn" | "error"
    pub message: String,
}

/// Complete pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    pub steps: Vec<StepResult>,
    pub lake_summary: Vec<LakeInfo>,
    pub partition_summary: Vec<PartitionSummary>,
    pub final_columns: Vec<String>,       // G1..G228
    pub final_rows: Vec<HourlyRow>,       // result rows
    pub row_count: usize,
    pub col_count: usize,
}
