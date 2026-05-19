# Architecture Research

**Domain:** Rust CLI — streaming log parser with pipeline filtering and multi-format export
**Researched:** 2026-05-10
**Confidence:** HIGH (all findings from direct source code inspection)

## Standard Architecture

### System Overview

```
CLI entry (main.rs)
    ↓ Config::from_file() + apply_overrides() + validate()
    ↓ logging::init_logging()
    ↓ cli::run::handle_run()
         ↓
    [Optional pre-scan phase]
    scan_for_trxids_by_transaction_filters()  ← rayon par_iter across files
         ↓ merge_found_trxids() → FiltersFeature.meta.trxids
         ↓
    build_pipeline()
         └→ FilterProcessor::new(&FiltersFeature)
               ├─ CompiledMetaFilters::from_meta()  ← regex compilation
               └─ CompiledSqlFilters::from_sql_filters()  ← separate, record-level

    [Main streaming pass]
    LogParser::from_path()
    parser.iter()  (or par_iter() for parallel CSV)
         ↓ each record
    pipeline.is_empty()?
    ├─ YES → fast path, no parse_meta()
    └─ NO  → record.parse_meta() → pipeline.run_with_meta()
                  └→ FilterProcessor::process_with_meta()
                        ├─ time range (string compare, no regex)
                        ├─ CompiledMetaFilters::should_keep()  ← AND semantics
    [SQL record-level filter — applied separately outside pipeline]
    sql_record_filter.matches(pm.sql)
         ↓ passes
    ExporterManager::export_one_preparsed()
         └→ CsvExporter or SqliteExporter
```

### Component Responsibilities

| Component | Responsibility | Key Detail |
|-----------|----------------|------------|
| `FiltersFeature` (config) | Deserialization, validation dispatch, pre-scan detection | Has deprecated `should_keep()` with OR semantics; hot path uses compiled structs |
| `CompiledMetaFilters` | Pre-compiled regex filters, AND semantics across fields | Built once at startup from `MetaFilters`; `has_filters` bool pre-cached |
| `CompiledSqlFilters` | Pre-compiled regex for `record_sql` include/exclude | Built in `handle_run()`, passed separately — NOT in pipeline |
| `FilterProcessor` | `LogProcessor` impl, wraps `CompiledMetaFilters` + time range | Lives inside `Pipeline`; reuses caller's `MetaParts` to avoid re-parse |
| `Pipeline` | Ordered list of `LogProcessor` impls; `is_empty()` fast path | `run_with_meta()` short-circuits on first false |
| `SqlFilters` | Literal substring match for transaction-level pre-scan `sql` field | NOT regex — field name misleading, code comment warns about this |
| `ExporterManager` | Enum dispatch to CsvExporter/SqliteExporter/DryRunExporter | Eliminates vtable overhead on hot path |
| `SqliteExporter` | Parameterized INSERT via `prepare_cached()`; batch COMMIT | SQL built dynamically in `initialize()` from `ordered_indices` |

## Current Data Flow for Filters

### Startup Sequence (relevant to PERF-11)

```
main.rs: run()
  1. clap arg parsing (lang detection + CommandFactory + from_arg_matches)
  2. color::init()
  3. check_for_updates_at_startup()          ← network I/O at startup
  4. load_config()                            ← fs::read_to_string + toml::from_str
  5. cfg.apply_overrides()
  6. cfg.validate()
       └→ filters.validate_regexes()          ← Regex::new() per pattern (validates only)
  7. logging::init_logging()
  8. preflight::check()
  9. handle_run():
       a. SqllogParser::new().log_files()     ← directory scan / glob
       b. [optional] scan_for_trxids...()    ← pre-scan, regex built inside
       c. build_pipeline()
            └→ FilterProcessor::new()
                 └→ CompiledMetaFilters::from_meta()  ← Regex::new() AGAIN (compiles)
                 └→ has_meta_filters bool
       d. CompiledSqlFilters::from_sql_filters()       ← Regex::new() AGAIN
```

**PERF-11 observation:** Regex patterns are validated (compiled) once in `validate_regexes()`,
then compiled AGAIN in `CompiledMetaFilters::from_meta()` and `CompiledSqlFilters::from_sql_filters()`.
This is two full compilations per pattern. The compiled `Regex` objects from validation are discarded.

### Hot Loop (process_log_file)

```
for result in parser.iter():
  if pipeline.is_empty():
    passes = true   // zero overhead — no parse_meta()
  else:
    meta = record.parse_meta()   // shared with FilterProcessor
    ok = pipeline.run_with_meta(&record, &meta)
    // FilterProcessor reuses meta, no second parse_meta()

  if passes:
    pm = record.parse_performance_metrics()  // skipped if !include_pm (D-05/D-06)
    if sql_record_filter.matches(pm.sql):    // applied OUTSIDE pipeline
      normalized = compute_normalized(...)
      exporter_manager.export_one_preparsed(...)
```

## FILTER-03: Exclusion Mode Integration

### Current State

`SqlFilters` (transaction-level, literal match) already has `exclude_patterns` — implemented and working.

`CompiledSqlFilters` (`record_sql`, regex match) also has `exclude_patterns` — implemented and working.

`CompiledMetaFilters` (metadata fields: username, ip, sess, thrd, stmt, app, tag, trxid) has **no exclusion** concept. All fields are include-only.

### What FILTER-03 Means

FILTER-03 is about adding exclusion patterns for **metadata fields** — i.e., "discard records where username matches X" or "discard records where ip matches Y". The SQL-level exclusion already exists.

### Integration Design

**Option A (recommended): Add exclude fields to `MetaFilters` config + `CompiledMetaFilters`**

Add parallel `exclude_*` fields alongside existing include fields in `MetaFilters`:

```toml
[features.filters]
enable = true
usernames = ["^admin"]          # include: keep if any match
exclude_usernames = ["^sys"]    # exclude: discard if any match
```

In `CompiledMetaFilters`, add corresponding `exclude_*: Option<Vec<Regex>>` fields.

In `should_keep()`, evaluate exclusions after inclusions. Short-circuit order:
1. Time range (no struct needed)
2. Exclusion check — if ANY exclude pattern matches ANY field → discard immediately
3. Inclusion check — all configured include fields must match

**Exclusion before inclusion** is the correct order: exclusion is a hard veto, more likely to be a small set of patterns.

```rust
// In CompiledMetaFilters::should_keep():
// Step 1: Exclusion (discard immediately if matches)
if self.exclude_usernames.as_deref().is_some_and(|p| p.iter().any(|re| re.is_match(meta.user))) {
    return false;
}
// ... other exclude_* fields ...

// Step 2: Inclusion (existing AND logic)
if !match_any_regex(self.usernames.as_deref(), meta.user) { return false; }
// ...
```

**Option B: Separate ExcludeProcessor in pipeline**

Add a second `Box<dyn LogProcessor>` after `FilterProcessor`. Cleaner separation but adds pipeline overhead (second pass through `processors.iter().all()`, second `process_with_meta()` dispatch). The fast path (`pipeline.is_empty()`) is already lost once any filter is configured, so marginal overhead is small. However, it doubles config surface and complicates the mental model.

