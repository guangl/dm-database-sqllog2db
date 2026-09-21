//! `dameng-cli` plugin entry point.

use dm_plugin_sdk::{Context, Plugin, PluginResult};

struct Sqllog2dbPlugin;

impl Plugin for Sqllog2dbPlugin {
    fn run(&self, _context: Context) -> PluginResult {
        Ok(dm_database_sqllog2db::cli::run_as_plugin())
    }
}

fn main() {
    dm_plugin_sdk::run(Sqllog2dbPlugin);
}
