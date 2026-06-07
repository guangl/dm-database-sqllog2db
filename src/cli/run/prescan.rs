use crate::config::Config;
use crate::error::{Error, Result};
use crate::pipeline::filters::{IndicatorFilters, SqlFilters};
use dm_database_parser_sqllog::{Filter, FilterBuilder, LogParserBuilder};

// ===== Pre-scan: 指标/SQL 过滤器构建 =====

pub(super) fn build_indicator_filters(indicators: &IndicatorFilters) -> Vec<Filter> {
    let mut filters = Vec::new();
    if let Some(min_ms) = indicators.min_runtime_ms {
        #[allow(clippy::cast_precision_loss)]
        filters.push(FilterBuilder::new().exec_time_gte(min_ms as f32).build());
    }
    if let Some(min_r) = indicators.min_row_count {
        // rowcount >= min_r: for u32, rowcount_gt(min_r - 1) works when min_r > 0
        let filter = if min_r == 0 {
            FilterBuilder::new().build()
        } else {
            FilterBuilder::new().rowcount_gt(min_r - 1).build()
        };
        filters.push(filter);
    }
    if let Some(ids) = &indicators.exec_ids {
        for &id in ids {
            filters.push(FilterBuilder::new().exec_id_eq(id).build());
        }
    }
    filters
}

pub(super) fn build_sql_include_filters(sf: &SqlFilters) -> Vec<Filter> {
    sf.includes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

pub(super) fn build_sql_exclude_filters(sf: &SqlFilters) -> Vec<Filter> {
    sf.excludes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

// ===== Pre-scan: 单文件扫描（rayon 并行 + 文件内去重）=====

/// 扫描单个日志文件，返回满足事务级过滤条件的去重 `trxid` 列表。
///
/// 文件内部使用 `par_iter()` 并行处理各行，无共享可变状态，
/// 可被上层跨文件的 `par_iter()` 安全调用（两级 rayon 嵌套并行）。
pub(super) fn scan_log_file_for_matches(file_path: &str, cfg: &Config) -> Vec<String> {
    use rayon::prelude::*;

    let parser = match LogParserBuilder::new(file_path).build() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Pre-scan: failed to open '{file_path}': {e}");
            return Vec::new();
        }
    };
    let filters = match &cfg.filter {
        Some(f) if f.has_transaction_filters() => f,
        _ => return Vec::new(),
    };

    let indicator_filters = build_indicator_filters(&filters.indicators);
    let sql_include_filters = build_sql_include_filters(&filters.sql);
    let sql_exclude_filters = build_sql_exclude_filters(&filters.sql);
    let has_sql_filters = filters.sql.has_filters();

    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
    let trxids: std::collections::HashSet<String> = records
        .par_iter()
        .filter_map(|record| {
            let indicator_match = !indicator_filters.is_empty()
                && indicator_filters.iter().any(|f| f.matches(record));

            // SQL match is an independent path — not subordinate to indicator match.
            // Both filter types evaluate independently; trxid is collected if either matches.
            let sql_match = has_sql_filters && {
                let include_ok = sql_include_filters.is_empty()
                    || sql_include_filters.iter().any(|f| f.matches(record));
                let exclude_ok = sql_exclude_filters.is_empty()
                    || !sql_exclude_filters.iter().any(|f| f.matches(record));
                include_ok && exclude_ok
            };

            if indicator_match || sql_match {
                Some(record.trxid.clone())
            } else {
                None
            }
        })
        .collect();
    trxids.into_iter().collect()
}

// ===== Pre-scan: 跨文件编排（两级 rayon 嵌套并行）=====

pub(super) fn scan_for_trxids_by_transaction_filters(
    log_files: &[std::path::PathBuf],
    cfg: &Config,
    jobs: usize,
) -> Result<Vec<String>> {
    use rayon::prelude::*;

    log::info!(
        "Pre-scanning {} files for transaction-level filters...",
        log_files.len()
    );

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?;

    let matched: std::collections::HashSet<String> = pool.install(|| {
        log_files
            .par_iter()
            .flat_map(|file| {
                if let Some(path) = file.to_str() {
                    scan_log_file_for_matches(path, cfg)
                } else {
                    log::warn!(
                        "Pre-scan: skipping file with non-UTF8 path: {}",
                        file.display()
                    );
                    Vec::new()
                }
            })
            .collect()
    });

    Ok(matched.into_iter().collect())
}
