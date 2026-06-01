# Phase 44: 热路径与内存优化 - Pattern Map

**Mapped:** 2026-05-24
**Files analyzed:** 4 (modified)
**Analogs found:** 4 / 4

---

## File Classification

| 修改文件 | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `src/pipeline/normalizer.rs` | utility / hot-path | transform | self（修改现有函数） | exact |
| `Cargo.toml` | config | — | self（已有 `[dev-dependencies]` 段） | exact |
| `benches/bench_csv.rs` | benchmark | batch | `benches/bench_parser.rs` | role-match |
| `benches/BENCHMARKS.md` | docs | — | self（追加新 phase 段落） | exact |

---

## Pattern Assignments

### `src/pipeline/normalizer.rs` — `compute_normalized` 热点优化（H-1 / H-2 / H-3）

**文件角色:** utility，transform 数据流  
**修改范围:** 第 8 行（`ParamBuffer` 类型别名）+ 第 350 行（H-1 insert）+ 第 367-369 行（H-2/H-3 clone）

#### 现有 `ParamBuffer` 类型别名（第 8 行）

```rust
// 当前
pub type ParamBuffer = HashMap<(String, String), Vec<ParamValue>>;
```

**优化后应改为（H-3：Arc 浅拷贝替代 Vec 深拷贝）：**

```rust
use std::sync::Arc;
pub type ParamBuffer = HashMap<(String, String), Arc<Vec<ParamValue>>>;
```

`Arc<Vec<ParamValue>>` 通过 `Deref` 自动降解为 `&[ParamValue]`，`apply_params_into` 签名无需修改。

#### H-1：PARAMS 记录 insert（第 349-351 行，当前代码）

```rust
// src/pipeline/normalizer.rs, lines 348-352
if pm_sql.starts_with("PARAMS(") {
    if let Some(params) = parse_params(pm_sql) {
        buffer.insert((record.sess_id.clone(), record.statement.clone()), params);
    }
}
```

**优化后（包装进 Arc，key clone 无法消除——tuple borrow 限制，见 RESEARCH.md Pitfall 3）：**

```rust
if pm_sql.starts_with("PARAMS(") {
    if let Some(params) = parse_params(pm_sql) {
        buffer.insert(
            (record.sess_id.clone(), record.statement.clone()),
            Arc::new(params),
        );
    }
}
```

#### H-2 / H-3：DML 记录 key clone + params Vec clone（第 367-369 行，当前代码）

```rust
// src/pipeline/normalizer.rs, lines 367-369
let key = (record.sess_id.clone(), record.statement.clone());

let params = buffer.get(&key)?.clone();
```

**优化后（Arc clone 替代 Vec clone，key clone 不变——HashMap 标准实现 borrow 限制）：**

```rust
let key = (record.sess_id.clone(), record.statement.clone());

let params = buffer.get(&key)?.clone(); // 此时 clone 仅复制 Arc 引用计数（原子操作）
```

注意：`ParamBuffer` 类型改为 `Arc<Vec<ParamValue>>` 后，这行代码**文字不变**，但语义从"深拷贝整个 Vec"变为"复制 Arc 指针"。`apply_params_into` 调用处 `&params` 通过 `Arc → Deref → &[ParamValue]` 自动转换，无需改动：

```rust
// src/pipeline/normalizer.rs, line 386 — 调用点无需修改
apply_params_into(pm_sql, &params, colon_style, scratch);
```

#### `apply_params_into` 签名参考（不修改，仅供对齐）

```rust
// src/pipeline/normalizer.rs, lines 189
fn apply_params_into(sql: &str, params: &[ParamValue], colon_style: bool, out: &mut Vec<u8>) {
```

`Arc<Vec<ParamValue>>` 实现 `Deref<Target=[ParamValue]>`，传入 `&*params` 或 `&params[..]` 均可，实际上 `&params` 在 Rust 中会自动 Deref，所以现有调用无需修改。

---

### `Cargo.toml` — 添加 jemalloc dev-dependencies

