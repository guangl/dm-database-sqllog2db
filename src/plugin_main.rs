//! `dameng-cli` plugin entry point.

use dm_plugin_sdk::{Context, Plugin, PluginResult};

struct Sqllog2dbPlugin;

impl Plugin for Sqllog2dbPlugin {
    fn run(&self, context: Context) -> PluginResult {
        if dm_database_sqllog2db::cli::completion::handle(&context.args) {
            return Ok(0);
        }
        Ok(dm_database_sqllog2db::cli::run_as_plugin())
    }
}

fn main() {
    dm_plugin_sdk::run(Sqllog2dbPlugin);
}
