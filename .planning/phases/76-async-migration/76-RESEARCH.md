# Phase 76: 异步解析路径迁移 - Research

**Researched:** 2026-06-11
**Domain:** Rust async migration — tokio + rayon interop, AsyncLogParser call sites
**Confidence:** HIGH

## Summary

Phase 76 的目标（将解析路径从同步 API 迁移到 `AsyncLogParser`）已在 commit `65c24fd` 中完全实现。本 phase 不存在待实现的代码工作——需要的是：**验证现有实现是否满足全部 5 条 Success Criteria，并将 REQUIREMENTS.md 和 ROADMAP.md 中 ASYNC-01 标记为完成**。

研究阶段对代码、构建、测试做了实际核查，结论如下：

- Success Criteria 1（AsyncLogParser + tokio 依赖）：**已满足** — `Cargo.toml` 已包含 `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }` 和 `dm-database-parser-sqllog = { version = "2.0.4", features = ["async"] }`，全部调用点已切换到 `AsyncLogParser`。
- Success Criteria 3（cargo test 全绿）：**已满足** — 本次研究中执行 `cargo test` 返回 408 lib + 87 integration + 7 watch_incremental + 1 jemalloc，共 503 个测试，全部通过，0 失败。
- Success Criteria 4（clippy clean + 无错误路径裸 unwrap）：**已满足** — `cargo clippy --all-targets -- -D warnings` 零警告零错误；生产代码 async 路径无裸 `unwrap()`，所有解析失败走 `log::warn!` + 跳过。
- Success Criteria 5（cargo build --release 成功，体积合理）：**已满足** — release 构建耗时约 21s，产出二进制 3.8MB（含 tokio multi-thread runtime，LTO fat + strip，体积在预期范围内）。
- Success Criteria 2（bench 吞吐量不低于 v1.19 基线）：**待运行** — baselines 目录已存档 v1.0 等早期基线，但无 v1.19/v1.20 对比数据可直接读取。bench 需要真实 `sqllogs/` 目录才能执行 `csv_export_real`，本地若无真实文件则该项验证需人工确认或跳过。

**Primary recommendation:** 本 phase 的 PLAN.md 应聚焦于验收任务（Wave 0 = 运行验证命令），而非实现任务。核心工作是：运行 bench 验证性能无退化，确认 5 条 SC 均绿，更新文档状态（REQUIREMENTS.md、ROADMAP.md、STATE.md）。

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `src/main.rs` 使用 `#[tokio::main]` 宏，`main()` 和 `run()` 均为 `async fn`
  - `features = ["rt-multi-thread", "macros"]` 已写入 `Cargo.toml`
- **D-02:** 不使用 `Runtime::new().block_on()` 作为主入口（保留给 bench 测试用）
- **D-03:** `parallel.rs` 和 `prescan.rs` 中的 rayon 路径采用 `block_in_place + Handle::current().block_on()` 桥接
- **D-04:** 不在 rayon 任务内部创建新的 `Runtime`
- **D-05:** `sequential.rs`、`collector.rs`、`processor.rs`、`scanner.rs` 全部改为 `async fn` + `.await`
- **D-06:** AsyncLogParser 解析错误：`log::warn!` 记录，跳过，继续处理；`parse_errors` 统计恒为 0
- **D-07:** bench 文件在 setup 中用 `Runtime::new().unwrap().block_on(...)` 驱动 async 解析

### Claude's Discretion

- bench 文件中 `Runtime::new().unwrap()` 是否改为 `.expect("tokio runtime")`——planner/verifier 在 clippy 检查时决定
- 各 async fn 签名上 `#[allow(clippy::too_many_arguments)]` 是否清理——属于代码风格，不影响功能正确性

### Deferred Ideas (OUT OF SCOPE)

- flamegraph CPU 热点分析（PROF-01）— Future phase
- heaptrack 峰值内存 profiling（PROF-02）— Future phase
- 将 AsyncLogParser 错误细节重新暴露到 error log — Future phase 或 upstream PR
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ASYNC-01 | 将解析路径从同步 API 切换为 `dm-database-parser-sqllog` 的 async API，解析主循环使用 `.await`（crate 已原生支持 async，添加 tokio 运行时并迁移调用点） | 实现已完成：所有调用点已使用 `AsyncLogParser`，tokio 依赖已添加。验收时需运行 5 条 Success Criteria 对应命令确认全绿。 |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| async 主入口 | CLI 主函数 (`main.rs`) | — | `#[tokio::main]` 标注在 bin crate 入口，runtime 生命周期由此管理 |
| 顺序解析路径 | 应用逻辑 (`sequential/processor/collector.rs`) | — | 纯 async fn + .await，tokio 提供调度 |
| rayon 并行路径 (CSV) | 应用逻辑 (`parallel.rs`) | tokio runtime bridge | rayon 线程内用 `block_in_place + Handle.block_on` 驱动 async |
| rayon prescan 路径 | 应用逻辑 (`prescan.rs`) | tokio runtime bridge | 同 parallel.rs 桥接模式 |
| SQLite 路径 | 应用逻辑 (`sqlite_parallel.rs`) | — | SQLite 写入串行，路径为纯 async fn + .await |
| watch scanner | 应用逻辑 (`scanner.rs`) | — | `scan_files` 为 async fn + .await |
| bench harness | 开发工具 (`benches/*.rs`) | tokio runtime | criterion 闭包内用 `Runtime::new().block_on()` 驱动 async |

