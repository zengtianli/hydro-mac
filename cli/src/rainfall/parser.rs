use super::types::Partition;
use regex::Regex;

/// Parse the custom static_PYLYSCS.txt format into Partition structs.
///
/// Format example:
/// ```text
/// 分区01-名称:QZPYQ(青洲)
/// 分区01包含概湖个数(个):43
/// 分区01包含概湖的名称:G007,G008,...
/// 分区01包含概湖对应的面积(m2):37.2145,10.3231,...
/// ```
pub fn parse_partition_config(content: &str) -> Result<Vec<Partition>, String> {
    let mut partitions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let name_re = Regex::new(r"^分区(\d{2})-名称:(.+)$").unwrap();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        if let Some(caps) = name_re.captures(line) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let full_label = caps[2].trim().to_string();

            // Extract code and display_name from "QZPYQ(青洲)" or just "QZPYQ"
            let (code, display_name) = if let Some(paren_start) = full_label.find('(') {
                let code = full_label[..paren_start].to_string();
                let dn = full_label[paren_start + 1..].trim_end_matches(')').to_string();
                (code, dn)
            } else {
                (full_label.clone(), full_label.clone())
            };

            // Look ahead for lake names and areas
            let mut lake_ids = Vec::new();
            let mut areas = Vec::new();

            for j in (i + 1)..lines.len().min(i + 5) {
                let next = lines[j].trim();
                if next.contains("包含概湖的名称") {
                    if let Some(colon_pos) = next.find(':') {
                        lake_ids = next[colon_pos + 1..]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                } else if next.contains("包含概湖对应的面积") {
                    if let Some(colon_pos) = next.find(':') {
                        areas = next[colon_pos + 1..]
                            .split(',')
                            .filter_map(|s| s.trim().parse::<f64>().ok())
                            .collect();
                    }
                }
            }

            if !lake_ids.is_empty() {
                partitions.push(Partition {
                    number,
                    code,
                    display_name,
                    full_label,
                    lake_ids,
                    areas,
                });
            }
        }

        i += 1;
    }

    if partitions.is_empty() {
        return Err("未能从分区配置文件中解析出任何分区".into());
    }

    Ok(partitions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_partition_config() {
        let content = r#"分区个数(个):2
分区名称:青洲平原区,黛山平原区
分区01-名称:QZPYQ(青洲)
分区01包含概湖个数(个):3
分区01包含概湖的名称:G007,G008,G021
分区01包含概湖对应的面积(m2):37.2145,10.3231,17.5497
分区02-名称:DSPYQ(黛山)
分区02包含概湖个数(个):2
分区02包含概湖的名称:G001,G002
分区02包含概湖对应的面积(m2):3.0777,10.2613
"#;
        let parts = parse_partition_config(content).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].code, "QZPYQ");
        assert_eq!(parts[0].lake_ids.len(), 3);
        assert_eq!(parts[0].areas.len(), 3);
        assert_eq!(parts[1].code, "DSPYQ");
        assert_eq!(parts[1].lake_ids.len(), 2);
    }
}
