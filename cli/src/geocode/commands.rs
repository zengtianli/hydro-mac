//! geocode CLI 编排 —— 复刻自原 apps/geocode/src-tauri/src/commands/{io,calc}.rs,去掉 #[tauri::command]。
//! 差异(相对 SSOT commands/):
//!   - 去 AppHandle/Emitter 进度事件(CLI 一次性返回,无 UI 事件流);
//!   - run_batch 的 api_key 为空时从环境变量 AMAP_KEY / AMAP_API_KEY 读(禁硬编码);
//!   - 行间限速用同步 sleep(编排循环本身是同步的,逐行 block_on 异步 API 调用);
//!   - 新增纯本地动作 convert(WGS-84 → GCJ-02 单点转换,复用 vendored coordinate.rs)。
use crate::geocode::api;
use crate::geocode::coordinate::wgs84_to_gcj02;
use crate::geocode::sample_data;
use crate::geocode::types::*;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

/// 经度列名匹配
const LNG_NAMES: &[&str] = &["经度", "JD", "lng", "longitude", "Lng", "LNG", "Longitude"];
/// 纬度列名匹配
const LAT_NAMES: &[&str] = &["纬度", "WD", "lat", "latitude", "Lat", "LAT", "Latitude"];
/// 地址列名匹配
const ADDR_NAMES: &[&str] = &["地址", "详细地址", "address", "Address"];
/// 公司名列名匹配
const COMPANY_NAMES: &[&str] = &[
    "公司名称",
    "企业名称",
    "名称",
    "单位名称",
    "用水户名称",
    "QYMC",
    "company",
    "Company",
];

/// 读取单元格为去空白字符串(同 hydro-common cell_str;本地内联避免动共享 common.rs)
fn cell_str(range: &Range<Data>, row: usize, col: usize) -> String {
    range
        .get((row, col))
        .map(|c| c.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 从 fields 中按候选列名查找值
fn find_field<'a>(
    fields: &'a HashMap<String, String>,
    candidates: &[&str],
) -> Option<&'a str> {
    for name in candidates {
        if let Some(val) = fields.get(*name) {
            if !val.is_empty() {
                return Some(val.as_str());
            }
        }
    }
    None
}

// ============ 纯本地动作 ============

pub fn get_sample_data() -> GeocodeInput {
    sample_data::sample_landmarks()
}

/// WGS-84 → GCJ-02 单点转换输出
#[derive(Debug, Clone, Serialize)]
pub struct ConvertOutput {
    pub wgs_lng: f64,
    pub wgs_lat: f64,
    pub gcj_lng: f64,
    pub gcj_lat: f64,
    pub out_of_china: bool,
}

/// WGS-84 → GCJ-02 单点转换(纯计算,离线)
pub fn convert(lng: f64, lat: f64) -> ConvertOutput {
    let (gcj_lng, gcj_lat) = wgs84_to_gcj02(lng, lat);
    ConvertOutput {
        wgs_lng: lng,
        wgs_lat: lat,
        gcj_lng,
        gcj_lat,
        out_of_china: gcj_lng == lng && gcj_lat == lat,
    }
}

pub fn read_excel(path: String, function_type: String) -> Result<GeocodeInput, String> {
    let mut wb: Xlsx<_> = open_workbook(&path).map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_names = wb.sheet_names().to_vec();
    let first_sheet = sheet_names.first().ok_or("文件中没有工作表")?.clone();

    let range = wb
        .worksheet_range(&first_sheet)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    if range.height() < 2 {
        return Err("工作表行数不足，至少需要表头+1行数据".into());
    }

    // 读取表头
    let ncols = range.width();
    let headers: Vec<String> = (0..ncols).map(|c| cell_str(&range, 0, c)).collect();

    // 读取数据行
    let mut rows = Vec::new();
    for r in 1..range.height() {
        let mut fields = HashMap::new();
        for (c, header) in headers.iter().enumerate() {
            if !header.is_empty() {
                fields.insert(header.clone(), cell_str(&range, r, c));
            }
        }
        rows.push(InputRow {
            index: r - 1,
            fields,
        });
    }

    // 确定功能类型
    let ft = match function_type.as_str() {
        "reverse" => FunctionType::Reverse,
        "forward" => FunctionType::Forward,
        "company" => FunctionType::CompanySearch,
        _ => FunctionType::Reverse,
    };

    Ok(GeocodeInput {
        rows,
        function_type: ft,
        coordinate_system: "WGS-84".to_string(),
        api_key: String::new(),
    })
}

