use crate::color;
use crate::config::Config;

pub fn handle_show_config(cfg: &Config, config_path: &str, diff: bool) {
    let def = if diff { Some(Config::default()) } else { None };

    let header = format!("Configuration ({config_path})");
    println!("{}", color::bold(&header));
    println!("{}", color::dim("═".repeat(header.len())));
    if diff {
        println!(
            "{}",
            color::dim("  Fields marked with * differ from defaults")
        );
    }
    println!();

    // [sqllog]
    println!("{}", color::cyan("[sqllog]"));
    kv(
        "path",
        &cfg.sqllog.path,
        def.as_ref().map(|d| d.sqllog.path.as_str()),
        diff,
    );
    println!();

    // [logging]
    println!("{}", color::cyan("[logging]"));
    kv(
        "file",
        &cfg.logging.file,
        def.as_ref().map(|d| d.logging.file.as_str()),
        diff,
    );
    kv(
        "level",
        &cfg.logging.level,
        def.as_ref().map(|d| d.logging.level.as_str()),
        diff,
    );
    let def_days = def.as_ref().map(|d| d.logging.retention_days.to_string());
    kv(
        "retention_days",
        &cfg.logging.retention_days.to_string(),
        def_days.as_deref(),
        diff,
    );
    println!();

    // [exporter.*]
    if let Some(csv) = &cfg.exporter.csv {
        let def_csv = def.as_ref().and_then(|d| d.exporter.csv.as_ref());
        println!("{}", color::cyan("[exporter.csv]"));
        kv("file", &csv.file, def_csv.map(|d| d.file.as_str()), diff);
        let def_ow = def_csv.map(|d| if d.overwrite { "true" } else { "false" });
        let def_ap = def_csv.map(|d| if d.append { "true" } else { "false" });
        kv("overwrite", &csv.overwrite.to_string(), def_ow, diff);
        kv("append", &csv.append.to_string(), def_ap, diff);
        println!();
    }

    if let Some(sqlite) = &cfg.exporter.sqlite {
        // sqlite is not in defaults, so all fields are "new" when diff=true
        let def_sqlite = def.as_ref().and_then(|d| d.exporter.sqlite.as_ref());
        println!("{}", color::cyan("[exporter.sqlite]"));
        kv(
            "database_url",
            &sqlite.database_url,
            def_sqlite.map(|d| d.database_url.as_str()),
            diff,
        );
        kv(
            "table_name",
            &sqlite.table_name,
            def_sqlite.map(|d| d.table_name.as_str()),
            diff,
        );
        let def_ow = def_sqlite.map(|d| if d.overwrite { "true" } else { "false" });
        let def_ap = def_sqlite.map(|d| if d.append { "true" } else { "false" });
        kv("overwrite", &sqlite.overwrite.to_string(), def_ow, diff);
        kv("append", &sqlite.append.to_string(), def_ap, diff);
        println!();
    }

    // [replace_parameters]
    if let Some(rp) = &cfg.replace_parameters {
        println!("{}", color::cyan("[replace_parameters]"));
        kv("enable", &rp.enable.to_string(), None, diff);
        if !rp.placeholders.is_empty() {
            kv(
                "placeholders",
                &format!("{:?}", rp.placeholders),
                None,
                diff,
            );
        }
        println!();
    }

    if let Some(f) = &cfg.filter {
        println!("{}", color::cyan("[filter]"));
        kv("enable", &f.enable.to_string(), None, diff);
        // include 子表
        if let Some(users) = &f.include.users {
            kv("include.users", &users.join(", "), None, diff);
        }
        if let Some(ips) = &f.include.ips {
            kv("include.ips", &ips.join(", "), None, diff);
        }
        if let Some(sess) = &f.include.sessions {
            kv("include.sessions", &sess.join(", "), None, diff);
        }
        if let Some(thrds) = &f.include.threads {
            kv("include.threads", &thrds.join(", "), None, diff);
        }
        if let Some(stmts) = &f.include.statements {
            kv("include.statements", &stmts.join(", "), None, diff);
        }
        if let Some(apps) = &f.include.apps {
            kv("include.apps", &apps.join(", "), None, diff);
        }
        if let Some(tags) = &f.include.tags {
            kv("include.tags", &tags.join(", "), None, diff);
        }
        if let Some(s) = &f.include.start_ts {
            kv("include.start_ts", s, None, diff);
        }
        if let Some(e) = &f.include.end_ts {
            kv("include.end_ts", e, None, diff);
        }
        if let Some(ids) = &f.include.trxids {
            kv(
                "include.trxids",
                &format!("{} entries", ids.len()),
                None,
                diff,
            );
        }
        // exclude 子表
        if let Some(users) = &f.exclude.users {
            kv("exclude.users", &users.join(", "), None, diff);
        }
        if let Some(ips) = &f.exclude.ips {
            kv("exclude.ips", &ips.join(", "), None, diff);
        }
        if let Some(sess) = &f.exclude.sessions {
            kv("exclude.sessions", &sess.join(", "), None, diff);
        }
        if let Some(thrds) = &f.exclude.threads {
            kv("exclude.threads", &thrds.join(", "), None, diff);
        }
        if let Some(stmts) = &f.exclude.statements {
            kv("exclude.statements", &stmts.join(", "), None, diff);
        }
        if let Some(apps) = &f.exclude.apps {
            kv("exclude.apps", &apps.join(", "), None, diff);
        }
        if let Some(tags) = &f.exclude.tags {
            kv("exclude.tags", &tags.join(", "), None, diff);
        }
        // indicators
        if let Some(ids) = &f.indicators.exec_ids {
            kv(
                "indicators.exec_ids",
                &format!("{} entries", ids.len()),
                None,
                diff,
            );
        }
        if let Some(min_rt) = f.indicators.min_runtime_ms {
            kv("indicators.min_runtime_ms", &min_rt.to_string(), None, diff);
        }
        if let Some(min_rc) = f.indicators.min_row_count {
            kv("indicators.min_row_count", &min_rc.to_string(), None, diff);
        }
        // sql (事务级字面量过滤)
        if let Some(incs) = &f.sql.includes {
            kv("sql.includes", &incs.join(", "), None, diff);
        }
        if let Some(excs) = &f.sql.excludes {
            kv("sql.excludes", &excs.join(", "), None, diff);
        }
        // record_sql (记录级正则过滤)
        if let Some(incs) = &f.record_sql.includes {
            kv("record_sql.includes", &incs.join(", "), None, diff);
        }
        if let Some(excs) = &f.record_sql.excludes {
            kv("record_sql.excludes", &excs.join(", "), None, diff);
        }
        println!();
    }

    if let Some(ta) = &cfg.template {
        println!("{}", color::cyan("[template]"));
        kv("enable", &ta.enable.to_string(), None, diff);
        if let Some(report) = &ta.report {
            kv("report.enabled", &report.enabled.to_string(), None, diff);
            if !report.csv_report_path.is_empty() {
                kv(
                    "report.csv_report_path",
                    &report.csv_report_path,
                    None,
                    diff,
                );
            }
            if !report.sqlite_report_path.is_empty() {
                kv(
                    "report.sqlite_report_path",
                    &report.sqlite_report_path,
                    None,
                    diff,
                );
            }
        }
        println!();
    }

    if let Some(charts) = &cfg.charts {
        println!("{}", color::cyan("[charts]"));
        kv("output_dir", &charts.output_dir, None, diff);
        kv("top_n", &charts.top_n.to_string(), None, diff);
        kv(
            "frequency_bar",
            &charts.frequency_bar.to_string(),
            None,
            diff,
        );
        kv("latency_hist", &charts.latency_hist.to_string(), None, diff);
        println!();
    }
}

