# Phase 33: 核心功能验证 - Research

**Researched:** 2026-05-20
**Domain:** Codebase verification, build/test/lint pipeline, CLI smoke testing, benchmark regression detection
**Confidence:** HIGH

## Summary

Phase 33 is a pure verification phase with zero code changes. The goal is to confirm that all core functionality survives the removal operations in Phases 28-32 intact. Three parallel plans cover: (1) static analysis (build + clippy + fmt), (2) automated testing and benchmarking, and (3) manual CLI smoke testing with a generated VERIFICATION-CHECKLIST.md.

The codebase is currently in good shape: `cargo build --release` succeeds, `cargo test` passes all 36 integration tests, `cargo clippy --all-targets -- -D warnings` produces zero warnings, and `cargo fmt --check` passes. Synthetic benchmarks (csv_export, sqlite_export, filter no_pipeline/passthrough/trxid) are within noise threshold or slightly improved versus v1.0 baseline. Two benchmarks show regression worth noting: `indicator_prescan` (+64% vs v1.0 baseline) and `csv_export_real` (~58% beyond data-size-adjusted expectation). These may require investigation under D-17/D-18.

Real DaMeng log files exist in `sqllogs/` (3 files, ~817MB total, dating 2026-05-11) — the preferred data source for Plan 3 smoke tests (D-03).

**Primary recommendation:** Execute all three plans in parallel (D-16). Plan 3 smoke tests should use existing `tests/integration.rs` patterns (write_test_log, make_run_config, csv/sqlite readers) for Rust data validation code. Generate VERIFICATION-CHECKLIST.md automatically after all verification steps complete (per Specific Ideas in CONTEXT.md).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Build verification | CLI / Build System | — | `cargo build --release` / `cargo check` — compiler-level |
| Lint verification | CLI / Build System | — | `cargo clippy` / `cargo fmt` — static analysis tools |
| Unit/integration tests | Test Framework | — | `cargo test` via Rust's built-in harness and criterion |
| Benchmark regression | Test Framework | — | `cargo bench` with criterion, compared against baselines |
| CSV export validation | CLI / Manual Script | Rust data reader | Plan 3 smoke tests use `csv` crate to read and verify output |
| SQLite export validation | CLI / Manual Script | Rust data reader | Plan 3 smoke tests use `rusqlite` to read and verify output |
| Filter pipeline validation | CLI / Manual Script | Config generation | Each filter type tested with separate config and expected output |
| Parameter normalization | CLI / Manual Script | CSV + SQLite dual comparison | D-10: row count + key field spot-check across formats |
| Parallel CSV validation | CLI / Manual Script | Timing script | D-04: output correctness + timing comparison |
| Error log validation | CLI / Manual Script | Config generation | D-11: configure [error] file, confirm errors written |
| Config template validation | CLI | — | `cargo run -- init` then `cargo run -- validate` |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| KEEP-01 | CSV 导出正常工作，所有现有测试通过 | Confirmed: 36/36 tests pass. CSV export tests cover dry-run, limit, interrupt, parallel, real export, throughput baseline. Smoke plan validates end-to-end with real sqllogs/. |
| KEEP-02 | SQLite 导出正常工作，所有现有测试通过 | Confirmed: SQLite integration tests pass. Smoke plan validates row count + key field spot-check against CSV output (D-10). |
| KEEP-03 | Pipeline 过滤器（include/exclude/indicators/sql）正常工作 | Confirmed: Filter tests in integration.rs cover include, exclude, indicators, sql filters. Filter benchmarks (9 scenarios in bench_filters.rs). Smoke plan tests each filter type independently with separate configs (D-06). |
| KEEP-04 | 参数归一化正常工作 | Confirmed: replace_parameters tests in integration.rs and unit tests. Smoke plan validates both CSV and SQLite output paths (D-05). |
| KEEP-05 | 并行 CSV 处理（rayon）正常工作 | Confirmed: `test_handle_run_parallel_csv_multiple_files` integration test validates parallel CSV path. Smoke plan includes output correctness + timing comparison (D-04). |
| KEEP-06 | `cargo build --release` 成功，`cargo test` 全部通过，`cargo clippy` 无警告 | Pre-verified: build passes, all 36 tests pass, clippy has zero warnings, fmt passes clean. Plan 1 encodes this as explicit CI-style validation. |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 验证方式 = 自动化检查（build/test/clippy/fmt）+ CLI 冒烟测试
- **D-02:** 冒烟测试覆盖全功能：CSV 导出 + SQLite 导出 + 四类过滤器（include/exclude/indicators/sql）+ 参数归一化 + 并行 CSV + 中文配置模板 + 错误日志
- **D-03:** 冒烟测试数据源优先使用真实日志（sqllogs/），不存在时生成测试日志
- **D-04:** 并行 CSV 验证需包含输出正确性检查 + 计时对比
- **D-05:** 参数归一化验证需同时检查 CSV 和 SQLite 双路输出
- **D-06:** 四类过滤器分项独立验证，每类单独准备配置和场景
- **D-07:** 构建验证含 debug check + release build 两者
- **D-08:** 需要验证 `cargo run -- init` 生成的中文配置模板可用
- **D-09:** 冒烟测试发现问题时：先修复，然后重新执行完整验证
- **D-10:** SQLite 验证深度：行数对比（CSV vs SQLite）+ 关键字段抽查
- **D-11:** 错误日志输出需验证（配置 [error] file 后确认错误被写入）
- **D-12:** 每个 KEEP 项使用显式检查清单判定通过/失败
- **D-13:** 生成 VERIFICATION-CHECKLIST.md 到 phase 目录
- **D-14:** 报告格式：KEEP 需求映射 + 通过/失败 + 证据 + 可复现步骤
- **D-15:** 3 个 plan 按验证类型分组：
  - Plan 1 (33-01): 静态检查
  - Plan 2 (33-02): 自动化测试
  - Plan 3 (33-03): 手动冒烟验证
