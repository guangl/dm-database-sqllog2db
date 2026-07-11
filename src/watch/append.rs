//! Watch 触发前的 Config 追加模式注入。

use crate::config::Config;

/// watch 触发前强制 `tmp_cfg` 进入追加模式（per D-01/D-05），适用全量与增量路径。
pub(super) fn force_append_for_watch_trigger(cfg: &mut Config) {
    // WATCH-07 (D-01): CSV exporter 强制 append=true、overwrite=false
    if let Some(ref mut csv_cfg) = cfg.exporter.csv {
        csv_cfg.append = true;
        csv_cfg.overwrite = false;
    }
    // WATCH-07 (D-01): SQLite exporter 同样强制 append=true、overwrite=false，避免每次触发清空表
    if let Some(ref mut sqlite_cfg) = cfg.exporter.sqlite {
        sqlite_cfg.append = true;
        sqlite_cfg.overwrite = false;
    }
    // WATCH-08 (D-05): error log 追加模式，保留历史错误
    cfg.append_error_log = true;
}
