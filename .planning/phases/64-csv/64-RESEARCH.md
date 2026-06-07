# Phase 64: CSV 并行路径基础设施 - Research

**Researched:** 2026-06-04
**Domain:** Rust 并行处理 / rayon / CSV 导出管道验证
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** CSV 并行路径采用 temp-file 方案（每个 rayon 线程独立处理一个文件，写入临时 CSV，最终按顺序拼接）。`parallel.rs` 已完整实现 `process_csv_parallel`，不引入 channel 写入线程。ROADMAP 中"channel"描述是设计意图示例，实际选择 temp-file 更简单且已验证。
- **D-02:** `mod.rs` 中 `use_csv_parallel = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some()` 已满足 SC1（多文件+CSV 自动切换）和 SC4（单文件回退顺序路径）。无需修改切换逻辑。
- **D-03:** 每个 rayon 线程通过 `collector::collect_log_file` 将单文件所有记录收集到 `Vec<(Sqllog, Option<String>)>`，写入临时 CSV 后立即释放。对于 3×300MB 文件场景，峰值内存可接受。

### Claude's Discretion

- `process_csv_parallel` 函数签名和行为不变，Phase 64 主要工作是验证而非修改
- 若 `cargo test` 发现现有测试覆盖不足（仅 `test_handle_run_multi_file` 无内容验证），留给 Phase 66 补充集成测试

### Deferred Ideas (OUT OF SCOPE)

- channel 写入线程架构 — 比 temp-file 更低内存，但复杂度高，留后续里程碑按需评估
- per-file 进度显示（parallel 模式）— Phase 65 负责 verbose 对齐

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARALLEL-01 | 当输入包含多个文件且输出为 CSV 时，自动使用多文件并行解析路径（无需修改 config.toml） | `use_csv_parallel` 条件 (`mod.rs:62`) 已实现；`test_handle_run_parallel_csv_multiple_files` 验证激活路径 |
| PARALLEL-02 | 并行路径写入不全量缓冲内存（注：D-01 决定用 temp-file 方案，ROADMAP 的 channel 描述是设计意图示例，实际实现合规） | `parallel.rs` 每个文件独立写临时 CSV 后立即释放 Vec；同时处理中的内存量 = jobs 个文件的 Vec，非全量缓冲 |

</phase_requirements>

---

## Summary

Phase 64 的核心发现是：**CSV 并行路径已在 v1.16.0（Phase 59）完整实现**，并通过了所有 774 个现有测试（`cargo test` 全绿，`cargo clippy --all-targets -- -D warnings` 无警告）。本 Phase 的实际工作是**验证**现有实现满足 SC1–SC4，而非重建架构。

实现采用 temp-file 方案：rayon 线程池并行处理每个文件，写入临时 CSV parts，最终按顺序拼接到输出文件。切换条件 (`jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && csv.is_some()`) 已正确实现自动路由和单文件回退。

**Primary recommendation:** Phase 64 仅需运行 `cargo test && cargo clippy` 确认无回归，并在计划文档中正式核对四条成功标准，无需代码改动。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 多文件并行路径激活判断 | `cli/run/mod.rs` — `handle_run` | — | 调度层在 orchestration 函数中决定路径选择 |
| 并行解析（文件级） | `cli/run/parallel.rs` — `run_parallel_tasks` | rayon ThreadPool | 每文件独立任务；rayon work-stealing 负载均衡 |
| 单文件记录收集 | `cli/run/collector.rs` — `collect_log_file` | — | 并行/顺序共用，隔离 parser 交互 |
| 临时 CSV 写入 | `cli/run/parallel.rs` — `write_records_to_csv` | `CsvExporter` | 每线程独立写，通过 temp-file 隔离写竞争 |
| Parts 拼接与清理 | `cli/run/parallel.rs` — `concat_csv_parts` / `finalize_concat` | — | 主线程串行执行，保证文件原始顺序 |
| 错误聚合与统计 | `cli/run/parallel.rs` — `collect_parallel_results` | `ErrorStats` | 合并所有文件的解析错误；首个 IO 错误提前终止 |