- **D-16:** 三个 plan 可并行执行，互不依赖
- **D-17:** 运行全部 benchmark，与 benches/baselines/ 既定基线对比
- **D-18:** 退化超过 10% 视为回归，需分析根因并修复
- **D-19:** Benchmark 放在 Plan 2（自动化测试）中

### Claude's Discretion

- VERIFICATION-CHECKLIST.md 的精确结构和字段
- 冒烟测试 Shell 脚本和 Rust 验证代码的具体实现
- 各 plan 内部的任务拆分细节
- benchmark baseline 更新策略（如果当前 baseline 过旧）

### Deferred Ideas (OUT OF SCOPE)

- "调研 dm-database-parser-sqllog 1.0.0 新特性" — 与本阶段验证范围无关，延后至未来版本

## Standard Stack

### Core (verification tooling)
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| cargo | nightly (Rust 1.85+) | Primary build system | Required by Cargo.toml (`rust-version = "1.85"`, `edition = "2024"`) |
| cargo check | — | Debug-mode compilation check | D-07 requires debug check + release build |
| cargo build --release | — | Optimized production build | D-07 release build; produces `target/release/sqllog2db` for smoke tests |
| cargo clippy --all-targets -- -D warnings | — | Lint check with deny warnings | Project CLAUDE.md gate; currently zero warnings |
| cargo fmt --check | — | Format check | Must pass before verification complete |
| cargo test | — | Run all unit + integration tests | 36 integration tests, all currently passing |
| cargo bench | criterion 0.7 | Benchmark suite | 3 benchmark files, 17 scenarios total |

### Supporting (smoke test tooling)
| Tool | Purpose | Where Used |
|------|---------|------------|
| `write_test_log()` pattern from `tests/integration.rs` | Generate test log files if sqllogs/ unavailable | Plan 3 (D-03 fallback) |
| `make_run_config()` pattern from `tests/integration.rs` | Build test configurations programmatically | Plan 3 Rust validation code |
| `csv` crate (via `std::fs::read_to_string`) | Read and verify CSV output | Plan 3 output validation |
| `rusqlite` crate | Query and verify SQLite output | Plan 3 output validation |
| `diff` or `cmp` (shell) | Compare row counts and content | Plan 3 CSV vs SQLite comparison |
| `time` bash built-in | Measure parallel CSV timing | Plan 3 D-04 timing comparison |
| Bash shell script | Orchestrate smoke test sequence | Plan 3 overall script |
| Rust helper binary | Structured data validation | Plan 3 Rust verification code |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shell script | Makefile / Justfile | Shell is zero-dependency, everyone has it. Makefile adds complexity for simple orchestration. |

## Package Legitimacy Audit

