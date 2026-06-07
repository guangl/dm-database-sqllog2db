//! Pipeline 模块：字段定义、过滤器、归一化、处理管线。

pub mod filters;
pub mod normalizer;

mod field_mask;
mod normalize_config;
mod output_config;
mod processor;

#[cfg(test)]
mod tests;

pub use field_mask::{FIELD_NAMES, FieldMask};
pub use filters::FiltersFeature;
pub use normalize_config::NormalizeConfig;
pub use output_config::OutputConfig;
pub use processor::{LogProcessor, Pipeline};

pub(crate) use normalizer::compute_normalized;