**Recommendation: Option A.** Exclusion is semantically part of the same filter evaluation; users expect `exclude_usernames` to live beside `usernames`, not as a separate feature block.

### Config Surface (new fields)

```toml
[features.filters]
enable = true
# existing include fields unchanged
usernames = ["^admin"]

# new exclude_* fields (FILTER-03)
exclude_usernames = ["^sys_"]
exclude_client_ips = ["^10\\.0\\."]
exclude_sess_ids = []
exclude_thrd_ids = []
exclude_statements = []
exclude_appnames = []
exclude_tags = []
# exclude_trxids uses exact match (no regex), consistent with trxids
exclude_trxids = ["99999"]
```

### Files to Modify for FILTER-03

| File | Change | Type |
|------|--------|------|
| `src/features/filters.rs` | Add `exclude_*` fields to `MetaFilters`; add `exclude_*` `Option<Vec<Regex>>` to `CompiledMetaFilters`; update `from_meta()`, `should_keep()`, `has_filters()` | **MODIFY** |
| `src/features/filters.rs` | Add `validate_regexes()` entries for `exclude_*` fields | **MODIFY** |
| `src/features/mod.rs` | No changes needed | — |
| `src/cli/run.rs` | No changes needed (exclusion runs inside `FilterProcessor`) | — |

## PERF-10: Hot Path Optimization

### Current Hot Path Analysis

From source inspection:

1. **`parse_meta()` sharing** — already done. Pipeline reuses caller's `MetaParts`, no double parse.
2. **`parse_performance_metrics()` skipping** — already done via `include_pm` flag (D-05/D-06).
3. **`pipeline.is_empty()` fast path** — already done.
4. **`sql_record_filter` outside pipeline** — correct; avoids spurious `parse_performance_metrics()` for filtered records.

**Remaining opportunities:**

- `record.tag.is_some() && !f.matches(pm.sql)` check in hot loop: `pm` is parsed before this check. If `record_sql` filter is configured and `include_pm = false`, `pm.sql` comes from `record.body()` (already cheaply available), so `parse_performance_metrics()` is skipped correctly. This branch is already handled.

- `pb_pending` batched progress bar updates (every 4096 records) — already done.

- `interrupted` check every 1024 records via `trailing_zeros() >= 10` — already done.

- **Regex engine**: `regex` crate uses a DFA/NFA hybrid. For simple patterns (e.g., exact string `SYSDBA`), `regex::Regex::is_match()` has higher overhead than `str::contains()`. Could selectively downgrade patterns that contain no regex metacharacters to literal `str::contains()`. Requires detection of "no metacharacters" at compile time. Moderate complexity, potential 10-30% speedup for literal-pattern-heavy configs.

- **`CompiledMetaFilters::should_keep()` field ordering**: Currently checks `usernames`, `client_ips`, `sess_ids`, `thrd_ids`, `statements`, `appnames`, trxids, `tags`. Hottest filter to put first is the one most likely to reject early. Cannot be statically optimized without runtime statistics. Not worth changing unless profiling shows a specific field dominates.

- **`params_buffer.clear()` per file**: Already `clear()` reuses allocation. No improvement possible.

### Files to Analyze for PERF-10

Profiling gate: need flamegraph on real 1.1GB file with filters enabled. Without profiling data, the only safe PERF-10 action is the literal-vs-regex detection for metadata patterns.

## PERF-11: CLI Startup / Config Loading

### Startup Cost Breakdown

From `main.rs` analysis, the startup sequence for `run` command:

1. **clap arg parse**: One-time, unavoidable. ~1-5ms typical.
2. **`check_for_updates_at_startup()`**: **Network I/O** — DNS + HTTP request. Could be 10-500ms. Already guarded by `!cli.quiet`. Could be moved to background thread.
3. **`Config::from_file()`**: `fs::read_to_string` + `toml::from_str`. TOML parsing of a small file (~2KB) is <1ms.
4. **`cfg.validate()` → `validate_regexes()`**: Calls `Regex::new()` for each pattern to validate. For N patterns, N regex compilations.
5. **`CompiledMetaFilters::from_meta()`** in `build_pipeline()`: N more regex compilations for the SAME patterns. This is the **double-compile problem**.
6. **`CompiledSqlFilters::from_sql_filters()`**: Same — compiled again.
7. **`SqllogParser::log_files()`**: Directory traversal + sorting. Usually <5ms.

### Optimization: Eliminate Double Regex Compilation

**Problem**: `validate_regexes()` compiles then discards. `from_meta()` compiles again.

**Fix**: Cache compiled regexes from validation. Two approaches:

**Approach A: validate() returns compiled structs (recommended)**

Change `validate()` to return pre-built `CompiledMetaFilters` and `CompiledSqlFilters`, stored on `Config` or passed into `handle_run()`. Avoids any duplication.

```rust
// config.rs
impl Config {
    pub fn validate_and_compile(&self) -> Result<CompiledFilters> {
        // validate + compile in one pass
    }
}
```

This changes the `handle_run` signature slightly but eliminates double compilation entirely.

**Approach B: lazy_static / OnceLock on FiltersFeature**

Add `OnceLock<CompiledMetaFilters>` to `FiltersFeature`. Shared between validate and build_pipeline calls. More complex, requires `Arc` for thread safety in parallel CSV path.

**Recommendation: Approach A.** Simpler, no shared state. Parallel CSV path creates per-thread pipelines anyway, so compiled structs need to be `Clone` or rebuilt — `Regex` is `Clone`, so compiled structs can be cloned cheaply (shared `Arc<Regex>` inside).

**Optimization: Background update check**

Move `check_for_updates_at_startup()` to a background thread with `std::thread::spawn`. Startup proceeds immediately; update notification printed after run completes or asynchronously.

```rust
let update_handle = if !cli.quiet && !is_update_or_completions_cmd {
    Some(std::thread::spawn(check_for_updates_at_startup))
} else {
    None
};
// ... rest of startup ...
// at end: update_handle.map(|h| h.join())
```

### Files to Modify for PERF-11

| File | Change | Type |
|------|--------|------|
| `src/config.rs` | Add `validate_and_compile()` returning `CompiledFilters` wrapper | **MODIFY** |
| `src/features/filters.rs` | Export a `CompiledFilters` struct bundling `CompiledMetaFilters` + `CompiledSqlFilters` | **MODIFY** |
| `src/cli/run.rs` | Accept pre-compiled filters from `validate_and_compile()` in `build_pipeline()` | **MODIFY** |
| `src/main.rs` | Background thread for update check | **MODIFY** |

## DEBT-01: SQLite Silent Errors

### Current State

In `sqlite.rs` `initialize()`:
```rust
} else if !self.append {
    let _ = conn.execute(&format!("DELETE FROM {}", self.table_name), []);
    // ^ Error silently discarded with `let _`
}
```

This is the only confirmed silent discard. The `DROP TABLE IF EXISTS` and `CREATE TABLE IF NOT EXISTS` paths DO propagate errors.

### Fix

