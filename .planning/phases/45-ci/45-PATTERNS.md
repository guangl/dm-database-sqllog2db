# Phase 45: 并行扩展与 CI 基准集成 - Pattern Map

**Mapped:** 2026-05-24
**Files analyzed:** 5
**Analogs found:** 4 / 5

## File Classification

| 新建/修改文件 | Role | Data Flow | 最近 analog | 匹配质量 |
|---|---|---|---|---|
| `src/cli/run/sqlite_parallel.rs` | service | batch | `src/cli/run/parallel.rs` | role-match（同模块，不同写策略）|
| `src/cli/run/mod.rs` | controller | request-response | 自身（第 125 行扩展）| exact |
| `src/exporter/sqlite/mod.rs` | service | CRUD | 自身（`initialize_pragmas` 局部覆写）| exact |
| `.github/workflows/bench.yml` | config | event-driven | `.github/workflows/ci.yaml` | role-match |
| `scripts/collect_bench_results.sh` | utility | transform | 无现成 shell 脚本 analog | no-analog |

---

## Pattern Assignments

### `src/cli/run/sqlite_parallel.rs`（service，batch）

**Analog:** `src/cli/run/parallel.rs`

**Imports pattern**（parallel.rs 第 1–9 行）:
```rust
use crate::error::{Error, Result};
use crate::exporter::{CsvExporter, ExporterManager};
use crate::pipeline::normalizer::ParamBuffer;
use crate::pipeline::{CompiledSqlFilters, FieldMask, Pipeline};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::processor::process_log_file;
```

SQLite 路径不需要 `CsvExporter`，改为只导入 `ExporterManager`；不需要 `Path`（无临时目录）。

**函数签名模式**（parallel.rs 第 69–81 行）:
```rust
pub(super) fn process_csv_parallel(
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
) -> Result<(Vec<(PathBuf, usize)>, usize)>
```

SQLite 版本签名完全对称，返回类型改为 `Result<(usize, usize)>`（total_records, skipped_files），因为不需要返回临时文件路径列表。

**rayon 线程池构建 + par_iter 核心模式**（parallel.rs 第 115–168 行）:
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(e)))?;

type TaskResult = Option<(PathBuf, PathBuf, usize)>;
let results: Vec<Result<TaskResult>> = pool.install(|| {
    log_files
        .par_iter()
        .enumerate()
        .map(|(idx, file)| {
            if interrupted.load(Ordering::Relaxed) {
                return Ok(None);
            }
            // ... 每线程处理逻辑 ...
            Ok(Some((file.clone(), temp_path, count)))
        })
        .collect()
});
```

SQLite 路径的关键差异：线程内**不创建 ExporterManager**，而是将 `Vec<(Sqllog, Option<String>)>` collect 到 Vec，主线程再写入。原因：`SqliteExporter` 持有 `rusqlite::Connection`，不是 `Send`，不能跨线程共享。

**错误收集 + 主线程 merge 写入模式**（parallel.rs 第 172–191 行）:
```rust
let mut parts_info: Vec<(PathBuf, PathBuf, usize)> = Vec::with_capacity(log_files.len());
let mut first_err: Option<Error> = None;
let mut skipped = 0usize;
for result in results {
    match result {
        Ok(Some((orig, temp, count))) => {
            parts_info.push((orig, temp, count));
        }
        Ok(None) => skipped += 1,
        Err(e) if first_err.is_none() => first_err = Some(e),
        Err(_) => {}
    }
}
if let Some(e) = first_err {
    // 清理后返回错误
    return Err(e);
}
```

SQLite 路径：`Ok(Some(records_vec))`，主线程随后遍历写入 `ExporterManager`。

**并行模式 `reset_pb=false` 惯例**（parallel.rs 第 159 行注释）:
```rust
false, // 并行模式：不重置进度条，避免多线程互相重置计数
```

SQLite 并行路径同理，所有 `process_log_file` 调用传 `reset_pb=false`。

**WAL 覆写 PRAGMA（在 initialize 之后追加）**：
```rust
// 在 ExporterManager::initialize()? 之后追加，不修改 initialize_pragmas()
// 保持 benchmark 路径（OFF+OFF）不变
if let ExporterKind::Sqlite(ref sqlite_exp) = exporter_manager.exporter {
    // 通过 conn_ref 直接执行 WAL PRAGMA
}
```
注意：`ExporterManager` 没有暴露 conn 的直接访问方法，最简方案是在 `SqliteExporter` 上新增一个 `pub(crate) fn set_wal_mode(&self) -> Result<()>` 方法，内部执行 `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`，由 `process_sqlite_parallel` 在 `initialize()` 后调用。

---

### `src/cli/run/mod.rs`（controller，路由扩展）

**Analog:** 自身，精确修改第 125 行

**当前路由判断**（mod.rs 第 19 行 + 第 125 行）:
```rust
use parallel::process_csv_parallel;
// ...
let use_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.csv.is_some();
```

**扩展后的路由判断**（第 125 行替换）:
```rust
use parallel::{process_csv_parallel, process_sqlite_parallel};  // 新增 sqlite 导入
// ...
let use_csv_parallel =
    jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
