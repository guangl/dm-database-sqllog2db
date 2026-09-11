//! SQL 统计分析模块：提供 SQL 标准化、聚合与终端展示。

pub mod aggregate;
pub mod config;
pub mod normalize;
mod runner;

#[cfg(test)]
mod tests;

pub use runner::run_stats;

// Public re-exports for lib API consumers; may be unused in the bin target.
pub use config::StatsConfig;
