---
phase: 28-remove-charts-update-completions
reviewed: 2026-05-20T12:00:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - benches/bench_csv.rs
  - benches/bench_filters.rs
  - benches/bench_sqlite.rs
  - Cargo.toml
  - docs/architecture.md
  - README.md
  - src/cli/init.rs
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/cli/run/mod.rs
  - src/cli/run/parallel.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/cli/show_config.rs
  - src/config/apply_one.rs
  - src/config/mod.rs
  - src/config/validate.rs
  - src/error.rs
  - src/exporter/csv/mod.rs
  - src/exporter/csv/tests.rs
  - src/exporter/mod.rs
  - src/exporter/sqlite/mod.rs
  - src/exporter/sqlite/tests.rs
  - src/exporter/tests.rs
  - src/lang.rs
  - src/lib.rs
  - src/main.rs
  - src/pipeline/mod.rs
  - src/pipeline/normalizer.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 6
  info: 6
  total: 13
status: issues_found
---

# Phase 28: Code Review Report — Remove Charts, Self-Update, Shell Completions

**Reviewed:** 2026-05-20T12:00:00Z
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Phase 28 removed SVG charts, self-update, and shell completions/man page features from the sqllog2db project. The source code removal in `src/cli/` and `src/` is largely complete. However, the following categories of issues remain:

1. **Blocker**: All three benchmark files construct TOML configs using layout that is silently ignored by the current Config struct. The filter benchmarks in `bench_filters.rs` are completely broken — they measure the unfiltered fast path for all scenarios.
2. **Stale documentation**: README.md and docs/architecture.md still contain extensive descriptions of removed features (charts, template analysis, shell completions, man page, stats/digest commands).
3. **Dead code**: `normalize_template` (260+ lines), `FileError::ReadFailed`, `open_connection_only()`, and several other items carry `#[allow(dead_code)]` rather than being removed. The `lang.rs` module has a module-wide suppression that masks which items are truly unused.
4. **Cargo.toml**: Description mentions JSONL which was also removed.

---

## Critical Issues

### CR-01: bench_filters.rs configs use `[features.filters]` instead of `[filter]` — all filter benchmarks silently measure the unfiltered fast path

**File:** `benches/bench_filters.rs:72-158`
**Issue:** All six filter benchmark scenarios construct TOML configs using the old section name `[features.filters]` (e.g., line 76: `[features.filters]\nenable = true\nstart_ts = \"2000-01-01\"`). The current `Config` struct has no `features` field and captures filters under `filter: Option<FiltersFeature>` (mapped from TOML section `[filter]`). Since `Config` does not use `#[serde(deny_unknown_fields)]`, the `[features.filters]` section is silently ignored by serde, and `cfg.filter` remains `None`.

Consequence: `cfg.validate_and_compile()` returns `None` and `build_pipeline()` builds an empty pipeline for all scenarios. The seven benchmark scenarios (`no_pipeline`, `pipeline_passthrough`, `trxid_small`, `trxid_large`, `indicator_prescan`, `exclude_passthrough`, `exclude_active`) all run identical unfiltered fast-path code. Benchmark results are meaningless for measuring filter overhead.

Even if the section name were corrected, the benchmarks also use flat old-style field names (`usernames`, `start_ts`, `trxids`, `exclude_usernames`) which would need to be placed in the correct nested sub-section (`[filter.include]`, `[filter.exclude]`).

**Fix:** Replace `[features.filters]` with `[filter]` and use nested sub-sections. Example for `cfg_pipeline_passthrough`:

```rust
fn cfg_pipeline_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true
[filter.include]
start_ts = \"2000-01-01\"
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}
```

Similar fixes are needed for all six config builders. `exclude_usernames` should move to `[filter.exclude] users`, `trxids` should move to `[filter.include] trxids`, `usernames` should move to `[filter.include] users`.

---

## Warnings

### WR-01: All three bench files use nonexistent `[error]` TOML section