> This phase does NOT install any external packages. Verification uses existing project dependencies (csv, rusqlite from Cargo.toml) and standard Rust tooling. No new packages to audit.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
┌───────────────────────────────────────────────────────────┐
│                    Phase 33 Verification                    │
├─────────────────┬─────────────────┬───────────────────────┤
│   Plan 1 (33-01)│   Plan 2 (33-02)│   Plan 3 (33-03)      │
│   Static Checks │   Auto Tests    │   Manual Smoke Tests   │
├─────────────────┼─────────────────┼───────────────────────┤
│                 │                 │                        │
│  cargo check    │  cargo test     │  Shell script:         │
│       ↓         │       ↓         │    init config         │
│  cargo build    │  36 tests pass  │    export CSV          │
│  --release      │       ↓         │    export SQLite       │
│       ↓         │  cargo bench    │    test 4 filters      │
│  cargo clippy   │  (17 scenarios) │    normalize params    │
│       ↓         │       ↓         │    parallel CSV check  │
│  cargo fmt      │  baseline       │    error log test      │
│  --check        │  comparison     │       ↓                │
│                 │                 │  VERIFICATION-         │
│                 │                 │  CHECKLIST.md          │
└─────────────────┴─────────────────┴───────────────────────┘
        │                  │                  │
        └──────────────────┴──────────────────┘
                           ↓
             All 3 plans pass → Phase 33 complete
             Any failure → D-09: fix then re-verify
