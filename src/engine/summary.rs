use crate::error::ErrorStats;
use std::path::PathBuf;

/// 输出运行摘要（文件数、记录数、耗时、错误统计）。`quiet` 为 true 时不输出任何内容。
#[allow(clippy::fn_params_excessive_bools)]
pub(super) fn print_run_summary(
    quiet: bool,
    verbose: bool,
    use_parallel: bool,
    elapsed: f64,
    processed_files: &[(PathBuf, usize)],
    total_records: usize,
    skipped_files: usize,
    run_stats: &ErrorStats,
) {
    if !quiet {
        let mode_label = if use_parallel { " [parallel]" } else { "" };
        let skip_label = if skipped_files > 0 {
            format!(", {skipped_files} skipped")
        } else {
            String::new()
        };
        if verbose && !processed_files.is_empty() {
            for (path, count) in processed_files {
                eprintln!("Processed: {} — {} records", path.display(), count);
            }
        }
        eprintln!(
            "\n✓ SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {total_records} records total{skip_label}",
        );
        if run_stats.has_errors() {
            eprintln!(
                "  Errors: {} total ({} parse, {} export)",
                run_stats.total_errors, run_stats.parse_errors, run_stats.export_errors
            );
            eprintln!(
                "  errors by type: encoding={}, field_missing={}, parse_failed={}",
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::EncodingError)
                    .copied()
                    .unwrap_or(0),
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::FieldMissing)
                    .copied()
                    .unwrap_or(0),
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::ParseFailed)
                    .copied()
                    .unwrap_or(0),
            );
        }
        if run_stats.filtered_out > 0 {
            let total_read = total_records as u64 + run_stats.filtered_out;
            #[allow(clippy::cast_precision_loss)]
            let pct = if total_read > 0 {
                run_stats.filtered_out as f64 / total_read as f64 * 100.0
            } else {
                0.0
            };
            eprintln!(
                "  filtered: {} records ({pct:.1}% of {} total)",
                run_stats.filtered_out, total_read
            );
        }
        if run_stats
            .by_type
            .get(&crate::error::ErrorKind::EncodingError)
            .copied()
            .unwrap_or(0)
            > 0
        {
            eprintln!("  hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030");
        }
        if run_stats
            .by_type
            .get(&crate::error::ErrorKind::FieldMissing)
            .copied()
            .unwrap_or(0)
            > 0
        {
            eprintln!("  hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致");
        }
    }
}
