//! irrigation 内嵌示例数据 —— 复刻自原 apps/irrigation/src-tauri/src/sample_data.rs,
//! include_str! 改指 cli/data/sample/irrigation/(8 个 TSV/TXT 输入文件,与原 sample_data/ 逐字一致)。
use serde::{Deserialize, Serialize};

/// 8 个输入文件的内容集合。既是示例数据的载体,也是 `run` 动作的输入类型
/// (原 Tauri 前端把 8 个文件内容当 8 个 String 参数传;CLI 版收进一个结构)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleData {
    pub time_config: String,
    pub zones: String,
    pub single_crop_stages: String,
    pub double_crop_stages: String,
    pub crops: String,
    pub rainfall: String,
    pub evaporation: String,
    pub crop_areas: String,
}

pub fn get_sample_data() -> SampleData {
    SampleData {
        time_config: include_str!("../../data/sample/irrigation/in_TIME.txt").to_string(),
        zones: include_str!("../../data/sample/irrigation/static_fenqu.txt").to_string(),
        single_crop_stages: include_str!("../../data/sample/irrigation/static_single_crop.txt")
            .to_string(),
        double_crop_stages: include_str!("../../data/sample/irrigation/static_double_crop.txt")
            .to_string(),
        crops: include_str!("../../data/sample/irrigation/static_crops.txt").to_string(),
        rainfall: include_str!("../../data/sample/irrigation/in_JYGC.txt").to_string(),
        evaporation: include_str!("../../data/sample/irrigation/in_ZFGC.txt").to_string(),
        crop_areas: include_str!("../../data/sample/irrigation/in_dry_crop_area.txt").to_string(),
    }
}
