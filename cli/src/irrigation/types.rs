use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// 灌溉区域配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrigationZone {
    pub name: String,
    pub single_rice_area: f64,  // 单季稻面积 (km²)
    pub double_rice_area: f64,  // 双季稻面积 (km²)
    pub dryland_area: f64,      // 旱地面积 (km²)
    pub misc_area: f64,         // 杂地面积 (km²)
    pub water_surface_area: f64, // 水面面积 (km²)
    pub plain_area: f64,        // 平原面积 (km²)
    pub leakage_rate: f64,      // 水田渗漏率 (mm/d)
    pub dryland_leakage: f64,   // 旱地渗漏率
    pub flowering_ratio: f64,   // 开花期灌溉比例
    pub rotation_batches: usize, // 轮灌批次
}

/// 生长阶段参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthStage {
    pub start: String,    // 开始日期 (MM/DD 或 YYYY/MM/DD)
    pub end: String,      // 结束日期
    pub days: u32,        // 天数
    pub eva_ratio: f64,   // 蒸发系数
    pub h_min: f64,       // 最小水位 (mm)
    pub storage: f64,     // 设计蓄水位 (mm)
    pub h_max: f64,       // 最大水位 (mm)
}

/// 旱地作物参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crop {
    pub name: String,
    pub water_75: f64, // 75% 水文年需水量 (mm/d)
    pub water_90: f64, // 90% 水文年需水量 (mm/d)
}

impl Crop {
    /// 获取指定水文年的单位面积需水量
    pub fn get_daily_water(&self, hydro_year: u32) -> f64 {
        match hydro_year {
            75 => self.water_75,
            _ => self.water_90, // 默认 90%
        }
    }

    /// 计算需水量: mm/d * km² * 0.1 -> m³/d
    pub fn calculate_water_volume(&self, area_km2: f64, hydro_year: u32) -> f64 {
        self.get_daily_water(hydro_year) * area_km2 * 0.1
    }
}

/// 时间配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    pub forecast_date: String, // YYYY/MM/DD
    pub forecast_days: u32,
}

/// 气象数据 (一天, 一个区)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherRecord {
    pub date: String,
    pub rainfall: f64,
    pub evaporation: f64,
}

/// 旱地作物种植面积 (一个区)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropAreaEntry {
    pub zone_name: String,
    pub hydro_year: u32,
    pub crop_areas: Vec<(String, f64)>, // (作物名, 面积 km²)
}

/// 完整的灌溉计算输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrigationInput {
    pub time_config: TimeConfig,
    pub zones: Vec<IrrigationZone>,
    pub single_crop_stages: Vec<GrowthStage>,
    pub double_crop_stages: Vec<GrowthStage>,
    pub crops: Vec<Crop>,
    pub weather: Vec<ZoneWeather>,
    pub crop_areas: Vec<CropAreaEntry>,
}

/// 气象数据 (per zone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneWeather {
    pub zone_name: String,
    pub records: Vec<WeatherRecord>,
}

/// 单日水田水量平衡结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddyDayResult {
    pub end_h: f64,
    pub irrigation: f64,
    pub drainage: f64,
    pub leakage: f64,
    pub actual_evaporation: f64,
}

/// 单日单区计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyZoneResult {
    pub date: String,
    pub zone_name: String,
    pub paddy_irrigation: f64,     // 万m³
    pub paddy_drainage: f64,       // 万m³
    pub dryland_irrigation: f64,   // 万m³
    pub dryland_drainage: f64,     // 万m³
    pub flowering_irrigation: f64, // 万m³
    pub lowland_drainage: f64,     // 万m³
    pub water_surface_drainage: f64, // 万m³
}

/// 灌溉计算输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrigationOutput {
    pub daily_results: Vec<DailyZoneResult>,
    pub zone_names: Vec<String>,
    pub dates: Vec<String>,
    pub warnings: Vec<String>,
}

/// 计算模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalcMode {
    #[serde(rename = "crop")]
    Crop,       // 旱地作物
    #[serde(rename = "irrigation")]
    Irrigation, // 水稻灌溉
    #[serde(rename = "both")]
    Both,       // 综合
}

/// 每天的灌溉参数 (从 growth stages 展开)
#[derive(Debug, Clone)]
pub struct DailyParams {
    pub eva_ratio: f64,
    pub h_min: f64,
    pub storage: f64,
    pub h_max: f64,
}

/// 低洼地水位追踪
#[derive(Debug, Clone)]
pub struct LowlandState {
    pub water_level: f64,
}

impl LowlandState {
    pub fn new() -> Self {
        Self { water_level: 0.0 }
    }
}

/// 开花期配置
pub struct FloweringPeriod {
    pub start_mmdd: (u32, u32), // (month, day)
    pub end_mmdd: (u32, u32),
}

/// 默认开花期
pub fn default_flowering_periods() -> (FloweringPeriod, FloweringPeriod) {
    (
        // 单季稻: 6/26 - 10/22
        FloweringPeriod {
            start_mmdd: (6, 26),
            end_mmdd: (10, 22),
        },
        // 双季稻: 4/18 - 10/22
        FloweringPeriod {
            start_mmdd: (4, 18),
            end_mmdd: (10, 22),
        },
    )
}

/// 检查日期是否在开花期内
pub fn is_in_flowering_period(date: NaiveDate, period: &FloweringPeriod) -> bool {
    let (sm, sd) = period.start_mmdd;
    let (em, ed) = period.end_mmdd;
    let year = date.year();

    if let (Some(start), Some(end)) = (
        NaiveDate::from_ymd_opt(year, sm, sd),
        NaiveDate::from_ymd_opt(year, em, ed),
    ) {
        date >= start && date < end
    } else {
        false
    }
}