**Files:**
- `benches/bench_csv.rs:38-39`
- `benches/bench_sqlite.rs:42-43`
- `benches/bench_filters.rs:49-50`

**Issue:** The benchmark TOML configs include an `[error]` section (e.g., `[error]\nfile = "{dir}/errors.log"`). The `Config` struct has no `error` field. This section is silently ignored by serde. While this has no functional impact for bench_csv and bench_sqlite, it is misleading and creates maintenance burden. At minimum, the `[error]` section reference should be removed.

**Fix:** Remove the `[error]` section from all three benchmark TOML templates.

---

### WR-02: README.md contains extensive stale references to removed features

**File:** `README.md`
**Lines:** 36-41, 44, 49, 110, 133, 136, 161, 177, 191-192

**Issue:** After removing SVG charts, shell completions, man page, and template analysis in Phase 28, the README still describes these features in detail:

- Lines 36-41: Entire "### 模板分析与图表" subsection with TemplateAggregator, hdrhistogram, four SVG chart types, configuration-driven chart generation
- Line 44: `[template]`, `[charts]` as top-level config sections
- Line 49: `completions` and `man` shell integration
- Line 110: Shell completions installation instructions
- Line 133: `sqllog2db digest` command example
- Lines 161, 191-192: References to `docs/config-reference.md` and `docs/quickstart.md`
- Line 177: Full "## 图表功能" section on SVG chart generation

These references will confuse users trying to use the current version of the tool.

**Fix:** Remove or rewrite all sections referencing removed features. The "### 模板分析与图表" subsection and the "## 图表功能" section should be fully removed. Update configuration references, remove digest/stats command examples, remove shell completions/man page references.

---

### WR-03: docs/architecture.md contains stale references to removed modules

**File:** `docs/architecture.md`
**Lines:** 26, 41, 51-52, 57, 60-61, 67, 87-97, 123-125, 135, 201, 204

**Issue:** The architecture document still describes removed features:

- Line 26: `stats` and `digest` commands
- Line 41: `[template]`, `[charts]` as top-level config sections
- Lines 51-52: `stats.rs` and `digest.rs` module listings
- Line 57: `handle_stats` handler naming pattern reference
- Lines 60-61: "无过滤器、模板或图表" in pipeline description
- Line 67: `normalize_template` in pipeline section
- Lines 87-97: Entire "### 图表层 -- src/charts/" section with chart module structure
- Lines 123-125: TemplateAggregator with hdrhistogram
- Line 135: "无过滤器、模板或图表" in performance section
- Lines 201, 204: Charts in dependency diagram

**Fix:** Remove the charts section entirely (lines 87-97), remove or replace stats/digest references, update pipeline description to remove template/chart mentions, update the dependency diagram.

---

### WR-04: Cargo.toml description mentions removed JSONL exporter

**File:** `Cargo.toml:7`

**Issue:** The package description reads: "高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV/JSONL/SQLite". The JSONL exporter was removed in an earlier phase.

**Fix:** Change to: "高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV 或 SQLite"

---

### WR-05: `normalize_template` is dead production code with `#[allow(dead_code)]`

**File:** `src/pipeline/normalizer.rs:462-463`

**Issue:** The `normalize_template` function and its entire sub-graph of ~220 lines of production code (including `scan_sql_bytes`, `dispatch_byte`, `handle_quote`, `handle_line_comment`, `handle_block_comment`, `handle_word`, `try_fold_in_list`, `skip_quoted`, `is_subquery`, `is_keyword`, `is_ident_byte`, `prev_is_ident_byte`, and the `NEEDS_SPECIAL_NORM` lookup table) are never called outside tests. The `#[allow(dead_code)]` annotation on line 462 confirms the compiler detects this. These functions were migrated from the removed `fingerprint.rs` module to support template analysis which was also removed.

**Fix:** Either:
- Remove the function and all its support code (preferred -- ~220 lines of dead production code), or
- If intentionally kept for future use, gate behind a feature flag or document with an explicit comment explaining the retention rationale

