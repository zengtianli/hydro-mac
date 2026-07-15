use serde::{Deserialize, Serialize};

/// 支流信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub length: f64,        // 支流长度 L (m)
    pub join_position: f64, // 汇入干流的位置 (m)
    pub c0: f64,            // 支流入口浓度 (mg/L)
}

/// 河道功能区参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub zone_id: String,
    pub name: String,
    pub water_class: String,
    pub length: f64, // 河段长度 L (m)
    pub k: f64,      // 衰减系数 K (1/s)
    pub b: f64,      // 不均匀系数
    pub a: f64,      // 流速系数
    pub beta: f64,   // 流速指数
    pub cs: f64,     // 目标浓度 (mg/L)
    pub c0: f64,     // 初始浓度 (mg/L)
    pub main_name: String,
    pub branches: Vec<Branch>,
}

/// 水库功能区参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirZone {
    pub zone_id: String,
    pub name: String,
    pub k: f64,  // 衰减系数 K (1/s)
    pub b: f64,  // 不均匀系数
    pub cs: f64, // 目标浓度 (mg/L)
    pub c0: f64, // 初始浓度 (mg/L)
}

/// 分段计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentResult {
    pub name: String,
    pub seg_type: String, // "干流段" | "支流" | "混合" | "汇总"
    pub length: f64,
    pub q: f64,
    pub c0: f64,
    pub c_out: f64,
    pub w: f64,
    pub remark: String,
}

/// 流量列映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowColumnMap {
    pub main: String,
    pub branches: Vec<String>,
}

/// 逐日行: date string + column values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRow {
    pub date: String,
    pub values: Vec<(String, f64)>, // (column_name, value)
}

/// 月度记录 (year, month, {col: value})
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyRow {
    pub year: i32,
    pub month: u32,
    pub values: Vec<(String, f64)>,
}

/// 年月度平均
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneMonthlyAvgRow {
    pub zone_id: String,
    pub months: Vec<f64>,  // 12 values (1月..12月)
    pub summary: f64,      // 年合计 or 年平均
}

/// 分段过程 / 结果行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRow {
    pub zone_id: String,
    pub seg_name: String,
    pub seg_type: String,
    pub length: f64,
    pub avg_q: f64,
    pub avg_c0: f64,
    pub avg_c_out: f64,
    pub avg_w: f64,
    pub remark: String,
}

// ── 完整输入/输出 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityInput {
    pub zones: Vec<Zone>,
    pub flow_col_map: Vec<(String, FlowColumnMap)>, // zone_id -> map
    pub daily_flow: Vec<DailyRow>,
    pub reservoir_zones: Vec<ReservoirZone>,
    pub daily_volume: Vec<DailyRow>, // 水库逐日库容
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOutput {
    pub monthly_flow: Vec<MonthlyRow>,
    pub monthly_velocity: Vec<MonthlyRow>,
    pub monthly_capacity: Vec<MonthlyRow>,
    pub zone_avg_velocity: Vec<ZoneMonthlyAvgRow>,
    pub zone_avg_capacity: Vec<ZoneMonthlyAvgRow>,
    pub process_table: Vec<ProcessRow>,
    pub result_table: Vec<ProcessRow>,
    // Reservoir (optional)
    pub reservoir_monthly_volume: Vec<MonthlyRow>,
    pub reservoir_zone_avg_capacity: Vec<ZoneMonthlyAvgRow>,
}