Replace `let _ = conn.execute(...)` with proper error propagation:
```rust
conn.execute(&format!("DELETE FROM {}", self.table_name), [])
    .map_err(|e| Self::db_err(format!("clear table failed: {e}")))?;
```

Additionally, insert errors should be logged to the error log file, not just returned as `Err`. Currently, `export_one_preparsed()` returns `Err` which propagates up and terminates the run. For DEBT-01, the intent is: non-fatal insert errors should be written to the configured error log (`[error] file`) and processing continues, consistent with parse error behavior.

**This requires a design decision**: insert errors currently abort the export. DEBT-01 implies they should be logged and skipped. This mirrors how `Err` records from `parser.iter()` are handled (logged to `log::warn!`, processing continues).

### Integration Point

The error log path is in `cfg.error.file` (if an `[error]` section exists in config). The SQLite exporter currently has no reference to the error log path. Two options:

1. Pass error log path to `SqliteExporter::new()` and write directly (adds dependency).
2. Return a sentinel value from `export_one_preparsed()` indicating "logged but continuing" — but `Result<()>` already serves this; callers can log and continue.

**Recommendation**: In `handle_run()`, catch export errors from `exporter_manager.export_one_preparsed()` for SQLite path, log them to the error file (same mechanism as parse errors), increment a failed counter, and continue. This is the minimal change.

### Files to Modify for DEBT-01

| File | Change | Type |
|------|--------|------|
| `src/exporter/sqlite.rs` | Fix silent `let _` in `initialize()`; surface insert errors | **MODIFY** |
| `src/cli/run.rs` | Wrap `export_one_preparsed()` to log SQLite errors and continue (rather than abort) | **MODIFY** |

## DEBT-02: table_name SQL Injection

### Current State

`SqliteExporter` builds DDL and DML via string interpolation:
```rust
format!("DROP TABLE IF EXISTS {}", self.table_name)
format!("DELETE FROM {}", self.table_name)
format!("CREATE TABLE IF NOT EXISTS {table_name} (...)")
format!("INSERT INTO {table_name} VALUES (...)")
```

SQLite does not support `?` parameter binding for table names in DDL/DML. This is a structural constraint of the SQL wire protocol — parameters can only substitute values, not identifiers.

### Correct Fix: Whitelist Validation

Validate `table_name` at config parse time. A valid SQLite identifier contains only ASCII alphanumeric characters and underscores, does not start with a digit:

```rust
fn validate_table_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
```

Add this check to `SqliteExporter::validate()` in `config.rs`. If the name fails, return `ConfigError::InvalidValue`.

Additionally, quote the table name with double quotes in SQL strings as a defense-in-depth:
```rust
format!("DROP TABLE IF EXISTS \"{table_name}\"")
```

SQLite accepts double-quoted identifiers per the SQL standard (unlike MySQL backticks). This prevents any characters that slipped past validation from being parsed as SQL.

### Files to Modify for DEBT-02

| File | Change | Type |
|------|--------|------|
| `src/config.rs` | Add `validate_table_name()` helper; call in `SqliteExporter::validate()` | **MODIFY** |
| `src/exporter/sqlite.rs` | Wrap all `table_name` interpolations in double quotes | **MODIFY** |

## Component Interaction Map (v1.2)

```
Config (config.rs)
  └→ FiltersFeature (features/filters.rs)
       ├─ MetaFilters
       │    ├─ existing include fields (trxids, usernames, client_ips, ...)
       │    └─ [NEW FILTER-03] exclude_* fields
       ├─ IndicatorFilters
       ├─ SqlFilters (sql)         ← literal, pre-scan only
       └─ SqlFilters (record_sql)  ← literal, runtime (note: CompiledSqlFilters used in hot path)

  [PERF-11] validate_and_compile() → CompiledFilters
       ├─ CompiledMetaFilters  (from MetaFilters, includes new exclude_* Regex)
       └─ CompiledSqlFilters   (from record_sql SqlFilters)

cli/run.rs: handle_run()
  ├─ build_pipeline(cfg, compiled_filters)  ← [PERF-11] accepts pre-compiled
  │    └→ FilterProcessor { compiled_meta, exclude_meta, start_ts, end_ts }
  ├─ sql_record_filter: Option<&CompiledSqlFilters>  (unchanged)
  └─ process_log_file()
       └→ hot loop: pipeline + sql_record_filter
            └→ exporter_manager.export_one_preparsed()
                 └→ SqliteExporter  [DEBT-01: errors logged, not abort]
                                    [DEBT-02: table_name quoted]
```

## Build Order for v1.2 Phases

Based on dependencies:

1. **DEBT-02 first** — `table_name` validation is in `config.rs`; pure addition, no deps on other changes. Zero risk to filter or PERF work.

2. **DEBT-01** — SQLite error surfacing. Touches `sqlite.rs` and `run.rs` error handling. Independent of FILTER-03 and PERF.

3. **FILTER-03** — Extends `MetaFilters` + `CompiledMetaFilters`. Self-contained in `filters.rs`. No dependency on PERF-11 (can build with current double-compile; PERF-11 is an optimization on top).

4. **PERF-11** — Config/startup optimization. Requires knowing the final compiled filter shape after FILTER-03 adds `exclude_*` fields. Build after FILTER-03 to avoid rework.

5. **PERF-10** — Hot path optimization. Should be done after FILTER-03 (so exclude checking can be included in profiling) and after PERF-11 (so startup overhead is already reduced, isolating hot-loop from startup in measurements).

