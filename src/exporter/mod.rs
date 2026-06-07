//! Exporter 模块：导出器 trait、统计、kind 包装、管理器、共享工具。

pub mod csv;
pub(crate) mod projection;
pub mod sqlite;

mod api;
mod kind;
mod manager;
mod stats;
mod util;

#[cfg(test)]
mod tests;

pub use api::Exporter;
pub use stats::ExportStats;

pub(crate) use csv::CsvExporter;
pub(crate) use manager::ExporterManager;
pub(crate) use sqlite::SqliteExporter;
pub(crate) use util::{ensure_parent_dir, f32_ms_to_i64, strip_ip_prefix};
