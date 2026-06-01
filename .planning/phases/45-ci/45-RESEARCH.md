# Phase 45: 并行扩展与 CI 基准集成 - Research

**Researched:** 2026-05-24
**Domain:** Rust 并行处理（rayon + SQLite WAL）、GitHub Actions CI benchmark artifact
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**SQLite 并行策略**
- D-01: 实现多文件跨文件并行解析：rayon 并行解析各文件，结果在内存中合并（通过 channel 或 collect + merge），最终由单线程按批写入 SQLite（WAL 模式）。
- D-02: 避免多线程并发写入 SQLite，使用 WAL 模式 + 单 writer thread 策略（与现有 CSV 并行路径的 merge-then-write 模式一致）。
- D-03: SQLite 并行路径的正确性通过 `cargo test` 验证：并行输出与顺序模式输出一致（record 内容相同，顺序可不同）。

**CI Benchmark 集成**
- D-04: GitHub Actions workflow 文件：`.github/workflows/bench.yml`（或加入现有 CI workflow），PR 触发时运行 `cargo bench`。
- D-05: benchmark 输出格式：JSON（critcmp 兼容格式）。通过 criterion 的 `target/criterion/*/estimates.json`。
- D-06: artifact 内容：时间戳、commit SHA、各 benchmark 组的 mean/stddev，文件名包含 commit SHA。
- D-07: CI artifact 使用 `actions/upload-artifact` 上传，retention 天数由 Claude 合理设置（建议 30-90 天）。

