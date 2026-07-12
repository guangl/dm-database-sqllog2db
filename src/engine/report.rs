//! Run 收尾输出：终端运行摘要与解析错误日志文件写出。

use crate::error::ErrorStats;
use std::io::Write;
use std::path::PathBuf;

/// 输出运行摘要（文件数、记录数、耗时、错误统计）。`quiet` 为 true 时不输出任何内容。
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
            // 整数千分数运算避免 u64 → f64 有损转换；显示一位小数百分比。
            let permille = run_stats
                .filtered_out
                .saturating_mul(1000)
                .checked_div(total_read)
                .unwrap_or(0);
            eprintln!(
                "  filtered: {} records ({}.{}% of {} total)",
                run_stats.filtered_out,
                permille / 10,
                permille % 10,
                total_read
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

/// 将解析错误记录批量写出到配置的 error log 文件。`cfg.append_error_log=true` 时为追加模式（watch 触发），`false` 时为覆盖模式（run 子命令默认）。
/// 无配置或无错误时为空操作；写出失败仅 warn 不终止。
pub(super) fn write_error_log(cfg: &crate::config::Config, stats: &ErrorStats) {
    let Some(error_cfg) = cfg.error.as_ref() else {
        return;
    };
    if stats.parse_error_records.is_empty() {
        return;
    }
    let file = if cfg.append_error_log {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&error_cfg.file)
    } else {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&error_cfg.file)
    };
    let file = match file {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to create error log {}: {e}", error_cfg.file);
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);
    let truncated = stats.parse_errors > stats.parse_error_records.len();
    for rec in &stats.parse_error_records {
        let _ = writeln!(
            writer,
            "[ERROR] line {}: {}  reason: {}",
            rec.line_number,
            rec.raw_truncated,
            rec.kind.kind_display()
        );
    }
    if truncated {
        let _ = writeln!(
            writer,
            "[truncated; showing first 10000 of {} total parse errors]",
            stats.parse_errors
        );
    }
    if let Err(e) = writer.flush() {
        log::warn!("Failed to flush error log: {e}");
    }
}
