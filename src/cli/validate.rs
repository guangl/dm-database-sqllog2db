use crate::config::Config;
use log::info;

pub fn handle_validate(cfg: &Config) {
    info!("SQL log input path: {}", cfg.sqllog.path);
    info!("Log level: {}", cfg.logging.level);
    info!("Log file: {}", cfg.logging.file);
    info!("Log retention: {} days", cfg.logging.retention_days);

    match &cfg.replace_parameters {
        Some(rp) => info!(
            "replace_parameters: enable={}, placeholders={:?}",
            rp.enable, rp.placeholders
        ),
        None => {
            info!("replace_parameters: not configured (default enabled, auto-detect placeholders)");
        }
    }
    match &cfg.filter {
        Some(f) => {
            info!(
                "filter: {}",
                if f.enable {
                    "enabled"
                } else {
                    "configured but not explicitly enabled"
                }
            );
            if let Some(start) = &f.include.start_ts {
                info!("  start_ts = {start}");
            }
            if let Some(end) = &f.include.end_ts {
                info!("  end_ts = {end}");
            }
            if let Some(ids) = &f.include.trxids {
                info!("  trxids = {} entries", ids.len());
            }
            if let Some(users) = &f.include.users {
                info!("  include.users = {users:?}");
            }
            if let Some(ips) = &f.include.ips {
                info!("  include.ips = {ips:?}");
            }
            if let Some(ids) = &f.indicators.exec_ids {
                info!("  exec_ids = {} entries", ids.len());
            }
            if let Some(ms) = f.indicators.min_runtime_ms {
                info!("  min_runtime_ms = {ms}");
            }
            if let Some(rows) = f.indicators.min_row_count {
                info!("  min_row_count = {rows}");
            }
            if f.sql.has_filters() {
                info!(
                    "  sql.includes = {} entries, excludes = {} entries",
                    f.sql.includes.as_ref().map_or(0, Vec::len),
                    f.sql.excludes.as_ref().map_or(0, Vec::len),
                );
            }
        }
        None => info!("filter: not configured"),
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