## Standard Stack

### Core（已使用）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1 (实际: 1.52.3) | async 运行时，提供 `#[tokio::main]`、`block_in_place`、`Handle` | Rust async 生态事实标准 [VERIFIED: Cargo.toml] |
| dm-database-parser-sqllog | 2.0.4 | 解析 DaMeng SQL 日志，async feature 提供 `AsyncLogParser` | 项目专用 crate [VERIFIED: Cargo.toml] |

### 关键 API 用法（已实现，供验收参考）

**顺序路径（sequential.rs / processor.rs / collector.rs / scanner.rs / sqlite_parallel.rs）：**
```rust
// Source: src/cli/run/processor.rs:205
let records = match AsyncLogParser::new(file_path_buf).parse().await {
    Ok(r) => r,
    Err(e) => {
        log::warn!("parse failed for '{}': {e}", ...);
        // graceful skip
    }
};
```

**rayon 并行路径桥接（parallel.rs / prescan.rs）：**
```rust
// Source: src/cli/run/parallel.rs:118
let handle = tokio::runtime::Handle::current();
// ...在 block_in_place 内：
let records = match handle.block_on(AsyncLogParser::new(file).parse()) {
    Ok(r) => r,
    Err(e) => { log::warn!(...); }
};
```

**bench 文件 sync 闭包内驱动 async：**
```rust
// Source: benches/bench_csv.rs:54-56
tokio::runtime::Runtime::new()
    .unwrap()
    .block_on(handle_run(...))
```

**主入口：**
```rust
// Source: src/main.rs:95-96
#[tokio::main]
async fn main() { ... }
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| rayon 线程内驱动 async | 创建新 Runtime | `block_in_place + Handle.block_on` | 嵌套 Runtime 会 panic；futures::executor::block_on 不集成 tokio reactor 会死锁 [ASSUMED] |
| bench 闭包内驱动 async | 引入 async-std 等替代运行时 | `tokio::runtime::Runtime::new().block_on(...)` | 保持 tokio 生态一致性，criterion harness 本身是 sync [ASSUMED] |

## Common Pitfalls

### Pitfall 1: rayon 嵌套 Runtime panic
**What goes wrong:** 在已有 tokio 运行时的线程上调用 `Runtime::new().block_on()`，触发 "cannot start a runtime from within a runtime" panic。
**Why it happens:** tokio 不允许在运行时线程上嵌套创建新 runtime。
**How to avoid:** rayon worker 线程通过 `block_in_place + Handle::current().block_on()` 桥接——已在本项目正确实现。
**Warning signs:** 运行时 panic 信息含 "within a runtime"。

### Pitfall 2: `parse_errors` 统计断言不匹配
**What goes wrong:** 测试断言 `parse_errors > 0`，但 AsyncLogParser 不追踪逐条错误，导致测试失败。
**Why it happens:** 同步 `LogParserBuilder.iter()` 会将每条解析失败作为 `Err` 返回，可被统计；`AsyncLogParser.parse()` 整体返回，不暴露逐条错误。
**How to avoid:** 已更新：所有测试断言 `parse_errors == 0`（恒为 0）。
**Warning signs:** 集成测试中 `parse_errors` 相关断言失败。

### Pitfall 3: bench 中 `unwrap()` vs `expect()`
**What goes wrong:** `Runtime::new().unwrap()` 在 clippy 语义上无问题（bench 文件不在 `-D warnings` 覆盖范围内），但若改为 `expect("tokio runtime")` 更符合项目风格。
**Why it happens:** Claude's Discretion 项，非功能性问题。
**How to avoid:** planner 可在 Wave 0 中加入可选任务——将 bench 文件的 `.unwrap()` 改为 `.expect("tokio runtime")`。

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test（内置）+ criterion 0.7（bench） |
| Config file | Cargo.toml（[[bench]] entries） |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings && cargo build --release` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ASYNC-01 | AsyncLogParser 调用点正确 | unit + integration | `cargo test` | ✅ |
| SC-1 | tokio + async feature 在 Cargo.toml | build smoke | `cargo build` | ✅ |
| SC-2 | bench 吞吐量不低于 v1.19 基线 | bench | `cargo bench --bench bench_csv -- csv_export_real` | ✅ (需 sqllogs/ 目录) |
| SC-3 | 全量 cargo test 通过 | unit + integration | `cargo test` | ✅ |
| SC-4 | clippy clean + 无裸 unwrap in error paths | lint | `cargo clippy --all-targets -- -D warnings` | ✅ |
| SC-5 | release 构建成功，体积合理 | build | `cargo build --release && ls -lh target/release/sqllog2db` | ✅ |

