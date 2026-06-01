---
phase: "43"
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/43-parser-api-filter/43-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 43: Code Review Fix Report

**Fixed at:** 2026-05-25T00:00:00Z
**Source review:** .planning/phases/43-parser-api-filter/43-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### CR-01: CSV exporter silently drops `row_count` when `exec_id=0` and `exectime=0.0`

**Files modified:** `src/exporter/csv/writer.rs`
**Commit:** 8461dad
**Applied fix:** Added `|| sqllog.rowcount != 0` to both occurrences of the performance-metrics
condition: the `FieldMask::ALL` fast path at line 74, and the `has_metrics` variable at line 105.
Both now match the SQLite exporter condition at `write.rs:14`.

---

### WR-01: Fatal export error does not short-circuit the inner processing loop
### WR-02: `records_in_file` increments unconditionally even when export fails

**Files modified:** `src/cli/run/processor.rs`
**Commit:** 6021d75
**Applied fix:** Replaced the `map_or_else` closure with a `match export_result { ... }` block.
Fatal errors (`e.is_fatal()`) now `break 'outer` immediately after logging, stopping further
insert attempts. Non-fatal errors are logged and counted without incrementing `records_in_file`.
`records_in_file += 1` is now inside the `Ok(())` arm only, so failed exports are never
counted in the per-file record total.

---

### IN-01: `scan_for_trxids_by_transaction_filters` thread pool error mapped to opaque `io::Error`

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** 05c4551
**Applied fix:** Changed `.map_err(|e| Error::Io(std::io::Error::other(e)))` to
`.map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))`.
The error message now includes the "rayon thread pool: " prefix so users see a meaningful
diagnostic instead of an opaque "IO error" with no context about the actual failure.

---

_Fixed: 2026-05-25T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
