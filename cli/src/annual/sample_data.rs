use crate::annual::data_loader;
use crate::annual::types::*;

const SAMPLE_YONGSHUIL: &str = include_str!("../../data/sample/annual/2024_合计市_用水量.csv");
const SAMPLE_GONGSHUIL: &str = include_str!("../../data/sample/annual/2024_全省市_供水量.csv");
const SAMPLE_SHEHUI: &str = include_str!("../../data/sample/annual/2024_全省市_社会经济指标.csv");

/// 返回内嵌的示例数据索引
pub fn sample_index() -> DataIndex {
    DataIndex {
        files: vec![
            FileEntry {
                path: "(内嵌) 2024_合计市_用水量.csv".to_string(),
                year: 2024,
                city: "合计市".to_string(),
                table_name: "用水量".to_string(),
            },
            FileEntry {
                path: "(内嵌) 2024_全省市_供水量.csv".to_string(),
                year: 2024,
                city: "全省市".to_string(),
                table_name: "供水量".to_string(),
            },
            FileEntry {
                path: "(内嵌) 2024_全省市_社会经济指标.csv".to_string(),
                year: 2024,
                city: "全省市".to_string(),
                table_name: "社会经济指标".to_string(),
            },
        ],
        available_years: vec![2024],
        available_cities: vec!["全省市".to_string(), "合计市".to_string()],
        available_tables: vec![
            "供水量".to_string(),
            "用水量".to_string(),
            "社会经济指标".to_string(),
        ],
        total_files: 3,
    }
}

/// 加载内嵌示例 CSV 的数据行
pub fn load_sample_data(table: &str) -> Result<(Vec<String>, Vec<DataRow>), String> {
    match table {
        "用水量" => data_loader::parse_csv_content(SAMPLE_YONGSHUIL, 2024, "合计市", "用水量"),
        "供水量" => data_loader::parse_csv_content(SAMPLE_GONGSHUIL, 2024, "全省市", "供水量"),
        "社会经济指标" => {
            data_loader::parse_csv_content(SAMPLE_SHEHUI, 2024, "全省市", "社会经济指标")
        }
        _ => Err(format!("未知表类型: {}", table)),
    }
}

/// 获取示例数据的指标列名
pub fn sample_indicators(table: &str) -> Vec<String> {
    let content = match table {
        "用水量" => SAMPLE_YONGSHUIL,
        "供水量" => SAMPLE_GONGSHUIL,
        "社会经济指标" => SAMPLE_SHEHUI,
        _ => return vec![],
    };

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    match reader.headers() {
        Ok(headers) => headers
            .iter()
            .skip(9) // 跳过基础列（水系..面积）
            .map(|h| h.trim().replace('\u{feff}', "").to_string())
            .filter(|h| !h.is_empty())
            .collect(),
        Err(_) => vec![],
    }
}
