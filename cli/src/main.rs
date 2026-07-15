//! hydro-cli —— 水利计算 JSON CLI 后端。
//! 契约(同 macapp 舰队 Tier-B):`hydro-cli <calc> <action>`,JSON 从 stdin 读,JSON 信封写 stdout,一律 exit 0。
//!   成功 {"ok":true,"data":<结果>}   失败 {"ok":false,"error":"人话"}
//! 计算逻辑 = vendored 自作者上游 hydro 项目(私有)的纯 Rust calc,逐个计算器接入。
mod annual;
mod capacity;
mod efficiency;
mod reservoir;
mod district;
mod irrigation;
mod rainfall;
mod geocode;
mod common;

use serde::de::DeserializeOwned;
use serde_json::{json, to_value, Value};
use std::io::Read;

fn read_stdin_json() -> Value {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
        serde_json::from_str(&buf).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    }
}

fn emit_ok(data: Value) -> ! {
    println!("{}", json!({ "ok": true, "data": data }));
    std::process::exit(0);
}

fn emit_err(msg: String) -> ! {
    println!("{}", json!({ "ok": false, "error": msg }));
    std::process::exit(0);
}

/// 从输入 JSON 取一个字段并反序列化成 T(缺字段/解析失败 → 人话 Err)。
fn field<T: DeserializeOwned>(input: &Value, key: &str) -> Result<T, String> {
    let v = input
        .get(key)
        .cloned()
        .ok_or_else(|| format!("缺少字段: {}", key))?;
    serde_json::from_value(v).map_err(|e| format!("字段 {} 解析失败: {}", key, e))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let calc = args.get(1).map(String::as_str).unwrap_or("");
    let action = args.get(2).map(String::as_str).unwrap_or("");

    match calc {
        "list" => emit_ok(json!({ "calculators": ["annual", "capacity", "efficiency", "reservoir", "district", "irrigation", "rainfall", "geocode"] })),
        "annual" => annual_dispatch(action),
        "capacity" => capacity_dispatch(action),
        "efficiency" => efficiency_dispatch(action),
        "reservoir" => reservoir_dispatch(action),
        "district" => district_dispatch(action),
        "irrigation" => irrigation_dispatch(action),
        "rainfall" => rainfall_dispatch(action),
        "geocode" => geocode_dispatch(action),
        "" => emit_err("用法: hydro-cli <calc> <action> (JSON 经 stdin)".into()),
        _ => emit_err(format!("未知计算器: {} (已接入: annual, capacity, efficiency, reservoir, district, irrigation, rainfall, geocode)", calc)),
    }
}

fn annual_dispatch(action: &str) -> ! {
    use annual::commands as c;
    use annual::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "load-dir" => {
            let path: String = parse!("path");
            finish!(c::load_data_dir(path))
        }
        "indicators" => {
            let index: DataIndex = parse!("index");
            let table: String = parse!("table");
            finish!(c::get_indicators(index, table))
        }
        "query" => {
            let index: DataIndex = parse!("index");
            let qi: QueryInput = parse!("input");
            finish!(c::run_query(index, qi))
        }
        "aggregate" => {
            let index: DataIndex = parse!("index");
            let params: AggregateParams = parse!("params");
            finish!(c::run_aggregate(index, params))
        }
        "compare" => {
            let index: DataIndex = parse!("index");
            let params: CompareParams = parse!("params");
            finish!(c::run_compare(index, params))
        }
        "stats" => {
            let index: DataIndex = parse!("index");
            let cities: Vec<String> = parse!("cities");
            let years: Vec<i32> = parse!("years");
            let table: String = parse!("table");
            let indicator: String = parse!("indicator");
            finish!(c::run_stats(index, cities, years, table, indicator))
        }
        "export-excel" => {
            let path: String = parse!("path");
            let headers: Vec<String> = parse!("headers");
            let rows: Vec<Vec<Value>> = parse!("rows");
            match c::export_excel(path.clone(), headers, rows) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("annual 未知动作: {}", action)),
    }
}

fn capacity_dispatch(action: &str) -> ! {
    use capacity::commands as c;
    use capacity::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "read-excel" => {
            let path: String = parse!("path");
            finish!(c::read_excel(path))
        }
        "run" => {
            let ci: CapacityInput = parse!("input");
            finish!(c::run_capacity(ci))
        }
        "write-results" => {
            let path: String = parse!("path");
            let output: CapacityOutput = parse!("output");
            let start_month: u32 = parse!("start_month");
            match c::write_results(path.clone(), output, start_month) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("capacity 未知动作: {}", action)),
    }
}

