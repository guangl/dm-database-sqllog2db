use crate::config::Config;
use crate::error::Result;
use crate::pipeline::CompiledMetaFilters;
use ahash::HashSet as AHashSet;
use compact_str::CompactString;
use dm_database_parser_sqllog::LogParser;

/// 扫描单个日志文件，返回满足事务级过滤条件的去重 `trxid` 列表。
///
/// 文件内部使用 `par_iter()` 并行处理各行，无共享可变状态，
/// 可被上层跨文件的 `par_iter()` 安全调用（两级 rayon 嵌套并行）。
///
/// 结果在文件内去重：同一事务 ID 可能出现在数百条记录中，
/// 提前去重可显著减少跨文件合并时的中间数据量。
pub(super) fn scan_log_file_for_matches(file_path: &str, cfg: &Config) -> Vec<CompactString> {
    use rayon::prelude::*;

    let Ok(parser) = LogParser::from_path(file_path) else {
        return Vec::new();
    };
    let filters = match &cfg.filter {
        Some(f) if f.has_transaction_filters() => f,
        _ => return Vec::new(),
    };

    // trxid 用 CompactString：数字字符串 ≤23 字节，内联存储，无堆分配。
    // 收集到 HashSet 实现文件内去重，rayon 支持并行 collect 到 std::HashSet。
    let trxids: std::collections::HashSet<CompactString> = parser
        .par_iter()
        .filter_map(std::result::Result::ok)
        .filter_map(|result| {
            let mut matched = false;

            if let Some(ind) = result.parse_indicators() {
                if filters
                    .indicators
                    .matches(ind.exec_id, ind.exectime, i64::from(ind.rowcount))
                {
                    matched = true;
                }
            }
            if !matched && filters.sql.has_filters() {
                matched = filters.sql.matches(result.body().as_ref());
            }
            if matched {
                let meta = result.parse_meta();
                Some(CompactString::from(meta.trxid.as_ref()))
            } else {
                None
            }
        })
        .collect();
    trxids.into_iter().collect()
}

pub(super) fn scan_for_trxids_by_transaction_filters(
    log_files: &[std::path::PathBuf],
    cfg: &Config,
    jobs: usize,
) -> AHashSet<CompactString> {
    use rayon::prelude::*;

    eprintln!(
        "Pre-scanning {} files for transaction-level filters...",
        log_files.len()
    );

    // 使用与主流程相同的线程数（jobs），避免预扫描阶段无限制占用 CPU。
    // pool.install() 使内层 scan_log_file_for_matches 的 par_iter() 也在同一池内调度。
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .expect("failed to build pre-scan thread pool");

    let matched: Vec<CompactString> = pool.install(|| {
        log_files
            .par_iter()
            .flat_map(|file| scan_log_file_for_matches(&file.to_string_lossy(), cfg))
            .collect()
    });

    matched.into_iter().collect()
}

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