**质量门禁**
- D-08: 全链路质量门禁：`cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部绿灯。

### Claude's Discretion
- CI workflow 触发条件（只 PR 还是也包含 push to main）
- artifact 的具体 JSON schema 格式（只要包含 timestamp、SHA、mean、stddev）
- critcmp vs 自定义比较脚本的选择

### Deferred Ideas (OUT OF SCOPE)
- critcmp PR comment bot（自动评论性能变化）
- AsyncLogParser tokio 异步 SQLite 写入
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PERF-03 | 并行处理范围扩展——SQLite 导出支持批量并行写入，或多输入文件支持跨文件并行解析 | D-01 决策：多文件跨文件并行解析（parallel.rs 模式复刻 + SQLite WAL 模式写入） |
| BENCH-02 | GitHub Actions CI 集成 benchmark，每次 PR 导出基准报告（HTML 或 JSON），可对比历史基线 | D-04/D-05/D-06/D-07 决策：bench.yml + criterion estimates.json 收集 + upload-artifact |
</phase_requirements>

---

## Summary

Phase 45 有两个独立任务：（1）将现有 CSV 并行路径（`process_csv_parallel`）复刻为 SQLite 并行路径；（2）新增 `.github/workflows/bench.yml` workflow，PR 时自动运行 `cargo bench` 并上传 criterion JSON 报告。

**SQLite 并行路径**的核心挑战是 SQLite 不允许多线程并发写入。决策 D-01/D-02 已明确方案：rayon 并行解析各文件（每个线程将 `Sqllog` records 收集进 `Vec`），再在主线程按文件顺序依次写入同一个 `SqliteExporter`。与 CSV 路径的主要差异：CSV 路径每个线程写独立临时文件，最后 concat；SQLite 路径每个线程只做解析（不写出），主线程 merge-then-write。这避免了多 `Connection` 写冲突，也无需 WAL 的并发读，只是利用 WAL 模式的正确 journal 保证。

**CI benchmark**方面，criterion 已在 `dev-dependencies` 中配置（`version = "0.7"`），四大 bench 文件均已存在（`bench_csv.rs`/`bench_sqlite.rs`/`bench_filters.rs`/`bench_parser.rs`）。现有 `ci.yaml` 已有 `cargo bench --no-run` 编译检查，但未运行 bench 本体。新增 `bench.yml` 仅 PR 触发、仅在 ubuntu-latest 运行，通过 shell 脚本将 criterion 的 `target/criterion/*/new/estimates.json` 收集为单一 JSON 文件（含 timestamp + SHA + mean/stddev），用 `actions/upload-artifact` 上传。

**Primary recommendation:** 先实现 SQLite 并行路径（新建 `src/cli/run/sqlite_parallel.rs`），再新建 `bench.yml`，两者在一个 phase 分两个 plan 完成，各自质量门禁独立通过。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 多文件并行解析（rayon） | CLI run 层（`src/cli/run/`） | — | 解析调度由 `handle_run` 协调，与现有 CSV 并行路径同层 |
| 解析结果 merge（Vec 收集） | CLI run 层（`sqlite_parallel.rs`） | — | 与 `process_csv_parallel` 对称，属于同一模块 |
| SQLite 单线程写入 | Exporter 层（`SqliteExporter`） | — | `SqliteExporter` 持有 `Connection`，不可跨线程共享，必须单线程 |
| WAL 模式 PRAGMA | Exporter 层（`initialize_pragmas`） | — | 需要替换现有的 `JOURNAL_MODE=OFF` 为 `JOURNAL_MODE=WAL` |
| `use_parallel` 路由判断扩展 | CLI run 层（`mod.rs`，第 125 行） | — | 当前只对 CSV 并行，需同时覆盖 SQLite |
| Bench workflow | CI（`.github/workflows/`） | — | 纯 YAML 配置，不涉及 Rust 代码结构变更 |
| Criterion JSON 收集脚本 | CI workflow | — | shell 一次性脚本，解析 estimates.json |

---

## Standard Stack

### Core（均已在项目中使用）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rayon | 1.12 [VERIFIED: Cargo.toml] | 并行 par_iter，多文件并行解析 | 已用于 CSV 并行路径和 prescan |
| rusqlite | 0.39.0 [VERIFIED: Cargo.toml] | SQLite 写入 | 已用于 `SqliteExporter`，bundled 特性 |
| criterion | 0.7 [VERIFIED: Cargo.toml] | benchmark 框架，输出 estimates.json | 已有四大 bench 文件 |

### CI Actions（已在 ci.yaml 中使用）

| Action | Version | Purpose |
|--------|---------|---------|
| actions/checkout | v6 [VERIFIED: .github/workflows/ci.yaml] | 检出代码 |
| dtolnay/rust-toolchain | @stable [VERIFIED: ci.yaml] | 安装 Rust 工具链 |
| Swatinem/rust-cache | v2 [VERIFIED: ci.yaml] | 缓存 Cargo 编译产物 |
| actions/upload-artifact | v4 [ASSUMED] | 上传 bench artifact |

**Installation:** 无需新增依赖，所有库已在 `Cargo.toml` 中。

---

## Package Legitimacy Audit

本 phase 不新增任何外部 package 依赖，所有库均已在 `Cargo.toml` 中验证。CI Actions 均为 GitHub 官方或广泛使用的社区 action。

| Package | Registry | Status | Disposition |
|---------|----------|--------|-------------|
| rayon 1.12 | crates.io | 已在 Cargo.toml | Approved（无需重新安装）|
| rusqlite 0.39.0 | crates.io | 已在 Cargo.toml | Approved |
| criterion 0.7 | crates.io | 已在 dev-dependencies | Approved |
| actions/upload-artifact | GitHub Actions | [ASSUMED] v4 | 需确认版本号 |

**Packages removed due to slopcheck:** none
**Packages flagged as suspicious:** none

---

## Architecture Patterns

### System Architecture Diagram

```
多文件 SQLite 并行路径（新增）
═══════════════════════════════

log_files: [file1, file2, ..., fileN]
        │
        ▼
rayon::ThreadPool（jobs 个线程）
        │  par_iter + enumerate
        │
        ├─ Thread 1: process_log_file_collect(file1)
        │   └─ LogParserBuilder → iter() → filter → Vec<CollectedRow>
        │
        ├─ Thread 2: process_log_file_collect(file2)
        │   └─ LogParserBuilder → iter() → filter → Vec<CollectedRow>
        │
        └─ Thread N: ...
        │
        ▼
collect:  Vec<Result<Vec<CollectedRow>>>  （保持原始文件顺序）
        │
        ▼
主线程 merge: 按文件顺序展开为扁平 Vec<CollectedRow>（或按序 drain）
        │
        ▼
SqliteExporter（WAL 模式）
  initialize() → PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
  BEGIN TRANSACTION
  for row in merged_rows:
      export_one_preparsed(row)
      batch_commit_if_needed()
  COMMIT
  finalize()
        │
        ▼
output.db（单文件，正确顺序）
```

```
CI Benchmark Artifact 数据流
═══════════════════════════════

PR → bench.yml 触发
        │
        ▼
cargo bench（ubuntu-latest，release profile）
        │
        ▼
target/criterion/
  ├─ csv_export/1000/new/estimates.json
  ├─ csv_export/10000/new/estimates.json
  ├─ sqlite_export/1000/new/estimates.json
  ├─ filters/no_pipeline/new/estimates.json
  └─ ...（所有 bench group）
        │
        ▼
collect_bench_results.sh
  读取所有 estimates.json → 提取 mean.point_estimate + std_dev.point_estimate
  附加 timestamp + GITHUB_SHA
  写出 bench-results-${SHA}.json
        │
        ▼
actions/upload-artifact（retention: 60 days）
        │
        ▼
PR artifact 页面（可手动下载对比历史）
```

### Recommended Project Structure

```
src/cli/run/
├── mod.rs            — use_parallel 判断扩展（新增 SQLite 分支）
├── parallel.rs       — 现有 CSV 并行（不变）
├── sqlite_parallel.rs — 新建：process_sqlite_parallel
├── processor.rs      — 不变
├── prescan.rs        — 不变
├── filter_processor.rs — 不变
└── tests.rs          — 追加 SQLite 并行正确性测试

src/exporter/sqlite/
└── mod.rs            — initialize_pragmas 修改：WAL 替换 OFF

.github/workflows/
├── ci.yaml           — 不变
└── bench.yml         — 新建
```

### Pattern 1: SQLite 并行 — collect-merge-write

**What:** 多线程只做解析，主线程做写入，避免 SQLite 并发写冲突。

**When to use:** 任何 SQLite exporter + 多文件 + jobs > 1 的场景。

**Example（新增函数骨架）：**

```rust
// src/cli/run/sqlite_parallel.rs
// Source: 参照 parallel.rs 的 process_csv_parallel，适配 SQLite 单写模式

use crate::error::{Error, Result};
use crate::exporter::{ExporterManager, SqliteExporter};
use crate::pipeline::{CompiledSqlFilters, FieldMask, Pipeline};
use dm_database_parser_sqllog::Sqllog;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 每个线程解析一个文件，返回 (orig_path, records, parse_errors)
type TaskResult = Option<(PathBuf, Vec<Sqllog>, usize)>;

pub(super) fn process_sqlite_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    jobs: usize,
    show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    sql_record_filter: Option<&CompiledSqlFilters>,
) -> Result<(usize, usize)> {
    // Step 1: rayon 并行解析，每个线程收集 Vec<Sqllog>
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;

    let results: Vec<Result<TaskResult>> = pool.install(|| {
        log_files
            .par_iter()
            .map(|file| {
                if interrupted.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                // 解析文件，返回过滤后的记录 Vec（不写 SQLite）
                let (records, _parse_errors) = collect_log_file(
                    file, pipeline, do_normalize, placeholder_override,
                    sql_record_filter, interrupted,
                )?;
                Ok(Some((file.clone(), records, 0usize)))
            })
            .collect()
    });

    // Step 2: 主线程按顺序 merge + 写入 SQLite
    let mut exporter_manager = ExporterManager::from_config(cfg)?;
    exporter_manager.initialize()?;
    let mut total_records = 0usize;

    for result in results {
        match result? {
            Some((_path, records, _)) => {
                for record in &records {
                    // 传 normalized 字符串（如需要）
                    exporter_manager.export_one_preparsed(record, true, None)?;
                    total_records += 1;
                }
            }
            None => {}
        }
    }
    exporter_manager.finalize()?;
    Ok((total_records, 0))
}
```

**关键考量：**
- `collect_log_file` 中的 normalization（`compute_normalized`）需要在线程内完成，因为写入时需要 `normalized_sql` 字符串。
- 或者：线程内同时计算 `normalized_sql`，收集 `(Sqllog, Option<String>)` 对，主线程写入时直接传 `normalized.as_deref()`。
- 内存峰值：所有文件解析结果在内存中等待主线程写入。实际上与 CSV 路径（临时文件在磁盘）不同，这是内存换磁盘的取舍——对 Phase 45 的 PERF-03 要求可接受。

### Pattern 2: WAL 模式 PRAGMA 替换

**What:** 生产 SQLite 并行路径使用 WAL 模式，而不是 bench 中使用的 `JOURNAL_MODE=OFF`。

**Example:**

```rust
// src/exporter/sqlite/mod.rs — initialize_pragmas 修改
// Source: 代码库中已有 initialize_pragmas 函数，第 30-39 行

// 现有（benchmark 用）：
// PRAGMA journal_mode = OFF;
// PRAGMA synchronous = OFF;

// 生产并行路径需要（保证崩溃安全）：
// PRAGMA journal_mode = WAL;
// PRAGMA synchronous = NORMAL;

// 注意：WAL 模式比 OFF 模式慢（有 WAL 文件写入开销），
// 但对于"多文件并行解析 + 单线程写入"场景已无并发冲突，
// WAL 只是用于崩溃恢复保证，不是并发目的。
```

**重要：** 需要区分"benchmark 路径"（保留 OFF+OFF 高性能）和"生产路径"（改为 WAL+NORMAL）。可通过 config 或条件编译区分，或在 `SqliteExporterConfig` 中添加 `journal_mode` 字段（更干净）。

**最简方案（Claude's Discretion）：** 直接在 `initialize_pragmas` 改为 WAL+NORMAL，benchmark 的 `make_config` 单独设 PRAGMA（bench 代码不经过 `initialize_pragmas`）。实际上 benchmark 在 `make_config` 中直接传 TOML，现有 `initialize_pragmas` 已硬编码 OFF+OFF——可以保持 `initialize_pragmas` 不变，另外在 `process_sqlite_parallel` 路径的 `initialize` 前或后追加 WAL PRAGMA。

**最终推荐：** 在 `SqliteExporter` 的 `initialize` 中追加可选的 WAL 切换，通过 `SqliteExporterConfig` 的新字段 `wal_mode: bool`（默认 false，兼容已有行为）。但 D-02 的决策是"WAL 模式 + 单 writer thread"——解析为：并行场景下启用 WAL，非并行场景可保持 OFF（性能优先）。

**最简可行（Claude's Discretion）：** 在 `process_sqlite_parallel` 中，`ExporterManager::initialize()` 后追加一条 `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`。不修改 `initialize_pragmas`，不改 config schema。

### Pattern 3: `use_parallel` 路由扩展

**What:** 在 `mod.rs` 第 125 行扩展条件，覆盖 SQLite exporter。

```rust
// 现有（src/cli/run/mod.rs 第 125 行）：
let use_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.csv.is_some();

// 扩展后：
let use_csv_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.csv.is_some();
let use_sqlite_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.sqlite.is_some();
let use_parallel = use_csv_parallel || use_sqlite_parallel;

// 或保持 use_parallel 含义，分支中区分：
if use_csv_parallel {
    // 现有 process_csv_parallel
} else if use_sqlite_parallel {
    // 新增 process_sqlite_parallel
} else {
    // 顺序路径
}
```

### Pattern 4: Criterion estimates.json 结构

**What:** criterion 对每个 benchmark group 下每个 benchmark ID 输出 `estimates.json`，路径为 `target/criterion/<group>/<id>/new/estimates.json`。

**JSON 结构（已验证 `benches/baselines/sqlite_export/1000/new/estimates.json`）：**

```json
{
  "mean": {
    "confidence_interval": { "confidence_level": 0.95, "lower_bound": ..., "upper_bound": ... },
    "point_estimate": 839195.83,   // 纳秒
    "standard_error": 1359.65
  },
  "median": { ... },
  "median_abs_dev": { ... },
  "slope": null,
  "std_dev": {
    "confidence_interval": { ... },
    "point_estimate": 6205.05,     // 纳秒
    "standard_error": 807.83
  }
}
```

**Artifact 收集脚本逻辑：**

```bash
#!/bin/bash
# collect_bench_results.sh
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"
OUTPUT="bench-results-${SHA:0:8}.json"

echo "{" > "$OUTPUT"
echo "  \"timestamp\": \"$TIMESTAMP\"," >> "$OUTPUT"
echo "  \"commit_sha\": \"$SHA\"," >> "$OUTPUT"
echo "  \"benchmarks\": {" >> "$OUTPUT"

first=1
find target/criterion -name "estimates.json" -path "*/new/estimates.json" | while read f; do
    # 路径: target/criterion/<group>/<id>/new/estimates.json
    group=$(echo "$f" | awk -F/ '{print $(NF-3)}')
    id=$(echo "$f" | awk -F/ '{print $(NF-2)}')
    key="${group}/${id}"
    mean=$(python3 -c "import json,sys; d=json.load(open('$f')); print(d['mean']['point_estimate'])")
    stddev=$(python3 -c "import json,sys; d=json.load(open('$f')); print(d['std_dev']['point_estimate'])")
    if [ $first -eq 0 ]; then echo "," >> "$OUTPUT"; fi
    echo "    \"$key\": {\"mean_ns\": $mean, \"stddev_ns\": $stddev}" >> "$OUTPUT"
    first=0