let use_sqlite_parallel =
    jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
```

**分支结构**（mod.rs 第 127–143 行，CSV 并行分支作为参照）:
```rust
if use_parallel {  // 现为 use_csv_parallel
    info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
    let (processed_files, parallel_skipped) = process_csv_parallel(
        &log_files, final_cfg, &pipeline, jobs, show_progress,
        interrupted, do_normalize, placeholder_override,
        field_mask, &ordered_indices, sql_record_filter,
    )?;
    total_records = processed_files.iter().map(|(_, c)| *c).sum();
    skipped_files = parallel_skipped;
} else if use_sqlite_parallel {  // 新增分支，结构对称
    info!("Parsing and exporting SQL logs (SQLite parallel, {jobs} jobs)...");
    let (total, parallel_skipped) = process_sqlite_parallel(
        &log_files, final_cfg, &pipeline, jobs, show_progress,
        interrupted, do_normalize, placeholder_override,
        field_mask, &ordered_indices, sql_record_filter,
    )?;
    total_records = total;
    skipped_files = parallel_skipped;
} else {
    // 顺序路径不变
}
```

**完成摘要行**（mod.rs 第 187 行）:
```rust
let mode_label = if use_parallel { " [parallel]" } else { "" };
```
需扩展为：
```rust
let mode_label = if use_csv_parallel || use_sqlite_parallel { " [parallel]" } else { "" };
```

---

### `src/exporter/sqlite/mod.rs`（service，WAL 模式）

**Analog:** 自身，最小改动——新增一个辅助方法，不修改 `initialize_pragmas`

**`initialize_pragmas` 当前实现**（sqlite/mod.rs 第 30–42 行）:
```rust
fn initialize_pragmas(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA journal_mode = OFF;
         PRAGMA synchronous = OFF;
         PRAGMA cache_size = 1000000;
         PRAGMA locking_mode = EXCLUSIVE;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 30000000000;
         PRAGMA page_size = 65536;
         PRAGMA threads = 4;",
    )?;
    Ok(())
}
```

**不修改此函数**——benchmark 路径经过 `initialize_pragmas`，修改会破坏历史 benchmark 对比。

**新增辅助方法（`impl SqliteExporter` 块内）**:
```rust
/// 启用 WAL 模式（仅供并行路径在 initialize 之后调用）。
/// 不修改 initialize_pragmas，避免影响 benchmark 路径的 OFF+OFF 配置。
pub(crate) fn set_wal_mode(&self) -> Result<()> {
    let conn = self.conn_ref()?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )
    .map_err(|e| Self::db_err(format!("set WAL mode failed: {e}")))?;
    Ok(())
}
```

**`conn_ref` 模式**（sqlite/mod.rs 第 99–103 行）:
```rust
fn conn_ref(&self) -> Result<&Connection> {
    self.conn
        .as_ref()
        .ok_or_else(|| Self::db_err("not initialized"))
}
```

**`ExporterManager` 访问 SQLite exporter 的方式**：当前 `ExporterManager` 没有暴露底层 exporter 的直接访问。需在 `ExporterManager` 上增加：
```rust
pub(crate) fn set_sqlite_wal_mode(&mut self) -> Result<()> {
    match &self.exporter {
        ExporterKind::Sqlite(e) => e.set_wal_mode(),
        _ => Ok(()),  // 非 SQLite exporter 时静默 no-op
    }
}
```

**`export_one_preparsed` 热路径**（exporter/mod.rs 第 204–212 行）：
```rust
#[inline]
pub(crate) fn export_one_preparsed(
    &mut self,
    sqllog: &Sqllog,
    include_pm: bool,
    normalized: Option<&str>,
) -> Result<()> {
    self.exporter.export_one_preparsed(sqllog, include_pm, normalized)
}
```
主线程 merge 写入时调用此方法，`normalized` 传 `row.1.as_deref()`（线程内已计算好的 `Option<String>`）。

---

### `.github/workflows/bench.yml`（config，event-driven）

**Analog:** `.github/workflows/ci.yaml`

**文件头结构**（ci.yaml 第 1–12 行）:
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
```

bench.yml 对称结构（同时监听 PR 和 push to main，用于记录 merge 后基线）:
```yaml
name: Benchmark

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
```

**job steps 模式**（ci.yaml 第 21–29 行）:
```yaml
steps:
  - uses: actions/checkout@v6

  - name: Install Rust
    uses: dtolnay/rust-toolchain@stable

  - name: Cache cargo
    uses: Swatinem/rust-cache@v2
```

bench.yml 复用相同 actions 版本（已在 ci.yaml 中验证：checkout@v6, rust-toolchain@stable, rust-cache@v2）。

**关键差异**：bench.yml 不需要 matrix（只跑 ubuntu-latest），不需要 clippy/rustfmt，只需：
1. `cargo bench`（criterion 自动写 `target/criterion/*/new/estimates.json`）
2. `collect_bench_results.sh`（收集 JSON 并生成 artifact 文件）
3. `actions/upload-artifact@v4` 上传，retention-days: 60

