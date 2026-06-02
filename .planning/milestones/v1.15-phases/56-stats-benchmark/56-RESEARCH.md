# Phase 56: stats 模块清理与 benchmark 稳定化 - Research

**Researched:** 2026-06-02
**Domain:** Rust 代码重构 / 公共模块抽取 / CI benchmark 文档
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 新建独立模块（`src/scanner.rs` 或类似命名），将文件扫描逻辑（包含 parse error 处理、错误计数）抽取为公共函数，`run` 和 `stats` 共用
- **D-02:** 当前 `src/stats/mod.rs` 的 `scan_files_into_accumulator` 函数中的 `log::warn!` 处理改为走公共模块的 error log 路径，与 `run` 命令对齐
- **D-03:** `run` 命令的文件扫描部分同步重构为调用公共模块（保持行为不变，仅提取）
- **D-04:** `benches/BENCHMARKS.md` 新增一节，说明如何从 GitHub Actions artifacts 下载 `bench-results-*.json` 文件，以及如何手动对比历史数据

### Claude's Discretion

（无明确标注的 Claude's Discretion 区域）

### Deferred Ideas (OUT OF SCOPE)

- benchmark CI 门控（自动回归检测）
- parse error 影响退出码（退出码 1）
- crates.io 自动发布
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLEAN-01 | stats 模块删除遗留 warn! 占位符，stats/output.rs 所有函数不超过 40 行 | 代码审查已确认满足；验证任务仅需 grep 确认 |
| BENCH-01 | 确认 scripts/collect_bench_results.sh 存在，bench.yml 以信息性（non-blocking，continue-on-error）方式运行 | Phase 55 已实现；验证任务仅需 stat + grep 确认 |
</phase_requirements>

---

## Summary

本 Phase 包含两类工作：**验证类**（确认 Phase 55 已满足的成功标准）和**实现类**（代码重构 + 文档补充）。

验证类工作量极低：代码审查已确认 `src/cli/stats/mod.rs` 无 `warn!`、`src/stats/output.rs` 所有函数 ≤40 行、`scripts/collect_bench_results.sh` 存在且可执行、`bench.yml` 含 `continue-on-error: true`。planner 只需安排简单 grep/stat 检查任务即可。

实现类工作是核心：新建 `src/scanner.rs` 公共模块，将 `src/stats/mod.rs::scan_files_into_accumulator` 的扫描逻辑提取为通用函数，同时让 `src/cli/run/processor.rs` 的单文件扫描路径也通过该公共模块调用。关键发现：`run` 命令的 parse error 当前也是通过 `log::warn!` + `ErrorStats` 计数处理的（见 `processor.rs:144-152`），**不存在独立的 error log 文件写入**。因此 D-02 的"对齐 run 命令"指的是：stats 的 parse error 处理要同样返回 `ErrorStats`（计数），而非仅有 `log::warn!`。

`benches/BENCHMARKS.md` 补充一节 CI artifact 使用说明，无代码变更，风险极低。

**Primary recommendation:** 新建 `src/scanner.rs`，签名接受 callback + `&mut ErrorStats`，直接替换 `scan_files_into_accumulator` 内部实现，并让 `processor.rs` 的解析循环复用同一逻辑。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 文件扫描 + parse error 计数 | Library/Core（`src/scanner.rs`） | CLI 命令层调用 | 两个命令共用同一扫描逻辑，应下沉到 lib 层 |
| Parse error 可观测性 | 调用方（run/stats） | scanner 提供原始 ErrorStats | scanner 不决定展示方式，交给调用方 warn!/info! |
| Benchmark CI 文档 | 文档层（`benches/BENCHMARKS.md`） | — | 纯文档，无代码变更 |
| 成功标准验证 | 测试/检查层 | — | grep/stat 确认，不改代码 |

---

## Standard Stack

本 Phase 不引入新依赖。全部工作基于现有栈：