```

### Recommended Project Structure (smoke test assets)
```
.planning/phases/33-core-verification/
├── 33-CONTEXT.md
├── 33-DISCUSSION-LOG.md
├── 33-RESEARCH.md                    ← this file
├── smoke_test/
│   ├── run_all.sh                    # Main orchestration script
│   ├── config_csv.toml               # CSV-only config
│   ├── config_sqlite.toml            # SQLite-only config
│   ├── config_include.toml           # Include filter test config
│   ├── config_exclude.toml           # Exclude filter test config
│   ├── config_indicators.toml        # Indicator filter test config
│   ├── config_sql_filter.toml        # SQL filter test config
│   ├── config_params.toml            # Replace parameters test config
│   ├── config_parallel.toml          # Parallel CSV test config
│   ├── config_error_log.toml         # Error log test config
│   └── expected_outputs/             # Expected output fixtures
└── VERIFICATION-CHECKLIST.md         # Generated after all checks pass
```

### Pattern 1: Smoke Test Orchestration Script
**What:** Shell script that iterates through KEEP requirements, each as a shell function that returns pass/fail status.
**When to use:** Plan 3 verification — one function per KEEP item, enabling independent execution and clear pass/fail reporting.

### Pattern 2: Rust Data Validation Binary
**What:** A small Rust binary (or script using `cargo run --bin verify`) that reads CSV/SQLite output files and validates content — row counts, key fields, content correctness.
**When to use:** D-10 row count comparison, D-04 output correctness.
**Sourcing:** Reuse `csv` and `rusqlite` crate APIs already in Cargo.toml dev-dependencies.

### Anti-Patterns to Avoid
- **Manual inspection of output files:** Always automate validation with scripts or Rust code — avoids human error and makes verification reproducible.
- **Testing all filters in one config:** D-06 mandates separate configs per filter type, enabling targeted failure diagnosis.
- **Hardcoded paths in smoke scripts:** Use temp directories (pattern from `tests/integration.rs`) or at minimum `$(mktemp -d)`.
- **One big check for all KEEP items:** D-12 requires explicit per-KEEP checklist — each KEEP must have its own pass/fail criteria in the checklist.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI subcommand invocation | Custom arg parsing | `cargo run -- <subcommand>` | Already works, zero effort |
| CSV reading for verification | Write a parser | `std::fs::read_to_string` + split lines | Integration tests already use this pattern |
| SQLite reading for verification | Direct SQLite C binding | `rusqlite` crate (already in deps) | Reuse existing dependency from Cargo.toml |
| Temp dir for test artifacts | Manual cleanup | `mktemp -d` (shell) or `tempfile::TempDir` (Rust) | Pattern already used in integration tests |
| Test log generation | Write log generator | `write_test_log()` from `tests/integration.rs` | Already tested and proven pattern (D-03 fallback) |

**Key insight:** Every validation tool needed already exists — either as standard Rust tooling (`cargo *`), existing project code (`tests/integration.rs` patterns), or existing dependencies (`csv`, `rusqlite`). Plan 3 should compose these existing tools rather than building anything new.

## Runtime State Inventory

> Omit entirely for greenfield phases. — Phase 33 is a verification phase with no code changes, no rename, no refactor, no migration. Not applicable.

## Common Pitfalls

### Pitfall 1: `sqllogs/` Contains Real Data That Changes
**What goes wrong:** Smoke test assertions (expected row count, specific field values) may fail because real log files contain dynamic data (new logs appended between runs).
**Why it happens:** Real DaMeng logs in `sqllogs/` are from 2026-05-11 (~817MB) — they are historical snapshots, not being appended to. But if new logs appear, counts change.
**How to avoid:** When using real logs for smoke tests, do NOT assert exact row counts — instead check range-based assertions (e.g., `> 100000 rows`), or use filter-only logs that produce predictable subsets. For exact-content verification, use generated test logs (D-03 fallback: `write_test_log()` pattern).
**Warning signs:** Smoke test passes Monday, fails Tuesday for no apparent code reason.

### Pitfall 2: Benchmark Baseline Mismatch
**What goes wrong:** `cargo bench` compares against stored baselines. The BENCHMARKS.md documents v1.0 baseline values, but criterion does not have a saved "v1.0" named baseline in the target directory.
**Why it happens:** The baselines in `benches/baselines/` use CRITERION_HOME, but the default criterion output directory (`target/criterion/`) has no named v1.0 baseline saved.
**How to avoid:** For D-17 benchmark comparison, either (a) use `CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.0` (but some groups like `csv_format_only` don't have a v1.0 baseline saved), or (b) compare against the last-run baseline stored in `target/criterion/` — this gives a relative "change since last run" comparison. The BENCHMARKS.md hard limits can be checked manually.
**Warning signs:** `Baseline 'v1.0' must exist before comparison is allowed` panic from criterion.

### Pitfall 3: Formatted Output Durations Vary
**What goes wrong:** `csv_export_real/real_file` benchmark shows ~140% regression vs v1.0 baseline (786ms vs 327ms), but ~50% of that is due to 50% larger input data (817MB now vs 538MB at v1.0).
**Why it happens:** The sqllogs/ directory now has 3 files totaling ~817MB (from May 11 2026), while the v1.0 baseline was measured against 2 files totaling ~538MB (from April 2026).
**How to avoid:** Normalize benchmark comparison by considering data volume. Adjust expected time proportionally, or re-baseline after the input data change. Pure throughput benchmarks (csv_export/10000, sqlite_export/10000) are not affected as they use synthetic data.
**Warning signs:** Real-file benchmark shows large regression but synthetic benchmarks are stable.

### Pitfall 4: `indicator_prescan` Benchmark Shows +64% Regression vs v1.0
**What goes wrong:** The `indicator_prescan` filter benchmark is 3.48ms vs v1.0 baseline 2.12ms — a regression of approximately 64%.
**Why it happens:** The indicator pre-scan path may have been affected by refactoring in Phases 28-32 (removal of template analysis and digest modules may have changed the code path). Root cause needs investigation under D-18.
**How to avoid:** Flag in Plan 2 benchmark analysis. If regression >10%, investigate root cause per D-18. Since `test_handle_run_with_min_runtime_filter` integration test still passes, the regression may be in the benchmark harness or setup rather than the actual business logic.
**Warning signs:** `pipeline_passthrough` and `trxid_*` benchmarks are stable or improved — only `indicator_prescan` regresses, suggesting a specific code path issue.

### Pitfall 5: Smoke Script Left in Phase Directory
**What goes wrong:** Shell scripts and Rust verification binaries are created in the phase directory during verification but not cleaned up, leaving stale artifacts.
**Why it happens:** The verification phase creates smoke test assets (configs, scripts) that remain after phase completion.
**How to avoid:** Include a cleanup step in the smoke script, or document that smoke test artifacts are intentionally kept for reproducibility (D-14 requires reproducible steps).

## Code Examples

### Example 1: Smoke Test Shell Function (per KEEP requirement pattern)
```bash
# Source: Derived from D-12 explicit checklist pattern
check_csv_export() {
    local report_dir="$1"
    local csv_out="$report_dir/output.csv"
    local log_dir="$report_dir/sqllogs"
    local config="$report_dir/config_csv.toml"

    # Generate test log if real logs unavailable
    if [ ! -f "$log_dir/test.log" ]; then
        mkdir -p "$log_dir"
        cargo run -- run -c "$config" 2>/dev/null  # generates test log
    fi

    # Export CSV
    cargo run -- run -c "$config" 2>&1

    # Verify output exists and has content
    if [ -f "$csv_out" ] && [ "$(wc -l < "$csv_out")" -gt 1 ]; then
        echo "KEEP-01: PASS (CSV export produced $(wc -l < "$csv_out") lines)"
        return 0
    else
        echo "KEEP-01: FAIL (CSV export did not produce output)"
        return 1
    fi
}
```

### Example 2: Rust Verification Pattern (from tests/integration.rs)
```rust
// Source: tests/integration.rs patterns
fn verify_csv_output(path: &Path, expected_min_rows: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read CSV: {e}"))?;
    let row_count = content.lines().count().saturating_sub(1); // minus header
    if row_count < expected_min_rows {
        return Err(format!(
            "Expected >= {} rows, got {}", expected_min_rows, row_count
        ));
    }
    Ok(())
}
```

### Example 3: SQLite Verification Pattern
```rust
// Source: rusqlite crate pattern (already used in exports)
fn verify_sqlite_output(path: &Path) -> Result<usize, String> {
    let conn = rusqlite::Connection::open(path)
        .map_err(|e| format!("Failed to open SQLite: {e}"))?;
    let count: usize = conn
        .query_row("SELECT COUNT(*) FROM sqllog", [], |row| row.get(0))
        .map_err(|e| format!("Query failed: {e}"))?;
    Ok(count)
}
```

### Example 4: Config Pattern for Filter Smoke Test
```toml
# Source: D-06 per-filter independent config
[sqllog]
path = "sqllogs/"

