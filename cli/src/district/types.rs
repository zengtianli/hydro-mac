use serde::{Deserialize, Serialize};

/// 库容曲线 - 5个水位-容积点
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCurve {
    pub district: String,
    pub levels: Vec<f64>,  // [死水位, 低水位, 中水位, 高水位, 超蓄水位]
    pub volumes: Vec<f64>, // [死库容, 低库容, 中库容, 高库容, 超蓄库容]
}

/// 河区-水库映射
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirMapping {
    pub district: String,
    pub count: usize,
    pub reservoirs: Vec<String>,
}

/// TSV 表: 表头 + 行数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// 河区逐日数据行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRow {
    pub date: String,
    pub values: Vec<(String, f64)>,
}

/// 水平衡逐日结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterBalanceRow {
    pub date: String,
    /// 各字段名 → 值
    pub fields: Vec<(String, f64)>,
}

/// 单个河区的完整数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistrictData {
    pub name: String,
    pub code: String,
    pub inflow: Vec<DailyRow>,
    pub demand: Vec<DailyRow>,
    pub balance: Vec<WaterBalanceRow>,
}

/// 调度器输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerInput {
    /// 13 个 TSV 文件: key -> TsvTable
    pub files: Vec<(String, TsvTable)>,
}

/// 调度器输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerOutput {
    pub districts: Vec<DistrictData>,
    pub summary: Vec<WaterBalanceRow>,
    pub districts_processed: usize,
    pub total_water_demand: f64,
    pub total_water_supply: f64,
    pub total_shortage: f64,
}
