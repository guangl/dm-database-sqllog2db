---
phase: 18-template-chart-nesting
reviewed: 2026-05-17T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - src/cli/init.rs
  - src/cli/run.rs
  - src/cli/show_config.rs
  - src/cli/stats.rs
  - src/cli/validate.rs
  - src/config/mod.rs
  - src/exporter/csv.rs
  - src/exporter/mod.rs
  - src/exporter/sqlite.rs
  - src/main.rs
  - src/pipeline/filters.rs
  - src/pipeline/mod.rs
  - tests/integration.rs
findings:
  critical: 2
  warning: 4
  info: 2
  total: 8
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-05-17
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

The Phase 18 config migration (promoting `[pipeline.*]` subsections to top-level keys) is structurally correct — serde deserialization, the deprecated-key trap, and the CLI template strings all align properly. Integration tests cover the legacy-rejection and new-format paths adequately.

Two critical defects exist in how `template.output_sqlite_table` is handled end-to-end: the value is never validated at config load time, and when it reaches the SQL layer the identifier is used without quoting, meaning any SQLite reserved word used as a table name (`group`, `order`, `select`, `from`) will produce a runtime SQL syntax error rather than a clean validation failure.

---

## Critical Issues

### CR-01: `template.output_sqlite_table` never validated at config load time

**File:** `src/config/mod.rs` — `validate()` and `validate_and_compile()`

**Issue:** `Config::validate()` and `Config::validate_and_compile()` each call `validate_filter()`, `validate_output_fields()`, and `validate_charts()`, but there is no `validate_template()` call. The `template.output_sqlite_table` field is never checked for identifier legality during config validation. Invalid values (e.g. `"1bad"`, `"my-table"`, `"order"`) pass the `validate` command without error and only fail — with a cryptic runtime error — when `write_template_stats` is reached deep inside a run. By contrast, `exporter.sqlite.table_name` goes through `SqliteExporterConfig::validate()` which correctly enforces first-char-alphabetic. This asymmetry means `sqllog2db validate` gives a false green for an invalid template table name.

**Fix:** Add identifier validation in `config/mod.rs`. Add a `validate_template()` method and call it from both `validate()` and `validate_and_compile()`:

```rust
fn validate_template(&self) -> Result<()> {
    if let Some(tmpl) = &self.template {
        let name = tmpl.output_sqlite_table.trim();
        if !name.is_empty() {
            let mut chars = name.chars();
            let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !valid {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "template.output_sqlite_table".to_string(),
                    value: name.to_string(),
                    reason: "table name must start with a letter or underscore \
                             and contain only ASCII alphanumeric or underscore".to_string(),
                }));
            }
        }
    }
    Ok(())
}
```

Then add `self.validate_template()?;` in both `validate()` and `validate_and_compile()`.

---

### CR-02: Unquoted table name in `write_template_stats` SQL — reserved words cause syntax errors

**File:** `src/exporter/sqlite.rs` — DROP/CREATE/INSERT statements in `write_template_stats`

**Issue:** The `DROP TABLE IF EXISTS {table_name}`, `CREATE TABLE IF NOT EXISTS {table_name}`, and `INSERT INTO {table_name}` statements interpolate the table name without quoting the identifier. SQLite reserved words such as `group`, `select`, `order`, `from`, `index`, and `values` are legal identifiers when double-quoted but produce parse errors when unquoted. A user who sets `output_sqlite_table = "group"` will receive a database error at runtime instead of a config error at load time.

Note the inconsistency: the main exporter's `build_create_sql` and `build_insert_sql` use `\"{}\"` quoting throughout. The template stats path uses a different, weaker pattern.

**Fix:** Quote the table name with double quotes in all three SQL format strings:

```rust
// DROP
conn.execute(&format!("DROP TABLE IF EXISTS \"{table_name}\""), [])

// CREATE
conn.execute(
    &format!(
        "CREATE TABLE IF NOT EXISTS \"{table_name}\" \
         (template_key TEXT NOT NULL PRIMARY KEY, ...)"
    ),
    [],
)

// INSERT
conn.execute(
    &format!("INSERT INTO \"{table_name}\" VALUES (?,?,?,?,?,?,?,?,?,?)"),
    p,
)
```

