---
phase: 68-init-wizard
reviewed: 2026-06-06T10:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/cli/init.rs
  - src/cli/opts.rs
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 2
  warning: 2
  info: 3
  total: 7
status: issues_found
---

# Phase 68: Code Review Report

**Reviewed:** 2026-06-06T10:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Four files were reviewed for the Phase 68 init-wizard feature. Core logic is in `src/cli/init.rs` (non-interactive `handle_init`, interactive wizard `run_wizard` + `handle_init_interactive`, and a multi-step template string substitution engine). `src/cli/opts.rs` adds the `--interactive` flag. `src/main.rs` dispatches the commands. `tests/integration.rs` provides substantial e2e coverage.

Two critical blockers were found: (1) the interactive wizard runs all prompts before checking whether the output file already exists, wasting all user input on a late-failing error; (2) user-supplied paths are embedded verbatim into the TOML template with no escaping, producing silently corrupt config files when input contains `"` or (on Windows) `\`. Two warnings cover a fragile multi-line string substitution chain and a behavioral inconsistency in the `validate` command. Three info items cover dead code, a misleading help example, and a missing test branch.

## Critical Issues

### CR-01: `handle_init_interactive` runs the full wizard before checking file existence

**File:** `src/cli/init.rs:242-256`
**Issue:** The execution order is: lock stdin/stdout → run wizard (3-4 prompts) → call `write_config_file` (which is where the `--force` and `path.exists()` check finally occurs). A user without `--force` who runs `init -i` against an already-existing file goes through the complete wizard, answers all questions, and only then receives the "Configuration file already exists" error — all input is discarded. The test at `tests/integration.rs:2775` (`test_cli_init_interactive_existing_without_force_fails`) uses `write_stdin("\n\n\n")` — it feeds all three prompts before expecting failure — which confirms the bad order rather than catching it as a bug.

**Fix:** Check path existence before entering the wizard:
```rust
pub fn handle_init_interactive(output_path: &str, force: bool) -> Result<()> {
    // Early-exit check: do not run the wizard if the file already exists and --force is not set.
    let path = std::path::Path::new(output_path);
    if path.exists() && !force {
        error!("Configuration file already exists: {output_path}");
        info!("Tip: use --force to overwrite");
        return Err(Error::File(FileError::AlreadyExists {
            path: path.to_path_buf(),
        }));
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let answers = run_wizard(&mut reader, &mut writer)?;
    let content = apply_wizard_answers_to_template(&answers);
    write_config_file(path, &content, force)?;
    // ...
    Ok(())
}
```
The remaining TOCTOU window between check and write is acceptable for a local CLI tool; `write_config_file`'s own check still acts as the final gate.

---

### CR-02: User inputs embedded in TOML template without escaping — produces silently corrupt config files

**File:** `src/cli/init.rs:187-238`
**Issue:** All three substitution helpers (`apply_wizard_answers_to_template`, `apply_csv_substitutions`, `apply_sqlite_substitutions`) embed user-supplied strings directly into TOML double-quoted strings via `format!(r#"key = "{user_value}""#)` with zero escaping. Two concrete failure modes:

1. **Double-quote in any field** — A user who enters `my"dir/out.csv` as the CSV path causes the template to produce `file = "my"dir/out.csv"`, which is a TOML syntax error. The `init` command exits 0, the file is written, and the first failure the user sees is on the subsequent `validate` or `run` command with no indication that the wizard was the cause.

2. **Backslash on Windows** — A user entering `C:\Users\logs` produces `inputs = ["C:\Users\logs"]`. TOML treats `\U` and `\l` as invalid escape sequences, making the config unparseable on Windows without forward-slash normalization. The integration tests perform explicit `replace('\\', "/")` when constructing TOML strings (e.g., `tests/integration.rs:1043`), but the wizard itself does not.

The `sqlite_table` prompt text warns "仅含字母/数字/下划线" (ASCII identifiers only) but `prompt_line` enforces no such constraint. Config::validate() would eventually catch an illegal table name, but a name containing `"` breaks the TOML before validate can even parse it.

**Fix:** Add a minimal TOML escaper and apply it to all user-controlled values before substitution:
```rust
/// Escape a user string for embedding inside a TOML basic string (double-quoted).
fn toml_escape(s: &str) -> String {
    // In TOML basic strings, backslash and double-quote must be escaped.
    // Forward-slash normalization also handles Windows paths.
    s.replace('\\', "/").replace('"', "\\\"")
}

fn apply_wizard_answers_to_template(answers: &WizardAnswers) -> String {
    let escaped_inputs = toml_escape(&answers.inputs);
    let content = CONFIG_TEMPLATE_EN.to_string().replace(
        r#"inputs = ["sqllogs"]"#,
        &format!(r#"inputs = ["{}"]"#, escaped_inputs),
    );
    // ...
}

fn apply_csv_substitutions(content: &str, answers: &WizardAnswers) -> String {
    let csv_file = answers.csv_file.as_deref().unwrap_or("outputs/sqllog.csv");
    let escaped = toml_escape(csv_file);
    content.replace(
        r#"file = "outputs/sqllog.csv""#,
        &format!(r#"file = "{escaped}""#),
    )
}
// Apply toml_escape to sqlite_db and sqlite_table as well.
```

## Warnings

### WR-01: `apply_sqlite_substitutions` uses multi-line exact string matching — silently breaks on any template edit

**File:** `src/cli/init.rs:195-228`
**Issue:** The SQLite-mode template activation chains 7 sequential `.replace()` calls, two of which match multi-line exact substrings that span a comment line and a field value:

```rust
// Match 3: depends on exact newline between "overwrite = true" and "# Append"
.replace("overwrite = true\n# Append", "# overwrite = true\n# Append")

// Match 4: depends on exact comment text, newline, field value, blank line, and section header
.replace(
    "# Append to existing CSV file instead of overwriting (true/false)\nappend = false\n\n# Option 2:",
    "# Append to existing CSV file instead of overwriting (true/false)\n# append = false\n\n# Option 2:",
)
```

Any edit to `CONFIG_TEMPLATE_EN` (rewording a comment, adding a blank line, reordering fields) silently breaks these replacements. The result is a generated SQLite config that still contains an active `[exporter.csv]` section — violating the single-exporter rule. The test `test_apply_output_parses_as_config_sqlite` validates that the output parses and passes `cfg.validate()`, but `cfg.validate()` does NOT enforce the single-exporter constraint (it just requires at least one exporter), so this specific failure mode is undetected.

**Fix:** Maintain two separate template constants (`CONFIG_TEMPLATE_CSV` and `CONFIG_TEMPLATE_SQLITE`) and select between them in `apply_wizard_answers_to_template`. This eliminates the brittle replacement chain entirely and makes each template directly readable.

---

### WR-02: `validate` command silently succeeds on nonexistent config file

**File:** `src/main.rs:178-185`
**Issue:** The `validate` subcommand calls `load_config()`, which on `ConfigError::NotFound` silently falls back to `Config::default()` with only a `warn!` log (line 216). This means:

```
$ sqllog2db validate -c /nonexistent/file.toml
# → warn: "Configuration file not found: ..., using default configuration"  [via env_logger to stderr]
# → "Configuration valid."  [exit 0]
```

The user sees "Configuration valid." for a file that was never read. This is directly inconsistent with the `stats` subcommand (line 193), which calls `Config::from_file()` directly and hard-fails on NotFound. No integration test covers `validate -c nonexistent.toml`.

**Fix:** The `validate` command should call `Config::from_file()` directly, not `load_config()`:
```rust
Some(cli::opts::Commands::Validate { config }) => {
    let cfg = Config::from_file(Path::new(config))?;  // hard-fail on NotFound
    if let Err(e) = cfg.validate() {
        eprintln!("{}", format_validate_error(&e));
        std::process::exit(EXIT_FATAL);
    }
    cli::validate::handle_validate(&cfg);
    Ok(None)
}
```

## Info

### IN-01: `init_simple_logging` uses a redundant string intermediate for boolean→LevelFilter conversion

**File:** `src/main.rs:32-43`
**Issue:** The function converts `quiet: bool` to the string `"error"` or `"info"`, then immediately matches that string to produce a `LevelFilter`. The intermediate `level: &str` variable adds a layer of indirection with no benefit and introduces a catch-all `_ =>` arm that would silently default to `Info` for any misspelled level (impossible here, but a code-smell):

```rust
// Current
let level = if quiet { "error" } else { "info" };
let filter = match level {
    "error" => log::LevelFilter::Error,
    _ => log::LevelFilter::Info,
};
```

**Fix:**
```rust
fn init_simple_logging(quiet: bool) {
    let filter = if quiet { log::LevelFilter::Error } else { log::LevelFilter::Info };
    let _ = env_logger::Builder::from_default_env()
        .filter_level(filter)
        .try_init();
}
```

---

### IN-02: Two `run` help examples show identical commands with different descriptions

**File:** `src/cli/opts.rs:12-22`
**Issue:** The `after_help` block for the `run` subcommand contains:

```
Export all records from SQL log files:
    sqllog2db run -c config.toml

Export with SQL indicators filter configured:
    sqllog2db run -c config.toml
```

Both examples are byte-for-byte identical. The second description implies a different CLI invocation (using a filter), but the command is unchanged (the filter is controlled via config file). This misleads users into thinking there should be a `--filter` flag.

**Fix:** Replace the duplicate with a meaningful example that demonstrates actual CLI variation:
```
Override input paths from the command line:
    sqllog2db run -c config.toml --input 'sqllogs/2025-*.log'
```

---

### IN-03: Missing test for `init -i --force` overwriting an existing file

**File:** `tests/integration.rs`
**Issue:** The interactive init tests cover: all-defaults success (line 2631), custom inputs (line 2658), sqlite mode (line 2680), validates-output (line 2719), format-matches-non-interactive (line 2742), and fails-without-force (line 2775). The `--force` overwrite path for interactive mode is completely absent. The equivalent test exists for non-interactive mode (`test_handle_init_force_overwrites_existing` at line 181), but not for `init -i`.

**Fix:** Add:
```rust
#[test]
fn test_cli_init_interactive_force_overwrites_existing() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");
    std::fs::write(&out_file, "old content").unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .arg("--force")
        .write_stdin("\n\n\n")
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("[sqllog]"), "force should overwrite with template config");
    assert!(!content.contains("old content"), "old content must be replaced");
}
```

---

_Reviewed: 2026-06-06T10:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