| 组件 | 版本 | 用途 | 状态 |
|------|------|------|------|
| `dm_database_parser_sqllog` | 现有版本 | 日志解析迭代器 | 不变 |
| `log` | 现有版本 | warn!/info! | 不变 |
| `thiserror` | 现有版本 | 错误类型 | 不变 |

**无需运行 npm/pip/cargo add。**

---

## Package Legitimacy Audit

本 Phase 不安装任何新包，跳过此节。

---

## Architecture Patterns

### 系统架构（重构后）

```
src/stats/mod.rs::run_stats()
    ↓ scanner::scan_files(files, cfg, callback, &mut stats)
    ↓ callback: accumulator.update(&record)
    ↓ ErrorStats 返回调用方

src/cli/run/processor.rs::process_log_file()
    ↓ scanner::scan_files(...) 或 scanner::parse_one_file(...)
    ↓ ExporterManager::export_one_preparsed
    ↓ ErrorStats 合并
```

### 新模块结构

```
src/
├── scanner.rs       # 新增：公共文件扫描，返回 ErrorStats
├── stats/
│   └── mod.rs       # 重构：scan_files_into_accumulator 改为调用 scanner
└── cli/run/
    └── processor.rs # 重构：解析循环部分改为调用 scanner（可选深度）
```

### Pattern 1: 公共扫描函数签名

`scanner.rs` 的核心函数签名应满足：
- 接受文件列表 `&[PathBuf]`
- 接受记录处理 callback `impl FnMut(&Sqllog)`
- 填充 `&mut ErrorStats`（parse error 计数）
- 返回 `Result<()>`（文件打开/路径错误为 Err，parse error 不终止）

```rust
// [ASSUMED] — 基于代码库现有模式推断，具体签名由实现决定
pub fn scan_files<F>(
    log_files: &[std::path::PathBuf],
    on_record: &mut F,
    stats: &mut crate::error::ErrorStats,
) -> crate::error::Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
```

stats 调用侧（替换 `scan_files_into_accumulator` 内部）：

```rust
// [VERIFIED: codebase grep] — 当前 scan_files_into_accumulator 的行为基础
let mut error_stats = ErrorStats::default();
scanner::scan_files(&log_files, &mut |record| accumulator.update(record), &mut error_stats)?;
// 将 error_stats 通过 log::warn! 或 info! 暴露（由调用方决定）
```

### Anti-Patterns to Avoid

- **不要在 scanner.rs 内部决定是否 warn!/info!**：scanner 只负责计数，日志级别由调用方控制，否则两个命令的日志行为耦合
- **不要修改 process_log_file 的外部接口**：该函数有复杂的 ProgressBar、parallel、normalize 参数，此次重构范围是内部解析循环，不改签名
- **不要引入 async/Send 约束**：项目全程同步，scan 函数无需 Send

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parse error 计数 | 自定义 counter struct | `ErrorStats::add_parse_error()` | 已有，避免重复 |
| 日志文件解析迭代 | 自定义迭代器 | `dm_database_parser_sqllog::LogParserBuilder` | 已有，已在两处使用 |

---

## 关键代码发现（验证类）

### CLEAN-01 验证：stats/mod.rs 无 warn!

```
$ grep -n "warn!" src/cli/stats/mod.rs
（无输出）
```
[VERIFIED: codebase grep] — `src/cli/stats/mod.rs` 无任何 `warn!` 调用。

### CLEAN-01 验证：output.rs 函数长度

[VERIFIED: codebase grep + awk] — `src/stats/output.rs` 所有函数体行数：

| 函数 | 行数 |
|------|------|
| `write_csv_stats` | 10 |
| `write_slow_csv` | 21 |
| `write_frequent_csv` | 24 |
| `write_sqlite_stats` | 20 |
| `run_sqlite_transaction` | 10 |
| `write_slow_table` | 20 |
| `write_frequent_table` | 31 |
| `db_err` | 3 |

全部 ≤40 行，CLEAN-01 已满足。

### BENCH-01 验证

