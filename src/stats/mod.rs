//! SQL 统计分析模块（v1.13）：提供 SQL 标准化与统计聚合。

pub mod aggregate;
pub mod normalize;
pub mod output;
pub use normalize::normalize_sql;