**`timeout-minutes`**（Pitfall 4 对策）：在 job 级别设置 `timeout-minutes: 30`，防止 bench 超时。

**完整 job 骨架**:
```yaml
jobs:
  benchmark:
    name: Run Benchmarks
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench

      - name: Collect benchmark results
        run: bash scripts/collect_bench_results.sh
        env:
          GITHUB_SHA: ${{ github.sha }}

      - name: Upload benchmark artifact
        uses: actions/upload-artifact@v4
        with:
          name: bench-results-${{ github.sha }}
          path: bench-results-*.json
          retention-days: 60
```

---

### `scripts/collect_bench_results.sh`（utility，transform）

**Analog:** 无现成 shell 脚本 analog，参照 RESEARCH.md Pattern 4。

**实现策略：** 优先用 `jq`（ubuntu-latest 内置），以 `find ... -path "*/new/estimates.json"` 递归查找，不硬编码路径深度（Pitfall 5 对策）。

**脚本骨架**（来自 RESEARCH.md Pattern 4 验证版本）:
```bash
#!/usr/bin/env bash
set -euo pipefail

SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"
SHORT_SHA="${SHA:0:8}"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
OUTPUT="bench-results-${SHORT_SHA}.json"

# 收集所有 criterion estimates.json，提取 group/id/mean/stddev
jq -n \
  --arg ts "$TIMESTAMP" \
  --arg sha "$SHA" \
  --argjson benchmarks "$(
    find target/criterion -name 'estimates.json' -path '*/new/estimates.json' \
    | while IFS= read -r f; do
        # 路径: target/criterion/<group>/<id>/new/estimates.json
        group=$(echo "$f" | awk -F/ '{print $(NF-3)}')
        bench_id=$(echo "$f" | awk -F/ '{print $(NF-2)}')
        jq --arg g "$group" --arg i "$bench_id" \
          '{key: ($g + "/" + $i), mean_ns: .mean.point_estimate, stddev_ns: .std_dev.point_estimate}' \
          "$f"
      done | jq -s 'map({(.key): {mean_ns: .mean_ns, stddev_ns: .stddev_ns}}) | add // {}'
  )" \
  '{timestamp: $ts, commit_sha: $sha, benchmarks: $benchmarks}' \
> "$OUTPUT"

echo "Benchmark results written to $OUTPUT"
```

---

## Shared Patterns

### 错误处理
**来源:** `src/error/mod.rs`（通过 parallel.rs / processor.rs 使用方式确认）
**应用于:** `sqlite_parallel.rs`、`sqlite/mod.rs` 新增方法
```rust
// 构造 IO 错误
Error::Io(std::io::Error::other(e))

// 构造 Export 错误（sqlite 专用）
fn db_err(reason: impl Into<String>) -> Error {
    Error::Export(ExportError::DatabaseFailed { reason: reason.into() })
}
```

### 中断检查（热路径中每条记录前）
**来源:** `src/cli/run/parallel.rs`（第 130–132 行）
**应用于:** `sqlite_parallel.rs` 并行线程内循环
```rust
if interrupted.load(Ordering::Relaxed) {
    return Ok(None);
}
```

### `pub(super)` 可见性约定
**来源:** `src/cli/run/parallel.rs`（第 69 行 `pub(super) fn process_csv_parallel`）
**应用于:** `sqlite_parallel.rs` 的 `process_sqlite_parallel`，保持模块封装一致。

### 进度条不重置约定（并行路径）
**来源:** `src/cli/run/parallel.rs`（第 159 行注释）
**应用于:** `sqlite_parallel.rs` 所有 `process_log_file` 调用，传 `reset_pb=false`。

### 测试配置模板（`handle_run` 集成测试）
**来源:** `src/cli/run/tests.rs`（第 113–158 行，`test_parallel_merge_consistent`）
**应用于:** `src/cli/run/tests.rs` 追加的 `test_sqlite_parallel_matches_sequential`
```rust
// TOML 构造模式
let toml = format!(
    "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.sqlite]\ndatabase_url = \"{db}\"\ntable_name = \"sqllog\"\noverwrite = true\nappend = false\n",
    logdir = ..., errlog = ..., applog = ..., db = ...,
);
// 排序对比（D-03：顺序可不同，内容必须一致）
let mut seq_rows: Vec<String> = ...; seq_rows.sort();
let mut par_rows: Vec<String> = ...; par_rows.sort();
assert_eq!(seq_rows, par_rows);
```

---

## No Analog Found

| 文件 | Role | Data Flow | 原因 |
|---|---|---|---|
| `scripts/collect_bench_results.sh` | utility | transform | 项目中无现有 shell 脚本，参照 RESEARCH.md Pattern 4 实现 |

---

## Metadata

**Analog search scope:** `src/cli/run/`, `src/exporter/`, `.github/workflows/`
**Files scanned:** 8（parallel.rs, mod.rs, tests.rs, processor.rs, sqlite/mod.rs, exporter/mod.rs, pipeline/mod.rs, ci.yaml）
**Pattern extraction date:** 2026-05-24
