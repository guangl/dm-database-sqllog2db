---
phase: 29-remove-stats-digest
reviewed: 2026-05-20T10:30:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/lang.rs
  - src/main.rs
  - src/pipeline/mod.rs
  - src/pipeline/normalizer.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-05-20T10:30:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Reviewed 7 source files related to the removal of stats/digest modules. The code is generally well-structured with good error handling patterns. Two issues stand out: a silent-exit bug on the no-subcommand path, and a keyword-set gap in the SQL normalizer that can produce inconsistent normalization for DDL statements. Several tests have weak assertions that would not catch regressions.

---

## Critical Issues

### BL-01: Help output silently discarded when no subcommand provided

**File:** `src/main.rs:217-219`

**Issue:**
When `sqllog2db` is run without any subcommand, `cli.command` is `None` (the field is `Option<Commands>`). The fallback branch tries to display help by calling `Cli::try_parse_from(["sqllog2db", "--help"])`, but the returned `Result` is silently discarded with `let _ = ...`. Neither clap's `try_parse_from` nor `try_parse_from` automatically print help on error — they return `Err(clap::Error)` which must be explicitly handled. The result is that the user sees zero output (no help, no error) and the process exits with code 1.

**Fix:**
Replace the silent discard with `unwrap_or_else(|e| e.exit())` to properly display help:

```rust
None => {
    cli::opts::Cli::try_parse_from(["sqllog2db", "--help"])
        .unwrap_or_else(|e| e.exit());
    std::process::exit(1);
}
```

Alternatively, directly call `cmd.print_help()` using the already-constructed command at line 112, which avoids a redundant parse:

```rust
None => {
    use std::io::Write as _;
    let _ = write!(std::io::stdout(), "{}", cmd.render_help());
    std::process::exit(0);
}
```

---

## Warnings

### WR-01: Inconsistent fallback between `field_mask()` and `ordered_field_indices()`

**File:** `src/pipeline/mod.rs:204-223`

**Issue:**
When the user specifies unknown field names in `[output.fields]`, `field_mask()` (line 204) falls back to `FieldMask::ALL` (export all 15 fields). But `ordered_field_indices()` (line 212) silently drops unknown names via `filter_map` and returns only the valid subset. This creates an inconsistency: the mask says "export everything" but the exporter uses the much smaller ordered list. If `validate()` is not called before these methods, the mismatch causes data corruption (wrong number of fields exported).

**Fix:**
Make `ordered_field_indices()` consistent with `field_mask()` by falling back to all indices on error:

```rust
pub fn ordered_field_indices(&self) -> Vec<usize> {
    match &self.fields {
        None => (0..FIELD_NAMES.len()).collect(),
        Some(names) if names.is_empty() => (0..FIELD_NAMES.len()).collect(),
        Some(names) => {
            let result: Vec<usize> = names.iter()
                .filter_map(|name| FIELD_NAMES.iter().position(|&n| n == name.as_str()))
                .collect();
            // If any name was unknown, fall back to all fields to match field_mask()
            if result.len() != names.len() {
                (0..FIELD_NAMES.len()).collect()
            } else {
                result
            }
        }
    }
}
```

### WR-02: Missing DDL keywords in `is_keyword` causes inconsistent normalization

**File:** `src/pipeline/normalizer.rs:666-714`

