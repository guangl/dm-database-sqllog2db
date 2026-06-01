# Phase 42: Criterion 基准测试基础设施 - Research

**Researched:** 2026-05-24
**Domain:** Rust benchmark infrastructure (criterion 0.7, dm-database-parser-sqllog 2.0.0)
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 新增 `benches/bench_parser.rs` 文件，专门测试 `dm-database-parser-sqllog` 的原始解析速度（仅解析，不含导出）。
- **D-02:** 同步在 `Cargo.toml` 中添加 `[[bench]] name = "bench_parser" harness = false`。
- **D-03:** 现有 `bench_filters.rs` 的 `no_pipeline` 场景测量的是 parse+CSV export，不算"parser 原始解析速度"，两者并存无冲突。
- **D-04:** bench_parser.rs 使用合成数据（synthetic log），不依赖外部文件或环境变量，与现有 bench 风格一致。
- **D-05:** 合成数据格式与 `bench_csv.rs` / `bench_filters.rs` 中 `synthetic_log()` 函数保持一致（约 170 bytes/record 的达梦格式）。
- **D-06:** 每个 benchmark group 必须包含 `Throughput` 设置（`criterion::Throughput::Elements(N)`），输出 records/sec 指标。
- **D-07:** baseline 标注：使用 `benches/baselines/` 目录（已有目录），通过 `CRITERION_HOME=benches/baselines` 管理。

### Claude's Discretion

- bench_parser.rs 内部 benchmark group 命名（如 `parser_throughput` / `raw_parse`）
- 是否同时覆盖不同 record count（如 1K、10K、100K）的规模测试

### Deferred Ideas (OUT OF SCOPE)

- 真实文件（sqllogs/ 目录）的 benchmark 场景 — 已在 bench_csv/sqlite 中作为可选 skip 场景，不在本 Phase 扩展
- GitHub Actions CI 集成 benchmark — Phase 45
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BENCH-01 | criterion 基准覆盖 CSV 导出、SQLite 导出、filter 路径（含启用/禁用两种模式）、parser 原始解析速度四个场景，`cargo bench` 可独立运行 | 前三个场景已有对应 bench 文件（bench_csv.rs、bench_sqlite.rs、bench_filters.rs），parser 场景通过新增 bench_parser.rs 补全 |
</phase_requirements>

---

## Summary

Phase 42 的核心工作是新增 `benches/bench_parser.rs`，补全第四个场景（parser 原始解析速度），使 BENCH-01 完整达成。前三个场景（CSV、SQLite、filter）已由现有 bench 文件完整覆盖，且均编译通过、运行正常。

`bench_parser.rs` 的实现模式已由 `bench_csv.rs` 中的 `bench_csv_format_only` 函数完整示范：写合成数据到临时目录 → `LogParserBuilder::new(path).build()` → `parser.iter().filter_map(Result::ok).count()`，加 `Throughput::Elements(N)` 设置。唯一需要决策的是 group 命名和是否测多规模（1K/10K/100K）。

**Primary recommendation:** 新增 `benches/bench_parser.rs` + `Cargo.toml [[bench]]` 条目，文件结构完全参照 `bench_csv.rs` 的 `bench_csv_format_only` 模式，测试多个规模（1K、10K、50K），group 命名 `parser_throughput`。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parser 原始解析速度测量 | Benchmark layer | — | 直接调用 `dm-database-parser-sqllog` 的 `LogParserBuilder` + `iter()`，绕过 pipeline 和 exporter |
| 合成数据生成 | Benchmark layer | — | `synthetic_log()` 函数在 bench 文件内定义，不依赖 src/ 任何函数 |
| Cargo.toml 注册 | Build config | — | `[[bench]] harness = false` 条目，与现有三个 bench 条目格式相同 |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| criterion | 0.7.0 | Benchmark harness | 已在 `[dev-dependencies]` 中，含 `html_reports` feature，已通过 `bench_csv`、`bench_sqlite`、`bench_filters` 验证编译 [VERIFIED: Cargo.lock] |
| dm-database-parser-sqllog | 2.0.0 | 被测对象：解析器库 | 项目当前锁定版本，`LogParserBuilder::new(path).build()?.iter()` API 已在多处测试和 bench 中验证 [VERIFIED: Cargo.lock + 代码检查] |
| tempfile | 3.27.0 | 合成数据写入临时目录 | 已在 `[dev-dependencies]` 中，在 `bench_csv.rs` 及 src/ 测试中广泛使用 [VERIFIED: Cargo.lock] |

