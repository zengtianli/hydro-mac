use crate::annual::types::*;
use std::collections::HashMap;
use std::path::Path;

/// 从文件名解析年份、城市、表名
fn parse_filename(filename: &str) -> Option<(i32, String, String)> {
    let stem = filename.strip_suffix(".csv")?;
    let parts: Vec<&str> = stem.splitn(3, '_').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let city = parts[1].to_string();
    let table = parts[2].to_string();
    Some((year, city, table))
}

/// 扫描数据目录，构建文件索引
pub fn scan_data_dir(dir_path: &str) -> Result<DataIndex, String> {
    let path = Path::new(dir_path);
    if !path.is_dir() {
        return Err(format!("路径不是目录: {}", dir_path));
    }

    let mut files = Vec::new();
    let mut years = std::collections::BTreeSet::new();
    let mut cities = std::collections::BTreeSet::new();
    let mut tables = std::collections::BTreeSet::new();

    let entries = std::fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".csv") {
            continue;
        }
        if let Some((year, city, table)) = parse_filename(&file_name) {
            years.insert(year);
            cities.insert(city.clone());
            tables.insert(table.clone());
            files.push(FileEntry {
                path: entry.path().to_string_lossy().to_string(),
                year,
                city,
                table_name: table,
            });
        }
    }

    Ok(DataIndex {
        total_files: files.len(),
        files,
        available_years: years.into_iter().collect(),
        available_cities: cities.into_iter().collect(),
        available_tables: tables.into_iter().collect(),
    })
}

/// 从 CSV 内容解析数据行
pub fn parse_csv_content(
    content: &str,
    year: i32,
    city: &str,
    table_name: &str,
) -> Result<(Vec<String>, Vec<DataRow>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    // 读取表头（第一行）
    let raw_headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("读取表头失败: {}", e))?
        .iter()
        .map(|h| h.trim().replace('\u{feff}', "").to_string())
        .collect();

    // 只取第一条数据行（汇总行），跳过后面的分类标题行
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("读取记录失败: {}", e))?;

        // 跳过非数据行（检查"市"列是否有值，或者面积列是否是数字）
        let city_col = record.get(8).unwrap_or("").trim().to_string();
        if city_col.is_empty() {
            continue;
        }
        // 跳过标题行（市列包含"水系"等非城市名）
        if city_col == "水系" || city_col.contains("所属") {
            continue;
        }

        let mut values = HashMap::new();
        for (i, header) in raw_headers.iter().enumerate() {
            if header.is_empty() {
                continue;
            }
            let val = record.get(i).unwrap_or("").trim().to_string();
            if val.is_empty() {
                values.insert(header.clone(), serde_json::Value::Null);
            } else if let Ok(f) = val.parse::<f64>() {
                values.insert(
                    header.clone(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
                    ),
                );
            } else {
                values.insert(header.clone(), serde_json::Value::String(val));
            }
        }

        let district = record.get(7).unwrap_or("").trim().to_string();

        rows.push(DataRow {
            year,
            city: city.to_string(),
            table_name: table_name.to_string(),
            district,
            values,
        });
    }

    Ok((raw_headers, rows))
}

/// 加载指定文件的 CSV 数据
pub fn load_csv_file(entry: &FileEntry) -> Result<(Vec<String>, Vec<DataRow>), String> {
    let content =
        std::fs::read_to_string(&entry.path).map_err(|e| format!("读取文件失败: {}", e))?;
    parse_csv_content(&content, entry.year, &entry.city, &entry.table_name)
}

/// 根据过滤条件加载多个 CSV 文件的数据
pub fn load_filtered_data(
    index: &DataIndex,
    cities: &[String],
    years: &[i32],
    table: &str,
) -> Result<(Vec<String>, Vec<DataRow>), String> {
    let mut all_rows = Vec::new();
    let mut all_headers = Vec::new();

    for entry in &index.files {
        if entry.table_name != table {
            continue;
        }
        if !years.is_empty() && !years.contains(&entry.year) {
            continue;
        }
        if !cities.is_empty() && !cities.contains(&entry.city) {
            continue;
        }
        let (headers, rows) = load_csv_file(entry)?;
        if all_headers.is_empty() {
            all_headers = headers;
        }
        all_rows.extend(rows);
    }

    // 按年份、城市排序
    all_rows.sort_by(|a, b| a.year.cmp(&b.year).then(a.city.cmp(&b.city)));

    Ok((all_headers, all_rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filename() {
        let result = parse_filename("2024_合计市_用水量.csv");
        assert!(result.is_some());
        let (year, city, table) = result.unwrap();
        assert_eq!(year, 2024);
        assert_eq!(city, "合计市");
        assert_eq!(table, "用水量");
    }

    #[test]
    fn test_parse_filename_invalid() {
        assert!(parse_filename("invalid.csv").is_none());
        assert!(parse_filename("not_a_csv.txt").is_none());
    }
}