done

echo "  }" >> "$OUTPUT"
echo "}" >> "$OUTPUT"
```

**或使用纯 bash + jq（更简洁，如果 ubuntu-latest 有 jq）：**

```bash
# ubuntu-latest 默认有 jq
jq -n \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg sha "$GITHUB_SHA" \
  --slurpfile results <(
    find target/criterion -name "estimates.json" -path "*/new/estimates.json" \
    | while read f; do
        group=$(cut -d/ -f3 <<< "$f")
        id=$(cut -d/ -f4 <<< "$f")
        jq --arg g "$group" --arg i "$id" \
          '{key: ($g+"/"+$i), mean: .mean.point_estimate, stddev: .std_dev.point_estimate}' "$f"
      done | jq -s .
  ) \
  '{timestamp: $ts, commit_sha: $sha,
    benchmarks: ($results[0] | map({(.key): {mean_ns: .mean, stddev_ns: .stddev}}) | add)}' \
> "bench-results-${GITHUB_SHA:0:8}.json"
```

### Pattern 5: bench.yml Workflow 骨架

```yaml
# .github/workflows/bench.yml
name: Benchmark

on:
  pull_request:
    branches: [main]
  # Claude's Discretion: 也在 push to main 时运行，记录每次合并后的基线
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  benchmark:
    name: Run Benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench -- --output-format bencher 2>&1 | tee bench_raw.txt
        # 或直接 cargo bench（criterion 自动写 target/criterion/*/new/estimates.json）

      - name: Collect benchmark results
        run: |
          SHA="${{ github.sha }}"
          TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
          # 用 jq 收集 estimates.json（见 Pattern 4）
          # 输出到 bench-results-${SHA:0:8}.json

      - name: Upload benchmark artifact
        uses: actions/upload-artifact@v4
        with:
          name: bench-results-${{ github.sha }}
          path: bench-results-*.json
          retention-days: 60