**Dependency graph:**
```
DEBT-02 → (none)
DEBT-01 → (none)
FILTER-03 → (none)
PERF-11 → FILTER-03 (needs final filter shape)
PERF-10 → FILTER-03, PERF-11 (profile after both)
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Adding Exclusion as a Separate Pipeline Stage

**What people do:** Create `ExcludeProcessor` implementing `LogProcessor`, add it to `Pipeline` after `FilterProcessor`.

**Why it's wrong:** Doubles the number of `process_with_meta()` calls in the hot loop. Splits the conceptual "filter" into two separated structs, making config validation and compilation more complex (two places to validate, two places to compile).

**Do this instead:** Extend `CompiledMetaFilters::should_keep()` with exclusion logic. All filter evaluation in one function call.

### Anti-Pattern 2: Compiling Regex in `SqlFilters::matches()`

**What people do:** Move regex compilation into the matches() call for simplicity.

**Why it's wrong:** `SqlFilters` (transaction-level) is called in pre-scan `par_iter()` across millions of records. Compiling regex per-call would be catastrophic.

**Do this instead:** Pre-compile all regexes at startup in the `Compiled*` structs. `SqlFilters` stays as literal-only, `CompiledSqlFilters` handles regex. This distinction is already in the codebase and must be preserved.

### Anti-Pattern 3: Storing `CompiledMetaFilters` in `FiltersFeature`

**What people do:** Add `OnceLock<CompiledMetaFilters>` to `FiltersFeature` for lazy compilation.

**Why it's wrong:** `FiltersFeature` is deserialized from config and may be cloned (pre-scan path clones `Config`). `OnceLock` is not `Clone`. `Arc<OnceLock<...>>` adds complexity. The double-compilation in PERF-11 is better fixed by returning compiled structs from `validate_and_compile()`.

**Do this instead:** `validate_and_compile()` on `Config` returns a separate `CompiledFilters` value that is passed into `handle_run()`. The config structs remain `Clone`-able and serde-deserializable.

### Anti-Pattern 4: Quoting table_name with Backticks

**What people do:** Use MySQL-style backtick quoting: `` `table_name` ``

**Why it's wrong:** SQLite uses double-quote for identifier quoting (per SQL standard). Backticks are accepted by SQLite in compatibility mode but are not standard and create confusion.

**Do this instead:** Use `"table_name"` (double-quote) in all SQL strings generated for SQLite.

## Sources

- Direct source inspection: `src/features/filters.rs`, `src/features/mod.rs`, `src/cli/run.rs`, `src/config.rs`, `src/exporter/sqlite.rs`, `src/exporter/mod.rs`, `src/main.rs`
- Confidence: HIGH — all claims verified from current code

---
*Architecture research for: sqllog2db v1.2 — FILTER-03, PERF-10, PERF-11, DEBT-01, DEBT-02*
*Researched: 2026-05-10*

---

# v1.3 Architecture: SQL Template Analysis & SVG Chart Generation

**Researched:** 2026-05-15
**Confidence:** HIGH (all findings from direct source code inspection + confirmed library versions)

## v1.3 New Data Flow

```
Input .log files
    ↓ SqllogParser
    ↓ LogParser.iter()
    ↓ Pipeline (filter processors — unchanged, pipeline.is_empty() fast path preserved)
    ↓ compute_normalized()               (unchanged)
    ↓ normalize_template()               [NEW] — post-process normalized_sql → canonical key
    ├─→ TemplateAggregator.observe()     [NEW] — side-channel accumulation (Option<&mut>)
    ↓ ExporterManager.export_one_preparsed()   (unchanged)
    ↓ Output file(s)                     (unchanged)

After streaming completes:
    TemplateAggregator.finalize()
    ↓ TemplateStats (owned, move semantics)
    ├─→ ReportWriter.write()             [NEW] — standalone JSON/CSV report file
    ├─→ ExporterManager.write_templates() [NEW] — sql_templates table / companion CSV
    └─→ ChartGenerator.render()          [NEW] — 4 SVG chart files
```

## Core Decision: Aggregator Placement

### Option A — Extend LogProcessor trait with finalize()

Add `fn finalize(&self) -> Option<AggregationResult>` to the `LogProcessor` trait.

**Problems:**
- `process()` takes `&self` (immutable). Aggregation is stateful — requires `Mutex<HashMap>` interior mutability in the hot loop, adding ~20ns lock overhead per record.
- `finalize()` return type would need a type-erased enum or `Box<dyn Any>` — fragile.
- Merges two orthogonal concepts: filter semantics (bool per record) and accumulation (stateful mutation).
- The `pipeline.is_empty()` fast path must now check both filter-empty AND aggregator-empty — complicates the invariant.
- Parallel CSV path runs separate `Pipeline` clones per rayon thread; aggregating across threads would require `Arc<Mutex<TemplateAggregator>>` in the hot loop.

**Verdict: REJECT.**

### Option B — Aggregator as separate struct, called from process_log_file() (RECOMMENDED)

```rust
// src/features/template_aggregator.rs
pub struct TemplateAggregator {
    templates: HashMap<String, TemplateEntry>,
    config: TemplateAnalysisConfig,
}

impl TemplateAggregator {
    pub fn observe(&mut self, template_key: &str, exec_time_us: u32, ts_prefix: &str);
    pub fn finalize(self) -> TemplateStats;
    pub fn merge(&mut self, other: TemplateAggregator);  // for parallel path
}
```

`process_log_file()` gains `aggregator: Option<&mut TemplateAggregator>` parameter. When `None`, zero overhead. When `Some`, one `HashMap::entry()` call per passing record.

**Advantages:**
- `pipeline.is_empty()` fast path completely unaffected — aggregator is a separate code path.
- No interior mutability needed — `&mut self` owned by `handle_run()`.
- Lifecycle is explicit: created before streaming, consumed by `finalize()` after streaming.
- Parallel CSV path: each rayon task creates a local `TemplateAggregator`, returns it alongside the record count, then caller merges via `merge()`.
- Clean separation: Pipeline = filter (returns bool), Aggregator = accumulate (mutation).

**Verdict: RECOMMENDED.**

### Option C — Second streaming pass for aggregation

Re-stream all log files a second time dedicated to template aggregation.

**Problems:** Doubles I/O for large log sets (GBs of data). Requires re-computing `normalized_sql` and `normalize_template()` again. No memory benefit since we still need the HashMap in memory.

**Verdict: REJECT.**

## Component Map

```
src/
├── features/
│   ├── mod.rs                     MODIFY: add TemplateAnalysisConfig to FeaturesConfig
│   ├── template_normalizer.rs     NEW — normalize_template(normalized_sql: &str) -> String
│   └── template_aggregator.rs     NEW — TemplateAggregator + TemplateStats
├── exporter/
│   ├── mod.rs                     MODIFY: add write_templates() to ExporterManager
│   ├── csv.rs                     MODIFY: impl companion _templates.csv writer
│   └── sqlite.rs                  MODIFY: impl sql_templates table writer
├── report/
│   └── mod.rs                     NEW — standalone JSON/CSV report writer
├── chart/
│   └── mod.rs                     NEW — SVG chart generation via charts-rs
└── cli/
    └── run.rs                     MODIFY: wire aggregator into process_log_file(),
                                           call finalize() + write_templates() + render()