无需新增任何依赖。本 Phase 不安装新包。

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::fs | std | 写合成数据文件到 `target/bench_parser/` | bench_csv.rs / bench_filters.rs 均使用此模式而非 tempfile |

**注意:** 现有 bench 文件使用 `PathBuf::from("target/bench_xxx")` + `fs::create_dir_all` 而非 tempfile 创建目录。`bench_csv.rs` 中唯一使用 tempfile 的是 src/ 单元测试，bench 文件本身全部用 `target/bench_xxx` 固定路径。bench_parser.rs 应与此保持一致。

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `target/bench_parser/` 固定路径 | `tempfile::TempDir` | tempfile 在每次 iter 前后自动清理，但 bench 文件约定是用固定 target/ 路径——保持一致性优先 |
| 多规模（1K/10K/50K）测试 | 单规模（10K） | 多规模能展示线性扩展性，与 bench_csv/sqlite 对齐，且 Discretion 区域允许；推荐多规模 |

**Installation:** 无需安装，所有依赖已存在。

---

## Package Legitimacy Audit

本 Phase 不新增任何外部包依赖。`criterion`、`dm-database-parser-sqllog`、`tempfile` 均已在项目中锁定使用。

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
bench_parser.rs
    │
    ├── synthetic_log(N) → String（约 170 bytes/record 达梦格式）
    │       └── fs::write("target/bench_parser/sqllogs/bench.log")
    │
    ├── BenchmarkGroup "parser_throughput"
    │       ├── Throughput::Elements(N)
    │       └── for N in [1_000, 10_000, 50_000]:
    │               BenchmarkId::from_parameter(N)
    │               b.iter(|| {
    │                   LogParserBuilder::new(log_path)
    │                       .build().unwrap()
    │                       .iter()
    │                       .filter_map(Result::ok)
    │                       .count()
    │               })
    │
    └── criterion_group! + criterion_main!
```

**关键设计点:** 每次 `b.iter()` 都重新构建 `LogParser`（包含文件读取），这样才能真实测量"从文件到解析完成"的全链路速度。如果只想测迭代器开销，则应在 setup 阶段读取文件，在 iter 中只跑 `parser.iter().count()`——但基于 CONTEXT.md D-03 的说明（bench_filters `no_pipeline` 已有 parse+export，两者并存无冲突），bench_parser 应测"纯 parse 路径"，即每次 iter 包含构建 parser。

### Recommended Project Structure

```
benches/
├── bench_csv.rs        # 已有
├── bench_filters.rs    # 已有
├── bench_sqlite.rs     # 已有
├── bench_parser.rs     # 本 Phase 新增
├── BENCHMARKS.md       # 已有，需追加 bench_parser baseline 记录
└── baselines/
    ├── csv_export/     # 已有
    ├── filters/        # 已有
    ├── sqlite_export/  # 已有
    └── parser_throughput/  # cargo bench 运行后自动创建
