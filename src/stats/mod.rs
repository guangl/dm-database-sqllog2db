//! SQL 统计分析模块（v1.13）：提供 SQL 标准化、聚合与输出。

pub mod aggregate;
pub mod config;
pub mod normalize;
pub mod output;

mod runner;

#[cfg(test)]
mod tests;

pub use runner::run_stats;

// Public re-exports for lib API consumers; may be unused in the bin target.
#[allow(unused_imports)]
pub use config::StatsConfig;
#[allow(unused_imports)]
pub use config::validate_time_str;
