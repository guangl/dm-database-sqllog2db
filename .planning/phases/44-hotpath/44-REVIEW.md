---
phase: "44"
status: has_findings
depth: standard
files_reviewed: 3
files_reviewed_list:
  - tests/jemalloc_peak.rs
  - src/pipeline/normalizer.rs
  - src/exporter/csv/mod.rs
findings:
  critical: 1
  warning: 3
  info: 1
  total: 5
---

# Phase 44: Code Review Report

**Reviewed:** 2026-05-25T00:00:00Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** has_findings

## Summary

Reviewed three files from the phase 44 hot-path optimization work: a jemalloc baseline measurement integration test, the SQL parameter normalizer, and the CSV exporter. The normalizer logic is sound for all practical inputs. The CSV exporter correctly mirrors header and data column logic. The primary issues are a flaky assertion in the jemalloc test that will cause CI failures on warm runs, a factually incorrect doc comment that misrepresents when `compute_normalized` returns `None`, a GB18030 fallback branch that is unreachable in practice, and a TOCTOU race in `CsvExporter::initialize` for append mode.

## Critical Issues

### CR-01: Flaky assertion in `test_jemalloc_peak_baseline` fails on warm runs

**File:** `tests/jemalloc_peak.rs:151`
**Issue:** The assert `heap_pressure > 0` can fail spuriously. `heap_pressure` is `allocated_delta` when positive, otherwise `resident_delta`. Both values can be zero simultaneously: `allocated_delta` is routinely zero because `handle_run` frees its allocations before returning (acknowledged by the comment at line 128), and `resident_delta` can be zero when jemalloc's arena already has enough mapped physical pages from a previous allocation (jemalloc's lazy page-return only provides a delta when it must request new pages from the OS). On a warm build machine — any CI runner that executes the test more than once per process lifecycle — both deltas are zero and the test fails with no real defect in the code under test.

The `final_resident > 0` check at line 147 is a reliable liveness guard (absolute value, not delta), but `heap_pressure > 0` (line 151) is not.

**Fix:**
```rust
// Remove the unreliable delta-based assertion entirely, or replace it with
// a measurement that reflects actual peak allocation (e.g. jemalloc's stats.allocated peak
// via epoch reset). As a minimal fix, drop the assert and rely on println output only:

// Remove lines 151-154:
// assert!(
//     heap_pressure > 0,
//     "heap pressure (resident_delta or allocated_delta) must be positive — ..."
// );

// This test is a *measurement* baseline, not a correctness assertion.
// The only assertion that should remain is the liveness check on final_resident.
```

## Warnings

### WR-01: `compute_normalized` doc comment factually incorrect — wrong section and wrong content

**File:** `src/pipeline/normalizer.rs:338`
**Issue:** The `# Panics` section reads: *"Returns `None` only if the result contains bytes that are neither valid UTF-8 nor valid GB18030"*. This is wrong in two ways:

1. The `# Panics` rustdoc section should document panic conditions, not `None` return conditions.
2. The claim "Returns `None` only if..." is factually false. The function returns `None` for five distinct reasons: no `tag` on the record, tag is not one of INS/DEL/UPD/SEL, no placeholders detected in the SQL, no matching entry in `buffer`, and param count mismatch. The GB18030 encoding error is neither the only nor the primary cause of a `None` return.

In practice the function never panics (the `expect` at line 408 guards against non-UTF-8 output that the safety invariant at line 188–191 proves cannot occur). The real panic condition is already documented implicitly by that invariant.

**Fix:**
```rust
/// # Returns
///
/// - `Some(&str)` — the SQL with all placeholders replaced by their bound values,
///   written into `scratch`. The reference borrows `scratch`; the caller must not
///   modify `scratch` while it is live.
/// - `None` — if any of the following hold:
///   - the record has no `tag` (e.g. a `PARAMS` record — its values are stored in `buffer`)
///   - the tag is not `INS`, `DEL`, `UPD`, or `SEL`
///   - the SQL contains no recognisable placeholders
///   - no matching params entry exists in `buffer` for this (`sess_id`, `stmt`) key
///   - the number of bound params does not equal the number of placeholders in the SQL
///
/// # Panics
///
/// Will not panic in practice: all bytes written to `scratch` are either taken verbatim
/// from the UTF-8 input SQL or from UTF-8 `ParamValue` strings. The `expect` on line N
/// is an internal consistency assertion that should never fire.
```