**文件角色:** config  
**修改范围:** `[dev-dependencies]` 段（当前第 85-87 行）

#### 现有 `[dev-dependencies]` 段（第 85-87 行）

```toml
[dev-dependencies]
tempfile = "3.27.0"
criterion = { version = "0.7", features = ["html_reports"] }
```

**优化后追加（D-01/D-03：仅 dev-dep，不影响生产 allocator）：**

```toml
[dev-dependencies]
tempfile = "3.27.0"
criterion = { version = "0.7", features = ["html_reports"] }
tikv-jemallocator = "0.6.1"
tikv-jemalloc-ctl = "0.6.1"
```

#### 已有 `[profile.flamegraph]`（第 80-83 行，勿修改，直接可用）

```toml
[profile.flamegraph]
inherits = "release"
debug = true
strip = "none"
```

#### `[lints.rust]` 中 `unsafe_code = "warn"`（第 57 行）

新增 `#[global_allocator]` 属于 safe Rust，不触发 `unsafe_code` lint。但 `tikv-jemallocator` 内部包含 unsafe；放在 test/bench cfg 下编译时需确认 clippy 不报告新警告。

---

### `benches/bench_csv.rs` — 集成 jemalloc 统计或保存 phase44 baseline

**文件角色:** benchmark，batch 数据流  
**最近 Analog:** `benches/bench_parser.rs`（同为 criterion benchmark，结构相同）

#### bench_parser.rs 顶层结构（第 1-10 行，copy pattern）

```rust
/// Baseline benchmark: parser throughput.
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_parser_sqllog::LogParserBuilder;
use std::fs;
use std::path::PathBuf;
```

#### bench_csv.rs 现有顶层结构（第 1-13 行）

```rust
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::config::Config;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
```

#### jemalloc 统计函数模式（新增，参考 RESEARCH.md Code Examples）

按 D-02 决策，jemalloc 统计**不集成进 criterion benchmark**（会干扰测量精度），而是作为独立 `#[test]` 函数。以下模式用于在 `tests/` 下新增文件时参考：

```rust
// 文件顶部（cfg 隔离，避免影响生产 allocator——D-03）
#[cfg(test)]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// 测量辅助函数（40 行以内，满足项目规约）
#[cfg(test)]
fn measure_alloc_delta(f: impl FnOnce()) -> usize {
    use tikv_jemalloc_ctl::{epoch, stats};
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();
    e.advance().unwrap();               // 刷新统计，防止读到陈旧数据（Pitfall 5）
    let before = alloc.read().unwrap();
    f();
    e.advance().unwrap();
    let after = alloc.read().unwrap();
    after.saturating_sub(before)
}
```

#### criterion baseline 保存命令模式（来自 BENCHMARKS.md）

```bash
# 保存优化前基线（Wave 0）
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --save-baseline phase44-before

# 优化后对比（Wave 1+）
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase44-before
```

**bench_csv.rs 的 `bench_csv_export` 函数结构（第 54-80 行，这是 PERF-01 要对比的 group）：**

```rust
fn bench_csv_export(c: &mut Criterion) {
    let bench_dir = PathBuf::from("target/bench_csv");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("csv_export");

    for &n in &[1_000usize, 10_000, 50_000] {
        fs::write(sqllog_dir.join("bench.log"), synthetic_log(n)).unwrap();
        let cfg = make_config(&sqllog_dir, &bench_dir);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &cfg, |b, cfg| {
            b.iter(|| {
                handle_run(cfg, true, &Arc::new(AtomicBool::new(false)), None).unwrap();
            });
        });
    }

    group.finish();
}
```

---

### `benches/BENCHMARKS.md` — 追加 Phase 44 基线记录段

**文件角色:** docs，追加现有 phase 历史格式  
**Analog:** 文件内现有各 phase 段落（Phase 4、5、6、9、10、42）

#### 现有 phase 段落结构（以 Phase 42 为例，第 509-555 行）