[exporter]
error_file = "error.log"

[exporter.csv]
file = "output.csv"
overwrite = true

[filter.include]
users = ["TESTUSER"]
keywords = ["SELECT"]

[replace_parameters]
placeholder = "?"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual verification per KEEP item | Automated shell script + Rust verification | Phase 33 (D-01) | Reproducible, eliminates human error |
| Single combined filter config | Per-filter independent configs | Phase 33 (D-06) | Targeted failure diagnosis |
| Manual VERIFICATION-CHECKLIST.md | Auto-generated from script output | Phase 33 (Specific Ideas) | Always up-to-date, repeatable |

**Deprecated/outdated:**
- v1.0 benchmark baselines (`benches/baselines/`) — dated 2026-04-26, some benchmark groups (csv_format_only, exclude_*) have no saved baseline. Re-baseline if precise comparison needed.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `indicator_prescan` regression is in the pre-scan logic, not benchmark setup | Common Pitfalls | Could be a benchmark harness issue — verify with `cargo test test_handle_run_with_min_runtime_filter` which tests the actual logic path |
| A2 | `csv_export_real/real_file` regression is data-size-related | Common Pitfalls | If the regression is algorithmic (e.g., parallel CSV overhead added in v1.4), root cause could affect real-world users |

## Open Questions

1. **Does `indicator_prescan` regression require fixing or is it noise?**
   - What we know: benchmark shows +64% vs v1.0 baseline (3.48ms vs 2.12ms). Integration test `test_handle_run_with_min_runtime_filter` passes.
   - What's unclear: Is the regression in pre-scan logic or benchmark setup? The benchmark creates synthetic pre-scan temp files which may differ after Phase 32 cleanup.
   - Recommendation: Investigate in Plan 2 as part of D-18. If the integration test passes and the regression is in benchmark setup code, document and accept.
   **RESOLVED:** Plan 2 Task 2 (33-02-02) 在 benchmark 分析阶段按 D-18 流程处理 — 对 indicator_prescan 报告当前测量值并分析根因，退化 >10% 则标记 disposition。