**Issue:**
The `is_keyword` function only recognizes a limited set of SQL keywords (max length 8). Common DDL keywords are missing, including: `TABLE`, `INDEX`, `VIEW`, `SCHEMA`, `COLUMN`, `CONSTRAINT`, `PRIMARY`, `FOREIGN`, `UNIQUE`, `CHECK`, `DEFAULT`, `KEY`, `ADD`, `MODIFY`, `TRUNCATE`, `RENAME`, `COMMENT`, `CASCADE`, `RESTRICT`, `AUTO_INCREMENT`, `DATABASE`, `SEQUENCE`, `TRIGGER`, `PROCEDURE`, `FUNCTION`, `PACKAGE`, `TYPE`, `BEFORE`, `AFTER`, `EACH`, `ROW`, `STATEMENT`, `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `GRANT`, `REVOKE`, `EXECUTE`, `DECLARE`, `CURSOR`, `LOOP`, `EXIT`, `CONTINUE`, `WHILE`, `FOR`, `IF`, `THEN`, `ELSE`, `ELSIF`, `GOTO`, `RETURN`, `RAISE`, `PRAGMA`, `AUTHID`, `DETERMINISTIC`, `LANGUAGE`, `WRAPPED`, `COMPILE`, `REUSE`, `SETTINGS`, `NOT`, `AND`, `OR`, `XOR`, `ALL`, `ANY`, `SOME`, `EXISTS`, `UNIQUE`, `DISTINCT`, `ASC`, `DESC`, `NULLS`, `FIRST`, `LAST`, `FETCH`, `NEXT`, `ROWS`, `ONLY`, `WITH`, `TIES`, `TABLESAMPLE`, `SYSTEM`, `BERNOULLI`, `PIVOT`, `UNPIVOT`, `MATCHED`, `MERGE`, `WHEN`, `THEN`, `ELSE`, `SOURCE`, `TARGET`, `USING`.

Because these are missing, `CREATE TABLE t` normalizes to `CREATE table t` (case-preserved), while `CREATE TABLE T` normalizes correctly to `CREATE TABLE T`. Two structurally identical SQL statements get different template keys, defeating the purpose of normalization.

**Fix:**
Expand the keyword set. Either add the most common missing keywords, or switch to a dynamic approach (uppercase any word that consists entirely of uppercase letters in the original SQL):

One pragmatic approach is to treat all-uppercase words of any length as keywords (not just the explicit list), since normalization is about CASE-FOLDING — not semantic keyword detection:

```rust
fn is_keyword(word: &[u8]) -> bool {
    // All-uppercase words are treated as keywords for case normalization
    if word.iter().all(|&b| b.is_ascii_uppercase()) {
        return true;
    }
    // ... existing list for mixed/lowercase common keywords ...
}
```

### WR-03: Weak assertion in `test_resume_reprocesses_changed_file`

**File:** `tests/integration.rs:338-340`

**Issue:**
The assertion `rows >= 1` is too weak. After reprocessing a file that grew from 5 to 10 records, the test checks only that output exists but not the expected count. Even if the reprocessing logic is broken and produces 0 data rows (just a header), the assertion passes because the CSV header line makes `rows >= 1` true.

**Fix:**
Assert the exact expected line count (header + 10 data rows = 11):

```rust
let rows = std::fs::read_to_string(&csv2).unwrap().lines().count();
assert_eq!(
    rows, 11,
    "expected header + 10 data rows from reprocessed file, got {rows}"
);
```

---

## Info

### IN-01: `#![allow(dead_code)]` in lang.rs suppresses warnings unnecessarily

**File:** `src/lang.rs:11`

**Issue:**
The `#![allow(dead_code)]` attribute is applied at the module level. All public/crate-visible items in this module appear to be used within the crate (`detect`, `apply_zh`, `Lang`). If the attribute exists to silence warnings during incremental development, it should be removed after confirming no dead code remains. Keeping it risks masking real dead code that should be cleaned up.

### IN-02: Fragile `split(',')` for CSV field counting in tests

**File:** `tests/integration.rs:1117, 1170`

**Issue:**
Two tests (`test_e2e_template_normalization` line 1117, `test_e2e_field_projection` line 1170) use `data_line.split(',').count()` to count CSV fields. This approach is fragile — if the SQL statement in test data ever contains a comma, the field count will be wrong and assertions could silently pass or fail for the wrong reason. The comments at lines 1119-1120 and 1171-1172 acknowledge this limitation.

Using a proper CSV parser like the `csv` crate would make these tests robust against future changes to test data.

---

_Reviewed: 2026-05-20T10:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
