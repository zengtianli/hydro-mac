use serde::{Deserialize, Serialize};

/// Time step option
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeStep {
    #[serde(rename = "日")]
    Daily,
    #[serde(rename = "旬")]
    TenDay,
    #[serde(rename = "月")]
    Monthly,
}

/// Z-V curve point (water level vs volume)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZvPoint {
    pub water_level: f64,
    pub volume: f64,
}

/// Q-Z curve point (tailwater flow vs level)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QzPoint {
    pub q_down: f64,
    pub water_level: f64,
}

/// Dispatch line: monthly control volumes and power targets
/// Each month has pairs of (volume, power) defining zones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchLine {
    /// "MM-DD" -> (volumes[], powers[]) — monthly dispatch points
    pub monthly_points: Vec<DispatchMonthEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchMonthEntry {
    pub mmdd: String,
    pub volumes: Vec<f64>,
    pub powers: Vec<f64>,
}

/// Water user demand time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDemand {
    pub name: String,
    pub from_downstream: bool, // true = dam-foot withdrawal, false = in-reservoir
    pub values: Vec<f64>,
}

/// Daily lookup table for a variable (normal year / leap year)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyLookup {
    /// 366 entries for "01-01" through "12-31" (including "02-29")
    /// Each entry: (mmdd, normal_year_value, leap_year_value)
    pub entries: Vec<(String, f64, f64)>,
}

/// Reservoir physical parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservoir {
    pub name: String,

    // Water levels
    pub h_dead: f64,
    pub h_normal: f64,
    pub h_wet_season_limit: f64,
    pub h_typhoon_limit: f64,

    // Characteristic volumes (derived from ZV curve)
    pub v_dead: f64,
    pub v_normal: f64,
    pub v_wet_season: f64,
    pub v_typhoon: f64,

    // Z-V and Q-Z curves
    pub zv_curve: Vec<ZvPoint>,
    pub zq_curve: Vec<QzPoint>, // empty if const_z_down

    // Hydropower params
    pub k0: f64,  // power coefficient
    pub k1: f64,  // head loss coefficient
    pub k2: f64,  // non-uniformity coefficient
    pub qm: f64,  // max turbine flow
    pub wpv: f64, // installed capacity (kW)
    pub dh_max: f64,
    pub dh_min: f64,

    // Loss
    pub loss_type: i32, // 0 = ratio (per mille), 1 = fixed flow
    pub loss_ratio: f64,
    pub loss_value: f64,

    // Tailwater
    pub const_z_down: bool,
    pub z_down_lookup: Option<DailyLookup>, // if const_z_down

    // Flood control periods
    pub wet_season_start: String, // "MM-DD"
    pub wet_season_end: String,
    pub typhoon_start: String,
    pub typhoon_end: String,

    // Power generation limits
    pub h_dead_power: f64,
    pub h_limited_power: f64,
    pub v_dead_power: f64,
    pub v_limited_power: f64,

    // Daily lookup tables
    pub v_flood_day: DailyLookup,
    pub v_dead_power_day: DailyLookup,
    pub v_limited_power_day: DailyLookup,
    pub v_limited_supply_day: DailyLookup,

    // Dispatch lines
    pub dispatch_line: DispatchLine,

    // Time series
    pub dates: Vec<String>,   // ISO date strings
    pub q_inflow: Vec<f64>,   // inflow
    pub q_upstream: Vec<f64>, // from upstream reservoir outflow
    pub q_eco: Vec<f64>,      // ecological flow demand

    // Supply users
    pub supply_from_reservoir: bool,
    pub user_demand_reservoir: Vec<UserDemand>, // in-reservoir users
    pub supply_from_downstream: bool,
    pub user_demand_downstream: Vec<UserDemand>, // dam-foot users
    pub supply_order: Vec<String>,               // priority order of user names

    // Reservoir type
    pub cal_mode: String, // "年调节" or "日调节"
}

/// Global calculation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcParams {
    pub time_step: TimeStep,
    pub up_name: String,
    pub down_name: String,
    pub hydro_year_start: u32, // start month of hydrological year (default 4)
    pub hydro_year_end: u32,
    pub epsilon_v: f64,        // volume convergence threshold
    pub epsilon_w: f64,        // power convergence threshold
    pub max_iterations: usize, // convergence iterations

    // Cascade-specific
    pub up_v_special: Vec<f64>,  // upstream special volumes for supplemental release
    pub down_v_special: Vec<f64>,
    pub need_add_users: Vec<String>,  // users to supplement from upstream
    pub user_special: Vec<(String, f64)>,  // users with guaranteed minimum supply
    pub user_stop_supply: Vec<String>,  // users to stop when upstream low
    pub up_eco_as_inflow: bool,  // whether upstream eco flow counts as downstream inflow
}

/// Complete input to the calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirInput {
    pub params: CalcParams,
    pub upstream: Reservoir,
    pub downstream: Reservoir,
}

/// Single time step result for one reservoir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyResult {
    pub date: String,
    pub q_in: f64,
    pub q_gen: f64,
    pub q_thrown: f64,
    pub q_loss: f64,
    pub v_end: f64,
    pub z_end: f64,
    pub z_up: f64,
    pub z_down: f64,
    pub dh: f64,
    pub h_net: f64,
    pub power: f64,
    pub days: f64,
    // Supply info per user
    pub supply_detail: Vec<UserSupplyDetail>,
    pub eco_supply: f64,
    pub eco_lack: f64,
    pub eco_lack_day: f64,
    // After supplement
    pub q_gen_after: f64,
    pub q_thrown_after: f64,
    pub v_end_after: f64,
    pub z_end_after: f64,
    pub z_up_after: f64,
    pub z_down_after: f64,
    pub dh_after: f64,
    pub h_net_after: f64,
    pub power_after: f64,
    pub supply_detail_after: Vec<UserSupplyDetail>,
    pub supplement_q1: f64, // supplement to downstream volume
    pub supplement_q2: f64, // supplement to downstream user deficit
    pub supplement_q3: f64, // special user supplement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSupplyDetail {
    pub name: String,
    pub demand: f64,
    pub supply: f64,
    pub lack: f64,
    pub lack_day: f64,
}

/// Monthly aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyResult {
    pub year_month: String,
    pub values: Vec<(String, f64)>, // (column_name, value)
}

/// Yearly aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyResult {
    pub year: String,
    pub values: Vec<(String, f64)>,
}

/// One reservoir's full output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirOutput {
    pub name: String,
    pub daily: Vec<DailyResult>,
    pub monthly: Vec<MonthlyResult>,
    pub yearly: Vec<YearlyResult>,
    pub hydro_yearly: Vec<YearlyResult>,
    pub summary: Vec<(String, f64)>,
}

/// Complete output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleOutput {
    pub upstream: ReservoirOutput,
    pub downstream: ReservoirOutput,
}
