//! district 河区供需平衡调度 —— vendored 纯 calc + CLI 编排(复刻自原 commands/,去 tauri)。
pub mod types;
pub mod config;
pub mod interpolation;
pub mod balance;
pub mod scheduler;
pub mod sample_data;
pub mod commands;
