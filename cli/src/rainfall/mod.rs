//! rainfall 降雨数据分析(平原河网降雨-径流 6 步处理管线) —— vendored 纯 calc + CLI 编排(复刻自原 commands/,去 tauri)。
pub mod types;
pub mod parser;
pub mod pipeline;
pub mod sample_data;
pub mod commands;