/// 自动检测列名，返回找到的列名
pub fn detect_columns(input: GeocodeInput) -> HashMap<String, Option<String>> {
    let mut detected: HashMap<String, Option<String>> = HashMap::new();

    if input.rows.is_empty() {
        return detected;
    }

    let keys: Vec<String> = input.rows[0].fields.keys().cloned().collect();

    detected.insert(
        "lng".to_string(),
        keys.iter().find(|k| LNG_NAMES.contains(&k.as_str())).cloned(),
    );
    detected.insert(
        "lat".to_string(),
        keys.iter().find(|k| LAT_NAMES.contains(&k.as_str())).cloned(),
    );
    detected.insert(
        "address".to_string(),
        keys.iter().find(|k| ADDR_NAMES.contains(&k.as_str())).cloned(),
    );
    detected.insert(
        "company".to_string(),
        keys.iter()
            .find(|k| COMPANY_NAMES.contains(&k.as_str()))
            .cloned(),
    );

    detected
}

pub fn write_results(
    path: String,
    input: GeocodeInput,
    results: BatchOutput,
) -> Result<(), String> {
    let mut wb = Workbook::new();
    let bold = Format::new().set_bold();

    let ws = wb.add_worksheet();
    ws.set_name("编码结果").map_err(|e| e.to_string())?;

    if input.rows.is_empty() {
        wb.save(&path).map_err(|e| format!("保存失败: {}", e))?;
        return Ok(());
    }

    // 收集原始列名（按字段名排序保证稳定顺序）
    let first_row = &input.rows[0];
    let mut orig_headers: Vec<String> = Vec::new();
    let mut sorted_keys: Vec<&String> = first_row.fields.keys().collect();
    sorted_keys.sort();
    for k in sorted_keys {
        orig_headers.push(k.clone());
    }

    // 结果列
    let result_headers = match input.function_type {
        FunctionType::Reverse => vec![
            "地址", "省", "市", "区县", "区域编码", "GCJ02_经度", "GCJ02_纬度", "错误",
        ],
        FunctionType::Forward => vec!["经度", "纬度", "省", "市", "区县", "区域编码", "错误"],
        FunctionType::CompanySearch => vec![
            "地址", "经度", "纬度", "省", "市", "区县", "区域编码", "错误",
        ],
    };

    // 写入表头
    let mut col: u16 = 0;
    for h in &orig_headers {
        ws.write_string_with_format(0, col, h, &bold)
            .map_err(|e| e.to_string())?;
        col += 1;
    }
    for h in &result_headers {
        ws.write_string_with_format(0, col, *h, &bold)
            .map_err(|e| e.to_string())?;
        col += 1;
    }

    // 写入数据行
    for (ri, input_row) in input.rows.iter().enumerate() {
        let excel_row = (ri + 1) as u32;
        let mut col: u16 = 0;

        // 原始数据
        for h in &orig_headers {
            let val = input_row.fields.get(h).cloned().unwrap_or_default();
            ws.write_string(excel_row, col, &val)
                .map_err(|e| e.to_string())?;
            col += 1;
        }

        // 结果数据
        if let Some(result) = results.results.iter().find(|r| r.index == input_row.index) {
            match input.function_type {
                FunctionType::Reverse => {
                    ws.write_string(excel_row, col, &result.address)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 1, &result.province)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 2, &result.city)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 3, &result.district)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 4, &result.adcode)
                        .map_err(|e| e.to_string())?;
                    if let Some(lng) = result.lng {
                        ws.write_number(excel_row, col + 5, lng)
                            .map_err(|e| e.to_string())?;
                    }
                    if let Some(lat) = result.lat {
                        ws.write_number(excel_row, col + 6, lat)
                            .map_err(|e| e.to_string())?;
                    }
                    let err = result.error.as_deref().unwrap_or("");
                    ws.write_string(excel_row, col + 7, err)
                        .map_err(|e| e.to_string())?;
                }
                FunctionType::Forward => {
                    if let Some(lng) = result.lng {
                        ws.write_number(excel_row, col, lng)
                            .map_err(|e| e.to_string())?;
                    }
                    if let Some(lat) = result.lat {
                        ws.write_number(excel_row, col + 1, lat)
                            .map_err(|e| e.to_string())?;
                    }
                    ws.write_string(excel_row, col + 2, &result.province)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 3, &result.city)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 4, &result.district)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 5, &result.adcode)
                        .map_err(|e| e.to_string())?;
                    let err = result.error.as_deref().unwrap_or("");
                    ws.write_string(excel_row, col + 6, err)
                        .map_err(|e| e.to_string())?;
                }
                FunctionType::CompanySearch => {
                    ws.write_string(excel_row, col, &result.address)
                        .map_err(|e| e.to_string())?;
                    if let Some(lng) = result.lng {
                        ws.write_number(excel_row, col + 1, lng)
                            .map_err(|e| e.to_string())?;
                    }
                    if let Some(lat) = result.lat {
                        ws.write_number(excel_row, col + 2, lat)
                            .map_err(|e| e.to_string())?;
                    }
                    ws.write_string(excel_row, col + 3, &result.province)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 4, &result.city)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 5, &result.district)
                        .map_err(|e| e.to_string())?;
                    ws.write_string(excel_row, col + 6, &result.adcode)
                        .map_err(|e| e.to_string())?;
                    let err = result.error.as_deref().unwrap_or("");
                    ws.write_string(excel_row, col + 7, err)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    wb.save(&path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ============ 高德 API 动作(需 AMAP key + 网络) ============

/// 批量地理编码。api_key 为空时从环境变量 AMAP_KEY / AMAP_API_KEY 读。
pub fn run_batch(mut input: GeocodeInput) -> Result<BatchOutput, String> {
    if input.api_key.trim().is_empty() {
        input.api_key = std::env::var("AMAP_KEY")
            .or_else(|_| std::env::var("AMAP_API_KEY"))
            .unwrap_or_default();
    }
    if input.api_key.trim().is_empty() {
        return Err("未配置高德 API key: 请在输入里给 api_key,或设环境变量 AMAP_KEY".into());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("异步运行时启动失败: {}", e))?;

    let total = input.rows.len();
    let mut results = Vec::with_capacity(total);
    let mut success_count = 0usize;

    for (i, row) in input.rows.iter().enumerate() {
        let result = match input.function_type {
            FunctionType::Reverse => rt.block_on(process_reverse(
                row,
                &input.api_key,
                &input.coordinate_system,
            )),
            FunctionType::Forward => rt.block_on(process_forward(row, &input.api_key)),
            FunctionType::CompanySearch => rt.block_on(process_company(row, &input.api_key)),
        };

        if result.error.is_none() {
            success_count += 1;
        }
        results.push(result);

        // 速率限制: 300ms(同 SSOT;同步循环里用同步 sleep)
        if i + 1 < total {
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    Ok(BatchOutput {
        results,
        success_count,
        total,
    })
}

async fn process_reverse(row: &InputRow, api_key: &str, coord_system: &str) -> GeocodeResult {
    let lng_str = find_field(&row.fields, LNG_NAMES);
    let lat_str = find_field(&row.fields, LAT_NAMES);

    let (lng, lat) = match (lng_str, lat_str) {
        (Some(lng_s), Some(lat_s)) => match (lng_s.parse::<f64>(), lat_s.parse::<f64>()) {
            (Ok(lng), Ok(lat)) => (lng, lat),
            _ => {
                return error_result(row.index, "坐标格式错误");
            }
        },
        _ => {
            return error_result(row.index, "未找到经纬度列");
        }
    };

    // WGS-84 → GCJ-02 转换
    let (gcj_lng, gcj_lat) = if coord_system == "WGS-84" {
        wgs84_to_gcj02(lng, lat)
    } else {
        (lng, lat)
    };

    retry_api_call(|| api::reverse_geocode(gcj_lng, gcj_lat, api_key, row.index)).await
}

async fn process_forward(row: &InputRow, api_key: &str) -> GeocodeResult {
    let address = find_field(&row.fields, ADDR_NAMES);

    match address {
        Some(addr) if !addr.is_empty() => {
            retry_api_call(|| api::forward_geocode(addr, api_key, None, row.index)).await
        }
        _ => error_result(row.index, "未找到地址列"),
    }
}

async fn process_company(row: &InputRow, api_key: &str) -> GeocodeResult {
    let company = find_field(&row.fields, COMPANY_NAMES);

    match company {
        Some(name) if !name.is_empty() => {
            retry_api_call(|| api::search_company(name, api_key, None, row.index)).await
        }
        _ => error_result(row.index, "未找到公司名称列"),
    }
}

/// 带重试的 API 调用（限流错误重试 3 次，退避 2s/4s/6s,同 SSOT）
async fn retry_api_call<F, Fut>(make_call: F) -> GeocodeResult
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = GeocodeResult>,
{
    let mut result = make_call().await;

    for attempt in 1..=3u64 {
        if let Some(ref err) = result.error {
            if err.contains("EXCEEDED_THE_LIMIT") {
                tokio::time::sleep(Duration::from_secs(2 * attempt)).await;
                result = make_call().await;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

fn error_result(index: usize, msg: &str) -> GeocodeResult {
    GeocodeResult {
        index,
        address: String::new(),
        province: String::new(),
        city: String::new(),
        district: String::new(),
        adcode: String::new(),
        lng: None,
        lat: None,
        error: Some(msg.into()),
    }
}