```markdown
## Phase 42 — Parser baseline（v1.11）

**Date:** 2026-05-24
**Goal:** ...
**Test environment:** Apple Silicon (Darwin 25.5.0), release build (...)

### parser_throughput（合成日志，三规模）

| Records | Median time | Throughput |
|--------:|------------:|-----------:|
| ...

### Criterion 输出原文

<details>
<summary>cargo bench --bench bench_parser --save-baseline v1.0（parser_throughput，Phase 42）</summary>
...
</details>

### 结论

- [x] ...
```

**Phase 44 段落应使用相同结构**，包含：
1. `## Phase 44 — 热路径与内存优化` 标题
2. `**Date:**`、`**Goal:**`、`**Test environment:**`
3. Wave 0 基线表格（phase44-before 中位数）
4. Wave 1 优化后对比表格（vs phase44-before）
5. jemalloc 峰值堆统计数值（Wave 0 基线 → Wave 1 优化后）
6. `<details>` 包裹的 criterion 原文输出
7. 结论 checklist（PERF-01 / PERF-02）

---

## Shared Patterns

### 热循环 fast-path 保护模式

**来源:** `src/cli/run/processor.rs` 第 69-73 行  
**适用:** 任何修改 `params_buffer` 或 `compute_normalized` 调用路径时，必须保留此 fast-path 检查不变：

```rust
// 快速路径：params_buffer 为空且当前是 DML 记录（有 tag），
// 则不可能存在待替换参数，完全跳过 compute_normalized。
let ns = if do_normalize && (!params_buffer.is_empty() || record.tag.is_none()) {
    crate::pipeline::compute_normalized(...)
} else {
    None
};
```

修改 `ParamBuffer` 类型后，`params_buffer.is_empty()` 检查仍然有效（`HashMap::is_empty()` 与 value 类型无关）。

### 错误日志模式

**来源:** `src/cli/run/processor.rs` 第 110-120 行  
**适用:** 所有新增 `Result`-returning 函数的错误处理，沿用现有 `map_or_else` + `file_stats` 统计模式：

```rust
exporter_manager
    .export_one_preparsed(&record, include_pm, ns)
    .map_or_else(
        |e| {
            if e.is_fatal() {
                file_stats.set_fatal(e.to_string());
            } else {
                file_stats.add_export_error();
            }
            eprintln!("[{}] {file_path}: {e}", e.severity());
            log::warn!("{file_path} | export error: {e:?}");
        },
        |()| {},
    );
```

### BufWriter 容量常量（H-4 备查）

**来源:** `src/exporter/csv/mod.rs` 第 124 行  
**当前实际值（非 CLAUDE.md 记述的 16MB）：**

```rust
let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file); // 2 MB
```

若 H-4 实测有收益，可调整为 `16 * 1024 * 1024`（16MB），但需 criterion 对比验证（A5 假设）。

### clippy 豁免模式（函数行数超限时）

**来源:** `Cargo.toml` 第 70 行  
`too_many_lines = "allow"` 已在全局 lint 配置中豁免，`compute_normalized`（64 行）无需额外 `#[allow]` 属性。

---

## No Analog Found

所有文件均有直接 analog 或为修改现有文件，无需依赖 RESEARCH.md 的纯推断模式。

| 文件 | 说明 |
|------|------|
| `tests/jemalloc_peak.rs`（可选新增） | 项目内无 jemalloc 测试先例；模式来自 RESEARCH.md Code Examples（标注 ASSUMED）。Wave 0 实现时需以 `cargo test` 验证 API 形态。 |

---

## Metadata

**Analog search scope:** `src/pipeline/`, `src/exporter/csv/`, `src/cli/run/`, `benches/`, `tests/`, `Cargo.toml`  
**Files read:** normalizer.rs, Cargo.toml, bench_csv.rs, bench_parser.rs, BENCHMARKS.md, csv/mod.rs, processor.rs, integration.rs  
**Pattern extraction date:** 2026-05-24
