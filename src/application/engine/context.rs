//! Shared run configuration and driver input/output types.
use crate::config::Config;
use crate::pipeline::filters::build_pipeline;
use crate::pipeline::{FieldMask, NormalizeConfig, OutputConfig, Pipeline};
use indicatif::ProgressBar;
use std::path::PathBuf;

/// 运行上下文：配置、pipeline 与归一化/投影选项，贯穿所有执行路径。
pub(super) struct RunContext<'a> {
    pub(super) cfg: &'a Config,
    pub(super) pipeline: Pipeline,
    pub(super) do_normalize: bool,
    pub(super) normalize_tags: Vec<String>,
    pub(super) placeholder_override: Option<bool>,
}

/// 终端展示环境：安静/详细模式与可选进度条（`show_progress ≡ pb.is_some()`）。
pub(super) struct Console<'a> {
    pub(super) quiet: bool,
    pub(super) verbose: bool,
    pub(super) pb: Option<&'a ProgressBar>,
}

/// 已处理文件及各自的记录数（顺序与输入文件一致）。
pub(super) type FileCounts = Vec<(PathBuf, usize)>;

pub(super) fn build_run_context(cfg: &Config) -> RunContext<'_> {
    let pipeline = build_pipeline(cfg);
    let field_mask = cfg
        .output
        .as_ref()
        .map_or(FieldMask::ALL, OutputConfig::field_mask);
    let do_normalize = field_mask.includes_normalized_sql() && cfg.replace_parameters.is_some();
    let normalize_tags = cfg
        .replace_parameters
        .as_ref()
        .map_or_else(Vec::new, |normalize| normalize.tags.clone());
    let placeholder_override = cfg
        .replace_parameters
        .as_ref()
        .and_then(NormalizeConfig::placeholder_override);
    RunContext {
        cfg,
        pipeline,
        do_normalize,
        normalize_tags,
        placeholder_override,
    }
}
