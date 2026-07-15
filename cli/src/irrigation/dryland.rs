use super::types::*;

/// 计算旱地作物日需水量 (单区, 单日)
/// 返回 万m³/d
pub fn calculate_dryland_demand(
    crops: &[Crop],
    crop_area_entry: &CropAreaEntry,
) -> f64 {
    let hydro_year = crop_area_entry.hydro_year;
    let mut total_water_m3 = 0.0;

    for (crop_name, area_km2) in &crop_area_entry.crop_areas {
        // 跳过水稻类 (它们不算旱地)
        if crop_name == "单季稻" || crop_name == "双季稻" {
            continue;
        }
        if *area_km2 <= 0.0 {
            continue;
        }

        if let Some(crop) = crops.iter().find(|c| c.name == *crop_name) {
            total_water_m3 += crop.calculate_water_volume(*area_km2, hydro_year);
        }
    }

    // 转换 m³ -> 万m³
    total_water_m3 / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dryland_demand() {
        let crops = vec![
            Crop {
                name: "小麦".into(),
                water_75: 0.2488,
                water_90: 0.3199,
            },
            Crop {
                name: "玉米".into(),
                water_75: 0.3750,
                water_90: 0.4821,
            },
        ];

        let entry = CropAreaEntry {
            zone_name: "test".into(),
            hydro_year: 90,
            crop_areas: vec![
                ("小麦".into(), 4.32),
                ("玉米".into(), 0.0),
                ("单季稻".into(), 38.0), // should be skipped
            ],
        };

        let demand = calculate_dryland_demand(&crops, &entry);
        // 小麦: 0.3199 * 4.32 * 0.1 = 0.1381968 m³/d
        // 玉米: 0 (area=0)
        // 万m³: 0.1381968 / 10000 = 0.00001381968
        let expected = 0.3199 * 4.32 * 0.1 / 10000.0;
        assert!((demand - expected).abs() < 1e-10);
    }

    #[test]
    fn test_crop_water_volume() {
        let crop = Crop {
            name: "test".into(),
            water_75: 1.0,
            water_90: 2.0,
        };
        // 2.0 * 10.0 * 0.1 = 2.0 m³/d
        assert!((crop.calculate_water_volume(10.0, 90) - 2.0).abs() < 1e-9);
        assert!((crop.calculate_water_volume(10.0, 75) - 1.0).abs() < 1e-9);
    }
}
