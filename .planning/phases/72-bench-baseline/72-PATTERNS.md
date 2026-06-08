# Phase 72: 基准体系完善 - Pattern Map

**Mapped:** 2026-06-08
**Files analyzed:** 2 (1 modified document + 1 new data directory)
**Analogs found:** 2 / 2

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `benches/BENCHMARKS.md` | documentation | append-only | `benches/BENCHMARKS.md` §Phase 56 (末尾追加段落模式) | exact |
| `benches/baselines/*/v1.20/` | data archive | file-I/O (criterion auto-generated) | `benches/baselines/csv_export/1000/v1.0/` | exact |

---

## Pattern Assignments

### `benches/BENCHMARKS.md` (documentation, append-only)

**Analog:** `benches/BENCHMARKS.md` 全文——每个历史 Phase（4/5/6/9/10/42/44/56）均是末尾追加，从不修改已有段落。

**段落标题格式** (lines 128, 204, 285, 315, 391, 508, 558, 720):
```markdown
## Phase N — <中文描述>（vX.Y）
```

**Phase 9 冷启动段落结构** (lines 315–388) — Phase 72 的直接模板:
```markdown
## Phase 9 — CLI 冷启动基线（PERF-11）

**Date:** 2026-05-14
**Goal:** 量化...
**Test environment:** Apple Silicon (Darwin 25.4.0), release build (`opt-level=3`, LTO=fat, strip=symbols, panic=abort)

### 测量命令

```bash
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
```

### 对比维度（per D-08）

| 命令 | Phase 9 (v1.9) mean | ... |
|------|----|----|

<details>
<summary>sqllog2db --version</summary>

```
Benchmark 1: ./target/release/sqllog2db --version
  Time (mean ± σ):       2.9 ms ±   0.4 ms    [User: 1.7 ms, System: 0.8 ms]
  Range (min … max):     2.5 ms …   5.9 ms    356 runs
```

</details>

### 结论

- [x] 验收条目 1
- [x] 验收条目 2
```

**Criterion 输出原文折叠格式** (lines 148–184, Phase 4 示例):
```markdown
<details>
<summary>cargo bench --baseline v1.0（...描述...）</summary>

```
criterion 原始输出粘贴于此
```

</details>
```

**"How to compare against this baseline" 段落** (lines 26–38) — D-06 要求追加 v1.20 命令:
```markdown
## How to compare against this baseline

baseline JSON 数据存档在 `benches/baselines/`，criterion 通过 `CRITERION_HOME` 环境变量定位。

```bash
# 对比当前修改与 v1.0 baseline
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline v1.0

# 保存新的 named baseline（例如 Phase 4 优化后）
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --save-baseline phase4
```

criterion 输出会标注 "Performance has improved" / "Performance has regressed" / "No change in performance detected"。
```
> Phase 72 需在此段落末尾（line 38 之后、`---` 之前）**追加** v1.20 对比命令示例（D-06）。

---

### `benches/baselines/*/v1.20/` (data archive, criterion auto-generated)

**Analog:** `benches/baselines/csv_export/1000/v1.0/`

**目录结构** — criterion `--save-baseline v1.20` 自动创建，无需手动:
```
benches/baselines/
├── csv_export/
│   ├── 1000/v1.20/{benchmark,estimates,sample,tukey}.json
│   ├── 10000/v1.20/
│   └── 50000/v1.20/
├── csv_export_real/                  # sqllogs/ 不存在时自动 skip
├── csv_format_only/
├── filters/
│   ├── no_pipeline/v1.20/
│   ├── pipeline_passthrough/v1.20/
│   └── ...（其余 filter 场景）
├── parser_throughput/
│   ├── 1000/v1.20/
│   ├── 10000/v1.20/
│   └── 50000/v1.20/
├── sqlite_export/
│   ├── 1000/v1.20/
│   ├── 10000/v1.20/
│   └── 50000/v1.20/
└── sqlite_export_real/               # sqllogs/ 不存在时自动 skip
```

