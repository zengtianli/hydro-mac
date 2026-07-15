//! reservoir 水库群调度 —— vendored 纯 calc + CLI 编排(复刻自原 commands/,去 tauri)。
pub mod types;
pub mod interp;
pub mod physics;
pub mod dispatch;
pub mod balance;
pub mod scheduler;
pub mod statistics;
pub mod sample_data;
pub mod commands;
