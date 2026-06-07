use crate::error::ErrorStats;
use std::io::Write;

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