```

**关于 `--output-format bencher` 与 estimates.json 的选择：**

D-05 决策是"通过 criterion 的 `target/criterion/*/estimates.json`"。**直接运行 `cargo bench`（不加 `--output-format`）即可**，criterion 0.7 会自动在 `target/criterion/<group>/<id>/new/estimates.json` 写出 JSON。`--output-format bencher` 输出的是 stdout 格式，两者可以并存但不互斥。

### Anti-Patterns to Avoid

- **多线程写 SQLite（即使有 WAL）：** rusqlite 的 `Connection` 不是 `Sync`，不能跨线程共享；每线程独立连接 + 并发写入会产生 `SQLITE_BUSY` 错误。D-02 明确禁止。
- **在线程内创建 `SqliteExporter`：** 与上同理，每线程写独立 .db 后合并 SQLite 文件在技术上可行，但实现复杂（需要 ATTACH DATABASE + INSERT SELECT）。D-01 决策是 collect+merge 方案，不走这路。
- **直接修改 bench 的 PRAGMA：** 现有 benchmark 使用 `JOURNAL_MODE=OFF SYNCHRONOUS=OFF` 是性能基准。修改 PRAGMA 会导致 benchmark 数值变化，使历史对比失效。应保持 benchmark PRAGMA 不变，只在生产路径（`process_sqlite_parallel`）中切换 WAL。
- **在 CI bench job 中运行真实文件 benchmark：** `sqllogs/` 目录不存在于 CI 环境，`bench_sqlite_real_file` / `csv_export_real` 会自动 skip（代码已处理）。无需特殊处理。
- **parallel path 中重置进度条：** 参见 `parallel.rs` 第 159 行注释，并行模式 `reset_pb=false`，SQLite 并行路径同理。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 并行文件处理调度 | 手写线程 pool + channel | rayon `par_iter` | 已用于 CSV 路径，work-stealing，异常处理简洁 |
| SQLite 批量提交 | 手写 transaction 计数 | 现有 `batch_commit_if_needed()` | 已实现，重用即可 |
| JSON benchmark 报告解析 | 手写 JSON parser | `jq`（ubuntu-latest 内置） | 一行命令提取 mean/stddev |
| Criterion benchmark 结果存储 | 手写 DB | criterion 的 `estimates.json` | 已自动生成，无需额外工具 |

**Key insight:** SQLite 并行的难点不在并发写，而在"并行解析 + 顺序写"的协调。criterion 已经解决了 JSON 输出问题，CI 只需收集文件。

---

## Common Pitfalls

### Pitfall 1: WAL 模式与 benchmark PRAGMA 混用

**What goes wrong:** 如果修改 `initialize_pragmas` 改为 WAL+NORMAL，benchmark 数值会因 journal 开销增大而下降，破坏历史对比。

**Why it happens:** `bench_sqlite.rs` 的 `make_config` 创建 `SqliteExporter`，后者调用 `initialize_pragmas`。

**How to avoid:** WAL 切换只在 `process_sqlite_parallel` 中局部执行（独立的 PRAGMA 语句），不修改 `initialize_pragmas`。

**Warning signs:** `cargo bench --bench bench_sqlite` 相比历史 baseline 出现 >5% 退化。

### Pitfall 2: Vec 内存峰值过高

**What goes wrong:** 多文件并行解析时，所有文件的解析结果（`Vec<Sqllog>`）同时在内存中，可能导致 OOM。

**Why it happens:** 与 CSV 路径不同（写临时文件，内存占用低），SQLite 路径将全部 records 持有到主线程写完为止。

**How to avoid:** Phase 45 的 PERF-03 目标是"功能正确性"，不要求最低内存。在 tests 中使用小规模数据（<1000 条），实际生产中大文件应在文档中说明此取舍。如内存成为问题，future work 可改为 channel 流式传递。

**Warning signs:** 真实 1GB 文件测试时 RSS 大幅上升。

### Pitfall 3: 并行路径顺序不确定性与测试

**What goes wrong:** D-03 要求"并行输出与顺序模式一致（record 内容相同，顺序可不同）"，但如果测试断言行顺序则会 flaky。

**Why it happens:** rayon `par_iter` 的执行顺序在 `collect` 后是保持原索引顺序的（enumerate + index），但 records 内部的顺序取决于文件内容顺序，merge 后顺序取决于文件列表顺序。

**How to avoid:** 测试用多个小文件，验证时 `sort()` + `assert_eq!(sorted_parallel, sorted_sequential)`，不比较原始顺序。

**Warning signs:** 测试在某些机器上通过、某些失败。

### Pitfall 4: CI cargo bench 超时

**What goes wrong:** `cargo bench` 在 GitHub Actions 上可能超过默认 6 小时超时，尤其是 SQLite bench（sample_size=20）和 real-file bench（skip）加在一起仍很耗时。

**Why it happens:** bench_sqlite 和 bench_filters 合成 benchmark 每次需要文件 I/O，criterion 100 samples × 50K records 耗时显著。

**How to avoid:** 在 CI bench.yml 中，只运行合成 benchmark（跳过 real-file 自动处理），或明确设置 `timeout-minutes: 30`。现有代码已处理 `sqllogs/` 不存在的情况（自动 skip）。

**Warning signs:** Action job 超时，artifact 未上传。

### Pitfall 5: criterion 0.7 的 estimates.json 路径

**What goes wrong:** criterion 0.7 的输出路径格式为 `target/criterion/<group>/<benchmark_id>/new/estimates.json`，但实际 group 名称是 benchmark group name（如 `csv_export`），benchmark_id 是 `BenchmarkId::from_parameter(n)` 的字符串表示（如 `1000`）。

**Why it happens:** 收集脚本如果路径假设错误会找不到文件。

**How to avoid:** 在脚本中使用 `find target/criterion -name "estimates.json" -path "*/new/estimates.json"` 递归查找，不硬编码路径深度。

