use crate::config::Config;
use crate::error::{Error, Result};
use crate::pipeline::filters::types::{IndicatorFilters, SqlFilters};
use dm_database_parser_sqllog::{Filter, FilterBuilder, LogParserBuilder};

// ===== Pre-scan: 指标/SQL 过滤器构建 =====

fn build_indicator_filters(indicators: &IndicatorFilters) -> Vec<Filter> {
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

fn build_sql_include_filters(sf: &SqlFilters) -> Vec<Filter> {
    sf.includes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

fn build_sql_exclude_filters(sf: &SqlFilters) -> Vec<Filter> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::pipeline::FiltersFeature;
    use crate::pipeline::filters::types::{IndicatorFilters, SqlFilters};

    #[test]
    fn test_build_indicator_filters_min_row_count_zero() {
        let indicators = IndicatorFilters {
            min_row_count: Some(0),
            ..IndicatorFilters::default()
        };
        let filters = build_indicator_filters(&indicators);
        assert_eq!(
            filters.len(),
            1,
            "min_row_count=0 应构建一个全匹配 Filter（FilterBuilder::new().build() 分支）"
        );
    }

    #[test]
    fn test_build_indicator_filters_min_row_count_positive() {
        let indicators = IndicatorFilters {
            min_row_count: Some(5),
            ..IndicatorFilters::default()
        };
        let filters = build_indicator_filters(&indicators);
        assert_eq!(
            filters.len(),
            1,
            "min_row_count=5 应构建一个带 rowcount_gt(4) 约束的 Filter"
        );
    }

    #[test]
    fn test_build_indicator_filters_empty_returns_empty() {
        let indicators = IndicatorFilters::default();
        let filters = build_indicator_filters(&indicators);
        assert_eq!(filters.len(), 0, "所有字段均为 None 时应返回空 Vec<Filter>");
    }

    #[test]
    fn test_build_sql_exclude_filters_multiple_returns_correct_count() {
        let sf = SqlFilters {
            excludes: Some(vec![
                "SELECT 1".into(),
                "DROP".into(),
                "DELETE FROM x".into(),
            ]),
            includes: None,
        };
        let filters = build_sql_exclude_filters(&sf);
        assert_eq!(
            filters.len(),
            3,
            "3 个 exclude 模式应构建 3 个 Filter（非空 excludes 分支）"
        );
    }

    #[test]
    fn test_build_sql_exclude_filters_none_returns_empty() {
        let sf = SqlFilters::default();
        let filters = build_sql_exclude_filters(&sf);
        assert_eq!(
            filters.len(),
            0,
            "excludes=None 应通过 unwrap_or(&[]) 返回空 Vec<Filter>"
        );
    }

    #[test]
    fn test_build_sql_include_filters_multiple() {
        let sf = SqlFilters {
            includes: Some(vec!["SELECT".into(), "UPDATE".into()]),
            excludes: None,
        };
        let filters = build_sql_include_filters(&sf);
        assert_eq!(filters.len(), 2, "2 个 include 模式应构建 2 个 Filter");
    }

    /// 验证 `min_row_count=0` 的 `FilterBuilder::new().build()` 分支产生全匹配过滤器：
    /// 扫描真实日志文件时所有记录的 trxid 均应被返回。
    #[test]
    fn test_min_row_count_zero_matches_all_records() {
        use std::fmt::Write as _;

        let dir = tempfile::TempDir::new().unwrap();
        let logfile = dir.path().join("test.log");

        // 写入 3 条不同 trxid 的记录（rowcount 故意各不相同）
        let mut buf = String::new();
        for i in 0..3_usize {
            writeln!(
                buf,
                "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:U trxid:{i} stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT {i}. EXECTIME: 1(ms) ROWCOUNT: {i}(rows) EXEC_ID: {i}.",
            ).unwrap();
        }
        std::fs::write(&logfile, &buf).unwrap();

        let cfg = Config {
            filter: Some(FiltersFeature {
                enable: true,
                indicators: IndicatorFilters {
                    min_row_count: Some(0),
                    ..IndicatorFilters::default()
                },
                ..FiltersFeature::default()
            }),
            ..Config::default()
        };

        let matched = scan_log_file_for_matches(logfile.to_str().unwrap(), &cfg);
        assert_eq!(
            matched.len(),
            3,
            "min_row_count=0 应匹配所有记录（全匹配 Filter），实际匹配: {matched:?}",
        );
    }

    /// 验证 `exec_ids` 过滤器正确产生独立的 `Filter`——每个 ID 一个 `Filter`
    #[test]
    fn test_build_indicator_filters_exec_ids_multiple() {
        use std::collections::HashSet;
        let indicators = IndicatorFilters {
            exec_ids: Some(HashSet::from([1_i64, 2, 42])),
            ..IndicatorFilters::default()
        };
        let filters = build_indicator_filters(&indicators);
        assert_eq!(
            filters.len(),
            3,
            "3 个 exec_ids 应产生 3 个独立的 Filter（每个 ID 一个）"
        );
    }
}
