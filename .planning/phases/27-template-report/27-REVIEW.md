---
phase: 27-template-report
reviewed: 2026-05-19T12:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/pipeline/mod.rs
  - src/pipeline/template_reporter.rs
  - src/config/mod.rs
  - src/cli/run/mod.rs
  - src/exporter/csv/mod.rs
findings:
  critical: 0
  warning: 5
  info: 2
  total: 7
status: issues_found
---

# Phase 27: Code Review Report — Template Reporter

**Reviewed:** 2026-05-19T12:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Phase 27 adds `TemplateReporter` (CSV + SQLite template report output), `TemplatesReportConfig`, and wires it into `handle_run()` alongside the existing `[template]` companion path. The code is generally correct — no security vulnerabilities, data loss risks, or crashes were found. However, there are five warnings around code duplication, unused re-exports, and missing defense-in-depth checks, plus two info items for test maintenance debt.

## Warnings

### WR-01: TemplateReporter::write_csv duplicates companion::write_companion_rows

**File:** `src/pipeline/template_reporter.rs:14-101`
**File:** `src/exporter/csv/companion.rs:20-80`
**Issue:** `TemplateReporter::write_csv` independently implements the same CSV format (identical header, identical data row layout, identical `write_csv_escaped` + `itoa` usage) as `companion::write_companion_rows` / `companion::format_companion_row`. The code appears to have been copy-pasted with the same structure, same buffer sizes, same error handling style. This violates DRY — any future change to the CSV column set, order, or formatting must be made in two places, and they will diverge.

**Fix:** Have `TemplateReporter::write_csv` delegate to `write_companion_rows`, or extract the shared formatting logic into a common helper in `crate::exporter::csv`.

```rust
// In template_reporter.rs, replace the entire body of write_csv with:
crate::exporter::csv::write_companion_rows(path, stats)
```

Note: this requires making `write_companion_rows` `pub(crate)` instead of `pub(crate)` (it already is) — but it does require the `crate::exporter::csv` module path to be accessible from the pipeline crate. If module visibility is a concern, extract a shared `format_template_stats_csv` helper into `crate::pipeline` or `crate::exporter`.

---

### WR-02: Template report path derivation duplicated between parallel and sequential branches

**File:** `src/cli/run/mod.rs:146-199` (parallel branch) and `:289-332` (sequential branch)
**Issue:** Approximately 50 lines of nearly identical logic is duplicated: `derive_template_report_paths`, user override resolution, `templates_report_enabled` check, `TemplateReporter::write_csv`/`write_sqlite` calls. The two blocks differ only in the `else` branch (parallel opens its own `SqliteExporter`; sequential uses `exporter_manager.write_template_stats`). This duplication means any future change to report path resolution or writing must be applied twice.

**Fix:** Extract the shared template report writing logic into a helper function. For example:

```rust
fn write_template_reports(
    cfg: &Config,
    stats: &[TemplateStats],
) -> Result<()> {
    if !templates_report_enabled(cfg) {
        return Ok(());
    }
    let (derived_csv, derived_sqlite) = derive_template_report_paths(cfg);
    let csv_path = cfg.templates.as_ref()
        .and_then(|t| if t.csv_report_path.trim().is_empty() { None } else { Some(PathBuf::from(&t.csv_report_path)) })
        .or(derived_csv);
    let sqlite_path = cfg.templates.as_ref()
        .and_then(|t| if t.sqlite_report_path.trim().is_empty() { None } else { Some(PathBuf::from(&t.sqlite_report_path)) })
        .or(derived_sqlite);
    if let Some(ref path) = csv_path {
        TemplateReporter::write_csv(path, stats)?;
    }
    if let Some(ref path) = sqlite_path {
        TemplateReporter::write_sqlite(path, stats)?;
    }
    Ok(())
}
```

Then call it once from each branch instead of duplicating the inline logic.

---

### WR-03: Misleading doc comment on TemplatesReportConfig::enabled

**File:** `src/pipeline/mod.rs:155`
**Issue:** The doc comment says `"默认 true，跟随 template.enable"` ("default true, follows template.enable"). However, the implementation does NOT follow `template.enable` — it uses `#[serde(default = "default_true")]` which hardcodes `true`, a static value that does not depend on any other config field. The actual behavior does not match the documented behavior.

