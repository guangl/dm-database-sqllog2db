//! Compatibility checks for the `dameng-cli` plugin contract.

use std::path::PathBuf;
use std::process::Command;

/// Minimal configuration accepted by `validate`.
const MINIMAL_CONFIG: &str =
    "[sqllog]\ninputs = [\"sqllogs\"]\n\n[exporter.csv]\nfile = \"outputs/sqllog.csv\"\n";

fn plugin_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dm-sqllog2db"));
    for variable in [
        "DM_PLUGIN_API_VERSION",
        "DM_PLUGIN_CAPABILITIES",
        "DM_PLUGIN_DIR",
        "DM_HOME",
        "DM_PLUGIN_HOME",
        "DM_PLUGIN_CONFIG_DIR",
        "DM_PLUGIN_DATA_DIR",
        "DM_PLUGIN_CACHE_DIR",
        "SQLLOG2DB_CONFIG",
    ] {
        command.env_remove(variable);
    }
    command
}

/// A plugin launched the way the host launches it, with per-plugin directories.
struct PluginHome {
    root: tempfile::TempDir,
    plugin_dir: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
    cache_dir: PathBuf,
}

impl PluginHome {
    fn new() -> Self {
        let root = tempfile::TempDir::new().unwrap();
        let plugin_dir = root.path().join("plugins/sqllog2db");
        let config_dir = root.path().join("config/sqllog2db");
        let data_dir = root.path().join("data/sqllog2db");
        let cache_dir = root.path().join("cache/sqllog2db");
        for directory in [&plugin_dir, &config_dir, &data_dir, &cache_dir] {
            std::fs::create_dir_all(directory).unwrap();
        }

        Self {
            root,
            plugin_dir,
            config_dir,
            data_dir,
            cache_dir,
        }
    }

    /// Build a fresh hosted invocation; each test run gets its own command.
    fn command(&self) -> Command {
        let mut command = plugin_command();
        command
            .env("DM_PLUGIN_API_VERSION", "1")
            .env("DM_PLUGIN_CAPABILITIES", "config-dirs-v1")
            .env("DM_PLUGIN_DIR", &self.plugin_dir)
            .env("DM_PLUGIN_HOME", self.root.path())
            .env("DM_PLUGIN_CONFIG_DIR", &self.config_dir)
            .env("DM_PLUGIN_DATA_DIR", &self.data_dir)
            .env("DM_PLUGIN_CACHE_DIR", &self.cache_dir);
        command
    }

    /// Path of the plugin's own configuration file inside the host directory.
    fn default_config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Write a custom configuration outside the host config directory.
    fn external_config(&self) -> PathBuf {
        let path = self.root.path().join("custom.toml");
        std::fs::write(&path, MINIMAL_CONFIG).unwrap();
        path
    }
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
    assert_eq!(manifest["min_host_version"].as_str(), Some("0.4.0"));

    assert_eq!(manifest["completion"].as_bool(), Some(true));
    assert!(manifest.get("permissions").is_none());
}

#[test]
fn plugin_rejects_direct_execution_without_the_host_protocol() {
    let output = plugin_command().arg("--help").output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Run this plugin through dm <plugin>")
    );
}

#[test]
fn plugin_preserves_cli_arguments_and_success_exit_code() {
    let home = PluginHome::new();
    let output = home.command().arg("--help").output().unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: dm sqllog2db"));
    assert!(stdout.contains("parsing DM database SQL logs"));
}

#[test]
fn plugin_seeds_the_host_config_file_and_reuses_it() {
    let home = PluginHome::new();
    let config_file = home.default_config_file();
    assert!(!config_file.exists());

    let output = home.command().arg("validate").output().unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));

    let generated = std::fs::read_to_string(&config_file).unwrap();
    assert!(
        generated.contains("[exporter.parquet]"),
        "the seeded file should be the default template, got: {generated}"
    );

    // A later run must use the file as-is instead of regenerating it.
    std::fs::write(&config_file, MINIMAL_CONFIG).unwrap();
    let output = home.command().arg("validate").output().unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(
        std::fs::read_to_string(&config_file).unwrap(),
        MINIMAL_CONFIG
    );
}

#[test]
fn plugin_help_reports_the_host_config_file_as_default() {
    let home = PluginHome::new();
    let output = home.command().args(["run", "--help"]).output().unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&home.default_config_file().display().to_string()),
        "run --help should advertise the host config path, got: {stdout}"
    );
}

#[test]
fn plugin_explicit_config_flag_skips_the_host_default() {
    let home = PluginHome::new();
    let custom = home.external_config();

    let output = home
        .command()
        .arg("validate")
        .arg("-c")
        .arg(&custom)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        !home.default_config_file().exists(),
        "an explicit -c must not seed the host config file"
    );
}

