# Phase 72: 基准体系完善 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-08
**Phase:** 72-bench-baseline
**Mode:** --auto (fully autonomous, no user interaction)
**Areas discussed:** Hyperfine 冷启动测量, Criterion v1.20 Baseline 存档, BENCHMARKS.md 更新方式

---

## Hyperfine 冷启动测量

| Option | Description | Selected |
|--------|-------------|----------|
| `--version` only | 最小化测量，快速 | |
| `--version` + `validate` | 与 Phase 9 一致，含配置文件加载路径 | ✓ |
| `--version` + `validate` + `run` | 全命令覆盖，但 run 依赖大文件，不稳定 | |

**Auto-selected:** `--version` + `validate`（与 Phase 9 历史基线一致，推荐默认）
**Notes:** Phase 9（v1.9）已建立 ~3ms 基线，保持一致便于跨版本对比。`run` 受 I/O 影响大，暂跳过。

---

## Criterion v1.20 Baseline 存档路径

| Option | Description | Selected |
|--------|-------------|----------|
| `CRITERION_HOME=benches/baselines` | 存档至 repo，版本可追溯 | ✓ |
| 默认 `target/criterion/` | criterion 默认，不进 repo | |

**Auto-selected:** `CRITERION_HOME=benches/baselines`（已有文档模式，与 Phase 4/42/44 一致）
**Notes:** 现有 BENCHMARKS.md 已有此模式文档，baselines/ 下已有多个历史快照，保持一致。

---

## Benchmark 文件范围

| Option | Description | Selected |
|--------|-------------|----------|
| 全部 4 个（csv, sqlite, filters, parser） | 完整 v1.20 基准覆盖 | ✓ |
| 仅 csv + sqlite | 核心导出路径 | |

**Auto-selected:** 全部 4 个（comprehensive coverage，与 Phase 73–74 优化对比需求匹配）

---

## BENCHMARKS.md 更新方式

| Option | Description | Selected |
|--------|-------------|----------|
| 末尾追加新段落 | 历史数据保留，符合现有惯例 | ✓ |
| 重构文档结构 | 清晰但风险高（改动历史记录） | |

**Auto-selected:** 末尾追加（Phase 4/5/6/9/10/42/44/56 全部是追加模式）

---

## Claude's Discretion

- hyperfine `--warmup` 次数（保持 3，历史惯例）
- criterion 样本数（使用默认值，无需指定）
- 是否测量 `run` 命令（暂跳过，I/O 不稳定）

## Deferred Ideas

- hyperfine CI 自动化（bench.yml 加入冷启动步骤）— 未来 phase 单独评估
- `--export-json` 自动存档 hyperfine 输出 — 当前手动记录数值满足 BENCH-01 要求