fn efficiency_dispatch(action: &str) -> ! {
    use efficiency::commands as c;
    use efficiency::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "read-excel" => {
            let path: String = parse!("path");
            finish!(c::read_excel(path))
        }
        "run" => {
            let ai: AssessmentInput = parse!("input");
            finish!(c::run_assessment(ai))
        }
        "export-excel" => {
            let path: String = parse!("path");
            let output: AssessmentOutput = parse!("output");
            match c::write_results(path.clone(), output) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("efficiency 未知动作: {}", action)),
    }
}

fn reservoir_dispatch(action: &str) -> ! {
    use reservoir::commands as c;
    use reservoir::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "read-excel" => {
            let path: String = parse!("path");
            finish!(c::read_excel(path))
        }
        "run" => {
            let ri: ReservoirInput = parse!("input");
            finish!(c::run_schedule(ri))
        }
        "write-results" => {
            let path: String = parse!("path");
            let output: ScheduleOutput = parse!("output");
            match c::write_results(path.clone(), output) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("reservoir 未知动作: {}", action)),
    }
}

fn district_dispatch(action: &str) -> ! {
    use district::commands as c;
    use district::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "load-dir" => {
            let path: String = parse!("path");
            finish!(c::read_input_files(path))
        }
        "run" => {
            let si: SchedulerInput = parse!("input");
            finish!(c::run(si))
        }
        "export-dir" => {
            let dir: String = parse!("dir");
            let output: SchedulerOutput = parse!("output");
            match c::export_dir(dir.clone(), output) {
                Ok(()) => emit_ok(json!({ "dir": dir })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("district 未知动作: {}", action)),
    }
}

fn irrigation_dispatch(action: &str) -> ! {
    use irrigation::commands as c;
    use irrigation::sample_data::SampleData;
    use irrigation::types::IrrigationOutput;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "load-dir" => {
            let path: String = parse!("path");
            finish!(c::load_data_dir(path))
        }
        "run" => {
            let contents: SampleData = parse!("contents");
            // mode 可省,默认 both(同 SSOT:未知/缺省一律 Both)
            let mode = input
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("both")
                .to_string();
            finish!(c::run_irrigation(contents, mode))
        }
        "format-irrigation-tsv" => {
            let output: IrrigationOutput = parse!("output");
            finish!(c::format_irrigation_tsv(output))
        }
        "format-drainage-tsv" => {
            let output: IrrigationOutput = parse!("output");
            finish!(c::format_drainage_tsv(output))
        }
        _ => emit_err(format!("irrigation 未知动作: {}", action)),
    }
}

fn rainfall_dispatch(action: &str) -> ! {
    use rainfall::commands as c;
    use rainfall::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => finish!(c::get_sample_data()),
        "load-dir" => {
            let path: String = parse!("path");
            finish!(c::read_input_files(path))
        }
        "run-pipeline" => {
            let pi: PipelineInput = parse!("input");
            let steps: Vec<u32> = parse!("steps");
            finish!(c::run_pipeline(pi, steps))
        }
        "write-output" => {
            let path: String = parse!("path");
            let output: PipelineOutput = parse!("output");
            match c::write_output(path.clone(), output) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("rainfall 未知动作: {}", action)),
    }
}

fn geocode_dispatch(action: &str) -> ! {
    use geocode::commands as c;
    use geocode::types::*;
    let input = read_stdin_json();

    macro_rules! parse {
        ($key:expr) => {
            match field(&input, $key) {
                Ok(v) => v,
                Err(e) => emit_err(e),
            }
        };
    }
    macro_rules! finish {
        ($r:expr) => {
            match $r {
                Ok(v) => emit_ok(to_value(v).unwrap_or(json!(null))),
                Err(e) => emit_err(e),
            }
        };
    }

    match action {
        "sample" => emit_ok(to_value(c::get_sample_data()).unwrap_or(json!(null))),
        "convert" => {
            let lng: f64 = parse!("lng");
            let lat: f64 = parse!("lat");
            emit_ok(to_value(c::convert(lng, lat)).unwrap_or(json!(null)))
        }
        "read-excel" => {
            let path: String = parse!("path");
            let function_type: String = parse!("function_type");
            finish!(c::read_excel(path, function_type))
        }
        "detect-columns" => {
            let gi: GeocodeInput = parse!("input");
            emit_ok(to_value(c::detect_columns(gi)).unwrap_or(json!(null)))
        }
        "run-batch" => {
            let gi: GeocodeInput = parse!("input");
            finish!(c::run_batch(gi))
        }
        "write-results" => {
            let path: String = parse!("path");
            let gi: GeocodeInput = parse!("input");
            let results: BatchOutput = parse!("results");
            match c::write_results(path.clone(), gi, results) {
                Ok(()) => emit_ok(json!({ "path": path })),
                Err(e) => emit_err(e),
            }
        }
        _ => emit_err(format!("geocode 未知动作: {}", action)),
    }
}
