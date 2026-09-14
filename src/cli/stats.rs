//! Stats 子命令入口：合并 CLI 与配置选项后委托 `crate::stats::run_stats`。

use crate::config::Config;
use crate::error::Result;

fn merge_stats_options(
    cfg: &Config,
    cli_top: Option<u32>,
    cli_from: Option<String>,
    cli_to: Option<String>,
) -> (u32, Option<String>, Option<String>) {
    let effective_top: u32 = cli_top.or(cfg.stats.top).unwrap_or(20);
    let effective_from: Option<String> = cli_from.or_else(|| cfg.stats.from.clone());
    let effective_to: Option<String> = cli_to.or_else(|| cfg.stats.to.clone());
    (effective_top, effective_from, effective_to)
}

/// Handle the `stats` subcommand.
///
/// Merges CLI args with config values using priority: CLI > config > default.
/// `top` defaults to 20 when neither CLI nor config provides a value.
/// `cfg` must already have verbosity applied before calling this function.
///
/// # Errors
///
/// 时间范围格式非法或未找到任何日志文件时返回错误。
pub fn handle_stats(
    cfg: &Config,
    top: Option<u32>,
    from: Option<String>,
    to: Option<String>,
) -> Result<()> {
    let (effective_top, effective_from, effective_to) = merge_stats_options(cfg, top, from, to);

    let mut merged_cfg = cfg.clone();
    merged_cfg.stats.top = Some(effective_top);
    merged_cfg.stats.from = effective_from;
    merged_cfg.stats.to = effective_to;

    log::info!(
        "stats: top={effective_top} from={:?} to={:?}",
        merged_cfg.stats.from,
        merged_cfg.stats.to
    );

    crate::stats::run_stats(&merged_cfg, effective_top)
}

#[cfg(test)]
#[path = "../../tests/unit/cli/stats.rs"]
mod tests;