**Warning signs:** `bench-results-*.json` 的 `benchmarks` 字段为空 `{}`。

---

## Code Examples

### SQLite 并行路径核心框架

```rust
// Source: 参照 src/cli/run/parallel.rs process_csv_parallel（VERIFIED: codebase）

// collect_log_file：解析单文件，返回 Vec<(Sqllog, Option<String>)>
// 其中 Option<String> 是 normalized_sql（如 do_normalize=true）
fn collect_log_file(
    file: &PathBuf,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    sql_record_filter: Option<&CompiledSqlFilters>,
    interrupted: &Arc<AtomicBool>,
) -> Result<Vec<(Sqllog, Option<String>)>> {
    use dm_database_parser_sqllog::LogParserBuilder;
    let parser = LogParserBuilder::new(file.to_str().unwrap_or_default())
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    let mut rows = Vec::new();
    let mut params_buf = crate::pipeline::normalizer::ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    for record in parser.iter().flatten() {
        if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if !pipeline.is_empty() && !pipeline.run_with_meta(&record) {
            continue;
        }
        if let Some(f) = sql_record_filter {
            if !f.matches(&record.sql) {
                continue;
            }
        }
        let normalized = if do_normalize {
            Some(crate::pipeline::compute_normalized(
                &record.sql,
                &mut params_buf,
                &mut ns_scratch,
                placeholder_override,
            ))
        } else {
            None
        };
        rows.push((record, normalized));
    }
    Ok(rows)
}
```