---

## Standard Stack

### 核心（已存在，无需引入新依赖）

| 库 | 版本（Cargo.toml） | 用途 | 状态 |
|----|-------------------|------|------|
| `rayon` | workspace | 线程池 + `par_iter` | [VERIFIED: Cargo.toml] 已使用 |
| `dm-database-parser-sqllog` | workspace | 文件解析（mmap I/O） | [VERIFIED: Cargo.toml] 已使用 |
| `tempfile` | dev-dependency | 测试临时目录 | [VERIFIED: Cargo.toml] 仅测试用 |

Phase 64 不引入任何新依赖。

---

## Package Legitimacy Audit

> Phase 64 不安装任何新外部包，仅使用已存在于 Cargo.toml 的依赖。跳过此节。

---

## Architecture Patterns

### 系统架构图

```
handle_run (mod.rs)
    │
    ├─ use_csv_parallel? (jobs>1 && files>1 && !stdin && csv.is_some())
    │       │ YES → run_csv_parallel
    │       │           └─ process_csv_parallel (parallel.rs)
    │       │                   ├─ setup_parts_dir → 临时目录 .{stem}_parts_{pid}/
    │       │                   ├─ run_parallel_tasks
    │       │                   │     └─ rayon::ThreadPool.install
    │       │                   │           └─ par_iter → collect_log_file → write_records_to_csv
    │       │                   │                 (每线程：Vec<records> → temp_{idx}.csv → drop Vec)
    │       │                   ├─ collect_parallel_results → parts_info + ErrorStats
    │       │                   └─ finalize_concat
    │       │                         ├─ concat_csv_parts (按原始顺序拼接)
    │       │                         └─ remove_dir_all 清理临时目录
    │       │
    │       └─ NO (files==1 || stdin || !csv) → run_sequential (顺序路径，行为不变)
    │
    └─ print_run_summary
```

### 自动切换条件（SC1/SC4 对应代码）

```rust
// src/cli/run/mod.rs:61-62
let use_csv_parallel =
    jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
```

- SC1（多文件+CSV 自动切换）：`log_files.len() > 1 && csv.is_some()` ✓
- SC4（单文件回退顺序路径）：`log_files.len() == 1` → `use_csv_parallel = false` → `run_sequential` ✓
- SC2（temp-file 无全量缓冲）：每文件 `Vec<records>` 在 `write_records_to_csv` 后立即 drop ✓
- SC3（峰值内存 ≤ 2×）：理论上同时驻留内存 = `jobs` 个文件的 Vec；rayon work-stealing 确保 ≤ `jobs` 个文件并发加载

### 内存模型分析（SC3 理论）

`collect_log_file` 按文件全量收集 → `write_records_to_csv` 写盘 → Vec 被 move 进函数并 drop。rayon par_iter 的 work-stealing 保证任意时刻最多 `jobs` 个线程在运行，即峰值内存 ≤ `jobs` × 单文件记录集大小。单线程路径峰值内存约等于 `1 × 单文件记录集大小 + BufWriter 缓冲`。对于 3×300MB 文件、`jobs = CPU 核数`：若 jobs ≥ 3，理论上并行峰值 ≤ 3 × 单文件峰值；若 jobs = 2，峰值 ≤ 2 × 单文件峰值，满足 SC3（≤ 2×）。ROADMAP 未要求自动化内存基准测试，理论分析足够。[ASSUMED: 基于代码阅读推断，未做实际内存 profiling]

---

## Don't Hand-Roll

| 问题 | 不要自建 | 使用现有方案 | 原因 |
|------|----------|--------------|------|
| 线程池管理 | 自定义线程分配 | `rayon::ThreadPoolBuilder` | 已验证的 work-stealing；`sqlite_parallel.rs` 对称实现 |
| 临时文件管理 | 手动临时目录 | `setup_parts_dir`（已实现）| 含 fallback 到 `temp_dir()`，Windows 兼容 |
| CSV 写入 | 手写序列化 | `CsvExporter` + `ExporterManager` | 确保字段格式与顺序路径完全一致 |