[VERIFIED: codebase file check] — `scripts/collect_bench_results.sh` 存在，内容完整（读取 `target/criterion`，输出 `bench-results-${SHORT_SHA}.json`）。

[VERIFIED: codebase file check] — `.github/workflows/bench.yml` 第 25 行：`continue-on-error: true`。

---

## 重构关键发现

### [error] 配置段不对应代码中的任何字段

[VERIFIED: codebase grep] — `Config` struct（`src/config/mod.rs`）**没有** `error` 字段。`config.toml` 中的 `[error]` section 被 serde（无 `deny_unknown_fields`）静默忽略。

**影响：** CONTEXT.md 中的"error log 写入通过 `cfg.error` 配置获取路径"描述**与当前代码不符**。`run` 命令的 parse error 处理是：
- `processor.rs:144-146`：`file_stats.add_parse_error()` + `log::warn!("{file_path} | {e:?}")`
- `processor.rs:151-152`：`log::warn!("{file_path}: {errors_in_file} parse errors")`

不存在将 parse error 写入独立文件的逻辑。"对齐 run 命令"实际含义是：stats 的 parse error 也应该增加 `ErrorStats` 计数（而不是只有 `log::warn!`），以便调用方能感知到 parse error 数量。

**Planner 注意：** D-02 的实现目标是为 `scan_files_into_accumulator`（或其替代的公共函数）引入 `ErrorStats` 返回，而非接入不存在的 error log 文件写入。

### scan_files_into_accumulator 当前与 run 的差异

| 方面 | stats（当前） | run（processor.rs） |
|------|--------------|-------------------|
| Parse error 处理 | `log::warn!` only | `log::warn!` + `file_stats.add_parse_error()` |
| 错误计数返回 | 无 | `(usize, ErrorStats)` |
| 文件打开失败 | `return Err(...)` | `return Err(...)` |

重构后 scanner 公共函数需要补齐 `ErrorStats` 计数（差异行），`log::warn!` 保留（CONTEXT 第 85 行明确：不写 error log 时仍需 warn! 或 info! 可观测）。

### src/stats/mod.rs::scan_files_into_accumulator 当前实现

[VERIFIED: codebase read] — 位于第 38-68 行，共 31 行，当前完全自包含，使用 `LogParserBuilder` 直接扫描。重构时此函数体替换为调用 `scanner::scan_files`，函数本身可保留或直接内联到 `run_stats`。

---

## Common Pitfalls

### Pitfall 1: process_log_file 重构范围失控

**What goes wrong:** 试图将 `process_log_file` 整个签名改为调用 `scanner`，触碰 ProgressBar、parallel、normalize、ExporterManager 等复杂参数。
**Why it happens:** `process_log_file` 的职责是解析+导出+进度显示，scanner 只负责解析。
**How to avoid:** D-03 的实现范围是"文件扫描部分"（即内部的 `LogParserBuilder::new(...).build()` + 迭代循环），不是整个函数。可以将 parser 创建提取到 scanner，主循环留在 `processor.rs`，或者直接不改 processor，只改 stats。
**Warning signs:** 若改动超过 `processor.rs` 第 52-68 行范围，应停下重新评估。

### Pitfall 2: scanner 引入 lib.rs 循环依赖

**What goes wrong:** `scanner.rs` import 了 `cli` 模块的内容，或 `main.rs` 直接声明 scanner 却没有在 `lib.rs` 导出。
**Why it happens:** `stats/mod.rs` 是 lib 侧，`cli/run/processor.rs` 是 cli 侧，公共 scanner 必须在 lib 层，不能在 cli 层。
**How to avoid:** `scanner.rs` 只依赖 `crate::error`、`crate::config`（如需），以及 `dm_database_parser_sqllog`。在 `src/lib.rs` 中加 `pub(crate) mod scanner;`（或 `pub mod scanner`），`src/main.rs` 中同样加 `mod scanner;`。
**Warning signs:** 编译报 "use of undeclared crate or module"。