### 当前验证状态（研究期实际执行结果）

| 验收标准 | 命令 | 结果 |
|---------|------|------|
| SC-1 Cargo.toml | 读取文件 | PASS — tokio 1.52.3、dm-database-parser-sqllog 2.0.4 features=["async"] |
| SC-3 cargo test | `cargo test` | PASS — 503 tests, 0 failed |
| SC-4 clippy | `cargo clippy --all-targets -- -D warnings` | PASS — Finished, 0 warnings |
| SC-5 build | `cargo build --release` | PASS — 3.8MB binary |
| SC-2 bench | 未运行（需 sqllogs/） | PENDING |

### Wave 0 Gaps

- [ ] 若本地有 `sqllogs/` 目录：运行 `cargo bench --bench bench_csv -- csv_export_real/real_file` 并与 baselines/csv_export_real/ 对比
- [ ] 可选：将 bench 文件的 `Runtime::new().unwrap()` 改为 `.expect("tokio runtime")`（Claude's Discretion）

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `LogParserBuilder::new().build()?.iter()` 同步迭代 | `AsyncLogParser::new(path).parse().await` | commit 65c24fd (2026-06-11) | parse 结果为 `Vec<Sqllog>`，逐条错误不再暴露 |
| `async_rt::parse_file_sync` 桥接层 | 直接使用 `Handle.block_on` + `block_in_place` | commit 65c24fd | 消除中间层，rayon/tokio 混合路径更透明 |
| 同步 `main()` | `#[tokio::main] async fn main()` | commit 65c24fd | 整个 CLI 进入 tokio multi-thread runtime |

**Deprecated/outdated:**
- `crate::async_rt::parse_file_sync`：已从所有调用点移除（不再存在）
- `parse_errors` 非零断言：已更新为恒为 0

## Open Questions

1. **SC-2 bench 验证**
   - What we know: baselines/csv_export_real/ 存在 v1.0 等早期基线；当前 benches 文件使用 `Runtime::new().block_on(handle_run(...))` 驱动 async，与 criterion harness 兼容
   - What's unclear: 是否有真实 sqllogs/ 文件可运行 `csv_export_real` bench；v1.19 baseline 是否以 `v1.19` 命名存档
   - Recommendation: planner 在 Wave 0 加入条件任务——若有真实文件，运行 bench 对比；若无，在 commit message 中记录"bench 待真实文件验证"并完成 phase

2. **bench 文件 `.unwrap()` 风格**
   - 属于 Claude's Discretion，不影响 SC 验收
   - Recommendation: planner 可加入可选清理任务（Wave 1 或独立 sub-task）

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | cargo build/test/clippy | ✓ | rustc 1.85+ (rust-version = "1.85") | — |
| tokio | async runtime | ✓ | 1.52.3 (via Cargo.lock) | — |
| criterion | bench harness | ✓ | 0.7 (dev-dependency) | — |
| sqllogs/ 真实日志文件 | SC-2 bench 真实文件测试 | 未知 | — | 跳过 csv_export_real bench，记录为 PENDING |

**Missing dependencies with no fallback:** 无（所有构建和测试依赖均可用）

**Missing dependencies with fallback:**
- `sqllogs/`（真实 .log 文件目录）：SC-2 可在有文件时按需运行，不阻塞 phase 完成

## Sources

### Primary (HIGH confidence)
- `Cargo.toml` — 直接读取，确认 tokio 版本和 features
- `src/main.rs`, `src/cli/run/parallel.rs`, `src/cli/run/prescan.rs`, `src/cli/run/sequential.rs`, `src/cli/run/sqlite_parallel.rs`, `src/cli/run/processor.rs`, `src/cli/run/collector.rs`, `src/scanner.rs` — 直接读取，确认 AsyncLogParser 调用点
- `cargo test` 输出 — 实际执行，503 tests 全绿
- `cargo clippy --all-targets -- -D warnings` 输出 — 实际执行，无警告
- `cargo build --release` 输出 — 实际执行，3.8MB binary

### Secondary (MEDIUM confidence)
- `.planning/phases/76-async-migration/76-CONTEXT.md` — Phase 决策文档，提供 D-01~D-08 实现决策记录

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | rayon 嵌套 Runtime 会 panic（"cannot start a runtime from within a runtime"）— 未在本次研究中实际触发复现 | Don't Hand-Roll | 极低——这是 tokio 文档中的明确限制 |
| A2 | futures::executor::block_on 在 tokio reactor 内会死锁 | Don't Hand-Roll | 极低——本项目已使用正确的 block_in_place 方案且测试通过 |

## Metadata

**Confidence breakdown:**
- Implementation status: HIGH — 直接读取源码和执行命令验证
- Verification coverage: HIGH — SC 1/3/4/5 已实际执行验证
- SC-2 bench: MEDIUM — bench 基础设施已就绪，但真实文件 bench 未实际运行

**Research date:** 2026-06-11
**Valid until:** 2026-07-11（实现已固化，仅 SC-2 bench 需运行时验证）