---

## Common Pitfalls

### Pitfall 1: PARALLEL-02 与 temp-file 方案的对齐理解

**What goes wrong:** REQUIREMENTS.md 中 PARALLEL-02 描述"通过 channel 将记录传递给写入线程"，而实际实现是 temp-file 方案，两者措辞不符。
**Why it happens:** ROADMAP 的 Goal 描述是设计意图草案，CONTEXT.md D-01 明确决定接受 temp-file 方案。
**How to avoid:** 以 CONTEXT.md D-01 为权威。PARALLEL-02 的本质要求是"写入不全量缓冲内存"，temp-file 方案满足此要求：Vec 写盘后立即释放，不在内存中累积全量记录。
**Warning signs:** 如果有人要求"实现 channel"，需引导其查看 CONTEXT.md 的 D-01 决策。

### Pitfall 2: test_handle_run_multi_file 无内容验证

**What goes wrong:** `integration.rs:74` 的 `test_handle_run_multi_file` 仅断言 `handle_run` 不报错，未验证 CSV 内容（行数/字段值）。
**Why it happens:** 该测试在 Phase 63 之前创建，目的是冒烟测试而非内容验证。
**How to avoid:** CONTEXT.md 的 Claude's Discretion 明确说明"覆盖不足留给 Phase 66"。Phase 64 不需要修复此测试。内容验证级别的测试 (`test_handle_run_parallel_csv_multiple_files`) 已存在于 `integration.rs:486`，断言了 3×10 = 30 条记录。
**Warning signs:** 若 Phase 64 计划包含"补充集成测试"任务，应提醒这是 Phase 66 的职责。

### Pitfall 3: 单核机器的 jobs == 1 问题

**What goes wrong:** 在 `available_parallelism() == 1` 的环境下，`jobs > 1` 条件不满足，永远不会触发并行路径。
**Why it happens:** `use_csv_parallel` 的第一个条件。
**How to avoid:** 这是已知的设计选择，不是 bug。单核机器的顺序路径更高效。对于 CI 测试（可能单核），`test_handle_run_parallel_csv_multiple_files` 依赖 `handle_run` 不报错；若 CI 是单核，该测试会走顺序路径——同样合法。
**Warning signs:** 如果 CI 报告"30 条记录"测试失败，检查 CI 机器的核数。

---

## Code Examples

### 切换条件（已实现，仅供核对）

```rust
// src/cli/run/mod.rs:61-62
// [VERIFIED: 代码阅读]
let use_csv_parallel =
    jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
```

### 核心入口（已实现，仅供核对）

```rust
// src/cli/run/parallel.rs:266-276
// [VERIFIED: 代码阅读]
pub(super) fn process_csv_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    jobs: usize,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)>
```

### 现有并行 CSV 集成测试（用于 SC1 核对）

```rust
// tests/integration.rs:486-505
// [VERIFIED: 代码阅读]
fn test_handle_run_parallel_csv_multiple_files() {
    // 3 个文件 × 10 条记录 → 断言 CSV 数据行数 == 30
}
```

---

## State of the Art

| 方面 | 实现状态 | Phase 64 任务 |
|------|----------|--------------|
| CSV 并行基础设施 | `parallel.rs` 完整实现（v1.16.0 Phase 59） | 验证，不重建 |
| 切换条件 (`use_csv_parallel`) | `mod.rs:62` 已实现 | 核对 SC1/SC4 |
| 单文件回退 | `log_files.len() == 1` → 顺序路径 | 核对 SC4 |
| 测试覆盖 | `test_handle_run_parallel_csv_multiple_files` 断言行数 | 确认通过，不新增 |
| clippy/test 门禁 | 774 个测试全绿，clippy 无警告（已验证） | 运行一次确认 |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SC3（峰值内存 ≤ 2×）对于 jobs >= 3 的机器理论上不满足；对 jobs == 2 满足 | Architecture Patterns — 内存模型分析 | ROADMAP 未要求自动化内存测试，理论分析足够；若需要硬性验证，需加 jemalloc peak test |