### handle_run 路由扩展

```rust
// Source: src/cli/run/mod.rs 第 125 行（VERIFIED: codebase）
// 扩展前：
// let use_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.csv.is_some();
// 扩展后：
let use_csv_parallel  = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
let use_sqlite_parallel = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
```

### 正确性测试骨架

```rust
// src/cli/run/tests.rs — 追加
#[test]
fn test_sqlite_parallel_matches_sequential() {
    // 创建 3 个临时 log 文件，各含不同记录
    // 顺序模式导出 → sequential.db
    // 并行模式导出 → parallel.db（jobs=2 或 jobs=3）
    // 对比两个 db 的记录集（ORDER BY ts, trxid）
    // 断言记录数相同且内容逐条匹配（sort 后比较）
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| SQLite 仅顺序写入 | 多文件并行解析 + 单线程写入 | Phase 45（本次） | 多文件场景吞吐提升，单文件无变化 |
| CI 只编译不运行 bench | CI 运行 bench + 上传 artifact | Phase 45（本次） | PR 可对比性能变化 |
| `JOURNAL_MODE=OFF` | 生产路径 `JOURNAL_MODE=WAL` | Phase 45（本次，仅并行路径） | 崩溃安全保证，轻微写入开销增加 |

**Deprecated/outdated:**
- `use_parallel = ... && csv.is_some()`：Phase 45 后扩展为同时覆盖 SQLite。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `actions/upload-artifact` 当前稳定版本为 v4 | Standard Stack | 如果 v4 有 breaking change 需改版本号；ci.yaml 中的 checkout@v6 表明项目使用新版 actions，v4 概率极高 |
| A2 | ubuntu-latest 内置 `jq` | Bench artifact 收集脚本 | 如果没有 jq 则改用 python3（ubuntu-latest 必有 python3） |
| A3 | collect 后 rayon `par_iter().enumerate()` 保持原始索引顺序 | SQLite 并行架构 | rayon 的 collect 保证索引顺序（已知 rayon 行为）；如不保证需改用 sort_by |

**如果此表为空：** 所有关键声明已通过代码库直接验证。本表列出少量行为假设，均有合理理由，风险低。

---

## Open Questions (RESOLVED)

1. **normalized_sql 在线程内计算 vs 主线程计算**
   - What we know: `compute_normalized` 是纯函数，接受 `&record`/`&str` + mutable scratch buffer，无全局状态
   - What's unclear: 每线程独立 `ParamBuffer` + `ns_scratch` 效率与主线程统一计算的差异
   - Recommendation: 在线程内计算（避免跨线程传 raw SQL 再计算），每线程 `ParamBuffer::default()` 开销极小
   - **RESOLVED:** 在线程内计算 `normalized_sql`。每线程独立 `ParamBuffer::default()` 和 `Vec<u8> ns_scratch`，与 processor.rs 单文件循环行为对称。`compute_normalized` 返回 `Option<&str>` 后立即用 `.map(|s| s.to_owned())` 复制为 `Option<String>` 以便跨线程传递（&str 借用 scratch 在函数返回后失效）。PARAMS 记录（`record.tag.is_none()`）即便未通过 filter 也必须调用 `compute_normalized` 更新 `params_buffer`（mirror processor.rs 第 134-143 行）。

2. **bench.yml 触发条件**
   - What we know: D-04 说"PR 触发"，Claude's Discretion 说也可以包含 push to main
   - What's unclear: 是否需要 push to main 也跑 bench（用于记录每次 merge 后的基线）
   - Recommendation: 同时触发 PR 和 push to main；PR 时做对比，push to main 时存档为永久基线
   - **RESOLVED:** 双触发 — `pull_request: branches: [main]` 用于 PR 性能审查；`push: branches: [main]` 用于在每次合入 main 后记录永久基线 artifact，供后续 PR 通过 commit SHA 反查对比。

3. **WAL 模式切换的最优位置**
   - What we know: `initialize_pragmas` 当前硬编码 OFF+OFF；benchmark 不经过 `process_sqlite_parallel`
   - What's unclear: 是在 `SqliteExporterConfig` 增加字段，还是在 `process_sqlite_parallel` 中覆写 PRAGMA
   - Recommendation: 在 `process_sqlite_parallel` 启动后追加覆写 PRAGMA（最小改动，不影响 benchmark 路径）
   - **RESOLVED:** 在 `process_sqlite_parallel` 的 `ExporterManager::initialize()` 之后追加 `set_sqlite_wal_mode()` 调用（执行 `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`）。**不修改** `initialize_pragmas` 的 OFF+OFF 配置，从而完全隔离 benchmark 路径（`bench_sqlite.rs` 走 `initialize_pragmas` 不走 `set_wal_mode`），保留历史 baseline 可比性。WAL 切换通过 `SqliteExporter::set_wal_mode` + `ExporterManager::set_sqlite_wal_mode` 包装暴露（非 SQLite exporter 时静默 no-op）。

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rayon | SQLite 并行解析 | ✓ | 1.12（Cargo.toml） | — |
| rusqlite | SQLite 写入 | ✓ | 0.39.0 bundled | — |
| criterion | benchmark 运行 | ✓ | 0.7 dev-dep | — |
| jq | CI artifact 收集 | ✓（ubuntu-latest 内置）[ASSUMED] | — | python3 一行替代 |
| actions/upload-artifact | CI artifact 上传 | ✓（GitHub 官方 action） | v4 [ASSUMED] | — |

**Missing dependencies with no fallback:** none

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test（内置）+ criterion（benchmark） |
| Config file | Cargo.toml（[[bench]] 条目已存在） |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo bench --no-run` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PERF-03 | SQLite 多文件并行解析输出与顺序模式一致 | unit/integration | `cargo test test_sqlite_parallel_matches_sequential` | ❌ Wave 0（需新建） |
| PERF-03 | 多文件场景（>1 file）使用并行路径 | unit | `cargo test test_sqlite_parallel_routing` | ❌ Wave 0 |
| BENCH-02 | bench.yml workflow 文件存在 | manual（CI） | — | ❌ Wave 0 |
| BENCH-02 | `cargo bench` 在 CI 正常运行 | CI | `.github/workflows/bench.yml` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Phase gate:** `cargo build --release && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`