```

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `template_normalizer.rs` | `normalize_template(normalized_sql) -> String` — lowercase, whitespace collapse, comment strip, IN list unification | Called inline in hot loop in `run.rs` |
| `template_aggregator.rs` | `observe(key, time_us, ts)` — accumulate per-template count + exec times + user/schema frequency | Called from `process_log_file()` via `Option<&mut TemplateAggregator>` |
| `report/mod.rs` | Serialize `TemplateStats` → JSON or CSV standalone file | Takes owned `TemplateStats`, uses `serde_json` (already in deps) |
| `exporter/csv.rs` | Write `{stem}_templates.csv` companion file | Takes `&TemplateStats` |
| `exporter/sqlite.rs` | Write `sql_templates` table | Takes `&TemplateStats`, uses existing `rusqlite` |
| `chart/mod.rs` | Render 4 SVG chart files to output dir | Takes `&TemplateStats`, writes SVG via `charts-rs` |

## Template Normalizer Design

Input: `normalized_sql` already produced by `compute_normalized()` — parameters already replaced.

Transforms (pure string ops, no external parser needed):

1. ASCII lowercase (DM SQL keywords are ASCII)
2. Strip `--` line comments and `/* */` block comments using `memchr` (already in deps)
3. Collapse any sequence of whitespace to single space, trim
4. Normalize `IN (?, ?, ...)` → `IN (?+)` — regex or manual scan

**No external SQL parser dependency needed.** `sqlparser` and `sqlformat` crates are designed for formatting/pretty-printing, not for the minimal string normalization required here. Using `memchr` (already in Cargo.toml) for comment detection keeps the dependency footprint flat. Confidence: MEDIUM — validate edge cases during implementation.

## Memory Footprint Analysis

### Per-template storage with Vec<u32>

```rust
struct TemplateEntry {
    count: u64,               // 8 bytes
    exec_times_us: Vec<u32>,  // 24 bytes header + N * 4 bytes (u32 microseconds)
    min_us: u32,              // 4 bytes
    max_us: u32,              // 4 bytes
    // CHART-04: time bucket counters (e.g., 24 hourly buckets * 8 bytes = 192 bytes)
    // CHART-05: HashMap<user_key, u64> — separate, small
}
```

Use `u32` microseconds (max ~71 minutes per query, covers all realistic DM exec times). Halves storage vs `u64`.

### Scale estimates

| Scenario | Templates | Records/template | Vec memory |
|----------|-----------|-----------------|------------|
| Typical workload | 500 | 20,000 | ~40 MB |
| Large workload | 2,000 | 50,000 | ~400 MB |
| Pathological | 50,000 | 1,000 | ~200 MB |

Real-world SQL logs: most workloads have 50–500 unique templates. A 1.1 GB log at ~1.55M records/sec implies ~10M records. 500 templates × 20K records = 160 MB at u64; 80 MB at u32.

**Decision:** Store all execution times (`Vec<u32>`) by default. Enables exact p50/p95/p99 via sort. Add config option `max_samples_per_template: u32` (default 0 = unlimited) as escape hatch. Warn in logs if any template exceeds 100K samples.

**Fallback option (if needed):** `hdrhistogram` v7.5.4 (confirmed via `cargo info`). Fixed ~24 KB per histogram, ~1-2% percentile error. At 500 templates = 12 MB. Adopt only if Vec approach causes OOM in practice.

## Parallel CSV Path: Aggregator Strategy

Current `process_csv_parallel()` spawns rayon tasks that each write a temp CSV, then concatenates. For aggregation:

1. Each rayon task creates a local `TemplateAggregator`.
2. `process_log_file()` signature change: add `aggregator: Option<&mut TemplateAggregator>`.
3. After all tasks complete, merge partial aggregators via `TemplateAggregator::merge()`.
4. `merge()` cost: O(unique templates) — negligible vs I/O time.

No `Arc<Mutex<>>` needed in the hot path. Preserves the existing parallel architecture.

## SVG Generation: Call Chain Position

```
handle_run()
    ↓ [streaming loop — process_log_file per file]
    ↓ exporter_manager.finalize()             writes main output (unchanged)
    ↓ let stats = aggregator.finalize()       → TemplateStats (if aggregator enabled)
    ↓ report_writer.write(&stats)             → standalone JSON/CSV (if configured)
    ↓ exporter_manager.write_templates(&stats) → sql_templates table / companion CSV
    ↓ chart_generator.render(&stats)          → SVG files (if [charts] config present)
```

SVG generation lives **after** all record-level export. It takes `&TemplateStats` (immutable, finalized). No dependency on exporter type. Called from `handle_run()` directly, not from inside `ExporterManager`. Preserves single responsibility of each component.

## Dual-Output Without Coupling

Two separate write operations consuming `&TemplateStats`:

1. `ReportWriter` (standalone) — config-gated by `[features.template_analysis.output]`. Always available regardless of active exporter. Uses `serde_json` (already in Cargo.toml).

2. `ExporterManager::write_templates()` — delegates to `CsvExporter::write_templates()` or `SqliteExporter::write_templates()`. Each exporter implements its own format-specific logic. Neither knows about chart generation.

3. `ChartGenerator::render()` — separate module, separate config section `[charts]`. Knows nothing about exporters or reports.

No coupling between the three output paths. All consume the same `TemplateStats` value.

## Config Changes

```toml
[features.template_analysis]
enable = true
max_samples_per_template = 0   # 0 = unlimited

[features.template_analysis.output]
format = "json"                # or "csv"
file = "templates_report.json"