### Pitfall 3: test_run_stats_skips_parse_errors 测试失败

**What goes wrong:** 重构后 `scan_files_into_accumulator` 改为调用新 scanner，parse error 处理路径变化，导致现有测试（`src/stats/mod.rs:163`）中的断言失败。
**Why it happens:** 该测试混合了合法行和非法行，验证 parse error 不终止流程。
**How to avoid:** scanner 必须保留"parse error 不终止，继续下一条记录"语义（当前 `match parse_result { Err(err) => log::warn!(...) }` 的行为）。
**Warning signs:** `cargo test` 报 `test_run_stats_skips_parse_errors FAILED`。

---

## Code Examples

### 当前 scan_files_into_accumulator（重构对象）

```rust
// [VERIFIED: src/stats/mod.rs:38-68]
fn scan_files_into_accumulator(
    log_files: &[std::path::PathBuf],
    accumulator: &mut StatsAccumulator,
) -> Result<()> {
    for file_path in log_files {
        log::info!("stats: scanning {}", file_path.display());
        let file_path_str = file_path.to_str().ok_or_else(|| { /* ... */ })?;
        let parser = dm_database_parser_sqllog::LogParserBuilder::new(file_path_str)
            .build()
            .map_err(|err| { /* ... */ })?;
        for parse_result in parser.iter() {
            match parse_result {
                Ok(record) => accumulator.update(&record),
                Err(err) => log::warn!("parse error in {}: {err}", file_path.display()),
            }
        }
    }
    Ok(())
}
```

### run 命令的 parse error 处理（行为参考）

```rust
// [VERIFIED: src/cli/run/processor.rs:143-153]
Err(e) => {
    errors_in_file += 1;
    file_stats.add_parse_error();
    log::warn!("{file_path} | {e:?}");
}
// ...
if errors_in_file > 0 {
    log::warn!("{file_path}: {errors_in_file} parse errors");
}
```

### bench.yml artifact 上传段（文档说明基础）

```yaml
# [VERIFIED: .github/workflows/bench.yml:41-47]
- name: Upload benchmark artifact
  uses: actions/upload-artifact@v4
  with:
    name: bench-results-${{ github.sha }}
    path: bench-results-*.json
    retention-days: 60
```

---

## State of the Art

本 Phase 为代码整洁与文档补充，无外部技术演进需跟进。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | scanner 公共函数签名为 `(files, callback, &mut ErrorStats) -> Result<()>` | Architecture Patterns | 低：签名调整不影响语义，planner 可在实现时根据实际调用场景微调 |
| A2 | D-03 的"run 命令文件扫描部分"仅指 processor.rs 的 parser 创建+迭代循环，不含 ProgressBar 等 | Common Pitfalls | 中：若用户期望更大范围提取，实现成本上升，但行为不变 |

---

## Open Questions (RESOLVED)

1. **D-03 重构深度**
   - What we know: `process_log_file` 含 ProgressBar、parallel、normalize、ExporterManager 复杂参数，整体不适合与 stats 共用
   - What's unclear: 是否只要求 stats 侧调用 scanner，run 侧可以不改（"行为不变，仅提取"描述模糊）
   - Recommendation: Planner 应将 D-03 拆分为两个任务：(a) 新建 scanner 并替换 stats 侧，(b) run 侧提取 parser 创建部分到 scanner（内部调用），两者都可以独立验证
   - **RESOLVED:** 56-02 Task 1 采用主备两方案——主方案（回调改造，≤30 行新代码）/ 备方案（辅助函数小范围提取），若两方案都需 >50 行改动或破坏现有测试则保留 processor.rs 原样并在 SUMMARY 中说明。

