---
phase: 72-bench-baseline
reviewed: 2026-06-08T00:00:00Z
depth: standard
files_reviewed: 1
files_reviewed_list:
  - benches/BENCHMARKS.md
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 72: Code Review Report

**Reviewed:** 2026-06-08
**Depth:** standard
**Files Reviewed:** 1
**Status:** issues_found

## Summary

Phase 72 adds a new section to `benches/BENCHMARKS.md` recording CLI cold-start timings via
hyperfine and archiving Criterion v1.20 baselines. The Criterion baseline count (19 directories)
was verified against the filesystem and is accurate. The arithmetic in the comparison table is
correct. The "How to compare" section was correctly updated with the v1.20 command.

One critical accuracy defect was found: the `validate` hyperfine measurement was taken while the
binary was exiting with a non-zero status code (exit 2), meaning the timing reflects the binary's
failure path rather than the success path. This makes the Phase 9 vs Phase 72 comparison for
`validate` apples-to-oranges, and the implied "-0.6 ms improvement" misleading. Two warnings
document the lack of explanation for the non-zero exit and an unverifiable version label. One
info item flags missing `-N` flag guidance.

---

## Critical Issues

### CR-01: `validate` hyperfine measurement benchmarks failure path, not success path

**File:** `benches/BENCHMARKS.md:772-778` (Phase 72 section, validate raw output block)

**Issue:** The hyperfine raw output for `validate -c config.toml` includes
`Warning: Ignoring non-zero exit code.` This is not cosmetic. The binary exits with
`EXIT_FATAL` (code 2) because `config.toml` in the project root uses the old
`[sqllog] directory = "sqllogs"` key. As of v1.20, `SqllogConfig` was refactored from a
`path: String` field with `#[serde(alias = "directory")]` to an `inputs: Vec<String>` field
without that alias. When `config.toml` is parsed, `directory` is an unknown key silently
ignored by serde, `inputs` receives `Vec::default()` (i.e., `[]`), and
`SqllogConfig::validate()` immediately returns
`ConfigError::InvalidValue { field: "sqllog.inputs", … inputs cannot be an empty array }`,
causing the binary to `std::process::exit(EXIT_FATAL)`.

This was confirmed by running the binary directly:

```
$ cargo run --release -- validate -c config.toml
[FAIL] Configuration error: Invalid configuration value sqllog.inputs = '[]':
       inputs cannot be an empty array; …
$ echo $?
2
```

Consequence: the Phase 72 `validate` timing of **2.2 ms** measures startup + TOML parse +
`validate()` short-circuit failure. The Phase 9 `validate` timing of **~2.8 ms** measured
startup + TOML parse + `validate()` success + `handle_validate()` print. The
`−0.6 ms` delta in the comparison table does not represent a cold-start improvement; it
conflates two different execution paths.

**Fix:** There are two acceptable remedies:

Option A — Annotate the existing data as a known-broken measurement and flag it clearly:
```markdown
| `validate -c config.toml` | ~2.8 ms | 2.2 ms (see ¹) | −0.6 ms |

¹ v1.20 binary exits non-zero for this config.toml (inputs field renamed from `directory`
to `inputs` in v1.12; config.toml not updated). Timing reflects failure path, not
successful validation. Do not use this value to assert a cold-start regression/improvement.
```

Option B — Re-run the measurement with a correctly formed config (one that has
`inputs = ["sqllogs"]`) so that the binary exits 0, and record a comparable success-path
timing. This is the cleaner option for a baseline document intended to be cited in future phases.

---

## Warnings

### WR-01: Non-zero exit code included verbatim with no explanatory context

**File:** `benches/BENCHMARKS.md:777`

**Issue:** The raw hyperfine output block contains:
```
  Warning: Ignoring non-zero exit code.
```
The document does not explain what caused the non-zero exit, whether the measurement is still
meaningful, or what a future engineer should do when they encounter this warning. Anyone reading
the Phase 72 section to reproduce or interpret the baseline will either be confused or silently
assume the timing is valid.

**Fix:** Add a footnote immediately after the `</details>` block (line 780) or inside the
`<details>` block, stating:

```markdown
> **Note:** hyperfine reported a non-zero exit code because `config.toml` uses the deprecated
> `[sqllog] directory` key (renamed to `inputs` in v1.12). The binary fails during
> `validate()`. See CR-01 for implications on comparability with Phase 9 data.
```

---

### WR-02: Phase 9 version label "(v1.9)" in comparison table header is unverifiable

**File:** `benches/BENCHMARKS.md:750`

**Issue:** The comparison table header reads:
```
| 命令 | Phase 9 (v1.9) mean | Phase 72 (v1.20) mean | 差值 |
```
The Phase 9 section in this file (lines 318–390) carries no version label. The label `v1.9`
is not recorded in Phase 9's own entry, and other planning artifacts confirm that v1.9 had no
independent git tag. The label was introduced in the 72-01-PLAN.md requirement spec and
propagated into the document, but it is an inference that cannot be verified from the
document itself. A future engineer comparing baselines across phases would not be able to
confirm what package version corresponds to "Phase 9 (v1.9)".

Additionally, this file's header line (line 3) anchors v1.0 to
`package version 0.10.7`, but no analogous anchor exists for v1.9, making it impossible to
verify the label without cross-referencing git history.

**Fix:** Either drop the version suffix and write `Phase 9` only, or add a parenthetical that
matches verifiable metadata. If the intent is to communicate the era, a date is more reliable:

```markdown
| 命令 | Phase 9 (~2026-05-14) mean | Phase 72 (v1.20, 2026-06-08) mean | 差值 |
```

---

## Info

### IN-01: Hyperfine measurement commands in the Phase 72 section omit `-N`/`--shell=none`

**File:** `benches/BENCHMARKS.md:745-748`

**Issue:** Both hyperfine raw outputs include the warning:
```
  Warning: Command took less than 5 ms to complete. Note that the results might be inaccurate
  because hyperfine can not calibrate the shell startup time much more precise than this limit.
  You can try to use the `-N`/`--shell=none` option to disable the shell completely.
```
The reproduction commands in the section do not include `-N`, meaning any future engineer who
runs these commands to reproduce or update the baseline will get the same "might be inaccurate"
results. For a document whose stated purpose is to record a "baseline" for regression detection,
using the recommended flag would improve measurement quality.

**Fix:** Update the reproduction commands block (lines 745-748) to include `-N`:

```bash
hyperfine -N --warmup 3 './target/release/sqllog2db --version'
hyperfine -N --warmup 3 './target/release/sqllog2db validate -c config.toml'
```

Note: `-N` disables the intermediate shell (`/bin/sh -c`) used by hyperfine to launch the
command. This means shell startup time is excluded from the measurement, giving a more accurate
baseline for the binary itself. Apply consistently to both commands.

---

_Reviewed: 2026-06-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