#[test]
fn plugin_config_environment_variable_skips_the_host_default() {
    let home = PluginHome::new();
    let custom = home.external_config();

    let output = home
        .command()
        .env("SQLLOG2DB_CONFIG", &custom)
        .arg("validate")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        !home.default_config_file().exists(),
        "SQLLOG2DB_CONFIG must not seed the host config file"
    );
}

#[test]
fn plugin_rejects_legacy_home_environment() {
    let root = tempfile::TempDir::new().unwrap();

    let output = plugin_command()
        .env("DM_PLUGIN_API_VERSION", "1")
        .env("DM_PLUGIN_CAPABILITIES", "config-dirs-v1")
        .env("DM_PLUGIN_DIR", root.path())
        .env("DM_PLUGIN_CONFIG_DIR", root.path())
        .env("DM_PLUGIN_DATA_DIR", root.path())
        .env("DM_PLUGIN_CACHE_DIR", root.path())
        .arg("--help")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Missing DM_PLUGIN_HOME"));
}

#[test]
fn explicit_host_config_path_is_never_seeded() {
    for command in ["run", "validate", "stats"] {
        for from_env in [false, true] {
            let home = PluginHome::new();
            let path = home.default_config_file();
            let mut invocation = home.command();
            invocation.arg(command);
            if from_env {
                invocation.env("SQLLOG2DB_CONFIG", &path);
            } else {
                invocation.arg("-c").arg(&path);
            }
            let _ = invocation.output().unwrap();
            assert!(!path.exists(), "{command}, from_env={from_env}");
        }
    }
}

#[test]
fn init_honors_environment_and_explicit_output_precedence() {
    let home = PluginHome::new();
    let default = home.default_config_file();
    std::fs::write(&default, "host file must survive").unwrap();
    let custom = home.external_config();
    let output = home
        .command()
        .env("SQLLOG2DB_CONFIG", &custom)
        .args(["init", "--force"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        std::fs::read_to_string(&custom)
            .unwrap()
            .contains("[exporter.parquet]")
    );
    assert_eq!(
        std::fs::read_to_string(&default).unwrap(),
        "host file must survive"
    );

    let explicit = home.root.path().join("explicit.toml");
    std::fs::write(&custom, "environment file must survive").unwrap();
    let output = home
        .command()
        .env("SQLLOG2DB_CONFIG", &custom)
        .args(["init", "-o"])
        .arg(&explicit)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(explicit.is_file());
    assert_eq!(
        std::fs::read_to_string(&custom).unwrap(),
        "environment file must survive"
    );
}

#[test]
fn plugin_completion_is_read_only_and_context_aware() {
    let home = PluginHome::new();
    std::fs::remove_dir_all(&home.config_dir).unwrap();
    std::fs::remove_dir_all(&home.data_dir).unwrap();
    std::fs::remove_dir_all(&home.cache_dir).unwrap();
    let query = |words: &[&str]| {
        let output = home
            .command()
            .env("DM_PLUGIN_CAPABILITIES", "config-dirs-v1,completion-v1")
            .env("SQLLOG2DB_CONFIG", "missing.toml")
            .env("RUST_LOG", "trace")
            .current_dir(home.root.path())
            .arg("__complete")
            .args(words)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", stderr_of(&output));
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert!(query(&[""]).contains(&"run".to_owned()));
    assert_eq!(query(&["st"]), ["stats"]);
    assert!(query(&["run", "--"]).contains(&"--input".to_owned()));
    assert!(!query(&["--quiet", "run", "--"]).contains(&"--verbose".to_owned()));
    assert!(query(&["run", "-i", "one.log", "--"]).contains(&"--input".to_owned()));
    assert_eq!(query(&["stats", "--top", ""]), Vec::<String>::new());
    assert_eq!(query(&["stats", "--from=20"]), Vec::<String>::new());
    assert_eq!(query(&["run", "--", ""]), Vec::<String>::new());
    std::fs::write(home.root.path().join("sample config.toml"), "invalid").unwrap();
    std::fs::create_dir(home.root.path().join("samples")).unwrap();
    assert_eq!(
        query(&["run", "-c", "sam"]),
        ["sample config.toml", "samples/"]
    );
    assert_eq!(
        query(&["init", "--output=sam"]),
        ["--output=sample config.toml", "--output=samples/"]
    );
    assert!(!home.config_dir.exists());
    assert!(!home.data_dir.exists());
    assert!(!home.cache_dir.exists());
    assert!(!home.root.path().join("config.toml").exists());
}