2. **scanner 模块可见性**
   - What we know: stats 是 lib 侧，cli/run 是 cli/bin 侧，公共函数需要两者都可见
   - What's unclear: 是否需要 `pub mod scanner` 对外 API 暴露，还是 `pub(crate)` 内部使用
   - Recommendation: 使用 `pub(crate) mod scanner`，与现有 `parser.rs`（`pub(crate) mod parser`）保持一致
   - **RESOLVED:** 使用 `pub(crate) mod scanner`，在 `src/lib.rs` 中注册，与现有 `pub(crate) mod parser` 模式一致。

---

## Environment Availability

Step 2.6: SKIPPED（本 Phase 为纯代码/文档修改，无外部工具依赖）

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置测试框架（cargo test）|
| Config file | Cargo.toml（无额外配置文件）|
| Quick run command | `cargo test -p sqllog2db -- stats` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLEAN-01 | stats/mod.rs 无 warn! 占位符 | 静态检查 | `grep -n "warn!" src/cli/stats/mod.rs` 返回空 | ✅（grep 检查，无测试文件）|
| CLEAN-01 | output.rs 所有函数 ≤40 行 | 静态检查 | `awk` 函数行数统计 | ✅（已验证满足）|
| BENCH-01 | collect_bench_results.sh 存在可执行 | 文件检查 | `stat scripts/collect_bench_results.sh` | ✅ |
| BENCH-01 | bench.yml 含 continue-on-error | 文件检查 | `grep "continue-on-error" .github/workflows/bench.yml` | ✅ |
| D-01/D-02 | scanner 公共模块：parse error 不终止 | 单元测试 | `cargo test -- test_run_stats_skips_parse_errors` | ✅ Wave 0 已有 |
| D-01/D-02 | scanner 公共模块：ErrorStats 计数 | 单元测试 | 新增（见 Wave 0 Gaps）| ❌ Wave 0 需新建 |
| D-03 | run 命令行为不变 | 集成测试 | `cargo test -- integration` | ✅（现有集成测试覆盖）|

### Sampling Rate

- **Per task commit:** `cargo test -- stats && cargo clippy -p sqllog2db -- -D warnings`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/scanner.rs` 的单元测试：`test_scan_files_counts_parse_errors` — 验证 parse error 不终止 + ErrorStats 计数
- [ ] `src/scanner.rs` 的单元测试：`test_scan_files_returns_err_on_invalid_path` — 验证文件打开失败返回 Err

*(现有 `test_run_stats_skips_parse_errors` 已覆盖顶层行为，scanner 层测试属于单元粒度补充)*

---

## Security Domain

本 Phase 不涉及认证、会话、加密、网络请求或用户输入验证，跳过 Security Domain 分析。

---

## Sources

### Primary (HIGH confidence)

- `src/stats/mod.rs`（直接读取）— scan_files_into_accumulator 实现，parse error 处理模式
- `src/cli/run/processor.rs`（直接读取）— run 命令的 parse error 处理（ErrorStats 计数 + log::warn!）
- `src/cli/stats/mod.rs`（直接读取 + grep 确认）— 无 warn! 调用
- `src/stats/output.rs`（直接读取 + awk 确认）— 所有函数 ≤40 行
- `src/config/mod.rs`（直接读取）— Config struct 无 error 字段
- `.github/workflows/bench.yml`（直接读取）— continue-on-error: true 第 25 行
- `scripts/collect_bench_results.sh`（直接读取）— 存在且内容完整
- `src/lib.rs`（直接读取）— 模块树，pub(crate) mod parser 的可见性模式

### Secondary (MEDIUM confidence)

- `benches/BENCHMARKS.md`（直接读取）— 现有 benchmark 文档结构，D-04 新增节的参考

---

## Metadata

**Confidence breakdown:**

- 验证类（CLEAN-01、BENCH-01）: HIGH — 代码已直接阅读并 grep 确认
- 重构架构（D-01/D-02/D-03）: HIGH（边界清晰）/ MEDIUM（D-03 重构深度有歧义）
- 文档（D-04）: HIGH — 现有文档结构清晰，新增节内容明确

**Research date:** 2026-06-02
**Valid until:** 2026-07-02（稳定代码库，30 天有效）