---

### WR-06: `src/lang.rs` entire module suppressed with `#![allow(dead_code)]`

**File:** `src/lang.rs:11`

**Issue:** The module has `#![allow(dead_code)]` at the top level, suppressing all dead code warnings for the entire module. While the comment (lines 9-10) explains some items are binary-crate-only, this blanket suppression masks which items are genuinely unused. A targeted approach would provide better maintainability.

**Fix:** Replace `#![allow(dead_code)]` with individual annotations on specific items that are truly unused in lib context:

```rust
// Remove #![allow(dead_code)] at line 11
// Add per-item annotations as needed:
#[allow(dead_code)]
fn from_env() -> Lang { ... }
#[allow(dead_code)]
fn from_args(args: &[String]) -> Option<Lang> { ... }
```

---

## Info

### IN-01: `error.rs:59` `FileError::ReadFailed` dead code with Phase 32 TODO

**File:** `src/error.rs:59`

**Issue:** `FileError::ReadFailed` is marked `#[allow(dead_code)] // TODO: Phase 32 统一清理`. This variant is never constructed in production code. The TODO from a previous phase remains unresolved.

**Fix:** Remove the `ReadFailed` variant and its `#[allow(dead_code)]` annotation.

---

### IN-02: `exporter/sqlite/mod.rs:131` `open_connection_only()` is dead code

**File:** `src/exporter/sqlite/mod.rs:131`

**Issue:** `SqliteExporter::open_connection_only()` has `#[allow(dead_code)]`. Its doc comment says it was intended for "写入模板统计的场景" -- but template statistics were removed. This function is never called in production code.

**Fix:** Remove the function.

---

### IN-03: `color.rs:9` `init()` function has spurious `#[allow(dead_code)]`

**File:** `src/color.rs:9`

**Issue:** `pub fn init(no_color: bool)` is marked `#[allow(dead_code)]` with a comment that it is "仅在 binary crate (main.rs) 中调用". However, it IS called at `src/main.rs:121`, and `main.rs` is in the same crate. The annotation appears to be a false positive. Try removing it to verify.

**Fix:** Remove the `#[allow(dead_code)]` annotation and verify the code compiles.

---

### IN-04: `pipeline/filters/compiled.rs:216` `CompiledSqlFilters::has_filters()` is dead code

**File:** `src/pipeline/filters/compiled.rs:216`

**Issue:** `CompiledSqlFilters::has_filters()` is marked `#[allow(dead_code)]`. SQL-level filter checking is done via `SqlFilters::has_filters()` on the config side before compilation. This compiled variant is unused.

**Fix:** Remove the method or verify actual usage and remove the annotation.

---

### IN-05: `validate.rs` uses Chinese messages in `handle_validate` while rest of the codebase uses English log messages

**File:** `src/cli/validate.rs:6-60`

**Issue:** All `info!()` calls in `handle_validate` use Chinese-language strings (e.g., "SQL日志输入路径", "日志级别", "日志文件") while the rest of the codebase (e.g., `src/main.rs`, `src/exporter/mod.rs`) uses English log messages. This is inconsistent but not functionally wrong.

**Fix:** Either translate to English for consistency, or add language-aware log message selection.

---

### IN-06: `logging.rs` comments reference `lib crate` but the module is `pub(crate)`

**File:** `src/logging.rs:12, 28, 157`

**Issue:** Comments on the dead-code-suppressed items say "仅在 binary crate (main.rs) 和 `#[cfg(test)]` 中使用；lib crate 生产代码不直接引用". The `logging` module is declared as `pub(crate) mod logging` in `src/lib.rs`, so there is no separate lib/binary distinction within this crate. The concept of "lib crate production code" does not apply to items in the same crate. These comments are misleading.

**Fix:** Remove or rephrase the "lib crate" references since everything is in the same crate with `pub(crate)` visibility.

---

_Reviewed: 2026-05-20T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
