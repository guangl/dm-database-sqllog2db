//! Compatibility checks for the `dameng-cli` plugin contract.

use std::process::Command;

fn plugin_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dm-sqllog2db"))
}

#[test]
fn plugin_manifest_matches_the_cargo_package() {
    let manifest: toml::Value = toml::from_str(include_str!("../dm-plugin.toml")).unwrap();

    assert_eq!(
        manifest["name"].as_str(),
        Some("sqllog2db"),
        "the manifest name determines the dm subcommand and binary name"
    );
    assert_eq!(
        manifest["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        manifest["api_version"].as_integer(),
        Some(i64::from(dm_plugin_sdk::API_VERSION))
    );
}

#[test]
fn plugin_rejects_direct_execution_without_the_host_protocol() {
    let output = plugin_command()
        .env_remove("DM_PLUGIN_API_VERSION")
        .env_remove("DM_PLUGIN_CAPABILITIES")
        .env_remove("DM_PLUGIN_DIR")
        .env_remove("DM_HOME")
        .env_remove("DM_PLUGIN_CONFIG_DIR")
        .env_remove("DM_PLUGIN_DATA_DIR")
        .env_remove("DM_PLUGIN_CACHE_DIR")
        .arg("--help")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Run this plugin through dm <plugin>")
    );
}

#[test]
fn plugin_preserves_cli_arguments_and_success_exit_code() {
    let root = tempfile::TempDir::new().unwrap();
    let plugin_dir = root.path().join("plugins/sqllog2db");
    let config_dir = root.path().join("config/sqllog2db");
    let data_dir = root.path().join("data/sqllog2db");
    let cache_dir = root.path().join("cache/sqllog2db");
    for directory in [&plugin_dir, &config_dir, &data_dir, &cache_dir] {
        std::fs::create_dir_all(directory).unwrap();
    }

    let output = plugin_command()
        .env("DM_PLUGIN_API_VERSION", "1")
        .env("DM_PLUGIN_CAPABILITIES", "config-dirs-v1")
        .env("DM_PLUGIN_DIR", plugin_dir)
        .env("DM_HOME", root.path())
        .env("DM_PLUGIN_CONFIG_DIR", config_dir)
        .env("DM_PLUGIN_DATA_DIR", data_dir)
        .env("DM_PLUGIN_CACHE_DIR", cache_dir)
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: dm sqllog2db"));
    assert!(stdout.contains("parsing DM database SQL logs"));
}