[charts]
output_dir = "charts/"
top_n = 20
```

Both sections are `Option<T>` in Rust structs. When absent: zero overhead — no `TemplateAggregator` created, no `normalize_template()` called.

Integration into config validation: `template_analysis.output.format` must be "json" or "csv"; `charts.output_dir` must be a valid path prefix.

## Build Order (Dependency-Driven)

### Phase 1: Template Normalizer

**Files:** `src/features/template_normalizer.rs` (new), `src/features/mod.rs` (add config struct), `src/config.rs` (add `TemplateAnalysisConfig`)

- Pure function `normalize_template(normalized_sql: &str) -> String`
- `TemplateAnalysisConfig` stub (enable + max_samples)
- Tests: IN list normalization, comment stripping, whitespace collapse, mixed-case keywords

Dependencies: `memchr` (already in Cargo.toml). No new crates.

### Phase 2: TemplateAggregator (depends on Phase 1)

**Files:** `src/features/template_aggregator.rs` (new), `src/cli/run.rs` (modify)

- `TemplateAggregator::observe()`, `finalize()`, `merge()`
- `TemplateEntry` with `Vec<u32>` exec times + percentile computation
- Wire into `process_log_file()`: add `Option<&mut TemplateAggregator>` param
- Wire into `handle_run()`: create aggregator from config, call finalize after streaming
- Handle parallel path: partial aggregators per rayon task, merge after pool completes

**Highest-risk phase**: changes `process_log_file()` signature, touches hot loop and parallel path.

### Phase 3: Standalone Report Writer (depends on Phase 2)

**Files:** `src/report/mod.rs` (new), `src/cli/run.rs` (call after finalize)

- Serialize `TemplateStats` → JSON via `serde_json` or CSV via `itoa`/`ryu`
- Both already in Cargo.toml

### Phase 4: Exporter Integration (depends on Phase 2, parallel to Phase 3)

**Files:** `src/exporter/csv.rs`, `src/exporter/sqlite.rs`, `src/exporter/mod.rs`

- `write_templates(&TemplateStats) -> Result<()>` on each exporter
- SQLite: `CREATE TABLE sql_templates (template TEXT, count INTEGER, avg_ms REAL, ...)` + batch insert
- CSV: write `{stem}_templates.csv` adjacent to main output

### Phase 5: SVG Charts (depends on Phase 2, parallel to Phase 3/4)

**New dependency:** `charts-rs = "0.4.2"` (Apache-2.0, confirmed via `cargo search`)
**Files:** `src/chart/mod.rs` (new), `src/config.rs` (add `ChartsConfig`), `Cargo.toml`

- Bar chart: Top N template frequency (CHART-02) — horizontal bar, `charts-rs` HorizontalBar
- Histogram: Execution time distribution (CHART-03) — Bar chart with fixed buckets
- Line chart: SQL execution frequency time trend (CHART-04) — requires timestamp bucketing in TemplateEntry
- Pie chart: User/Schema distribution (CHART-05) — `charts-rs` Pie

**charts-rs vs plotters decision:** `charts-rs` (0.4.2) provides ECharts-inspired high-level API with direct bar/pie/line chart types and SVG-only output option. `plotters` (0.3.7) requires assembling charts from drawing primitives. For 4 standard chart types with no custom rendering requirements, `charts-rs` wins on development speed. Maintenance confirmed active as of 2025.

## Integration Points Summary

| Component | Type | File | Scope of Change |
|-----------|------|------|-----------------|
| `template_normalizer.rs` | New file | `src/features/` | ~60 lines |
| `template_aggregator.rs` | New file | `src/features/` | ~120 lines |
| `report/mod.rs` | New module | `src/report/` | ~80 lines |
| `chart/mod.rs` | New module | `src/chart/` | ~150 lines |
| `features/mod.rs` | Modified | — | Add `TemplateAnalysisConfig` to `FeaturesConfig` |
| `cli/run.rs` | Modified | — | `process_log_file()` gains `Option<&mut TemplateAggregator>`; `handle_run()` gains aggregator lifecycle + post-streaming writes |
| `exporter/mod.rs` | Modified | — | Add `write_templates()` to `ExporterManager` |
| `exporter/csv.rs` | Modified | — | Companion CSV writer |
| `exporter/sqlite.rs` | Modified | — | `sql_templates` table writer |
| `config.rs` | Modified | — | `TemplateAnalysisConfig`, `ChartsConfig` |
| `Cargo.toml` | Modified | — | Add `charts-rs = "0.4.2"` |

## Anti-Patterns to Avoid (v1.3)

### Anti-Pattern: Extending LogProcessor for aggregation

Adding `finalize()` or mutable accumulation to `LogProcessor` is architecturally incorrect. The trait is designed for stateless filter evaluation. Aggregation requires mutable state. Keep these concerns separate.

### Anti-Pattern: Calling normalize_template() when feature is disabled

The `normalize_template()` call must be guarded by the same `Option<&mut TemplateAggregator>` check as the aggregation step. If the aggregator is `None`, the normalizer should not run — it allocates a `String` for the canonical key that would be immediately discarded.

### Anti-Pattern: SVG generation inside ExporterManager

SVG charts are not "export" — they are derived analytics output. Putting `ChartGenerator` inside `ExporterManager` would couple chart generation to the active exporter type and violate the exporter's single responsibility.

### Anti-Pattern: Storing timestamps as strings in TemplateEntry

For CHART-04 (time trend), storing full timestamp strings per record is expensive. Instead, bucket by hour (or configurable interval) at `observe()` time: `let bucket = parse_hour_bucket(ts)` → increment `HashMap<u32, u64>` counter. Fixed memory regardless of record count.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Aggregator placement (Option B) | HIGH | Verified against existing `&mut` patterns in codebase; `Option<&mut T>` is idiomatic Rust |
| Memory estimates | MEDIUM | Based on real-world template count assumption; depends on actual log diversity |
| Template normalizer without external parser | MEDIUM | Sufficient for typical DM SQL; edge cases (nested comments, string literals containing `--`) need testing |
| charts-rs for SVG | MEDIUM | Version 0.4.2 confirmed active; API surface for 4 chart types needs validation during Phase 5 |
| Build order (Phase 1→2→3/4/5) | HIGH | Dependency chain verified from code inspection |
| Parallel path merge strategy | HIGH | Consistent with existing rayon + merge pattern used in pre-scan phase |

## Sources

- Code inspection: `src/features/mod.rs`, `src/cli/run.rs`, `src/exporter/mod.rs`, `src/config.rs`
- `charts-rs` v0.4.2: confirmed via `cargo search` (2026-05-15)
- `hdrhistogram` v7.5.4: confirmed via `cargo info` (2026-05-15)
- `plotters` v0.3.7: confirmed via `cargo search`
- Memory analysis: derived from project benchmark (1.55M records/sec on 1.1GB file, per PROJECT.md)

---
*v1.3 architecture research appended: 2026-05-15*

---

# v1.5 Architecture: Documentation Structure & GitHub Pages Integration

**Researched:** 2026-05-18
**Confidence:** HIGH (ecosystem research from mdBook docs, peaceiris/actions-gh-pages docs, official GitHub Pages docs)

## Overview: Documentation as a Build Artifact

The v1.5 milestone introduces two related but distinct documentation systems:

1. **Project documentation** — Markdown files in `docs/` for developers and users (quickstart, architecture, contributing, changelog). These live in the main repo and are versioned alongside code.

2. **GitHub Pages site** — A rendered static site (via mdBook) deployed to `guangl.github.io/sqllog2db/`. This is a **build artifact** generated from the `site/` source directory, not committed to `main`.

Both systems coexist without conflict. The mdBook source lives in the repo but the rendered output (`site/book/`) is gitignored. Only the `gh-pages` branch contains the deployable HTML.

## Directory Structure

```
sqllog2db/
├── docs/                          # [1] Project documentation (Markdown, versioned with code)
│   ├── quickstart.md              # Quick start guide
│   ├── architecture.md            # Architecture documentation
│   ├── configuration.md           # Configuration reference
│   ├── faq.md                     # Frequently asked questions
│   └── flamegraphs/               # Existing performance profiles (unchanged)
│
├── site/                          # [2] GitHub Pages mdBook source (net-new)
│   ├── book.toml                  # mdBook configuration
│   └── src/
│       ├── SUMMARY.md             # Table of contents
│       ├── index.md               # Landing page / hero section
│       ├── quickstart.md          # Quick start (symlink or copy from docs/)
│       ├── features.md            # Feature overview with screenshots
│       ├── configuration.md       # Configuration reference
│       ├── performance.md         # Benchmarks and performance data
│       ├── architecture.md        # Architecture overview
│       ├── changelog.md           # CHANGELOG (symlink to ../../CHANGELOG.md)
│       ├── faq.md                 # FAQ
│       └── CONTRIBUTING.md        # Contributing guide (symlink to ../../CONTRIBUTING.md)
│
├── CHANGELOG.md                   # [3] Root-level docs (existing, unmodified location)
├── README.md                      # Existing, kept as crate landing
├── CONTRIBUTING.md                # Net-new (root for discoverability)
├── SECURITY.md                    # Net-new (root for discoverability)
│
├── .github/
│   └── workflows/
│       ├── ci.yaml                # Existing — add doc build step
│       ├── release.yaml           # Existing — unchanged
│       └── pages.yaml             # Net-new — GitHub Pages deploy workflow
│
└── .gitignore                     # Add: /site/book/  (mdBook build output)
```

### Directory Rationale

- **`docs/`** follows the Cargo convention — `cargo doc` uses it for raw Markdown documentation, and it is excluded from the crate package via `Cargo.toml`'s `exclude = ["/docs/*"]`.
- **`site/`** is separate from `docs/` to avoid namespace collision between mdBook sources and project Markdown. mdBook expects a `src/SUMMARY.md` at its root, and putting it in `docs/src/SUMMARY.md` would conflict with the convention of `docs/` being purely project documentation.
- **Root-level `CONTRIBUTING.md`** and **`SECURITY.md`** follow GitHub's automatic discovery convention — GitHub shows these links in the repo sidebar if they exist at root.
- **Symlinks** in `site/src/` avoid content duplication. Files like `CHANGELOG.md` and `CONTRIBUTING.md` can be symlinked from their root location rather than copied.

## Static Site Architecture: mdBook

### Why mdBook

- **Rust-native** — built in Rust, deployed to Rust projects. Natural fit for a Rust CLI tool.
- **Integrated search** — generated site includes client-side search, zero external services.
- **Low maintenance** — one binary, one `book.toml`, no npm/node/pip dependencies.
- **GitHub Pages ready** — `mdbook build` outputs a flat `book/` directory that deploys directly.
- **Used by the Rust project itself** — proven for developer-facing documentation.

### Configuration (book.toml)

```toml
[book]
title = "sqllog2db"
authors = ["guangl"]
description = "高性能 CLI 工具：解析达梦数据库 SQL 日志并导出到 CSV/SQLite"
language = "zh"          # Primary language is Chinese (tool targets DM DBAs)

