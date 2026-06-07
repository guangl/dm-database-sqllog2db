//! Filters 模块：过滤器类型、impl 方法、test 套件分文件组织。

mod feature_ops;
mod indicator_ops;
mod serde_helpers;
mod sql_ops;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use serde_helpers::TrxidSet;
// pub: required by tests/integration.rs
pub use types::{FiltersFeature, IndicatorFilters, SqlFilters};
