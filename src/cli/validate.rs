use crate::config::Config;
use log::info;

pub fn handle_validate(cfg: &Config) {
    info!("SQL log inputs ({} entries):", cfg.sqllog.inputs.len());
    for (i, input) in cfg.sqllog.inputs.iter().enumerate() {
        info!("  [{i}] {input}");
    }
    println!("Configuration valid.");
}
