//! Filters 模块：过滤器类型、impl 方法、test 套件分文件组织。

mod processor;
mod serde_helpers;
pub(crate) mod transaction;
pub mod types;

#[cfg(test)]
#[path = "../../../tests/unit/pipeline/filters/mod.rs"]
mod tests;

pub(crate) use processor::build_pipeline;

// pub: required by tests/integration.rs
pub use types::FiltersFeature;
