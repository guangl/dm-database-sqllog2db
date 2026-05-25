---
status: complete
phase: 45-ci
source: 41-01-SUMMARY.md, 42-01-SUMMARY.md, 43-01-SUMMARY.md, 43-02-SUMMARY.md, 44-01-SUMMARY.md, 44-02-SUMMARY.md, 44-03-SUMMARY.md, 45-01-SUMMARY.md, 45-02-SUMMARY.md
started: 2026-05-25T02:01:30Z
updated: 2026-05-25T02:05:30Z
covers: "41,42,43,44,45"
---

## Current Test

[testing complete]

## Tests

### 1. Release 构建无警告 (Phase 41)
expected: `cargo build --release` 退出码 0，无任何 warning: 或 error: 行
result: pass

### 2. 全量测试通过 (Phase 41-45)
expected: `cargo test` 全部通过，0 failed（lib + integration + jemalloc_peak 共计 272+ tests）
result: pass

### 3. 四套 Benchmark 可编译 (Phase 42)
expected: `cargo bench --no-run` 退出码 0，输出显示 bench_csv / bench_filters / bench_parser / bench_sqlite 四个 Executable 行
result: pass

### 4. Parser 基线文件存在 (Phase 42)
expected: `benches/baselines/parser_throughput/` 下有 1000/10000/50000 三个目录，各含 `v1.0/estimates.json`
result: pass

### 5. 无遗留 i64 类型转换 (Phase 43)
expected: `grep -rn "i64::from(result.rowcount)" src/` 输出为空（已替换为直接传 u32）
result: pass

### 6. Arc ParamBuffer 已启用 (Phase 44)
expected: `grep -c "Arc<Vec<ParamValue>>" src/pipeline/normalizer.rs` 输出 ≥ 1（type alias 行）
result: pass

### 7. BufWriter 16MB 容量 (Phase 44)
expected: `grep -c "16 \* 1024 \* 1024" src/exporter/csv/mod.rs` 输出 1（2MB 已替换）
result: pass

### 8. SQLite 并行路径测试 (Phase 45)
expected: `cargo test --lib test_sqlite_parallel` 输出 "1 passed; 0 failed"，test_sqlite_parallel_matches_sequential ok
result: pass

### 9. Benchmark CI Workflow 触发条件 (Phase 45)
expected: .github/workflows/bench.yml 同时监听 pull_request 和 push to main；continue-on-error: true；retention-days 在 30-90 范围内
result: skipped
reason: CI 尚未实际触发（文件配置本地核查通过，需推送 PR 后到 GitHub Actions 确认）

### 10. Collect 脚本可执行 (Phase 45)
expected: `scripts/collect_bench_results.sh` 存在且有执行权限（-rwxr-xr-x），前几行含 set -euo pipefail
result: pass

## Summary

total: 10
passed: 9
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps

[none yet]
