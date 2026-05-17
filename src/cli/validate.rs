use crate::config::Config;
use log::info;

pub fn handle_validate(cfg: &Config) {
    info!("SQL日志输入路径: {}", cfg.sqllog.path);
    info!("日志级别: {}", cfg.logging.level);
    info!("日志文件: {}", cfg.logging.file);
    info!("日志保留: {} 天", cfg.logging.retention_days);

    match &cfg.replace_parameters {
        Some(rp) => info!(
            "replace_parameters: enable={}, placeholders={:?}",
            rp.enable, rp.placeholders
        ),
        None => info!("replace_parameters: 未配置（默认启用，自动检测占位符）"),
    }
    match &cfg.filter {
        Some(f) => {
            info!(
                "filter: {}",
                if f.enable {
                    "启用"
                } else {
                    "配置但未明确启用"
                }
            );
            if let Some(start) = &f.include.start_ts {
                info!("  start_ts = {start}");
            }
            if let Some(end) = &f.include.end_ts {
                info!("  end_ts = {end}");
            }
            if let Some(ids) = &f.include.trxids {
                info!("  trxids = {} 条", ids.len());
            }
            if let Some(users) = &f.include.users {
                info!("  include.users = {users:?}");
            }
            if let Some(ips) = &f.include.ips {
                info!("  include.ips = {ips:?}");
            }
            if let Some(ids) = &f.indicators.exec_ids {
                info!("  exec_ids = {} 条", ids.len());
            }
            if let Some(ms) = f.indicators.min_runtime_ms {
                info!("  min_runtime_ms = {ms}");
            }
            if let Some(rows) = f.indicators.min_row_count {
                info!("  min_row_count = {rows}");
            }
            if f.sql.has_filters() {
                info!(
                    "  sql.includes = {} 条, excludes = {} 条",
                    f.sql.includes.as_ref().map_or(0, Vec::len),
                    f.sql.excludes.as_ref().map_or(0, Vec::len),
                );
            }
        }
        None => info!("filter: 未配置"),
    }

    if let Some(csv) = &cfg.exporter.csv {
        info!(
            "CSV export: {} (overwrite: {})",
            csv.file,
            if csv.overwrite { "yes" } else { "no" }
        );
    }
    if let Some(sqlite) = &cfg.exporter.sqlite {
        info!(
            "SQLite export: {} / {} (overwrite: {})",
            sqlite.database_url,
            sqlite.table_name,
            if sqlite.overwrite { "yes" } else { "no" }
        );
    }
}
