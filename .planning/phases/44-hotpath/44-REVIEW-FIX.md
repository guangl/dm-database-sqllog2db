---
phase: "44"
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/44-hotpath/44-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 44: Code Review Fix Report

**Fixed at:** 2026-05-25T00:00:00Z
**Source review:** .planning/phases/44-hotpath/44-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### CR-01: Flaky assertion in `test_jemalloc_peak_baseline` fails on warm runs

**Files modified:** `tests/jemalloc_peak.rs`
**Commit:** 504f076
**Applied fix:** Removed the `assert!(heap_pressure > 0, ...)` assertion. On warm runs, both `allocated_delta` and `resident_delta` can legitimately be 0 (handle_run frees temp memory, jemalloc reuses mapped pages). Replaced with a comment explaining this is a measurement baseline (PERF-02), not a correctness assertion. The reliable `final_resident > 0` liveness guard is retained. Added `let _ = heap_pressure;` to suppress unused variable warning.

### WR-01: `compute_normalized` doc comment factually incorrect

**Files modified:** `src/pipeline/normalizer.rs`
**Commit:** 2a5671f
**Applied fix:** Replaced the incorrect `# Panics` section (which wrongly described `None` return conditions as encoding errors) with a proper `# Returns` section documenting all five conditions that return `None`: no tag, tag not a DML, no placeholders, no matching buffer entry, and param count mismatch. The `# Panics` section was rewritten to accurately state that the internal `expect` is an unreachable consistency guard.

### WR-02: GB18030 fallback is dead code in `compute_normalized`

**Files modified:** `src/pipeline/normalizer.rs`, `Cargo.toml`
**Commit:** 545579e
**Applied fix:** Removed the unreachable `if std::str::from_utf8(scratch).is_err()` branch and the GB18030 decode block (lines 407-418). Replaced with a `debug_assert!` to preserve the UTF-8 invariant in debug builds, and a direct `std::str::from_utf8(scratch).expect(...)` for the final conversion. Also removed the now-unused `encoding_rs = "0.8"` dependency from `Cargo.toml`. Note: `unsafe { std::str::from_utf8_unchecked }` was considered but rejected because the project lints deny `unsafe_code`.

### WR-03: TOCTOU race in `CsvExporter::initialize` append mode

**Files modified:** `src/exporter/csv/mod.rs`
**Commit:** c6ccba3
**Applied fix:** Removed the pre-open `let file_exists = self.path.exists()` check. After opening the file, inspect its actual size with `file.metadata().map(|meta| meta.len() == 0).unwrap_or(true)` to determine `file_is_empty`. The header-write condition changed from `!append_mode || !file_exists` to `!append_mode || file_is_empty`. The decision is now based on the actual file state after opening, eliminating the TOCTOU window. Fallback to writing the header when metadata() fails (e.g., `/dev/null`).

### IN-01: `#[cfg(test)]` redundant in integration test file

**Files modified:** `tests/jemalloc_peak.rs`
**Commit:** bf9933d
**Applied fix:** Removed the `#[cfg(test)]` attribute from the `#[global_allocator]` declaration. Integration test files in `tests/` are always compiled as test binaries — the attribute was a no-op. Updated the module-level doc comment to clarify that the real isolation mechanism is `tikv-jemallocator` being a dev-dependency, not `cfg(test)`.

---

_Fixed: 2026-05-25T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