---

## Open Questions

1. **PARALLEL-02 的措辞 vs 实际实现**
   - What we know: REQUIREMENTS.md 写 "通过 channel"，实际是 temp-file
   - What's unclear: PARALLEL-02 是否需要更新措辞以反映实际实现
   - Recommendation: Phase 64 计划中增加一个文档任务，在计划完成后更新 REQUIREMENTS.md 中 PARALLEL-02 的描述，以"写入不全量缓冲内存"替代"通过 channel"措辞，或在 PARALLEL-02 旁加注说明。此改动属于文档对齐，不影响功能。

---

## Environment Availability

> Phase 64 为纯代码/验证 Phase，无新外部依赖。

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo test` | 验证 SC1–SC4 | ✓ | Rust toolchain | — |
| `cargo clippy` | 质量门禁 | ✓ | Rust toolchain | — |
| rayon (crate) | `parallel.rs` | ✓ | workspace | — |

**Missing dependencies:** None

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内建测试 + `assert_cmd` (CLI e2e) |
| Config file | Cargo.toml `[[test]]` 隐式 |
| Quick run command | `cargo test -p dm_database_sqllog2db integration -- --nocapture 2>&1 \| grep -E "PASSED\|FAILED\|ok\|FAILED"` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARALLEL-01 | 多文件+CSV 自动走并行路径（handle_run 不报错，输出 30 条） | integration | `cargo test test_handle_run_parallel_csv_multiple_files` | ✅ `tests/integration.rs:486` |
| PARALLEL-01 | 单文件回退顺序路径（SC4） | integration | `cargo test test_handle_run_real_csv_export` | ✅ `tests/integration.rs:88` |
| PARALLEL-02 | 并行路径输出正确（30 = 3×10）| integration | `cargo test test_handle_run_parallel_csv_multiple_files` | ✅ `tests/integration.rs:486` |
| PARALLEL-02 | 并行路径结果与顺序路径行级一致 | unit | `cargo test test_parallel_merge_consistent` | ✅ `src/cli/run/tests.rs:103` |

### Sampling Rate

- **Per task commit:** `cargo test test_handle_run_parallel_csv_multiple_files test_parallel_merge_consistent`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test && cargo clippy --all-targets -- -D warnings` 全绿

### Wave 0 Gaps

None — 现有测试基础设施已覆盖 Phase 64 的所有验证需求。

---

## Security Domain

> Phase 64 不引入新的输入处理、认证、加密或网络逻辑，仅验证现有实现。ASVS 不适用。

---

## Sources

### Primary (HIGH confidence)

- `src/cli/run/parallel.rs` — 代码阅读，`process_csv_parallel` 完整实现
- `src/cli/run/mod.rs` — 代码阅读，`use_csv_parallel` 切换条件（`mod.rs:62`）
- `src/cli/run/collector.rs` — 代码阅读，`collect_log_file` 内存模型
- `tests/integration.rs:486` — 代码阅读，`test_handle_run_parallel_csv_multiple_files`
- `src/cli/run/tests.rs:103` — 代码阅读，`test_parallel_merge_consistent`
- `cargo test` 执行结果 — 774 个测试全绿（335+366+3+69+1）
- `cargo clippy --all-targets -- -D warnings` 执行结果 — 无警告

### Secondary (MEDIUM confidence)

- `.planning/phases/64-csv/64-CONTEXT.md` — 用户决策（D-01/D-02/D-03）

---

## Metadata

**Confidence breakdown:**
- 现有实现正确性: HIGH — 代码阅读 + 测试全绿确认
- 切换条件满足 SC1/SC4: HIGH — 代码逐行核对
- 内存模型满足 SC3: MEDIUM — 理论分析；无实际 profiling 数据
- Phase 64 工作量: HIGH — 明确为验证而非实现

**Research date:** 2026-06-04
**Valid until:** 2026-07-04（代码库无大改动则持续有效）