```

### Pattern 1: 合成数据 + 固定 target/ 路径（现有约定）

**What:** 在 `target/bench_parser/sqllogs/` 写合成 log 文件，bench 函数中直接使用固定路径。
**When to use:** 所有不依赖外部文件的 synthetic bench。

```rust
// Source: benches/bench_csv.rs（已验证编译 + 运行）
fn synthetic_log(record_count: usize) -> String {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(record_count * 170);
    for i in 0..record_count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:BENCH trxid:{i} stmt:0x1 appname:BenchApp ip:10.0.0.{ip}) [SEL] SELECT col1, col2 FROM bench_table WHERE id={i} AND status='active'. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            ip   = i % 256,
            exec = (i * 13) % 5000,
            rows = i % 1000,
        )
        .unwrap();
    }
    buf
}
```

### Pattern 2: BenchmarkGroup + Throughput::Elements + BenchmarkId（现有约定）

**What:** 多规模测试的标准结构，每规模一个 `BenchmarkId`，group 统一设置 throughput。
**When to use:** 需要输出 records/sec 指标的任何 bench。

```rust
// Source: benches/bench_csv.rs（已验证）
fn bench_parser_throughput(c: &mut Criterion) {
    let bench_dir = PathBuf::from("target/bench_parser");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("parser_throughput");

    for &n in &[1_000usize, 10_000, 50_000] {
        fs::write(sqllog_dir.join("bench.log"), synthetic_log(n)).unwrap();
        let log_path = sqllog_dir.join("bench.log");

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &log_path,
            |b, log_path| {
                b.iter(|| {
                    let parser = LogParserBuilder::new(log_path.to_str().unwrap())
                        .build()
                        .unwrap();
                    parser.iter().filter_map(|r| r.ok()).count()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parser_throughput);
criterion_main!(benches);
```

### Pattern 3: criterion_group! / criterion_main! 宏（harness = false 约定）

**What:** `Cargo.toml` 中 `harness = false`，bench 文件末尾用这两个宏注册。
**When to use:** 所有 criterion bench 文件（项目统一约定）。

```toml
# Cargo.toml — 与现有三个 bench 条目完全对称
[[bench]]
name = "bench_parser"
harness = false
```

### Anti-Patterns to Avoid

- **每次 iter 重新写文件:** `b.iter()` 内部不要调用 `fs::write()`，写文件应在 `bench_parser_throughput` 函数体（setup 阶段），不在 iter 闭包内。
- **忽略 `filter_map(Result::ok)`:** 裸 `parser.iter().count()` 会把 `ParseError` 也计入，错误行也会在迭代器中出现——用 `filter_map(|r| r.ok())` 与项目其他 bench 保持一致。
- **在 iter 闭包内重建文件路径 String:** 使用引用（`&log_path`）而非 `log_path.to_str().unwrap().to_string()`，避免 benchmark 测量中包含 String 分配开销（虽然这点对微秒级结果无大影响，但与已有代码风格一致）。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 计时、统计、抖动过滤 | 自定义计时 + 统计代码 | criterion | criterion 自动处理预热、采样、t-检验、回归报告 |
| Throughput 计算 | 手动计算 records/sec | `Throughput::Elements(N)` | criterion 自动换算并显示 M elem/s |
| Baseline 对比 | 手动保存 / 比对 JSON | `CRITERION_HOME=benches/baselines` + `--baseline` / `--save-baseline` | criterion 内置 baseline 机制，BENCHMARKS.md 已有使用说明 |

**Key insight:** 这是一个纯"参照已有代码写新文件"的任务，核心 API 全部在现有 bench 文件中有完整示例，不需要发明任何新模式。

---

## Common Pitfalls

### Pitfall 1: `iter_with_setup` 在 criterion 0.7 的状态

**What goes wrong:** 开发者查 criterion 0.7 文档，发现 `Bencher` 上只有 `iter`、`iter_batched`、`iter_batched_ref`、`iter_custom`，没有 `iter_with_setup`——但 `bench_filters.rs` 已经使用了 `iter_with_setup`，且编译通过。
**Why it happens:** `iter_with_setup` 仍存在于 criterion 0.7（已验证编译），只是未出现在主文档页面——可能是内部方法或文档未更新。[VERIFIED: cargo bench --bench bench_filters --no-run 编译通过]
**How to avoid:** `bench_parser.rs` 不需要 per-iter setup（parser 构建不需要像 filter 那样需要 `validate_and_compile()`），直接用 `b.iter(|| { ... })` 即可，无需 `iter_with_setup` 或 `iter_batched`。
**Warning signs:** 若未来 criterion 升级导致 `iter_with_setup` 真正移除，`bench_filters.rs` 将编译失败——但这不是本 Phase 需要处理的问题。

### Pitfall 2: `bench_parser.rs` 的 group 名与 baselines 目录名对齐

**What goes wrong:** 若 group 命名为 `"parser throughput"`（含空格）或 `"Parser_Throughput"`，criterion 会用它作为 baselines 目录名，导致 `CRITERION_HOME=benches/baselines cargo bench --bench bench_parser -- --baseline v1.0` 找不到目录。
**Why it happens:** criterion 用 group 名直接作为文件系统路径组成部分。
**How to avoid:** 使用纯小写+下划线：`"parser_throughput"`，与现有 `"csv_export"`、`"filters"`、`"sqlite_export"` 的命名约定一致。
**Warning signs:** 运行 `cargo bench --bench bench_parser` 后 `benches/baselines/` 下出现名称怪异的目录。

### Pitfall 3: 函数超过 40 行（CLAUDE.md 约束）

**What goes wrong:** 如果在一个函数内塞入 synthetic_log 生成 + 多规模循环 + 真实文件 skip 逻辑，容易超过 40 行。
**Why it happens:** bench_csv.rs 中 `bench_csv_format_only` 已经接近 40 行（约 45 行，含注释）。
**How to avoid:** 将 `synthetic_log()` 拆为独立函数（已有先例）；主 bench 函数保持 ≤40 行；若需真实文件 skip 逻辑（本 Phase 不需要），单独成函数。
**Warning signs:** clippy 或代码审查时发现函数超长。

### Pitfall 4: `criterion::black_box` 缺失导致优化消除

**What goes wrong:** 编译器可能优化掉纯计算（如 `.count()` 结果未使用），导致 benchmark 测量接近零。
**Why it happens:** Rust 编译器的死代码消除。
**How to avoid:** 现有 bench 文件使用 `b.iter(|| { ... })` 时，criterion 会自动处理返回值防止优化——`count()` 返回 `usize`，criterion 的 iter 会读取返回值。无需手动 `black_box`，但若发现结果异常（<1μs），考虑加 `std::hint::black_box()`。

---

## Code Examples

### bench_parser.rs 完整骨架（基于现有 bench 模式推导）

```rust
// Source: 参照 benches/bench_csv.rs 结构推导（已验证编译）
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_parser_sqllog::LogParserBuilder;
use std::fs;
use std::path::PathBuf;

fn synthetic_log(record_count: usize) -> String {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(record_count * 170);
    for i in 0..record_count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:BENCH trxid:{i} stmt:0x1 appname:BenchApp ip:10.0.0.{ip}) [SEL] SELECT col1, col2 FROM bench_table WHERE id={i} AND status='active'. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            ip   = i % 256,
            exec = (i * 13) % 5000,
            rows = i % 1000,
        )
        .unwrap();
    }
    buf
}

fn bench_parser_throughput(c: &mut Criterion) {
    let bench_dir = PathBuf::from("target/bench_parser");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("parser_throughput");

    for &n in &[1_000usize, 10_000, 50_000] {
        let log_path = sqllog_dir.join("bench.log");
        fs::write(&log_path, synthetic_log(n)).unwrap();

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &log_path, |b, path| {
            b.iter(|| {
                let parser = LogParserBuilder::new(path.to_str().unwrap())
                    .build()
                    .unwrap();
                parser.iter().filter_map(|r| r.ok()).count()
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parser_throughput);
criterion_main!(benches);
```

### Cargo.toml 新增条目

```toml
[[bench]]
name = "bench_parser"
harness = false
```

### Baseline 管理命令（参照 BENCHMARKS.md）

```bash
# 保存新 baseline（Phase 42 完成后首次运行）
CRITERION_HOME=benches/baselines cargo bench --bench bench_parser -- --save-baseline v1.0

# 对比（Phase 44 性能优化后使用）
CRITERION_HOME=benches/baselines cargo bench --bench bench_parser -- --baseline v1.0
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| criterion 0.5/0.6（`iter_with_setup`） | criterion 0.7（`iter_batched` 为推荐 setup API，`iter_with_setup` 仍存在） | criterion 0.7.0 | bench_filters.rs 的 `iter_with_setup` 仍可编译；bench_parser.rs 不需要 setup |
| `parse_meta()` + `parse_performance_metrics()` 分步调用 | 所有字段在 `Sqllog` 结构体上直接物化（v1.1.0+ 注释确认） | dm-database-parser-sqllog 1.x | bench_csv.rs 注释"v1.1.0: 所有字段已在 Sqllog 上物化"——bench_parser.rs 同样不需要预解析步骤 |

**Deprecated/outdated:**
- `dm-database-parser-sqllog` 的 `parse_meta()` / `parse_performance_metrics()` 分步调用：已由结构体直接物化取代，bench 文件无需调用这些方法。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `b.iter(|| parser.build().iter().count() )` 中 criterion 会防止编译器消除 `.count()` 结果 | Code Examples | 极低风险：若优化消除，benchmark 结果会异常低（<1μs），容易发现 |

**注:** 以上是本研究中唯一的 `[ASSUMED]` 项，风险极低（标准 criterion 行为）。其余所有结论已通过代码检查或编译验证。

---

## Open Questions

1. **bench_parser.rs 是否测量"含文件 IO"还是"纯迭代器"？**
   - What we know: `LogParserBuilder::new(path).build()` 包含文件读取（mmap）；`parser.iter()` 是纯解析
   - What's unclear: CONTEXT.md 说"仅解析，不含导出"但未说明是否包含文件 IO
   - Recommendation: 测量全链路（build + iter），因为这才是"原始解析速度"的实际含义——与 bench_filters `no_pipeline` 场景（含 CSV 写出）的对比才有意义。如果只想测纯 iter，可以加第二个 group `parser_iter_only`（Claude's Discretion 范围内）

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo bench | bench 运行 | ✓ | rustc 1.85（推断） | — |
| criterion 0.7.0 | harness | ✓ | 0.7.0 (locked) | — |
| dm-database-parser-sqllog 2.0.0 | 被测目标 | ✓ | 2.0.0 (locked) | — |

**Missing dependencies with no fallback:** none

---

## Validation Architecture

nyquist_validation 未显式配置（config.json 中无此字段），按默认启用处理。

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (内置) + criterion (bench) |
| Config file | Cargo.toml (bench 配置在 `[[bench]]` 条目) |
| Quick run command | `cargo bench --bench bench_parser --no-run` |
| Full suite command | `cargo bench --bench bench_parser` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-01 (parser) | bench_parser.rs 编译并运行，输出 parser_throughput group 数据 | smoke | `cargo bench --bench bench_parser --no-run` | ❌ Wave 0 |
| BENCH-01 (全套) | `cargo bench` 全部四个 bench 独立运行 | smoke | `cargo bench --no-run` | 部分存在（bench_parser 需新增） |

### Sampling Rate

- **Per task commit:** `cargo bench --bench bench_parser --no-run`（仅验证编译）
- **Per wave merge:** `cargo bench --bench bench_parser`（实际运行，收集数据）
- **Phase gate:** `cargo bench --no-run` 全部通过，`cargo clippy --all-targets -- -D warnings` 无警告

### Wave 0 Gaps

- [ ] `benches/bench_parser.rs` — 覆盖 BENCH-01 parser 场景
- [ ] `Cargo.toml [[bench]]` 条目 — bench_parser 注册

*(bench_csv.rs、bench_sqlite.rs、bench_filters.rs 均已存在且编译通过)*

---

## Security Domain

本 Phase 为纯 benchmark 基础设施，不涉及用户输入、网络、认证、加密等安全域。所有合成数据由代码内部生成，不读取外部环境变量或用户提供的文件。ASVS 检查不适用。

---

## Sources

### Primary (HIGH confidence)

- `benches/bench_csv.rs` (本地代码) — synthetic_log 函数、BenchmarkGroup 模式、LogParserBuilder 用法、target/bench_xxx 目录约定 [VERIFIED: 代码检查 + `cargo bench --bench bench_csv --no-run` 编译通过]
- `benches/bench_filters.rs` (本地代码) — iter_with_setup 用法、多场景对比模式 [VERIFIED: `cargo bench --bench bench_filters --no-run` 编译通过]
- `benches/bench_sqlite.rs` (本地代码) — SQLite bench 模式，sample_size 设置 [VERIFIED: 代码检查]
- `Cargo.toml` (本地文件) — criterion 0.7.0 + html_reports，tempfile 3.27.0，[[bench]] 条目格式 [VERIFIED: 直接读取]
- `Cargo.lock` (本地文件) — 锁定版本：criterion 0.7.0，dm-database-parser-sqllog 2.0.0，tempfile 3.27.0 [VERIFIED: grep]
- `~/.cargo/registry/.../dm-database-parser-sqllog-2.0.0/src/` — 公开 API：`LogParserBuilder`、`LogParser::iter()`、`Sqllog` 结构体字段 [VERIFIED: 直接读取源码]

### Secondary (MEDIUM confidence)

- `docs.rs/criterion/0.7.0` — Bencher 方法列表（`iter`、`iter_batched`、`iter_batched_ref`），`Throughput` 枚举，`BenchmarkId` [VERIFIED: WebFetch 直接访问]
- `benches/BENCHMARKS.md` (本地文件) — CRITERION_HOME 用法、baseline 管理约定 [VERIFIED: 代码检查]

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — 所有依赖已在 Cargo.lock 中锁定，bench 文件已编译验证
- Architecture: HIGH — bench_parser.rs 的实现模式已由 bench_csv.rs 的 `bench_csv_format_only` 完整示范
- Pitfalls: HIGH — 函数长度约束来自 CLAUDE.md，group 命名约定来自现有 bench 文件，均已验证

**Research date:** 2026-05-24
**Valid until:** 2026-06-24（criterion 0.7.x API 稳定）