**benchmark.json 格式** (来自 `benches/baselines/csv_export/1000/v1.0/benchmark.json`):
```json
{"group_id":"csv_export","function_id":null,"value_str":"1000","throughput":{"Elements":1000},"full_id":"csv_export/1000","directory_name":"csv_export/1000","title":"csv_export/1000"}
```

**baselines/.gitignore** — 已排除临时目录，v1.20 JSON 文件不受影响:
```gitignore
# Only commit the saved baseline JSON data.
# Criterion writes 'new/' (latest run) and 'report/' (HTML) alongside it;
# those are local artifacts and should not be versioned.
**/new/
**/report/
```

**生成命令**（D-04/D-05 锁定）:
```bash
CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20
```

**验证命令**（D-06）:
```bash
CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
```

---

## Shared Patterns

### CRITERION_HOME 环境变量重定向模式
**Source:** `benches/BENCHMARKS.md` lines 26–38 ("How to compare against this baseline")
**Apply to:** `benches/baselines/*/v1.20/` 数据文件生成、BENCHMARKS.md 新段落命令示例
```bash
# 存档（写入）
CRITERION_HOME=benches/baselines cargo bench -- --save-baseline <name>

# 对比（读取）
CRITERION_HOME=benches/baselines cargo bench -- --baseline <name>
```

### hyperfine `--warmup 3` 冷启动测量模式
**Source:** `benches/BENCHMARKS.md` lines 322–327 (Phase 9 测量命令)
**Apply to:** BENCHMARKS.md Phase 72 新段落
```bash
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
```

### `<details>` 折叠原始输出模式
**Source:** `benches/BENCHMARKS.md` lines 148–184 (Phase 4), lines 347–378 (Phase 9)
**Apply to:** BENCHMARKS.md Phase 72 段落中的 hyperfine 原始输出块
```markdown
<details>
<summary>hyperfine 原始输出（--version）</summary>

```
[粘贴 hyperfine stdout]
```

</details>
```

### 段落末尾结论 checklist 模式
**Source:** `benches/BENCHMARKS.md` lines 381–387 (Phase 9 结论), lines 496–504 (Phase 10 结论)
**Apply to:** BENCHMARKS.md Phase 72 段落的结论部分
```markdown
### 结论

- [x] BENCH-01 hyperfine 数值已记录（两命令，与 Phase 9 ~3ms 对比）
- [x] BENCH-02 criterion v1.20 baseline 存档至 benches/baselines/
- [x] cargo test 全量通过，clippy/fmt 净化
```

---

## No Analog Found

无。本 phase 所有文件均有完全匹配的历史模式可参照。

---

## Key Constraints for Planner

| Constraint | Source | Detail |
|-----------|--------|--------|
| 仅追加 BENCHMARKS.md，不修改历史段落 | D-08 | Phase 4/5/6/9/10/42/44/56 内容只读 |
| hyperfine 前必须先 `cargo build --release` | RESEARCH.md §Pitfall 1 | debug binary 延迟数倍于 release |
| CRITERION_HOME 必须设置，否则写入 target/（被 .gitignore 排除） | RESEARCH.md §Pitfall 2 | 全命令形式：`CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` |
| csv_export_real / sqlite_export_real 会自动 skip（sqllogs/ 不存在） | RESEARCH.md §Pitfall 3 | 正常信息，不是错误；BENCHMARKS.md 段落中注明 |
| 提交时显式 `git add benches/baselines/` | RESEARCH.md §Pitfall 4 | 避免遗漏新生成的 v1.20 JSON 文件 |

---

## Metadata

**Analog search scope:** `benches/BENCHMARKS.md`, `benches/baselines/`, `Cargo.toml`
**Files scanned:** 4 (BENCHMARKS.md, baselines/.gitignore, baselines/csv_export/1000/v1.0/benchmark.json, Cargo.toml)
**Pattern extraction date:** 2026-06-08
