//! Config 模块：根 Config 结构、子模块（sqllog/exporter/logging）、validate。

mod error_log;
pub mod exporter;
pub mod logging;
mod root;
pub mod sqllog;
pub(crate) mod template;
mod validate;

#[cfg(test)]
mod tests;

// 以下 pub use 为 crate::config 路径的公开 API 重导出，
// 部分类型仅在测试代码中通过 crate::config::X 路径访问，
// 因此在非测试编译上下文中可能触发 unused_imports 警告。
#[allow(unused_imports)]
pub use crate::stats::config::StatsConfig;
#[allow(unused_imports)]
pub use error_log::ErrorLogConfig;
#[allow(unused_imports)]
pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use root::Config;
#[allow(unused_imports)]
pub use sqllog::SqllogConfig;
