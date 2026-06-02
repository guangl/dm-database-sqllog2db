---
phase: 56-stats-benchmark
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - benches/BENCHMARKS.md
  - src/cli/run/processor.rs
  - src/lib.rs
  - src/main.rs
  - src/scanner.rs
  - src/stats/mod.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 56: Code Review Report

**Reviewed:** 2026-06-02T00:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Phase 56 introduces `src/scanner.rs` as a shared scanning primitive consumed by both
`src/stats/mod.rs` and `src/cli/run/processor.rs`. The extraction is structurally sound:
error handling is consistent, the iterator is never silently swallowed, and
`build_parser` correctly maps both non-UTF8 and open-failure paths to `ParserError::InvalidPath`.

Three warnings were found. The most significant is a behavioral gap where `stats` command
parse errors are silently discarded at the process level (only emitted as `log::info!`, no
non-zero exit code). The second is an early-termination risk in `scan_files` that can yield
misleading error messages for multi-file stats runs. The third is a doc-comment inaccuracy
in `build_parser` that will confuse future callers. Two info items cover dead imports and a
minor test-log assumption.

---

## Warnings

### WR-01: `stats` parse errors produce no non-zero exit code

**File:** `src/stats/mod.rs:48-54` (and `src/main.rs:190`)

**Issue:** When `scan_files_into_accumulator` encounters parse errors, it logs them with
`log::info!` and returns `Ok(())`. The `stats` subcommand path in `main.rs` unconditionally
returns `Ok(None)` (line 190), which routes through the `Ok(None) => {}` arm (line 114) —
a silent zero exit. The `run` command surfaces the same class of error through exit code 1
(`EXIT_PARTIAL`). Users scripting `sqllog2db stats` cannot detect a partially corrupt input
from the exit status alone.

Additionally, the log level is `info`, not `warn`, so with default log configuration the
message may be suppressed. `src/cli/run/processor.rs:145` uses `log::warn!` for the same
condition.

**Fix:** Surface parse error counts back through `handle_stats` and propagate to an
`ErrorStats` return value, mirroring the `run` command:

```rust
// stats/mod.rs: return scan error count
fn scan_files_into_accumulator(...) -> Result<ErrorStats> {
    let mut scan_stats = ErrorStats::default();
    crate::scanner::scan_files(log_files, &mut |r| accumulator.update(r), &mut scan_stats)?;
    if scan_stats.has_errors() {
        log::warn!("stats: {} parse error(s) during scan", scan_stats.parse_errors);
    }
    Ok(scan_stats)
}

// main.rs Stats arm:
let stats = cli::stats::handle_stats(&cfg, *top, from.clone(), to.clone())?;
Ok(Some((stats, cli.quiet)))
```

---

### WR-02: `scan_files` early-termination on mid-run file-open failure yields misleading output for stats

**File:** `src/scanner.rs:44`

**Issue:** The `?` on `build_parser(file_path)?` propagates `Err` immediately, aborting
the loop over all remaining files. For `stats` this means: if file 1 of 3 succeeds, file 2
fails to open, and file 3 would also succeed — `run_stats` returns `Err` and the user sees
a parser error with no hint that file 1's data was processed. The accumulator is discarded
and no output is written, but the error message only names the failing file.

In the `run` command, `process_log_file` (via `build_parser`) also propagates `Err` for a
single file, but each file is called individually by the orchestrator which can continue to
the next file. The `scan_files` design does not offer that option to callers.

The behavior is documented ("文件路径不存在或无法打开时返回 Err，终止整个扫描") so this is
by design — but for a stats scan across many files, silently aborting after file 1 passes
and file 2 fails will frustrate users. At minimum, the error should include the total context:

**Fix (minimal):** Log a warning before returning that names the abort point and how many
files remain:

```rust
let parser = match build_parser(file_path) {
    Ok(p) => p,
    Err(e) => {
        let remaining = log_files.len()
            - log_files.iter().position(|f| f == file_path).unwrap_or(0);
        log::warn!(
            "scanner: aborting scan at {} ({} file(s) not yet scanned): {}",
            file_path.display(), remaining - 1, e
        );
        return Err(e.into());
    }
};
```

Or, for a more resilient design, skip the failing file (matching the `prescan.rs` pattern)
and count it as an error rather than aborting:

```rust
let parser = match build_parser(file_path) {
    Ok(p) => p,
    Err(e) => {
        stats.add_parse_error();
        log::warn!("scanner: skipping {}: {}", file_path.display(), e);
        continue;
    }
};
```

---

### WR-03: `build_parser` doc comment incorrectly references `PathNotFound`

**File:** `src/scanner.rs:9-11`

**Issue:** The doc comment states:

> 文件不存在或无法打开时返回 `Err(ParserError::InvalidPath)`。

But looking at `src/error.rs`, `ParserError` has a distinct `PathNotFound` variant.
The doc implies `build_parser` might return `PathNotFound`; it does not — it always maps
file-open failures to `InvalidPath`. This is a correct implementation but a misleading doc.
Callers (e.g., future match arms on the error) may expect `PathNotFound` and inadvertently
miss the `InvalidPath` arm.

**Fix:**

```rust
/// - 路径含非 UTF-8 字节时返回 `Err(ParserError::InvalidPath { reason: "non-UTF8 path" })`。
/// - 文件不存在または打开失败时也返回 `Err(ParserError::InvalidPath)`（非 `PathNotFound`）。
///   如需区分两者，请检查 `reason` 字段。
```

---

## Info

### IN-01: `scan_stats` parse errors logged at `info` level, inconsistent with `processor.rs` which uses `warn`

**File:** `src/stats/mod.rs:49-52`

**Issue:** `log::info!("stats: {} parse error(s)...", ...)` versus `processor.rs:145`
`log::warn!("...: {} parse errors")`. Parse errors represent data loss potential and
should be at `warn` level for consistent observability across both code paths.

**Fix:** Change line 50 from `log::info!` to `log::warn!`.

---

### IN-02: `benches/BENCHMARKS.md` Phase 56 section is a single footnote, no baseline numbers recorded

**File:** `benches/BENCHMARKS.md:719`

**Issue:** Phase 56 is only mentioned as "实施于 Phase 56（v1.15）— D-04" in a footnote
under the CI artifact section. If Phase 56 introduced new benchmark infrastructure
(per D-04), there is no corresponding baseline table or benchmark group entry for Phase 56,
unlike Phases 4, 5, 6, 9, 10, 42, and 44 which each have dedicated sections with criterion
output. This makes the document inconsistent — new infrastructure is described at the
bottom but has no performance record of its own.

**Fix:** If Phase 56 adds no new benchmark _groups_ (only CI artifact collection), add a
short Phase 56 section noting that explicitly so the document remains internally consistent:

```markdown
## Phase 56 — CI Benchmark Artifact Collection（v1.15）

**Date:** 2026-06-02
**Goal:** D-04 — CI artifact upload + download workflow（bench.yml）
**Benchmark impact:** No new criterion benchmark groups introduced. Existing baselines
(Phases 4/5/42/44) remain current. See "CI Benchmark Artifact 使用说明" above.
```

---

_Reviewed: 2026-06-02T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
