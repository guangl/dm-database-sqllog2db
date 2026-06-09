---
phase: 72-bench-baseline
reviewed: 2026-06-08T10:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - benches/BENCHMARKS.md
  - benches/hyperfine-validate.toml
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 72: Code Review Report

**Reviewed:** 2026-06-08
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Phase 72 adds a CLI cold-start baseline section to `benches/BENCHMARKS.md` (BENCH-01) and archives
19 Criterion v1.20 baseline directories (BENCH-02). Plan 72-03 created
`benches/hyperfine-validate.toml` as a minimal valid config fixture to ensure `validate` exits 0,
closing the prior CR-01 gap.

The Criterion baseline count (19 directories) was verified against the filesystem and matches the
document's claim. The arithmetic in the comparison table is correct. The `hyperfine-validate.toml`
fixture parses cleanly with the current `Config` schema: all field names are current, no deprecated
keys are used, and no filesystem existence checks are triggered by the `validate` subcommand, so
the fixture works as documented even when `sqllogs/` is absent. The raw hyperfine output for the
`validate` command no longer contains a non-zero exit warning, confirming the CR-01 fix succeeded.

Two warnings were found: the comparison table row label for Phase 9 names a command that Phase 9
never ran, and Phase 9's reproduction commands reference files that no longer work in the current
codebase. Two info items cover the omitted `-N` shell flag and an unacknowledged OS version
difference between compared phases.

---

## Warnings

### WR-01: Comparison table row label names a command Phase 9 never executed

**File:** `benches/BENCHMARKS.md:753`

**Issue:** The comparison table row reads:

```
| `validate -c benches/hyperfine-validate.toml` | ~2.8 ms | 2.4 ms | −0.4 ms |
```

The Phase 9 column value `~2.8 ms` was measured with `validate -c config.toml` (see Phase 9
section, line 328). `benches/hyperfine-validate.toml` did not exist in Phase 9 — it was created by
Plan 72-03. A reader who sees `Phase 9 (v1.9) mean: ~2.8 ms` next to the command
`validate -c benches/hyperfine-validate.toml` will infer that Phase 9 ran that exact command,
which is false.

The footnote below the table explains the fixture difference but does not correct the row label
itself. A future engineer querying the table in isolation (e.g., from a linked document or
automated script) will see incorrect metadata.

**Fix:** Use a path-agnostic label for the row, or split the command column to differentiate the
Phase 9 and Phase 72 commands:

```markdown
| 命令 | Phase 9 (v1.9) mean | Phase 72 (v1.20) mean | 差值 |
|------|--------------------|-----------------------|------|
| `--version` | ~2.9 ms | 2.1 ms | −0.8 ms |
| `validate`（成功路径）¹ | ~2.8 ms | 2.4 ms | −0.4 ms |

¹ Phase 9 measured `-c config.toml`（v1.9 时该键合法）；Phase 72 改用
  `-c benches/hyperfine-validate.toml`（v1.20 专用 fixture）。两者均为 exit 0 成功路径。
```

---

### WR-02: Phase 9 section reproduction commands reference files that no longer work

**File:** `benches/BENCHMARKS.md:327-329`

**Issue:** The Phase 9 "测量命令" block lists three commands:

```bash
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config_no_regex.toml'
```

Two of these are broken today:

1. `config.toml` — as of v1.12, `SqllogConfig` removed `#[serde(alias = "directory")]`. The root
   `config.toml` still uses `directory = "sqllogs"`. Serde silently drops the unknown key; `inputs`
   then receives `Vec::default()` (empty), and `SqllogConfig::validate()` returns an error, causing
   `exit(EXIT_FATAL)`. Running this command today benchmarks the failure path, not the success path
   Phase 9 originally measured.

2. `config_no_regex.toml` — this file no longer exists in the repository. The command would
   immediately fail with a file-not-found error.

Any engineer attempting to reproduce or extend the Phase 9 baseline using this section's commands
will get incomparable or broken results without any in-document warning.

**Fix:** Annotate the Phase 9 reproduction block to indicate that these commands apply to v1.9 only
and are not reproducible without the original environment:

```markdown
> **注意（历史记录）：** 以下命令适用于 Phase 9（v1.9）运行环境。在 v1.20 上执行时：
> - `config.toml` 因 `directory` 键在 v1.12 后不再被识别，`validate` 会以 exit 2 终止。
> - `config_no_regex.toml` 已从 repo 中移除。
> 如需在 v1.20 上复现冷启动计时，请参考 Phase 72 节的命令（使用
> `benches/hyperfine-validate.toml`）。
```

---

## Info

### IN-01: Hyperfine measurements omit `-N`/`--shell=none`, leaving shell startup overhead in results

**File:** `benches/BENCHMARKS.md:745-748` and `765`, `778`

**Issue:** Both raw hyperfine outputs include the warning:

```
Warning: Command took less than 5 ms to complete. Note that the results might be inaccurate
because hyperfine can not calibrate the shell startup time much more precise than this limit.
You can try to use the `-N`/`--shell=none` option to disable the shell completely.
```

The reproduction commands in the Phase 72 section do not include `-N`. For a 2-3 ms binary, the
shell startup overhead (typically 1-2 ms on macOS) is a meaningful fraction of the total measured
time. The comparison to Phase 9 is internally consistent (neither used `-N`) but both baselines
carry avoidable measurement noise.

**Fix:** Update the Phase 72 reproduction commands to add `-N`, and note that future baselines
should use the same flag for consistency:

```bash
hyperfine -N --warmup 3 './target/release/sqllog2db --version'
hyperfine -N --warmup 3 './target/release/sqllog2db validate -c benches/hyperfine-validate.toml'
```

Note: `-N` is not retroactively applicable to Phase 9 data; if Phase 9 vs Phase 72 comparison
values are both measured without `-N`, the delta is still valid. The flag matters most when
establishing fresh baselines for future phase comparisons.

---

### IN-02: OS version difference between Phase 9 and Phase 72 not acknowledged in comparison table

**File:** `benches/BENCHMARKS.md:750-753`

**Issue:** Phase 9 measurements were taken on Darwin 25.4.0 (line 322); Phase 72 measurements
were taken on Darwin 25.5.0 (line 739). The comparison table presents the two mean values side by
side without noting this difference. For 2-3 ms measurements, the OS delta is unlikely to be the
dominant factor, but it is an uncontrolled variable in the comparison.

**Fix:** Add a table footnote or a prose note:

```markdown
> ² Phase 9 使用 Darwin 25.4.0，Phase 72 使用 Darwin 25.5.0。对于 2-3ms 量级的冷启动，
>   OS 小版本差异影响预计在 ±0.1ms 误差范围内。
```

---

_Reviewed: 2026-06-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
