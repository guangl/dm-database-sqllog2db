---
phase: 50-sql
reviewed: 2026-06-01T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/cli/stats/mod.rs
  - src/exporter/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/stats/aggregate.rs
  - src/stats/mod.rs
  - src/stats/normalize.rs
  - src/stats/output.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 50–52: Code Review Report

**Reviewed:** 2026-06-01  
**Depth:** standard  
**Files Reviewed:** 11  
**Status:** issues_found

## Summary

Reviewed the SQL normalization (Phase 50), stats CLI command (Phase 51), and stats exporter (Phase 52) across all 11 changed files. The implementation is generally sound — the streaming accumulator, CSV/SQLite write paths, and CLI wiring all work correctly. Three warnings were found: the `stats` subcommand bypasses config validation entirely (allowing malformed configs to proceed silently), the timestamp column in `slow_sql.csv` is written without RFC 4180 quoting (inconsistent with the quoted `sql_text` column), and `normalize_sql` leaves the leading minus sign unabsorbed when replacing negative number literals. Two info items address deferred `--top` validation and unnecessarily wide public API visibility.

Prior-review findings (WR-01 through IN-03) are intentionally excluded.

## Warnings

### WR-04: `stats` subcommand skips `cfg.validate()` — malformed configs accepted silently

**File:** `src/main.rs:179-184`  
**Issue:** The `Run` command path calls `cfg.validate_and_compile()?` and the `Validate` command calls `cfg.validate()`, both rejecting invalid config before any processing begins. The `Stats` command path does neither:

```rust
Some(cli::opts::Commands::Stats { config, top }) => {
    let mut cfg = Config::from_file(Path::new(config))?;   // ← no validate()
    apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
    logging::init_logging(&cfg.logging, false)?;
    cli::stats::handle_stats(&cfg, *top, cli.quiet)?;
    Ok(None)
}
```

As a result, configs that would be rejected by `validate_and_compile()` — including legacy `[pipeline.*]` sections (silently ignored by TOML deserialization), the deprecated `sqllog.path` key, and empty `csv.file` strings — are accepted without error. The invalid log level case is incidentally caught by `init_logging → parse_log_level`, but all other validation gates are absent. A user migrating from an old config format could successfully run `stats` against a config that `run` refuses, producing misleading output or a silent downstream failure when the empty `csv.file` path is used.

**Fix:** Add a validation call immediately after `from_file`, matching the `Validate` command pattern:

```rust
Some(cli::opts::Commands::Stats { config, top }) => {
    let mut cfg = Config::from_file(Path::new(config))?;
    cfg.validate()?;   // ← add this
    apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
    logging::init_logging(&cfg.logging, false)?;
    cli::stats::handle_stats(&cfg, *top, cli.quiet)?;
    Ok(None)
}
```

---

### WR-05: `timestamp` column written unquoted in `slow_sql.csv` — inconsistent RFC 4180 formatting

**File:** `src/stats/output.rs:38-39`  
**Issue:** In `write_slow_csv`, the `sql_text` field is wrapped in double quotes and run through `write_csv_escaped`, but the `timestamp` field is written as raw bytes with no quoting:

```rust
line_buf.push(b'"');
write_csv_escaped(&mut line_buf, row.sql_text.as_bytes());  // quoted
line_buf.push(b'"');
line_buf.push(b',');
line_buf.extend_from_slice(itoa::Buffer::new().format(row.elapsed_ms).as_bytes()); // numeric, ok
line_buf.push(b',');
line_buf.extend_from_slice(row.timestamp.as_bytes());  // ← unquoted, contains space
line_buf.push(b'\n');
```

The DM log timestamp format is `"2025-01-15 10:30:28.001"` — it contains an embedded space. Writing it unquoted is technically readable by most CSV tools since it contains no comma, but it violates RFC 4180 (which requires quoting fields containing special characters including spaces per common extension), produces inconsistent column quoting within the same file, and will cause problems with strict CSV parsers that treat a bare space as a record separator in some locales.

**Fix:** Quote the timestamp field consistently with `sql_text`:

```rust
line_buf.push(b',');
line_buf.push(b'"');
line_buf.extend_from_slice(row.timestamp.as_bytes());
line_buf.push(b'"');
line_buf.push(b'\n');
```

