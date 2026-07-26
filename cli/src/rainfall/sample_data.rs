//! rainfall 内嵌示例数据 —— 与原版差异:原版运行时读一个外部数据目录(已不存在),
//! 本版照 annual 模板 include_str! 内嵌(源 = 项目自带示例数据,
//! input_YSH.txt 裁剪为前 10 个取水户的最小可跑集)。
use crate::rainfall::commands;
use crate::rainfall::parser;
use crate::rainfall::types::*;

const SAMPLE_STATIC: &str = include_str!("../../data/sample/rainfall/static_PYLYSCS.txt");
const SAMPLE_RAINFALL: &str = include_str!("../../data/sample/rainfall/input_FQNNGXL.txt");
const SAMPLE_BASELINE: &str = include_str!("../../data/sample/rainfall/input_GHJYL.txt");
const SAMPLE_INTAKE: &str = include_str!("../../data/sample/rainfall/input_YSH.txt");
const SAMPLE_USER_LAKE: &str = include_str!("../../data/sample/rainfall/input_YSH_GH.txt");

/// 加载内嵌示例的完整管线输入(15 分区 / 13 天降雨 / 基线流量 / 取水户)。
pub fn load_sample_input() -> Result<PipelineInput, String> {
    let partitions = parser::parse_partition_config(SAMPLE_STATIC)?;
    let rainfall = commands::parse_rainfall_content(SAMPLE_RAINFALL)?;
    let (baseline_columns, baseline) = commands::parse_baseline_content(SAMPLE_BASELINE)?;
    let users = commands::parse_user_intake_content(SAMPLE_INTAKE)?;
    let user_lake_map = commands::parse_user_lake_map_content(SAMPLE_USER_LAKE)?;

    Ok(PipelineInput {
        partitions,
        rainfall,
        baseline_columns,
        baseline,
        users,
        user_lake_map,
    })
}