---

## Warnings

### WR-01: Runtime identifier check in `write_template_stats` allows leading digits

**File:** `src/exporter/sqlite.rs` — identifier guard in `write_template_stats`

**Issue:** The guard checks only that every character is ASCII alphanumeric or underscore, but does not enforce that the first character is alphabetic or underscore. This passes `1bad_table` through. SQLite does not accept leading-digit bare identifiers. `SqliteExporterConfig::validate()` correctly rejects them with a first-char check; this runtime guard should match that stronger contract.

**Fix:** Align the runtime guard with `SqliteExporterConfig::validate()` pattern:

```rust
let mut chars = table_name.chars();
let valid_ident = chars.next()
    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
if !valid_ident {
    return Err(...);
}
```

---

### WR-02: Duplicate migration error string literal

**File:** `src/config/mod.rs` — `validate()` and `validate_and_compile()`

**Issue:** The migration guidance string `"配置格式已升级，请迁移以下字段：\n  ..."` is copy-pasted verbatim in two places. When the migration message needs updating, both sites must be changed in sync.

**Fix:** Extract as a module-level constant:

```rust
const PIPELINE_MIGRATION_MSG: &str =
    "配置格式已升级，请迁移以下字段：\n  [pipeline.template_analysis] → [template]\n  \
     [pipeline.charts] → [charts]\n  [pipeline.normalize] → [replace_parameters]\n  \
     [pipeline.filters.*] → [filter.*]\n  [pipeline.fields] → [output.fields]";
```

Then reference `PIPELINE_MIGRATION_MSG.to_string()` in both error sites.

---

### WR-03: Stale "Phase 14" comments in `run.rs`

**File:** `src/cli/run.rs` — parallel and sequential template stats write paths

**Issue:** Both the parallel and sequential paths contain the comment `// Phase 14 将消费 finalize() 结果并写出报告；此处先记录聚合摘要。` The actual template stats writing code is now implemented below these lines in Phase 18. The comment implies the write is pending future work, while the code immediately below it does the write. This actively misleads future maintainers.

**Fix:** Replace with a comment that describes what the code actually does:

```rust
// Write template stats to CSV / SQLite if configured.
```

---

### WR-04: `GroupAccumulator.count` and `BucketAccumulator.count` are `u32` — silent truncation above 4 billion records

**File:** `src/cli/stats.rs` — `GroupAccumulator` and `BucketAccumulator` structs

**Issue:** Both accumulator structs store `count: u32`, but the exported `GroupEntry.count` and `BucketEntry.count` fields are `u64`. For datasets where a single user or time bucket accumulates more than `u32::MAX` (≈4.3 billion) rows, the count silently wraps. The mismatch between the private `u32` accumulator and the public `u64` output type is a latent correctness bug.

**Fix:** Change both accumulator `count` fields to `u64` to match the output type:

```rust
struct GroupAccumulator {
    count: u64,   // was u32
    ...
}

struct BucketAccumulator {
    count: u64,   // was u32
    ...
}
```

---

## Info

### IN-01: `build_companion_path` is dead production code

**File:** `src/exporter/csv.rs`

**Issue:** `build_companion_path` is marked `#[allow(dead_code)]`. No test or production code path currently calls it; the explicit `output_csv_path` from config is used directly now. Dead code suppressed with `#[allow(dead_code)]` accumulates technical debt.

**Fix:** Remove the function if unused, or add a `#[cfg(test)]` gate and a unit test for it.

---

### IN-02: Unit tests in `run.rs` use `directory` key under `[sqllog]`

**File:** `src/cli/run.rs` — inline TOML strings in unit tests

**Issue:** The inline TOML strings in `run.rs` unit tests use `directory = "..."` under `[sqllog]`, but the canonical key is `path` (with `directory` as an alias). If the alias is removed in a future refactor, these tests will silently pass with a default-path config rather than failing noisily.

**Fix:** Update test TOML strings to use the canonical key `path = "..."` under `[sqllog]`.

---

_Reviewed: 2026-05-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