### Wave 0 Gaps
- [ ] `src/cli/run/tests.rs`（追加）— `test_sqlite_parallel_matches_sequential` — covers PERF-03
- [ ] `src/cli/run/sqlite_parallel.rs` — 新文件，Phase 实现主体
- [ ] `.github/workflows/bench.yml` — covers BENCH-02

---

## Security Domain

本 phase 不涉及认证、会话管理、访问控制或加密。主要 ASVS 相关点：

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | 有限 | SQL log 文件路径来自 config，已有 validation |
| V6 Cryptography | 否 | — |
| CI Secrets | 否 | bench.yml 不需要任何 secrets，只读代码库和上传 artifact |

**CI 安全注意事项（ASSUMED）：** `actions/upload-artifact` 上传的 artifact 对 PR 参与者可见，不含任何敏感数据（只有性能数字），无安全风险。

---

## Sources

### Primary (HIGH confidence)
- `src/cli/run/parallel.rs` — CSV 并行路径完整实现，SQLite 并行设计基准 [VERIFIED: codebase]
- `src/cli/run/mod.rs` — `use_parallel` 判定逻辑（第 125 行）[VERIFIED: codebase]
- `src/exporter/sqlite/mod.rs` — `SqliteExporter` 完整实现，`initialize_pragmas` [VERIFIED: codebase]
- `src/pipeline/mod.rs` — `Pipeline: LogProcessor: Send + Sync`，线程安全确认 [VERIFIED: codebase]
- `benches/baselines/sqlite_export/1000/new/estimates.json` — criterion JSON 结构实例 [VERIFIED: codebase]
- `.github/workflows/ci.yaml` — 现有 actions 版本（checkout@v6, rust-toolchain@stable, rust-cache@v2）[VERIFIED: codebase]
- `Cargo.toml` — rayon 1.12, rusqlite 0.39.0, criterion 0.7 [VERIFIED: codebase]

### Secondary (MEDIUM confidence)
- `benches/BENCHMARKS.md` — criterion estimates.json 路径格式说明、baseline 存储方式 [VERIFIED: codebase]
- `src/pipeline/filters/compiled.rs` — `CompiledSqlFilters`/`CompiledMetaFilters` 结构，`regex::Regex` 字段（`Regex: Send + Sync`）[VERIFIED: codebase]

### Tertiary (LOW confidence)
- actions/upload-artifact v4 版本号 [ASSUMED]
- ubuntu-latest 内置 jq [ASSUMED]

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 所有库已在 Cargo.toml 验证，版本已知
- Architecture: HIGH — 基于代码库直接分析（parallel.rs + sqlite/mod.rs 全读）
- Pitfalls: HIGH — 基于代码库中已有 PRAGMA 配置和 benchmark 历史数据
- CI Pattern: MEDIUM — ci.yaml 已读，bench.yml 为新建，具体 action 版本 ASSUMED

**Research date:** 2026-05-24
**Valid until:** 2026-06-24（criterion 和 rayon API 稳定，30 天有效）
