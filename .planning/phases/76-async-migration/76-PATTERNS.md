# Phase 76: 异步解析路径迁移 - Pattern Map

**Mapped:** 2026-06-11
**Files analyzed:** 10 (8 source + 2 planning docs)
**Analogs found:** 10 / 10

> **重要说明：** 本 phase 的异步迁移实现已在 commit `65c24fd` 中完全完成。本 PATTERNS.md 专注于**验收模式**——规划文档更新所需的模式，以及验证任务中需要核查的代码模式。

---

## File Classification

| 文件 | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `.planning/ROADMAP.md` | doc | — | `.planning/phases/75-parallel-shared/75-01-PLAN.md` (完成标记模式) | exact |
| `.planning/REQUIREMENTS.md` | doc | — | `.planning/REQUIREMENTS.md` 自身现有行 (Traceability 表格) | exact |
| `.planning/STATE.md` | doc | — | `.planning/STATE.md` 现有格式 | exact |
| `src/main.rs` | entrypoint | request-response | `src/main.rs` (已实现) | verify-only |
| `src/cli/run/parallel.rs` | service | batch + event-driven | `src/cli/run/parallel.rs` (已实现) | verify-only |
| `src/cli/run/prescan.rs` | service | batch | `src/cli/run/prescan.rs` (已实现) | verify-only |
| `src/cli/run/processor.rs` | service | streaming | `src/cli/run/processor.rs` (已实现) | verify-only |
| `src/cli/run/sequential.rs` | service | streaming | `src/cli/run/processor.rs` | role-match |
| `src/cli/run/sqlite_parallel.rs` | service | batch | `src/cli/run/parallel.rs` | role-match |
| `src/scanner.rs` | service | event-driven | `src/cli/run/processor.rs` | role-match |

---

## Pattern Assignments

### `.planning/ROADMAP.md` (doc update — phase completion marker)

**Analog:** ROADMAP.md Phase 75 条目（已完成状态格式）

**Phase 75 完成标记格式**（ROADMAP.md 第 249 行）：
```markdown
- [x] **Phase 75: 并行路径公共逻辑提取** — parallel.rs 与 sqlite_parallel.rs 共享模块（STRUCT-04） (completed 2026-06-11)
```

**Phase 76 待更新为**（当前第 250 行为 `[ ]` 未完成）：
```markdown
- [x] **Phase 76: 异步解析路径迁移** — 切换为 dm-database-parser-sqllog async API，添加 tokio（ASYNC-01） (completed 2026-06-11)
```

**里程碑状态标记格式**（ROADMAP.md 第 5 行，当前为 `🚧`）：
```markdown
- 🚧 **v1.20 性能全面提升** — Phases 72–76 (in progress)
```
Phase 76 完成后，若 v1.20 所有 Phase（72–76）均完成，则改为：
```markdown
- ✅ **v1.20 性能全面提升** — Phases 72–76 (shipped 2026-06-11)
```

---

### `.planning/REQUIREMENTS.md` (doc update — ASYNC-01 status)

**Analog:** REQUIREMENTS.md Traceability 表格现有行（第 51–59 行）

**当前格式**（第 59 行）：
```markdown
| ASYNC-01 | Phase 76 | Pending |
```

**更新为**：
```markdown
| ASYNC-01 | Phase 76 | Done |
```

**需求正文 checkbox**（第 29 行，当前为 `[ ]`）：
```markdown
- [ ] **ASYNC-01**: 将解析路径从同步 API 切换为 `dm-database-parser-sqllog` 的 async API，解析主循环使用 `.await`（crate 已原生支持 async，添加 tokio 运行时并迁移调用点）
```
更新为：
```markdown
- [x] **ASYNC-01**: 将解析路径从同步 API 切换为 `dm-database-parser-sqllog` 的 async API，解析主循环使用 `.await`（crate 已原生支持 async，添加 tokio 运行时并迁移调用点）
```

---

### `.planning/STATE.md` (doc update — current position)

**Analog:** STATE.md 现有格式（第 1–47 行）

**当前 Phase 字段**（第 27 行）：
```yaml
Phase: 75
```
更新为：
```yaml
Phase: 76
```

**status 字段**（第 8 行）：
```yaml
status: completed
```
Phase 76 完成后仍为 `completed`（v1.20 milestone 全部完成）。

---

### 验证目标文件（verify-only，不修改源码）

以下文件已实现，验收时只需读取核查关键模式是否存在，**不需要修改**。

#### `src/main.rs` (entrypoint, request-response)

**D-01 验证点**（第 95–96 行）：
```rust
#[tokio::main]
async fn main() {
```

**D-01 async run 验证点**（第 124 行）：
```rust
async fn run() -> Result<Option<(ErrorStats, bool)>> {
```

**验收检查：** `grep -n "tokio::main\|async fn main\|async fn run" src/main.rs`

---

#### `src/cli/run/parallel.rs` (service, batch + rayon bridge)

**D-03 桥接模式验证点**（第 107–108 行）：
```rust
let handle = tokio::runtime::Handle::current();
```

**D-03 block_in_place 模式验证点**（第 118–126 行）：
```rust
let records = match handle.block_on(AsyncLogParser::new(file).parse()) {
    Ok(r) => r,
    Err(e) => {
        log::warn!("parse failed for '{}': {e}", file.display());
        let mut file_stats = ErrorStats::default();
        file_stats.add_parse_error();
        return Ok((0, file_stats));
    }
};
```

