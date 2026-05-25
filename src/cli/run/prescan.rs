use crate::config::Config;
use crate::error::{Error, Result};
use crate::pipeline::CompiledMetaFilters;
use dm_database_parser_sqllog::LogParserBuilder;

// ===== Pre-scan: 单文件扫描（rayon 并行 + 文件内去重）=====

/// 扫描单个日志文件，返回满足事务级过滤条件的去重 `trxid` 列表。
///
/// 文件内部使用 `par_iter()` 并行处理各行，无共享可变状态，
/// 可被上层跨文件的 `par_iter()` 安全调用（两级 rayon 嵌套并行）。
///
/// 结果在文件内去重：同一事务 ID 可能出现在数百条记录中，
/// 提前去重可显著减少跨文件合并时的中间数据量。
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

    // 收集到 Vec 再并行处理（LogIterator 未实现 rayon::IntoParallelIterator trait，需先 collect 到 Vec 才能用 par_iter）
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
    let trxids: std::collections::HashSet<String> = records
        .par_iter()
        .filter_map(|result| {
            let mut matched = false;

            if filters
                .indicators
                .matches(result.exec_id, result.exectime, result.rowcount)
            {
                matched = true;
            }
            if !matched && filters.sql.has_filters() {
                matched = filters.sql.matches(&result.sql);
            }
            if matched {
                Some(result.trxid.clone())
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

    // 使用与主流程相同的线程数（jobs），避免预扫描阶段无限制占用 CPU。
    // pool.install() 使内层 scan_log_file_for_matches 的 par_iter() 也在同一池内调度。
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?;

    // Collect directly into a HashSet to deduplicate trxids across files in one pass,
    // avoiding duplicate String allocations when the same trxid spans multiple files.
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

// ===== Pre-scan -> Main-pass 衔接: 合并 trxids 后重新编译 CompiledMetaFilters =====

/// pre-scan 完成后重新编译 `CompiledMetaFilters`。
///
/// 若 `final_cfg` 含有 `filter.enable == true`，则从 `final_cfg` 重新编译以包含
/// 预扫描发现的 trxids；否则直接回传原始值（`compiled_meta` 来自入参）。
/// 回传原始值的情形：无 filters 配置、filters 禁用、或调用方传 None 时走 None 路径。
pub(super) fn recompile_meta_if_needed(
    final_cfg: &Config,
    original: Option<CompiledMetaFilters>,
) -> Result<Option<CompiledMetaFilters>> {
    let filters = match &final_cfg.filter {
        Some(f) if f.enable => f,
        _ => return Ok(original),
    };
    // 重新从 final_cfg 编译，以捕获 merge_found_trxids 写入的 trxids
    let recompiled = crate::pipeline::CompiledMetaFilters::try_from_include_exclude(
        &filters.include,
        &filters.exclude,
    )?;
    Ok(Some(recompiled))
}
