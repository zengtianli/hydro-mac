//! geocode 高德地理编码 —— vendored 纯 calc(api/coordinate/types) + CLI 编排(复刻自原 commands/,去 tauri)。
//! 高德 API 动作(run-batch)需 AMAP key(输入 api_key 或环境变量 AMAP_KEY);
//! sample/read-excel/detect-columns/convert/write-results 为纯本地动作,离线可用。
pub mod api;
pub mod coordinate;
pub mod types;
pub mod sample_data;
pub mod commands;