**验收检查：** `grep -n "block_in_place\|Handle::current\|block_on" src/cli/run/parallel.rs`

---

#### `src/cli/run/prescan.rs` (service, batch + rayon bridge)

**D-03 block_in_place 模式验证点**（第 68–76 行）：
```rust
let records = match tokio::task::block_in_place(|| {
    handle.block_on(AsyncLogParser::new(std::path::Path::new(file_path)).parse())
}) {
    Ok(r) => r,
    Err(e) => {
        log::warn!("Pre-scan: failed to parse '{file_path}': {e}");
        return Vec::new();
    }
};
```

**验收检查：** `grep -n "block_in_place\|block_on\|AsyncLogParser" src/cli/run/prescan.rs`

---

#### `src/cli/run/processor.rs` (service, streaming async)

**Import 模式验证点**（第 5 行）：
```rust
use dm_database_parser_sqllog::{AsyncLogParser, Sqllog};
```

**D-05 async fn 验证点：** processor.rs 中处理单文件的函数应为 `async fn`，使用 `.await` 调用 `AsyncLogParser::new(path).parse()`。

**验收检查：** `grep -n "async fn\|\.await\|AsyncLogParser" src/cli/run/processor.rs`

---

## Shared Patterns

### 模式 1：顺序路径 async + await（D-05）

**适用文件：** `sequential.rs`、`collector.rs`、`processor.rs`、`scanner.rs`、`sqlite_parallel.rs`

**Source pattern**（来自 `src/cli/run/processor.rs` + RESEARCH.md 第 83–89 行）：
```rust
// 正确模式（D-05）：顺序路径直接 .await
let records = match AsyncLogParser::new(file_path_buf).parse().await {
    Ok(r) => r,
    Err(e) => {
        log::warn!("parse failed for '{}': {e}", file_path_buf.display());
        // graceful skip — 继续处理下一文件
        return Ok(default_result);
    }
};
```

**D-06 注意：** `parse_errors` 统计恒为 0（AsyncLogParser 不追踪逐条错误），测试断言应为 `assert_eq!(stats.parse_errors, 0)`。

---

### 模式 2：rayon 线程 async 桥接（D-03）

**适用文件：** `parallel.rs`、`prescan.rs`

**Source pattern**（来自 `src/cli/run/parallel.rs` 第 107–126 行 + `src/cli/run/prescan.rs` 第 59–76 行）：
```rust
// 正确模式（D-03）：rayon worker 线程内桥接
// 1. 在进入 rayon 并行之前捕获 Handle
let handle = tokio::runtime::Handle::current();

// 2. rayon 任务内部，通过 block_in_place 告知 tokio 当前线程将阻塞
let records = match tokio::task::block_in_place(|| {
    handle.block_on(AsyncLogParser::new(file).parse())
}) {
    Ok(r) => r,
    Err(e) => {
        log::warn!("...: {e}");
        return default_on_error;
    }
};
```

**反模式警告（D-04）：**
```rust
// 禁止：嵌套 runtime，会 panic "cannot start a runtime from within a runtime"
let rt = tokio::runtime::Runtime::new().unwrap();
let records = rt.block_on(AsyncLogParser::new(file).parse()); // WRONG
```

---

### 模式 3：bench 文件同步驱动 async（D-07）

**适用文件：** `benches/bench_csv.rs`、`benches/bench_sqlite.rs`、`benches/bench_filters.rs`、`benches/bench_parser.rs`

**Source pattern**（来自 RESEARCH.md 第 105–109 行）：
```rust
// criterion 闭包为 sync，用独立 Runtime 驱动 async（bench 文件允许，不在 tokio::main 内）
tokio::runtime::Runtime::new()
    .unwrap()  // 或 .expect("tokio runtime") — Claude's Discretion
    .block_on(handle_run(...))
```

---

### 模式 4：文档更新格式（已完成 phase 标记）

**适用文件：** `ROADMAP.md`、`REQUIREMENTS.md`、`STATE.md`

**ROADMAP.md checkbox 格式：**
```markdown
- [x] **Phase N: 名称** — 描述（REQUIREMENT-ID） (completed YYYY-MM-DD)
```

**REQUIREMENTS.md Traceability 行格式：**
```markdown
| REQ-ID | Phase N | Done |
```

**REQUIREMENTS.md 正文 checkbox 格式：**
```markdown
- [x] **REQ-ID**: 需求描述
```

---

## 验收命令模板

| Success Criteria | 命令 | 预期结果 |
|-----------------|------|---------|
| SC-1: 依赖配置正确 | `grep -A2 "dm-database-parser-sqllog\|tokio" Cargo.toml` | features=["async"] + features=["rt-multi-thread","macros"] |
| SC-3: 全量测试通过 | `cargo test` | 503 tests, 0 failed |
| SC-4: clippy 零警告 | `cargo clippy --all-targets -- -D warnings` | Finished, 0 warnings |
| SC-5: release 构建 | `cargo build --release && ls -lh target/release/sqllog2db` | 二进制 ~3.8MB |
| SC-2: bench（有真实文件时） | `cargo bench --bench bench_csv -- csv_export_real` | 吞吐量 ≥ v1.19 基线 (~1.55M records/sec) |

---

## No Analog Found

无（所有文件均有实现或文档模式可参考）。

---

## Metadata

**Analog search scope:** `src/main.rs`、`src/cli/run/`、`.planning/ROADMAP.md`、`.planning/REQUIREMENTS.md`、`.planning/STATE.md`
**Files scanned:** 10
**Pattern extraction date:** 2026-06-11