Since timestamps never contain double quotes (fixed DM format), no escaping is needed.

---

### WR-06: `normalize_sql` leaves leading minus sign unabsorbed in negative number literals

**File:** `src/stats/normalize.rs:33-37`  
**Issue:** The byte-level scanner only replaces digit sequences, not the preceding `-` operator. When a negative number literal appears (e.g., `elapsed > -100`), the minus sign is emitted literally and only the digit sequence is replaced:

```
Input:  "WHERE elapsed > -100"
Output: "WHERE elapsed > -?"
```

This means `WHERE elapsed > -100` and `WHERE elapsed > 100` normalize to `"WHERE elapsed > -?"` and `"WHERE elapsed > ?"` respectively — two distinct keys in the `freq_map` `HashMap`. SQL patterns that differ only in the sign of a numeric literal will never be grouped together by the frequency aggregator. In a production DM log where the same query template alternates between positive and negative parameter values, this produces artificially split frequency counts and misleading `avg_elapsed_ms` values across two entries rather than one.

**Root cause:** In `normalize_sql`, the `b'-'` byte falls into the default arm (not a digit, not a quote):

```rust
_ => {
    output.push(byte);                          // '-' is pushed literally
    prev_was_ident_char = byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$';
    // false for '-', so prev_was_ident_char = false
    cursor += 1;
}
```

Then the next digit character correctly triggers number replacement, but the minus sign is already committed to output.

**Fix:** Detect `b'-'` (and optionally `b'+'`) followed by a digit when `!prev_was_ident_char`, and consume both as part of the number literal. The simplest approach is to check lookahead in the digit branch:

```rust
byte_val if byte_val.is_ascii_digit() && !prev_was_ident_char => {
    cursor = skip_number_literal(bytes, cursor, len);
    output.push(b'?');
    prev_was_ident_char = false;
}
b'-' | b'+' if !prev_was_ident_char
    && cursor + 1 < len
    && bytes[cursor + 1].is_ascii_digit() =>
{
    // consume sign + digits as a single literal
    cursor = skip_number_literal(bytes, cursor + 1, len);
    output.push(b'?');
    prev_was_ident_char = false;
}
```

Alternatively, if sign-absorption is intentionally out of scope, document it in the function's doc comment so callers understand the grouping limitation.

---

## Info

### IN-04: `--top 0` validation deferred to runtime rather than clap parse time

**File:** `src/cli/opts.rs:143-147`  
**Issue:** The `--top` argument is typed as `u32` with no `value_parser` range constraint. The doc comment says "Must be >= 1" but clap does not enforce this — `0` parses successfully as `u32`, and the error is produced much later by `handle_stats`. As a result, the user sees a `[CRITICAL] Configuration error` message (from `Error::Config(ConfigError::InvalidValue)`) rather than a clap-style `error: invalid value '0' for '--top'` at the argument parsing stage.

**Fix:** Add a range constraint to enforce the invariant at parse time:

```rust
#[arg(
    long = "top",
    default_value = "20",
    value_parser = clap::value_parser!(u32).range(1..),
    help = "Number of top records per table. Must be >= 1."
)]
top: u32,
```

This produces a uniform clap error and eliminates the need for the redundant `if top == 0` guard in `handle_stats`.

---

### IN-05: `write_csv_stats` and `write_sqlite_stats` are `pub` but only used crate-internally

**File:** `src/stats/output.rs:15, 74`  
**Issue:** Both functions are declared `pub`, making them part of the crate's external API (they are reachable via `dm_database_sqllog2db::stats::output::write_csv_stats`). They are only called from `src/stats/mod.rs` (within the same crate). Exporting them publicly widens the API surface unnecessarily — external consumers of the library could depend on them, creating a maintenance burden if the CSV/SQLite output format needs to change.

Note: `normalize_sql` is deliberately `pub` (documented in `stats/mod.rs` as the public API). The output functions do not appear to have the same intent.

**Fix:** Change to `pub(crate)`:

```rust
pub(crate) fn write_csv_stats(...) -> Result<()> { ... }
pub(crate) fn write_sqlite_stats(...) -> Result<()> { ... }
```

---

_Reviewed: 2026-06-01_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