[build]
build-dir = "book"       # Output to site/book/ (relative to site/)

[output.html]
site-url = "/sqllog2db/"
git-repository-url = "https://github.com/guangl/sqllog2db"
edit-url-template = "https://github.com/guangl/sqllog2db/edit/main/site/src/{path}"
default-theme = "ayu"    # Clean, modern theme
preferred-dark-theme = "navy"
no-section-label = false  # Section numbering for navigation
git-buttons = true       # "Edit" and "Star" buttons in top bar
```

### Navigation Design (SUMMARY.md)

```
# Summary

[sqllog2db](index.md)                        # Landing / hero page

# 快速开始 (Quick Start)

- [安装与构建](quickstart.md)                 # Installation & build
- [五分钟上手](quickstart.md#五分钟上手)        # 5-minute tutorial (anchor link)
- [配置文件详解](configuration.md)             # Configuration reference
- [命令参考](commands.md)                      # CLI commands reference

# 功能特性 (Features)

- [核心功能](features.md#核心功能)             # Core features
- [SQL 模板分析](features.md#sql-模板分析)     # Template analysis
- [SVG 图表可视化](features.md#svg-图表可视化)  # Chart visualization
- [过滤与字段控制](features.md#过滤与字段控制)   # Filters & field control

# 性能 (Performance)

- [基准测试](performance.md#基准测试)           # Benchmarks
- [优化策略](performance.md#优化策略)           # Optimization strategies

# 开发 (Development)

- [架构概览](architecture.md)                 # Architecture
- [贡献指南](CONTRIBUTING.md)                  # Contributing
- [更新日志](changelog.md)                     # Changelog

---

[常见问题 (FAQ)](faq.md)
```

### Navigation Principles

1. **Progressive disclosure** — top-level items cover the most common user needs (quick start, configuration). Deeper items (architecture, contributing) are secondary.
2. **Chinese-first** — navigation labels in Chinese because the target audience (DM DBAs) is Chinese-speaking. The tool's README and docs are also Chinese-first.
3. **Flat over nested** — at most one level of nesting. CLI tools don't need deep documentation hierarchies.
4. **Part titles as section headers** — `# 快速开始`, `# 功能特性`, `# 性能`, `# 开发` serve as unclickable section labels between groups.

## GitHub Pages Deployment Architecture

### Deployment Option Comparison

| Criterion | Branch-based (`gh-pages`) | actions/deploy-pages (GitHub native) |
|-----------|---------------------------|--------------------------------------|
| Setup complexity | Low — add branch, push | Medium — requires Pages deployment env + `id-token: write` |
| Token management | `GITHUB_TOKEN` works | Requires `id-token: write` permission |
| Build vs deploy sep. | Combined (peaceiris action does both) | Separated (`actions/upload-pages-artifact` → `actions/deploy-pages`) |
| Orphan branch | Single-commit history via `force_orphan: true` | Managed automatically |
| Rollback | Reset `gh-pages` branch | Via Pages settings UI |
| Custom domain | Via `cname` option | Via `actions/configure-pages` |
| Build env | Any runner | ubuntu-latest only |
| **Recommendation** | **RECOMMENDED** | Not needed for this project's scale |

**Decision: Branch-based deployment via `peaceiris/actions-gh-pages@v4`.**

Rationale:
- Simpler setup — one workflow, one action, one permission (`contents: write`).
- Orphan branch ensures clean, minimal history.
- The project is a single-user open-source tool — the advanced features of the native Pages deployer (deployment environments, preview URLs) are not needed.
- The `peaceiris` action is the most widely used Pages deployer on GitHub Actions (20K+ stars).

### Deploy Flow

```
                GitHub Actions (pages.yaml)
                     ↓
            Trigger: push to main
            (only if site/ or .github/ changed)
                     ↓
            actions/checkout@v6
                     ↓
            Install mdBook (via peaceiris/actions-mdbook@v2)
                     ↓
            mdbook build site/   ← outputs to site/book/
                     ↓
            peaceiris/actions-gh-pages@v4
              with:
                publish_dir: ./site/book
                publish_branch: gh-pages
                force_orphan: true
                user_name: "github-actions[bot]"
                user_email: "41898282+github-actions[bot]@users.noreply.github.com"
```

### Trigger Logic

- **Primary trigger**: push to `main` when files under `site/` or `.github/workflows/pages.yaml` change. This avoids rebuilding docs on every code commit.
- **Manual trigger**: `workflow_dispatch` for one-off rebuilds.
- **Pull request**: optional — could add a preview build that uploads artifacts (without deploying).

```yaml
on:
  push:
    branches: [main]
    paths:
      - "site/**"
      - ".github/workflows/pages.yaml"
  workflow_dispatch:
```

## CI/CD Integration Points

### Existing CI (ci.yaml) — What Changes

The existing `ci.yaml` runs on every push/PR to `main`:

```yaml
# Existing step (line 62-65):
- name: Check documentation
  run: cargo doc --no-deps
  env:
    RUSTDOCFLAGS: -D warnings
```

**Current behavior**: Builds Rust crate docs via `cargo doc`. Flags doc warnings as errors.

**No change needed** to this step. It validates Rust doc comments (`///`). This is orthogonal to the mdBook site.

**Addition to ci.yaml**: A new job (or step) to validate mdBook builds:

```yaml
docs:
  name: Docs Site Build
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v6
    - name: Install mdBook
      uses: peaceiris/actions-mdbook@v2
      with:
        mdbook-version: "latest"
    - name: Build docs site
      run: mdbook build site/
    - name: Check for broken links
      run: |
        # Verify all referenced files exist
        cd site/src
        for link in $(grep -roh '(\.\./[^)]*\.md)' . | sort -u); do
          test -f "../../$link" || test -f "$link" || echo "BROKEN: $link"
        done
```

**Purpose**: Catches build failures and broken links in the mdBook source before they reach the deploy workflow. This runs on every PR, not just deploys.

### New Workflow (pages.yaml) — Deploy Only

A separate deploy workflow, distinct from the `ci.yaml` validation:

```yaml
name: Pages

on:
  push:
    branches: [main]
    paths:
      - "site/**"
      - ".github/workflows/pages.yaml"
  workflow_dispatch:

concurrency:
  group: pages
  cancel-in-progress: true

permissions:
  contents: write

jobs:
  deploy:
    name: Build and Deploy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install mdBook
        uses: peaceiris/actions-mdbook@v2
        with:
          mdbook-version: "latest"

      - name: Build site
        run: mdbook build site/

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./site/book
          publish_branch: gh-pages
          force_orphan: true
          user_name: "github-actions[bot]"
          user_email: "41898282+github-actions[bot]@users.noreply.github.com"
```

**Key design decisions:**
- **Separate workflow** — not combined with `ci.yaml`. Deploy has different trigger conditions (`paths` filter) and permissions. Mixing them would either over-deploy (on every code push) or complicate the trigger logic.
- **`paths` filter on `site/**`** — avoids deploying when only source code changes. The documentation site only needs rebuilding when its source changes.
- **`force_orphan: true`** — the `gh-pages` branch always has exactly one commit. Minimizes storage and history.
- **`concurrency` group** — cancels in-progress deploys if a newer push arrives. Prevents race conditions.

## Build and Deploy Flow (Complete)

```
Developer pushes to main
    │
    ├── site/** changed?
    │   ├── YES → both workflows run:
    │   │          ci.yaml:   test + lint + cargo doc + cargo bench --no-run
    │   │          pages.yaml: mdbook build + deploy to gh-pages
    │   │
    │   └── NO  → only ci.yaml runs (no pages redeploy)
    │
    ├── src/** changed (code)?
    │       └── ci.yaml runs (test + lint + coverage)
    │
    └── tag v*
            └── release.yaml runs (binary builds + cargo publish)
```

### Rollback Strategy

- **Immediate**: Push the `main` branch to the previous commit containing the good docs. The `pages.yaml` workflow re-deploys.
- **Manual**: Go to GitHub repo Settings > Pages, change Source from "Deploy from a branch" to "None", then back to the `gh-pages` branch to force a re-deploy from the last commit.
- **Full revert**: Delete the `gh-pages` branch via `git push origin --delete gh-pages`, then re-run the workflow.

## Gitignore and Exclusion

### .gitignore additions

```
# mdBook build output
/site/book/
```

### Cargo.toml — existing exclusion (no change needed)

```toml
exclude = [
  "/docs/*",
  "/site/*",    # Add this — mdBook source not needed in the crate
  ...
]
```

## Risk Assessment

| Concern | Impact | Mitigation |
|---------|--------|------------|
| mdBook version drift | Broken builds, inconsistent features | Pin version (e.g., `mdbook-version: "0.4.43"`) in both ci.yaml and pages.yaml |
| Broken links in site | 404 on published site | Add link check job in ci.yaml (runs on PRs) |
| Accidental deploy on code-only change | Unnecessary rebuild | `paths` filter in pages.yaml |
| Symlinks not working in Actions checkout | Missing CHANGELOG etc. in site build | Use `actions/checkout@v6` (preserves symlinks); fallback: copy files via script |
| Large binary size from mdbook install | ~10MB download per deploy | `actions/cache` for mdbook binary; or use docker-based mdbook for build |
| git conflicts on gh-pages branch | None | `force_orphan: true` guarantees single-commit branch |

## Integration Points Summary

| Integration Point | Type | File | Change |
|-------------------|------|------|--------|
| CI validation job | **NET-NEW** | `.github/workflows/ci.yaml` | Add `docs` job: mdbook build + link check |
| GitHub Pages deploy | **NET-NEW** | `.github/workflows/pages.yaml` | New workflow: build + deploy |
| mdBook project | **NET-NEW** | `site/book.toml` | mdBook configuration |
| mdBook sources | **NET-NEW** | `site/src/*.md` | All markdown content for the site |
| Project docs | **NET-NEW** | `docs/quickstart.md`, `docs/architecture.md`, `docs/configuration.md`, `docs/faq.md` | Markdown documentation |
| Root docs | **NET-NEW** | `CONTRIBUTING.md`, `SECURITY.md` | GitHub-discoverable docs |
| Gitignore | **MODIFY** | `.gitignore` | Add `/site/book/` |
| Cargo.toml | **MODIFY** | `Cargo.toml` | Add `/site/*` to exclude |
| Root README.md | **MODIFY** | `README.md` | Update links to point to new docs |

## Anti-Patterns to Avoid

### Anti-Pattern: Putting mdBook source in docs/

**What people do:** Use `docs/` as both the project documentation directory and the mdBook source directory.

**Why it's wrong:** `docs/` is excluded from the crate via `Cargo.toml`, which is correct for raw docs. But mdBook expects a specific structure (`book.toml` at root, `src/SUMMARY.md`). Placing this in `docs/` creates confusion about what `docs/` contains (raw Markdown vs. mdBook project). Also, the `docs/` path is conventionally used by GitHub Pages for Jekyll — having mdBook inside it would conflict if GitHub Pages settings are changed.

**Do this instead:** Use `site/` as the mdBook project directory. Keep `docs/` for raw project documentation.

### Anti-Pattern: Deploying on every push

**What people do:** Trigger Pages deploy on every push to main, including pure code changes.

**Why it's wrong:** Unnecessary builds and deploys. Wastes CI minutes. Creates spurious deploys where the site content hasn't changed.

**Do this instead:** Use `paths` filter to only trigger the Pages workflow when `site/**` or `.github/workflows/pages.yaml` changes.

### Anti-Pattern: Content duplication via copy

**What people do:** Copy CHANGELOG.md, CONTRIBUTING.md into the mdBook source directory.

**Why it's wrong:** Two copies drift apart over time. Contributors update one but not the other.

**Do this instead:** Symlink from `site/src/` to the root files. If the runtime environment doesn't support symlinks (some Windows runners), add a script step in CI that copies files before building.

### Anti-Pattern: Combining mdBook build with cargo doc

**What people do:** Run `mdbook build` in the same job as `cargo doc` inside `ci.yaml`.

**Why it's wrong:** These have different purposes: `cargo doc` validates Rust doc comments (must run on every build); mdBook builds the marketing/landing site (only needs validation on PR). Combining them makes ci.yaml harder to reason about.

**Do this instead:** Keep mdBook validation in its own `docs` job inside `ci.yaml` (runs on every push/PR). Keep deploy in a separate `pages.yaml` (runs only on path-filtered main pushes).

## Sources

- mdBook User Guide: `rust-lang.github.io/mdBook/` (2026-05-18) — project structure, `book.toml` config, `SUMMARY.md` format, build command
- peaceiris/actions-mdbook: `github.com/marketplace/actions/mdbook-action` (2026-05-18) — mdBook GitHub Action for CI
- peaceiris/actions-gh-pages: `github.com/peaceiris/actions-gh-pages` (2026-05-18) — GitHub Pages deployment action, all options documented
- GitHub Pages documentation: `docs.github.com/en/pages` (2026-05-18) — deployment branch configuration
- Code inspection: `Cargo.toml`, `.github/workflows/ci.yaml`, `.github/workflows/release.yaml`, `docs/`, `README.md`, `.gitignore`
- Confidence: HIGH — all findings verified against official documentation or source code

---
*v1.5 architecture research (documentation & GitHub Pages): 2026-05-18*
