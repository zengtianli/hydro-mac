//! 复刻自 上游 hydro 项目(私有)reservoir 的 sample_data.rs(仅 use 路径改写)。
//! 注意:reservoir 的示例数据是**程序化生成**(正弦来水 366 天,SSOT 无 CSV 样例文件),
//! 故无 include_str!;参考 JSON 快照见 cli/data/sample/reservoir/sample_input.json(由本函数 dump)。
use crate::reservoir::interp::create_daily_lookup;
use crate::reservoir::types::*;

/// Generate minimal sample data for testing the UI without an Excel file.
pub fn sample_input() -> ReservoirInput {
    // Create simple Z-V curve
    let zv_up = vec![
        ZvPoint { water_level: 160.0, volume: 10000.0 },
        ZvPoint { water_level: 180.0, volume: 30000.0 },
        ZvPoint { water_level: 190.0, volume: 45000.0 },
        ZvPoint { water_level: 196.0, volume: 55919.0 },
        ZvPoint { water_level: 198.0, volume: 59994.0 },
        ZvPoint { water_level: 205.0, volume: 76000.0 },
        ZvPoint { water_level: 210.0, volume: 90000.0 },
    ];

    let zv_down = vec![
        ZvPoint { water_level: 90.0, volume: 1000.0 },
        ZvPoint { water_level: 100.0, volume: 3000.0 },
        ZvPoint { water_level: 105.0, volume: 5000.0 },
        ZvPoint { water_level: 108.0, volume: 7040.0 },
        ZvPoint { water_level: 111.73, volume: 9500.0 },
        ZvPoint { water_level: 115.0, volume: 12000.0 },
    ];

    let qz_down = vec![
        QzPoint { q_down: 0.0, water_level: 82.0 },
        QzPoint { q_down: 100.0, water_level: 84.0 },
        QzPoint { q_down: 300.0, water_level: 87.0 },
        QzPoint { q_down: 500.0, water_level: 90.0 },
    ];

    // Simple dispatch line (1 zone per month)
    let dispatch_months: Vec<DispatchMonthEntry> = (1..=12)
        .map(|m| DispatchMonthEntry {
            mmdd: format!("{:02}-01", m),
            volumes: vec![50000.0],
            powers: vec![10000.0],
        })
        .collect();

    let dispatch_months_down: Vec<DispatchMonthEntry> = (1..=12)
        .map(|m| DispatchMonthEntry {
            mmdd: format!("{:02}-01", m),
            volumes: vec![6000.0],
            powers: vec![5000.0],
        })
        .collect();

    // Generate 365 days of sample data (2020-01-01 to 2020-12-31)
    let mut dates = Vec::new();
    let mut q_inflow_up = Vec::new();
    let mut q_eco_up = Vec::new();
    let mut q_inflow_down = Vec::new();
    let mut q_eco_down = Vec::new();
    let mut user_demand_vals = Vec::new();

    let base = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    for day in 0..366 {
        let d = base + chrono::Duration::days(day);
        dates.push(d.format("%Y-%m-%d").to_string());

        // Sinusoidal inflow pattern
        let t = day as f64 / 365.0 * std::f64::consts::TAU;
        let inflow = 80.0 + 60.0 * (t - 1.0).sin(); // peak in summer
        q_inflow_up.push(inflow.max(20.0));
        q_eco_up.push(5.0);
        q_inflow_down.push(inflow.max(10.0) * 0.3);
        q_eco_down.push(3.0);
        user_demand_vals.push(8.0 + 4.0 * (t + 0.5).sin()); // seasonal demand
    }

    let v_flood_up = create_daily_lookup(
        &[
            ("06-01".to_string(), 76000.0),
            ("07-16".to_string(), 70000.0),
            ("10-16".to_string(), 90000.0),
        ],
        false,
    );
    let v_flood_down = create_daily_lookup(
        &[
            ("06-01".to_string(), 9500.0),
            ("07-16".to_string(), 8000.0),
            ("10-16".to_string(), 12000.0),
        ],
        false,
    );
    let v_dead_power_up = create_daily_lookup(
        &[("01-01".to_string(), 10000.0), ("12-31".to_string(), 10000.0)],
        false,
    );
    let v_dead_power_down = create_daily_lookup(
        &[("01-01".to_string(), 1000.0), ("12-31".to_string(), 1000.0)],
        false,
    );
    let v_limited_power_up = create_daily_lookup(
        &[("01-01".to_string(), 10000.0), ("12-31".to_string(), 10000.0)],
        false,
    );
    let v_limited_power_down = create_daily_lookup(
        &[("01-01".to_string(), 1000.0), ("12-31".to_string(), 1000.0)],
        false,
    );
    let v_limited_supply_up = create_daily_lookup(
        &[("01-01".to_string(), 10000.0), ("12-31".to_string(), 10000.0)],
        false,
    );
    let v_limited_supply_down = create_daily_lookup(
        &[("01-01".to_string(), 1000.0), ("12-31".to_string(), 1000.0)],
        false,
    );

    let upstream = Reservoir {
        name: "上游水库".to_string(),
        h_dead: 160.0,
        h_normal: 205.0,
        h_wet_season_limit: 198.0,
        h_typhoon_limit: 196.0,
        v_dead: 10000.0,
        v_normal: 76000.0,
        v_wet_season: 59994.0,
        v_typhoon: 55919.0,
        zv_curve: zv_up,
        zq_curve: vec![],
        k0: 8.5,
        k1: 0.000015,
        k2: 1.0,
        qm: 150.0,
        wpv: 80000.0,
        dh_max: 5.0,
        dh_min: 0.3,
        loss_type: 0,
        loss_ratio: 0.001,
        loss_value: 0.0,
        const_z_down: true,
        z_down_lookup: Some(create_daily_lookup(
            &[("01-01".to_string(), 105.0), ("12-31".to_string(), 105.0)],
            false,
        )),
        wet_season_start: "06-01".to_string(),
        wet_season_end: "07-15".to_string(),
        typhoon_start: "07-16".to_string(),
        typhoon_end: "10-15".to_string(),
        h_dead_power: 160.0,
        h_limited_power: 160.0,
        v_dead_power: 10000.0,
        v_limited_power: 10000.0,
        v_flood_day: v_flood_up,
        v_dead_power_day: v_dead_power_up,
        v_limited_power_day: v_limited_power_up,
        v_limited_supply_day: v_limited_supply_up,
        dispatch_line: DispatchLine { monthly_points: dispatch_months },
        dates: dates.clone(),
        q_inflow: q_inflow_up,
        q_upstream: vec![0.0; 366],
        q_eco: q_eco_up,
        supply_from_reservoir: false,
        user_demand_reservoir: vec![],
        supply_from_downstream: true,
        user_demand_downstream: vec![UserDemand {
            name: "示例用户".to_string(),
            from_downstream: true,
            values: user_demand_vals.clone(),
        }],
        supply_order: vec!["示例用户".to_string()],
        cal_mode: "年调节".to_string(),
    };

    let downstream = Reservoir {
        name: "下游水库".to_string(),
        h_dead: 90.0,
        h_normal: 111.73,
        h_wet_season_limit: 108.0,
        h_typhoon_limit: 105.0,
        v_dead: 1000.0,
        v_normal: 9500.0,
        v_wet_season: 7040.0,
        v_typhoon: 5000.0,
        zv_curve: zv_down,
        zq_curve: qz_down,
        k0: 8.5,
        k1: 0.00002,
        k2: 1.0,
        qm: 200.0,
        wpv: 60000.0,
        dh_max: 3.0,
        dh_min: 0.2,
        loss_type: 0,
        loss_ratio: 0.001,
        loss_value: 0.0,
        const_z_down: false,
        z_down_lookup: None,
        wet_season_start: "06-01".to_string(),
        wet_season_end: "07-15".to_string(),
        typhoon_start: "07-16".to_string(),
        typhoon_end: "10-15".to_string(),
        h_dead_power: 90.0,
        h_limited_power: 90.0,
        v_dead_power: 1000.0,
        v_limited_power: 1000.0,
        v_flood_day: v_flood_down,
        v_dead_power_day: v_dead_power_down,
        v_limited_power_day: v_limited_power_down,
        v_limited_supply_day: v_limited_supply_down,
        dispatch_line: DispatchLine { monthly_points: dispatch_months_down },
        dates: dates.clone(),
        q_inflow: q_inflow_down,
        q_upstream: vec![0.0; 366],
        q_eco: q_eco_down,
        supply_from_reservoir: false,
        user_demand_reservoir: vec![],
        supply_from_downstream: true,
        user_demand_downstream: vec![UserDemand {
            name: "下游用户".to_string(),
            from_downstream: true,
            values: user_demand_vals,
        }],
        supply_order: vec!["下游用户".to_string()],
        cal_mode: "年调节".to_string(),
    };

    let params = CalcParams {
        time_step: TimeStep::Daily,
        up_name: "上游水库".to_string(),
        down_name: "下游水库".to_string(),
        hydro_year_start: 4,
        hydro_year_end: 3,
        epsilon_v: 100.0,
        epsilon_w: 10.0,
        max_iterations: 2,
        up_v_special: vec![59994.0, 55919.0],
        down_v_special: vec![9500.0],
        need_add_users: vec!["下游用户".to_string()],
        user_special: vec![],
        user_stop_supply: vec![],
        up_eco_as_inflow: true,
    };

    ReservoirInput {
        params,
        upstream,
        downstream,
    }
}
