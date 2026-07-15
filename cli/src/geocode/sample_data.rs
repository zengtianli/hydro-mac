//! geocode 示例数据 —— 5 个浙江地标的 WGS-84 坐标。
//! 数据与 上游 sample_data.rs(代码内联的 5 地标)逐值一致,
//! 但按 hydro-mac 惯例落成数据文件嵌入(include_str! ← cli/data/sample/geocode/)。
use crate::geocode::types::{FunctionType, GeocodeInput, InputRow};
use std::collections::HashMap;

const SAMPLE_LANDMARKS: &str = include_str!("../../data/sample/geocode/landmarks.csv");

/// 返回内嵌的示例数据(逆地理编码形态: 名称/经度/纬度)
pub fn sample_landmarks() -> GeocodeInput {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(SAMPLE_LANDMARKS.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map(|h| {
            h.iter()
                .map(|s| s.trim().replace('\u{feff}', ""))
                .collect()
        })
        .unwrap_or_default();

    let mut rows = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = match record {
            Ok(r) => r,
            Err(_) => continue,
        };
        let mut fields = HashMap::new();
        for (c, header) in headers.iter().enumerate() {
            if !header.is_empty() {
                fields.insert(
                    header.clone(),
                    record.get(c).unwrap_or("").trim().to_string(),
                );
            }
        }
        rows.push(InputRow { index: i, fields });
    }

    GeocodeInput {
        rows,
        function_type: FunctionType::Reverse,
        coordinate_system: "WGS-84".to_string(),
        api_key: String::new(), // 由前端/环境变量补
    }
}