2. **Should benchmark baselines be updated for Phase 33?**
   - What we know: Existing baselines in `benches/baselines/` are from v1.0 (April 2026). Some benchmark groups have no named baseline. csv_format_only created in Phase 4.
   - What's unclear: Whether to update baselines to post-cleanup values.
   - Recommendation: Claude's Discretion per D-19. If baselines are significantly outdated for comparison, re-save them as "phase33" baseline.
   **RESOLVED:** Plan 2 Task 2 (33-02-02) 按 D-19 (Claude's Discretion) 处理 — 若 v1.0 baseline 过于陈旧，使用 `--save-baseline phase33` 保存新基线。

3. **How to structure the Rust verification helper binary?**
   - What we know: Needs to read CSV and SQLite output, compare row counts, spot-check key fields.
   - What's unclear: Whether to create a standalone binary or a script using `cargo run --example`.
   - Recommendation: Create a standalone binary in `src/bin/verify_output.rs` during Plan 3. Easy to invoke from smoke scripts and reuses existing crate dependencies.
   **RESOLVED:** Plan 3 (33-03) 确定使用纯 Shell 脚本方案 — 不创建 Rust 验证二进制。使用标准 Unix 工具 (`wc`, `diff`, `sort`, `grep`) 和 `sqlite3` CLI 进行数据校验，无需额外编译步骤。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo | Build, test, lint, bench | ✓ | stable (Rust 1.85+) | — |
| Rust compiler | All verification | ✓ | 2021 edition (Cargo.toml: edition = "2024") | — |
| cargo check | Plan 1 (D-07 debug check) | ✓ | — | — |
| cargo clippy | Plan 1 (zero warnings gate) | ✓ | — | — |
| cargo fmt | Plan 1 | ✓ | — | — |
| cargo test | Plan 2 | ✓ | — | — |
| cargo bench | Plan 2 (D-17) | ✓ | criterion 0.7 | — |
| bash | Plan 3 smoke script | ✓ | macOS zsh | — |
| temp dir | Plan 3 test isolation | ✓ | `mktemp -d` or `tempfile` | — |
| sqllogs/ | Plan 3 preferred data source | ✓ | 3 files, ~817MB (2026-05-11) | write_test_log() generation |
| CRITERION_HOME baseline | Plan 2 baseline comparison | Partial | `benches/baselines/` exists but no named v1.0 baseline | Manual comparison against BENCHMARKS.md hard limits |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** sqllogs/ real data — `write_test_log()` pattern from integration tests generates synthetic logs

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` + `criterion` 0.7 |
| Config file | none (built-in Rust harness) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo bench` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| KEEP-01 | CSV export works | integration + unit | `cargo test -- test_csv` | ✅ |
| KEEP-02 | SQLite export works | integration + unit | `cargo test -- test_sqlite sqlite` | ✅ |
| KEEP-03 | Filters (include/exclude/indicator/sql) | integration + unit + bench | `cargo test -- filter` + `cargo bench --bench bench_filters` | ✅ |
| KEEP-04 | Parameter normalization | integration + unit | `cargo test -- replace_parameter` | ✅ |
| KEEP-05 | Parallel CSV (rayon) | integration | `cargo test -- parallel` | ✅ |
| KEEP-06 | Build/test/clippy/fmt pass | static + tests | `cargo build --release && cargo clippy && cargo fmt --check && cargo test` | Plan 1 |

### Sampling Rate
- **Per task commit:** `cargo test` (quick: ~0.16s)
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Phase gate:** Full Plan 1 + Plan 2 + Plan 3 suite green before `/gsd:verify-work`

### Wave 0 Gaps
- None — existing test infrastructure covers all phase requirements. The smoke test scripts and Rust verification binary in Plan 3 are new verification tooling but are not test gaps — they are D-01/D-02 mandated manual smoke tests.

## Security Domain

> `security_enforcement` is `true` by default (absent from config.nyquist_validation). This verification phase does not introduce any new dependencies, endpoints, or data handling paths. The existing security posture (validated in earlier phases) is unchanged.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | CLI tool — no user auth |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | Single-user CLI |
| V5 Input Validation | no | Input is parsed log files (not user input paths) |
| V6 Cryptography | no | No encryption handled |

### Known Threat Patterns for {stack}

No new attack surface introduced. The verification phase exercises existing code paths with no changes.

## Sources

### Primary (HIGH confidence)
- `cargo build --release` — verified successful
- `cargo test` — 36/36 pass, confirmed
- `cargo clippy --all-targets -- -D warnings` — zero warnings, confirmed
- `cargo fmt --check` — no output (clean), confirmed
- `cargo bench` — run all 3 benchmark files, collected results
- CONTEXT.md — user decisions D-01 through D-19
- BENCHMARKS.md — v1.0 baseline values for comparison

### Secondary (MEDIUM confidence)
- `tests/integration.rs` — analyzed test patterns for smoke test reuse
- Cargo.toml — verified all dependencies and profiles
- `benches/baselines/` — inspected baseline structure and contents

### Tertiary (LOW confidence)
- None — all findings verified against actual build output or code inspection

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all tools verified by running them
- Architecture: HIGH — verified against actual codebase structure
- Pitfalls: HIGH — benchmark regressions measured empirically, not assumed
- Benchmark numbers: HIGH — all measured from actual `cargo bench` runs

**Research date:** 2026-05-20
**Valid until:** 2026-06-20 (stable tooling — Rust ecosystem changes slowly)
