---
phase: 72-bench-baseline
plan: "03"
subsystem: benchmarks
tags: [hyperfine, benchmark, cr-01, gap-closure, validate]
dependency_graph:
  requires: [72-01, 72-02]
  provides: [CR-01-closed, BENCH-01-complete]
  affects: [benches/BENCHMARKS.md]
tech_stack:
  added: []
  patterns: [hyperfine-fixture, validate-success-path]
key_files:
  created:
    - benches/hyperfine-validate.toml
  modified:
    - benches/BENCHMARKS.md
decisions:
  - "采纳 Option B：新建合法 fixture 重新采集 validate 成功路径数据，替换失败路径误导值"
  - "hyperfine-validate.toml 使用 inputs = [\"sqllogs\"] 字段（v1.12+ 合法），不含旧字段 directory"
  - "validate 成功路径 mean = 2.4 ms（Phase 9 基线 ~2.8 ms，差值 -0.4 ms）"
metrics:
  duration: "约 5 分钟（含 cargo build --release 36s + hyperfine 运行）"
  completed: "2026-06-08"
  tasks: 2
  files: 2
---

# Phase 72 Plan 03: CR-01 gap-closure（validate 成功路径基准重新采集）Summary

**One-liner:** 新建 `benches/hyperfine-validate.toml` fixture（inputs 非空，exit 0），重新采集 validate 成功路径 hyperfine 数据（2.4ms），替换 BENCHMARKS.md Phase 72 段落中基于失败路径（exit 2）的误导值，闭合 VERIFICATION CR-01 BLOCKER。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 创建 hyperfine-validate.toml fixture 并采集 validate 成功路径数值 | 3dcd35d | benches/hyperfine-validate.toml |
| 2 | 替换 BENCHMARKS.md Phase 72 段落 validate 行数据并移除失败路径痕迹 | e32b353 | benches/BENCHMARKS.md |

## What Was Built

### benches/hyperfine-validate.toml（新建）

hyperfine validate 子命令专用最小合法 config fixture：
- `[sqllog] inputs = ["sqllogs"]`（v1.12+ 合法字段，非空，validate 走成功路径）
- 含 `[error]`、`[logging]`、`[exporter.csv]` 完整结构（满足"只能配置一个导出器"约束）
- 不含旧字段 `directory`（消除 CR-01 根因：serde 静默丢弃 → inputs=[] → exit 2）
- 自带注释说明用途与"sqllogs/ 不必实际存在"的原因，未来开发者可独立复现

### benches/BENCHMARKS.md（修改，仅 Phase 72 段落内部）

三处替换 + 一处新增脚注，全部在 line 735 之后的 Phase 72 段落内：

1. **bash 代码块**：第二条命令路径从 `config.toml` 改为 `benches/hyperfine-validate.toml`
2. **对比表 validate 行**：命令字段更新为新 fixture 路径，mean 由 2.2ms → 2.4ms，差值由 -0.6ms → -0.4ms
3. **validate `<details>` 块**：替换为成功路径 stdout 原文（2.4ms，无 `Warning: Ignoring non-zero exit code.`）
4. **脚注**（对比表下方新增）：解释为何用专用 fixture、config.toml 旧字段根因

Phase 72 之外所有段落（Phase 4/5/6/9/10/42/44/56）及 --version 行、--version `<details>` 块、Criterion 部分、结论 checklist 零修改。

## CR-01 闭合证据

| 判别项 | 修改前 | 修改后 |
|--------|--------|--------|
| `validate -c <config>` 退出码 | exit 2（失败路径） | exit 0（成功路径） |
| hyperfine stdout 含 `Warning: Ignoring non-zero exit code.` | 是（line 777） | 否（已移除） |
| 对比表 Phase 72 mean 列数据来源 | 失败路径（立即 ConfigError 退出） | 成功路径（完整 validate + 打印） |
| 与 Phase 9 可比性 | 不可比（路径不同） | 同口径可比（均为成功路径） |

## Benchmark 数据（v1.20 成功路径）

| 命令 | Phase 9 (v1.9) mean | Phase 72 (v1.20) mean | 差值 |
|------|--------------------|-----------------------|------|
| `--version` | ~2.9 ms | 2.1 ms | −0.8 ms |
| `validate -c benches/hyperfine-validate.toml` | ~2.8 ms | 2.4 ms | −0.4 ms |

测量环境：Apple Silicon (Darwin 25.5.0), release build (opt-level=3, LTO=fat, strip=symbols, panic=abort)

## Deviations from Plan

None — 计划按原文执行，Option B 完整实施，hyperfine 环境可用，数据均为实测值。

## Verification

- `test -f benches/hyperfine-validate.toml` → 0
- `./target/release/sqllog2db validate -c benches/hyperfine-validate.toml` → exit 0
- `! grep -q "Warning: Ignoring non-zero exit code." benches/BENCHMARKS.md` → PASS
- `grep -c "^## Phase 72 — 基准体系完善（v1.20）$" benches/BENCHMARKS.md` → 1
- `grep -q "validate -c benches/hyperfine-validate.toml" benches/BENCHMARKS.md` → PASS
- `grep -q "validate 行使用 \`benches/hyperfine-validate.toml\`" benches/BENCHMARKS.md` → PASS
- `grep -q "| \`--version\` | ~2.9 ms | 2.1 ms |" benches/BENCHMARKS.md` → PASS（--version 行未变）
- `awk '/<details>/{o++} /<\/details>/{c++} END{exit !(o>=2 && o==c)}'` → 14/14 平衡
- `cargo clippy --all-targets -- -D warnings` → PASS
- `cargo test` → 912 passed, 0 failed

## Self-Check: PASSED

- benches/hyperfine-validate.toml: FOUND
- benches/BENCHMARKS.md: FOUND (modified)
- commit 3dcd35d: FOUND (feat task 1)
- commit e32b353: FOUND (docs task 2)
- validate exit=0: CONFIRMED
- Warning: Ignoring non-zero exit code. removed: CONFIRMED