### WR-02: GB18030 fallback in `compute_normalized` is dead code

**File:** `src/pipeline/normalizer.rs:395`
**Issue:** The safety invariant documented at lines 188–191 proves that `apply_params_into` always produces valid UTF-8: all bytes come either from `sql` (valid UTF-8) or from `ParamValue::Quoted`/`Bare` which are `String` values (always valid UTF-8). The ASCII control characters used as delimiters (`?`, `:`, `'`) cannot break multi-byte sequences. Therefore the `std::str::from_utf8(scratch).is_err()` branch at line 395 is unreachable, and the entire GB18030 decode block (lines 395–406) is dead code. It silently compiles to dead instructions in the hot path, and its presence suggests a misunderstanding about when transcoding is needed.

**Fix:**
```rust
// Replace lines 395-408 with a direct conversion, using debug_assert to preserve
// the invariant as a safety net:

debug_assert!(
    std::str::from_utf8(scratch).is_ok(),
    "apply_params_into produced invalid UTF-8 — invariant violated"
);
// SAFETY: apply_params_into only writes bytes from UTF-8 sources (sql + ParamValue Strings)
// and ASCII literals; the resulting byte sequence is always valid UTF-8.
Some(unsafe { std::str::from_utf8_unchecked(scratch) })
// Or, keeping safe code but dropping the dead branch:
Some(std::str::from_utf8(scratch).expect("apply_params_into produced invalid UTF-8"))
```

### WR-03: TOCTOU race in `CsvExporter::initialize` corrupts append-mode CSV

**File:** `src/exporter/csv/mod.rs:103`
**Issue:** `file_exists` is sampled via `self.path.exists()` at line 103, then used at line 126 to decide whether to write a CSV header. The file is not opened until lines 105–122. In append mode, if the file does not exist when `exists()` is called but is created by another process between that check and `open()`, the `open()` call succeeds (it appends to the newly-created existing file), but `!file_exists` is still `true`, causing a header row to be appended to existing data. This is a classic TOCTOU: the check and the use are not atomic.

Practically this requires a concurrent writer targeting the same output path, which is uncommon in single-threaded streaming mode. However, the SQLite parallel path introduced in milestone v1.10 (WAL mode, Phase 45) and the CSV append use-case together make concurrent access more likely.

**Fix:**
```rust
// After opening the file, inspect its actual size to determine whether to write a header,
// rather than sampling exists() before open():
let file = open_result?; // result of OpenOptions open

let file_is_empty = append_mode && {
    file.metadata()
        .map(|m| m.len() == 0)
        .unwrap_or(true) // if metadata fails, write header to be safe
};

let mut writer = BufWriter::with_capacity(16 * 1024 * 1024, file);
if !append_mode || file_is_empty {
    let header = self.build_header();
    writer.write_all(&header)?;
}
```

## Info

### IN-01: `#[cfg(test)]` on `#[global_allocator]` is redundant in an integration test

**File:** `tests/jemalloc_peak.rs:11`
**Issue:** Integration tests in the `tests/` directory are always compiled as test binaries — `cfg(test)` is always `true` in this context. The `#[cfg(test)]` attribute on lines 11–13 is therefore a no-op. The comment on line 8 cites this `cfg` as providing isolation ("D-03/Pitfall 1 防护"), but the real protection comes from `tikv-jemallocator` being a `dev-dependency` (which excludes it from release builds regardless of `cfg(test)`). The attribute can be removed to avoid implying false protection.

**Fix:** Remove lines 11 and 12 (`#[cfg(test)]` and the blank line following it); keep only:
```rust
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```
Update the module-level comment to reflect that dev-dependency exclusion is the actual protection mechanism.

---

_Reviewed: 2026-05-25T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