In practice, this causes no functional bug because `templates_report_enabled()` checks for the `[templates]` section's presence, and `enabled` only matters when the section exists. But the comment will mislead future maintainers.

**Fix:** Correct the doc comment to match the actual behavior:

```rust
/// 是否启用独立模板报告（默认 true；仅在显式配置 `[templates]` 段时生效）
```

---

### WR-04: Unused re-export with `#[allow(unused_imports)]`

**File:** `src/pipeline/mod.rs:18-19`
**Issue:** The re-export `pub(crate) use template_reporter::TemplateReporter;` is suppressed by `#[allow(unused_imports)]`, meaning the re-export is genuinely unused. The only consumer (`src/cli/run/mod.rs:6`) imports `TemplateReporter` via the full module path `crate::pipeline::template_reporter::TemplateReporter`, bypassing the re-export. This hides a dead code path behind a lint suppression.

**Fix:** Either (a) change the import in `cli/run/mod.rs` to use the re-export (`crate::pipeline::TemplateReporter`) and remove the `#[allow(unused_imports)]`, or (b) remove the re-export entirely and keep the direct path.

---

### WR-05: Missing `PRAGMA foreign_keys = ON` in template reporter SQLite

**File:** `src/pipeline/template_reporter.rs:119-130`
**Issue:** The `write_sqlite` function creates three tables with foreign key relationships (`template_stats.template_key_id REFERENCES template_keys(id)`, `latency_percentiles.template_key_id REFERENCES template_keys(id)`) but does not set `PRAGMA foreign_keys = ON`. SQLite defaults to NOT enforcing foreign keys. While the current INSERT order is correct (template_keys first, then stats/percentiles), this missing enforcement means any future code change that disrupts the insert order or references will silently produce orphaned rows.

**Fix:** Add `PRAGMA foreign_keys = ON;` to the pragma batch at line 119:

```rust
conn.execute_batch(
    "PRAGMA journal_mode = OFF;
     PRAGMA synchronous = OFF;
     PRAGMA cache_size = 1000000;
     PRAGMA temp_store = MEMORY;
     PRAGMA page_size = 65536;
     PRAGMA foreign_keys = ON;",
)
```

Note: Adding FK enforcement should be tested; if the existing DELETE order at line 158-160 fails under `foreign_keys = ON` (because `template_keys` is deleted before its children are deleted first), the DELETE order must be `latency_percentiles` -> `template_stats` -> `template_keys`, which it already is, so this should be safe.

---

## Info

### IN-01: Outdated test assertion scope

**File:** `src/config/mod.rs:280-288`
**Issue:** The test `test_config_has_5_top_level_optional_fields` checks 5 fields (`replace_parameters`, `template`, `filter`, `charts`, `output`) but the `Config` struct now has 6 optional fields (adding `templates`). The test still passes because the 6th field defaults to `None` via `#[serde(default)]`, but the test name and coverage are outdated. This is a maintenance signal — if someone later changes the default for `templates`, this test will not catch the regression.

**Fix:** Add `assert!(cfg.templates.is_none());` to the test and update the name to `test_config_has_6_top_level_optional_fields`.

---

### IN-02: Inconsistent template stats writing between parallel and sequential paths

**File:** `src/cli/run/mod.rs` (parallel branch `:191-198` vs sequential branch `:331`)
**Issue:** The parallel path's old-`[template]` fallback bypasses `ExporterManager` to write companion stats: it calls `write_companion_rows` directly and creates its own `SqliteExporter` connection (via `SqliteExporter::from_config` + `open_connection_only`). The sequential path uses `exporter_manager.write_template_stats(...)`. While both paths produce the same output (separate CSV companion file + SQLite table in the main database), the parallel path's approach is architecturally inconsistent: it opens a second connection to the main database instead of reusing the exporter abstraction. This is not a bug (the parallel path doesn't have a main `SqliteExporter` to reuse), but it couples the parallel path to the specific behavior of `SqliteExporter::open_connection_only` and `write_template_stats`, making refactoring harder.

**Suggested improvement:** If the parallel path ever needs to support SQLite as the main exporter, this inconsistency would block it. Consider whether `process_csv_parallel` should return an `ExporterManager` for fallback writes, or whether template companion stats in the parallel path could be deferred to a finalization step that constructs a proper exporter.

---

_Reviewed: 2026-05-19T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