/// Print a key=value line, optionally highlighting if the value differs from its default.
fn kv(key: &str, value: &str, default: Option<&str>, diff: bool) {
    let changed = diff && (default != Some(value));
    if changed {
        println!(
            "  {:<20} = {} {}",
            key,
            color::yellow(format!("\"{value}\"")),
            color::dim("*"),
        );
    } else {
        println!("  {:<20} = {}", key, color::green(format!("\"{value}\"")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ExporterConfig, SqliteExporterConfig};
    use crate::pipeline::{FiltersFeature, NormalizeConfig};

    #[test]
    fn test_handle_show_config_default_does_not_panic() {
        let cfg = Config::default();
        handle_show_config(&cfg, "config.toml", false);
    }

    #[test]
    fn test_handle_show_config_diff_no_changes() {
        let cfg = Config::default();
        // diff with no changes should not panic
        handle_show_config(&cfg, "config.toml", true);
    }

    #[test]
    fn test_handle_show_config_diff_with_changes() {
        let mut cfg = Config::default();
        cfg.sqllog.path = "/custom/logs".to_string();
        cfg.logging.level = "debug".to_string();
        handle_show_config(&cfg, "config.toml", true);
    }

    #[test]
    fn test_handle_show_config_with_sqlite_exporter() {
        let cfg = Config {
            exporter: ExporterConfig {
                csv: None,
                sqlite: Some(SqliteExporterConfig {
                    database_url: "out.db".to_string(),
                    table_name: "logs".to_string(),
                    overwrite: false,
                    append: true,
                    batch_size: 10_000,
                }),
            },
            ..Default::default()
        };
        handle_show_config(&cfg, "sqlite_config.toml", false);
    }

    #[test]
    fn test_handle_show_config_with_sqlite_diff() {
        let cfg = Config {
            exporter: ExporterConfig {
                csv: None,
                sqlite: Some(SqliteExporterConfig::default()),
            },
            ..Default::default()
        };
        // sqlite is not in defaults, so all fields should be marked
        handle_show_config(&cfg, "sqlite_config.toml", true);
    }

    #[test]
    fn test_handle_show_config_with_replace_parameters() {
        let cfg = Config {
            replace_parameters: Some(NormalizeConfig {
                enable: true,
                placeholders: vec!["?".to_string()],
            }),
            ..Default::default()
        };
        handle_show_config(&cfg, "rp_config.toml", false);
    }

    #[test]
    fn test_handle_show_config_with_filters() {
        let cfg = Config {
            filter: Some(FiltersFeature {
                enable: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        handle_show_config(&cfg, "filter_config.toml", false);
    }
}
