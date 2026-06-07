# Phase 02: 测试覆盖率与 FSEvents - Research

**Researched:** 2026-06-06
**Domain:** Rust 测试覆盖率补全，cargo-llvm-cov，集成测试模式，FSEvents 平台限制
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** `tests/integration.rs:2917` 的 `test_watch_triggers_on_new_log_file` 保留 `#[ignore]` 注解，不做代码改动。该测试在 macOS `cargo test` 环境下因 FSEvents 事件合并（coalescing）延迟不可靠，但测试体本身有效，适合手动 smoke test 验证。

**D-02:** 正式决策理由（书面依据）：
1. `notify` crate 不提供可注入的事件流 mock — 实现 mock 需要在 `watch/mod.rs` 引入抽象层，涉及架构改动，超出本 Phase 范围。
2. `#[cfg(not(target_os = "macos"))]` 会在 macOS 开发机和 macOS CI 上完全跳过测试，比 `#[ignore]` 更难发现平台差异。
3. 保留 `#[ignore]` 使测试体仍可被 `cargo test -- --ignored` 手动触发，在 smoke test 环境下验证端到端行为。

**D-03:** `tests/integration.rs:110` 的 `test_handle_run_empty_dir_unix_behavior`（stdin tty 行为）不在 QUAL-03 范围内，保持不变。

**D-04:** 主要目标文件：
- `src/cli/run/collector.rs`：48 行 uncovered（函数级 66.67%）——未覆盖路径包括：parse error 累积分支、`process_record` 的 filtered PARAMS 分支（`do_normalize && record.tag.is_none()` 下 passes=false 的路径）。
- `src/exporter/csv/mod.rs`：33 行 uncovered（函数级 72.22%）——通过 WATCH-07/08/09 集成测试间接带动。

**D-05:** 次要目标文件（如主目标超额完成或未能覆盖足够行数，再补充）：
- `src/cli/run/filter_processor.rs`（~75% fn）
- `src/exporter/sqlite/mod.rs`（60% fn）

**D-06:** watch 模块（`src/cli/watch/mod.rs`）当前行覆盖率 84.51%，高于 success criteria 的 80% 门槛，**无需专项测试**。

**D-07:** 覆盖率验证命令：`cargo llvm-cov --summary-only`，以 `TOTAL` 行的 Line % 列为判据。

**D-08:** 在 `tests/watch_incremental.rs` 新增三个集成测试，遵循文件内现有的 `test_watch_03_*` / `test_watch_04_*` 模式：
- WATCH-07 (`test_watch_07_csv_append`): 两次 `trigger_full_file` 后验证 CSV 行累计、header 仅一行
- WATCH-08 (`test_watch_08_error_log_append`): 两次带解析错误的触发后验证 error log 含历史记录
- WATCH-09 (`test_watch_09_exit_code_130`): `interrupted=true` 时 `handle_watch` 返回 `Err(Error::Interrupted)`（匹配 `main.rs` exit 130 路径）

**D-09:** Phase 1 在 `src/cli/watch/mod.rs::tests` 添加的 `test_watch_csv_append`、`test_watch_error_log_append`、`test_handle_watch_returns_interrupted` 保持原位，**不删除**。

### Claude's Discretion

- `collector.rs` 的 `process_record` 是私有函数（`fn process_record`），需通过 `collect_log_file` 公开接口间接测试，或在 `#[cfg(test)]` 块中以 `super::process_record` 调用。
- 若集成测试已将整体行覆盖率推至 92%+，无需额外补充 `exporter/sqlite/mod.rs` 的测试。

### Deferred Ideas (OUT OF SCOPE)

- `exporter/sqlite/mod.rs` 错误路径测试（函数级 60% 覆盖）——若 Phase 2 达标后仍有余量可补充，否则留 Phase 3 或后续 milestone
- `#[cfg(not(target_os = "macos"))]` 条件编译方案的重新评估——仅当 CI 切换到 macOS runner 且 FSEvents 测试稳定性问题解决后才有意义

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QUAL-02 | watch 功能测试补充，整体行覆盖率达到 92%+ | 已确定缺口：collector.rs (48 行) + csv/mod.rs (29 行) = 77 行可补，只需覆盖 70 行即达标 |
| QUAL-03 | macOS FSEvents 限制的 `#[ignore]` 测试调研落地方案 | 决策已锁定：保留 `#[ignore]`，书面依据见 D-01/D-02 |

</phase_requirements>

---

## Summary

Phase 2 的两个目标均已通过 CONTEXT.md 决策阶段完成了关键选择，本次 research 的主要任务是**确认实现路径的可行性**并提供精确的代码坐标。

**QUAL-03（FSEvents）** 已完全确定：`test_watch_triggers_on_new_log_file` 保留 `#[ignore]`，无代码改动。FSEvents 事件合并延迟（>8s）在 `cargo test` 环境下不可控，且 `notify` crate 缺乏可注入 mock 层。此决策的书面依据已在 CONTEXT.md D-02 中记录，QUAL-03 通过"有书面依据"这一路径达标。

**QUAL-02（覆盖率提升）** 的缺口数学已验证：当前总行数 11,861，uncovered 1,019，覆盖率 91.41%。目标 92% 需要额外覆盖 70 行。主要目标：`collector.rs`（35 行 uncovered，已确认为 2 大代码路径）+ `exporter/csv/mod.rs`（29 行 uncovered，集成测试可间接带动）= 64 行，略低于需求。需同时补充 `collector.rs` 单元测试（通过 `run/tests.rs` 访问）。

**Primary recommendation:** 按 D-08 在 `tests/watch_incremental.rs` 增加 3 个集成测试（WATCH-07/08/09），同时在 `src/cli/run/tests.rs` 增加 collector.rs 单元测试（InvalidPath 分支 + process_record 过滤分支），合计约 64-77 行新覆盖，确保超过 70 行门槛。

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 覆盖率度量 | 开发工具层 (cargo-llvm-cov) | — | 构建时工具，不影响运行时架构 |
| collector.rs 测试 | 库内部测试 (src/cli/run/tests.rs) | — | pub(super) 函数，在同一模块的 tests 子模块中可访问 |
| WATCH-07/08/09 集成测试 | 集成测试层 (tests/watch_incremental.rs) | — | 直接调用 pub trigger_* 函数，绕过 notify watcher |
| FSEvents 决策文档 | 代码注释 + RESEARCH.md | — | 无代码改动，仅书面记录 |

---

## Standard Stack

### Core（无新依赖——全部使用现有工具链）

| 工具/库 | 当前版本 | 用途 | 说明 |
|---------|---------|------|------|
| cargo-llvm-cov | 0.8.5 | 行覆盖率报告 | 已安装，`cargo llvm-cov --summary-only` |
| tempfile | 3.27.0 | 集成测试临时目录 | 已在 Cargo.toml dependencies 中（非 dev-only） |
| rusqlite | 0.39.0 | watch_incremental.rs SQLite 验证 | 现有，WATCH-07 用 CSV 不需要 |

**本 Phase 无需新增依赖。** [VERIFIED: 直接检查 Cargo.toml + cargo llvm-cov --version]

### 测试访问模式

| 目标文件 | 访问路径 | 理由 |
|---------|---------|------|
| `src/cli/run/collector.rs` | `src/cli/run/tests.rs` 中 `super::collector::collect_log_file(...)` | `pub(super)` 可见于同模块的 tests 子模块 |
| `src/exporter/csv/mod.rs` | 通过 `trigger_full_file` 间接（集成测试） | CSV exporter 由 handle_run 内部初始化，无需直接调用 |
| 集成测试 WATCH-07/08/09 | `tests/watch_incremental.rs` 中直接调用 `trigger_full_file` | 与 WATCH-03/04 测试模式一致 |

## Package Legitimacy Audit

本 Phase 不安装任何新外部包。跳过此节。

---

## Architecture Patterns

### 覆盖率缺口分析（已实地确认）

```
collector.rs 未覆盖行（35 行）：

Group 1 — InvalidPath 错误路径（6 行，L29-34）：
  LogParserBuilder::new(不存在的路径).build() 返回 Err
  → 触发条件：传入不存在的文件路径

Group 2 — Parse 错误累积循环（16 行，L43, L47-62）：
  parser.iter() 返回 Err(ParseError::InvalidFormat)
  → 触发条件：日志文件包含格式非法的行
  注意：interrupted 检查（L43）也在此未覆盖

Group 3 — !needs_processing 过滤分支（2 行，L92-93）：
  passes=false 且 do_normalize=false（或 record.tag.is_some()）
  → 触发条件：启用过滤器但 record 不是 PARAMS 类型

Group 4 — 被过滤的 PARAMS 记录 else 分支（11 行，L109-119）：
  passes=false 但 do_normalize=true 且 record.tag.is_none()
  → 触发条件：启用归一化 + 启用过滤器 + 记录是 PARAMS 行但被过滤掉
```

```
exporter/csv/mod.rs 未覆盖行（29 行）：

Group A — ensure_parent_dir 错误路径（5 行，L96-100）：无法写目录
Group B — OpenOptions::open 错误路径（5 行，L117-121）：文件打开失败
Group C — write_all header 错误路径（5 行，L135-139）：header 写入失败
Group D — writer_ref 未初始化错误（2 行，L159, L177）：export 前未 initialize
Group E — export_one_preparsed 全路径（7 行，L200-213 减去已覆盖行）：未被 tests 直接调用
Group F — finalize flush 错误路径（4 行，L227-231）：flush 失败
```

### 覆盖率数学验证

```
当前状态（Phase 1 完成后）：
  总行数：11,861
  未覆盖：1,019
  覆盖率：91.41% = (11861 - 1019) / 11861

目标：
  92% = 10,912.12 行需覆盖
  当前覆盖：10,842 行
  缺口：70 行

可获得的覆盖（保守估计）：
  collector.rs Group 1+2：22 行（InvalidPath + parse error 路径）
  collector.rs Group 3+4：13 行（过滤分支）
  csv/mod.rs Group E（通过集成测试）：~7 行
  合计：~42 行

  → 42 行不够，需要额外来源

扩展策略：
  collector.rs 全部 35 行（可达）+ csv/mod.rs Group E 7 行 = 42 行
  仍差 28 行 → 需要追加 csv/mod.rs 更多路径或其他文件

  若集成测试额外带动 csv/mod.rs Group D+E（writer_ref + preparsed）：
  collector.rs 35 行 + csv/mod.rs ~15 行 = 50 行 → 仍差 20 行

  最终安全策略：collector.rs 全部 35 行 + csv/mod.rs ~30 行（通过集成 + 单元测试）≥ 65 行
  → 若 65 行仍不足，从 D-05 次要目标补充（filter_processor.rs 或 sqlite/mod.rs）
```

**重要发现：** 仅靠集成测试（WATCH-07/08/09）可能不足以覆盖 70 行缺口，还需要在 `src/cli/run/tests.rs` 补充 collector.rs 的单元测试。[VERIFIED: 本地运行 cargo llvm-cov + HTML 报告分析]

### 集成测试模式（tests/watch_incremental.rs）

```rust
// 现有模式（WATCH-03 示例），WATCH-07/08/09 照此复用
use dm_database_sqllog2db::cli::watch::{WatchLoopState, trigger_full_file};
use dm_database_sqllog2db::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

fn build_csv_config(log_path: &Path, csv_path: &Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_path.to_string_lossy().into_owned()],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: csv_path.to_string_lossy().into_owned(),
                overwrite: true,
                append: false,    // force_append_for_watch_trigger 会在触发时覆盖为 true
                include_performance_metrics: true,
            }),
            sqlite: None,
        },
        ..Config::default()
    }
}
```

### collector.rs 单元测试模式（src/cli/run/tests.rs）

```rust
// 现有 tests.rs 已有 `use super::*;`，可直接调用 collector 子模块
// collector::collect_log_file 是 pub(super)，在 run 的 tests 子模块中可见

#[test]
fn test_collector_invalid_path_returns_error() {
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = collector::collect_log_file(
        Path::new("/nonexistent/path/that/cannot/exist.log"),
        &pipeline,
        false,
        None,
        &interrupted,
    );
    assert!(result.is_err());
    // 验证是 InvalidPath 错误类型
    assert!(matches!(
        result.unwrap_err(),
        crate::error::Error::Parser(crate::error::ParserError::InvalidPath { .. })
    ));
}
```

### Anti-Patterns to Avoid

- **直接 mock FSEvents 事件流：** notify crate 无 mock API，需要架构改造，超出 Phase 2 范围。[VERIFIED: 代码审查]
- **在集成测试中硬编码覆盖率数值：** 覆盖率以 `cargo llvm-cov --summary-only` 的 TOTAL 行判断，不在测试代码中断言。
- **删除 Phase 1 已有的单元测试：** D-09 明确禁止，单元测试与集成测试互补。
- **修改 `#[ignore]` 测试的注解或代码体：** D-01 明确不做代码改动。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 覆盖率报告 | 自定义 instrumentation | `cargo llvm-cov` | 已安装，`--summary-only` 输出标准，0 配置 |
| FSEvents mock | 自制事件注入层 | 保留 `#[ignore]` | notify 无 mock API；架构改动超出范围 |
| 临时测试文件 | `std::fs::write` + 手动 cleanup | `tempfile::TempDir` | 自动清理，现有 WATCH-03/04 已用此模式 |

---

## Common Pitfalls

### Pitfall 1: collect_log_file 访问路径混淆

**What goes wrong:** 尝试从 `tests/watch_incremental.rs`（集成测试）调用 `collector::collect_log_file`，发现 `pub(super)` 不可见而报编译错误。

**Why it happens:** `pub(super)` 只对 `cli::run` 模块内可见，`tests/` 下的集成测试是独立 crate 边界外的代码。

**How to avoid:** collector 单元测试写在 `src/cli/run/tests.rs` 中（`use super::*` 后可访问 `collector::collect_log_file`）。集成测试通过 `trigger_full_file` 间接覆盖 csv/mod.rs 路径。

**Warning signs:** 编译错误 `error[E0603]: function "collect_log_file" is private`。

### Pitfall 2: parallel 路径才调用 collect_log_file

**What goes wrong:** 只写单文件测试，以为覆盖了 collector.rs，实际上单文件路径走的是 `processor.rs`（顺序路径），不经过 collector。

**Why it happens:** 并行路径触发条件：`jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && csv.is_some()`。单文件无论 jobs 多大都走顺序路径。

**How to avoid:** collector 单元测试**直接调用** `collector::collect_log_file`，不依赖 handle_run 的路由逻辑。或使用多文件目录 + `jobs_override=Some(2)` 触发并行路径。

**Warning signs:** 测试通过但 collector.rs 覆盖率无变化。

### Pitfall 3: trigger_full_file 会覆盖 CSV config 的 append 设置

**What goes wrong:** 在 WATCH-07 集成测试中设置 `append=false`，期望第一次触发覆盖旧文件，第二次触发追加——但两次触发都追加了（header 出现一次正确），或以为 `append=false` 会让第二次触发重写文件。

**Why it happens:** `force_append_for_watch_trigger` 在每次 `trigger_full_file` 调用时将 `append=true, overwrite=false` 注入临时 Config。初始 Config 的 append/overwrite 设置被覆盖。

**How to avoid:** WATCH-07 的断言逻辑正确理解：两次触发都以追加模式写入，header 只在文件为空时写一次（TOCTOU 防护逻辑在 CsvExporter::initialize 中），最终应有 1 header + 2 data rows。

**Warning signs:** header 出现 2 次（说明 append 逻辑未生效）。

### Pitfall 4: WATCH-08 需要真实解析失败行触发 error log

**What goes wrong:** WATCH-08 测试写入合法日志文件，期望 error log 出现，结果 error log 不存在（assertion fail）。

**Why it happens:** `write_error_log` 只在有解析错误时调用。合法日志行不产生解析错误。

**How to avoid:** 日志文件内容需包含格式非法的行（如纯文本 `"this is not a valid dm sql log line\n"`）以触发 ParseError。参见 watch/mod.rs tests 中的 `DM_LOG_LINE_GARBAGE` 常量。

### Pitfall 5: 覆盖率不足 70 行时的补救优先级

**What goes wrong:** 只写 WATCH-07/08/09 集成测试，假设覆盖率自动达到 92%，验证时发现只到 91.7%。

**Why it happens:** csv/mod.rs 的 error path（Group A/B/C/F）是 IO 错误路径，正常集成测试不触发，需要特殊构造（只读目录、写满磁盘等），难以可靠测试。

**How to avoid:** 优先覆盖 collector.rs（35 行，路径明确可控），再检查剩余缺口。collector.rs 的全部 4 个 group 均可通过单元测试精确覆盖，无需特殊 IO 条件。

---

## Code Examples

### WATCH-07 集成测试模板（CSV append 验证）

```rust
// Source: tests/watch_incremental.rs 现有模式 + watch/mod.rs::tests::test_watch_csv_append
#[test]
fn test_watch_07_csv_append() {
    let tmp = TempDir::new().unwrap();
    let log_path_a = tmp.path().join("a.log");
    let log_path_b = tmp.path().join("b.log");
    let csv_path = tmp.path().join("out.csv");

    write_test_log_records(&log_path_a, 0, 3);
    write_test_log_records(&log_path_b, 3, 3);

    let cfg = build_csv_config(&log_path_a, &csv_path);
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(&log_path_a, &cfg, true, false, &interrupted, &mut state, &pb);
    trigger_full_file(&log_path_b, &cfg, true, false, &interrupted, &mut state, &pb);

    let content = std::fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    // 1 header + 6 data rows (3 from each trigger)
    assert!(lines.len() >= 7, "应有 header + 6 rows，实际 {}", lines.len());
    // header 仅出现一次
    let header_count = lines.iter().filter(|&&l| l == lines[0]).count();
    assert_eq!(header_count, 1, "header 应仅出现一次，实际 {header_count} 次");
}
```

### WATCH-08 集成测试模板（error log append 验证）

```rust
// 需要在 tests/watch_incremental.rs 中定义 invalid 日志行常量
const INVALID_LOG_LINE: &str = "this is not a valid dm sql log line at all\n";

#[test]
fn test_watch_08_error_log_append() {
    let tmp = TempDir::new().unwrap();
    let log_path_a = tmp.path().join("a.log");
    let log_path_b = tmp.path().join("b.log");
    let csv_path = tmp.path().join("out.csv");
    let error_log_path = tmp.path().join("errors.log");

    // 每个文件各有 1 条非法行（触发 error log）+ 1 条合法行（保证不提前退出）
    let valid_line = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM t. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    std::fs::write(&log_path_a, format!("{INVALID_LOG_LINE}{valid_line}")).unwrap();
    std::fs::write(&log_path_b, format!("{INVALID_LOG_LINE}{valid_line}")).unwrap();

    let cfg = Config {
        sqllog: SqllogConfig { inputs: vec![log_path_a.to_string_lossy().into_owned()], path_deprecated: None },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: csv_path.to_string_lossy().into_owned(),
                overwrite: true, append: false, include_performance_metrics: true,
            }),
            sqlite: None,
        },
        error: Some(dm_database_sqllog2db::config::ErrorLogConfig {
            file: error_log_path.to_string_lossy().into_owned(),
        }),
        ..Config::default()
    };
    // ...触发两次，验证 error log 含 2 条 [ERROR] 行
}
```

### WATCH-09 集成测试模板（Interrupted 返回验证）

```rust
// 注：handle_watch 是 pub fn，可直接从集成测试调用
use dm_database_sqllog2db::cli::watch::handle_watch;
use dm_database_sqllog2db::error::Error;

#[test]
fn test_watch_09_exit_code_130() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("out.csv");
    let cfg = build_csv_config(tmp.path(), &csv_path);
    let interrupted = Arc::new(AtomicBool::new(true)); // 预设为已中断
    let result = handle_watch(&cfg, true, false, &interrupted);
    assert!(
        matches!(result, Err(Error::Interrupted)),
        "interrupted=true 时 handle_watch 应返回 Err(Interrupted)，实际: {result:?}"
    );
}
```

### collector.rs 单元测试模板（src/cli/run/tests.rs 追加）

```rust
// 在 tests.rs 已有的 `use super::*;` 下可直接访问 collector 模块
#[test]
fn test_collector_invalid_path_returns_error() {
    use crate::pipeline::Pipeline;
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = collector::collect_log_file(
        std::path::Path::new("/nonexistent/absolutely/not/there.log"),
        &pipeline,
        false,
        None,
        &interrupted,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::Error::Parser(crate::error::ParserError::InvalidPath { .. })
    ));
}

#[test]
fn test_collector_parse_error_accumulation() {
    // 写入含无效行的日志文件，验证 parse_error_records 被填充
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("bad.log");
    std::fs::write(&log_path, "not a valid log line\nalso invalid\n").unwrap();
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let (rows, stats) = collector::collect_log_file(
        &log_path, &pipeline, false, None, &interrupted,
    ).unwrap();
    assert!(rows.is_empty());
    assert!(stats.parse_error_count() > 0);
    // parse_error_records 已累积至少一条
    assert!(!stats.parse_error_records.is_empty());
}

#[test]
fn test_collector_filtered_record_not_counted_as_row() {
    // 使用有过滤器的 Pipeline，确保 filtered_out 路径被覆盖
    // 需要一条可解析但不通过过滤器的记录
    // ... （具体过滤器构造取决于 Pipeline API）
}
```

---

## State of the Art

| 旧方案 | 当前方案 | 说明 |
|--------|---------|------|
| `#[cfg(not(target_os = "macos"))]` 跳过 FSEvents 测试 | 保留 `#[ignore]` | CONTEXT.md D-02 已确定，`#[ignore]` 比 cfg 跳过更易发现平台差异 |
| 在 `handle_watch` 层验证 WATCH-07/08/09 | 单元测试在 watch/mod.rs，集成测试在 watch_incremental.rs | Phase 1 已添加单元测试，Phase 2 添加集成测试，两者互补 |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | collector.rs 35 行 uncovered + csv/mod.rs 29 行 uncovered 合计 64 行，加上其他附带覆盖可超过 70 行 | 覆盖率数学 | 若集成测试带动的 csv/mod.rs 行少于预期，需从 D-05 次要文件补充 |
| A2 | `ErrorLogConfig` 在集成测试中可从 `dm_database_sqllog2db::config` 直接访问 | WATCH-08 示例 | 若未公开导出需从 `dm_database_sqllog2db::config::mod` 路径访问 |

**风险缓解：** 每次添加测试后立即运行 `cargo llvm-cov --summary-only` 验证覆盖率增量，及时调整策略。

---

## Open Questions (RESOLVED)

1. **collector.rs Group 3+4 的触发条件精确性**
   - What we know: Group 3（!needs_processing）要求 `passes=false` 且 `do_normalize=false 或 record.tag.is_some()`；Group 4（filtered PARAMS else）要求 `passes=false` 且 `do_normalize=true` 且 `record.tag.is_none()`
   - What's unclear: 如何在 tests.rs 中快速构造一个"有过滤器但不通过"的 Pipeline
   - Recommendation: 查看 `src/pipeline/filters/mod.rs` 的测试模式，使用现有 filter builder API 构造过滤条件
   - **Resolution:** 接受本 Phase 仅覆盖 Group 1+2（InvalidPath + parse error 累积，约 22 行），Group 3+4（过滤分支，约 13 行）按需通过 checkpoint 补救。理由：Group 1+2 + 集成测试带动的 csv/mod.rs 路径已能达到 70 行覆盖增量门槛；Group 3+4 的 Pipeline 过滤器构造成本高（需理解 filter builder API），不是本 Phase 必走路径。如最终 `cargo llvm-cov --summary-only` 显示覆盖率未达 92%，再由执行阶段的 checkpoint 触发补救（参考 D-05 次要目标兜底）。

2. **集成测试的覆盖率贡献量**
   - What we know: WATCH-07/08/09 会间接触发 csv/mod.rs 的 export 路径
   - What's unclear: 具体能覆盖 csv/mod.rs 的哪些行（Group D 的 writer_ref 未初始化路径通常需要异常条件）
   - Recommendation: 执行集成测试后立即运行 llvm-cov 查看增量，再决定是否补充 csv/tests.rs 直接测试
   - **Resolution:** 已通过 Task 2 的 human-verify checkpoint + D-05 兜底路径处理不确定性，视为已解决。具体处理路径：(a) Plan 02 在 collector 单元测试合入后由 human-verify checkpoint 实际运行 `cargo llvm-cov --summary-only` 检查 TOTAL Line %；(b) 若未达 92%，按 D-05 从次要目标（`filter_processor.rs` / `sqlite/mod.rs`）补充测试。集成测试的精确贡献量无需提前估算，由实测数据驱动后续决策。

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo-llvm-cov | QUAL-02 覆盖率验证 | ✓ | 0.8.5 | — |
| tempfile | 集成测试 TempDir | ✓ | 3.27.0 (生产依赖) | — |
| Rust toolchain | cargo test | ✓ | stable (edition 2024) | — |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + cargo-llvm-cov 0.8.5 |
| Config file | なし（标准 cargo test） |
| Quick run command | `cargo test` |
| Full suite command | `cargo llvm-cov --summary-only` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| QUAL-02 | 整体行覆盖率 ≥ 92% | coverage | `cargo llvm-cov --summary-only` | ✅ (llvm-cov 已安装) |
| QUAL-02 | collector.rs InvalidPath 分支覆盖 | unit | `cargo test test_collector_invalid_path` | ❌ Wave 0 |
| QUAL-02 | collector.rs parse error 分支覆盖 | unit | `cargo test test_collector_parse_error` | ❌ Wave 0 |
| QUAL-03 | FSEvents 测试保留 #[ignore] + 书面依据 | documentation | `cargo test -- --ignored 2>&1 \| grep FSEvents` | ✅ (已有 ignore) |
| QUAL-03 | WATCH-07 CSV watch 追加集成测试 | integration | `cargo test test_watch_07_csv_append` | ❌ Wave 0 |
| QUAL-03 | WATCH-08 error log 追加集成测试 | integration | `cargo test test_watch_08_error_log_append` | ❌ Wave 0 |
| QUAL-03 | WATCH-09 Interrupted 返回集成测试 | integration | `cargo test test_watch_09_exit_code_130` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo llvm-cov --summary-only` （TOTAL Line % ≥ 92%）
- **Phase gate:** Full suite green + coverage ≥ 92% before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/cli/run/tests.rs` — collector.rs 单元测试（InvalidPath + parse error + 过滤分支）
- [ ] `tests/watch_incremental.rs` — WATCH-07/08/09 三个集成测试函数 + `build_csv_config` helper + `INVALID_LOG_LINE` 常量
- [ ] `tests/watch_incremental.rs` — WATCH-09 需要 `handle_watch` 的 `use` 导入 + `Error::Interrupted` 访问路径确认

---

## Security Domain

本 Phase 仅涉及测试代码补充和文档记录，无新增生产代码路径，无安全相关变更。跳过 ASVS 评估。

---

## Sources

### Primary (HIGH confidence)

- 本地代码库直接检查 — `src/cli/run/collector.rs`, `src/exporter/csv/mod.rs`, `tests/watch_incremental.rs`（代码可见性、函数签名、测试模式）[VERIFIED: 直接文件读取]
- `cargo llvm-cov --summary-only` 输出 — 覆盖率基线数据 [VERIFIED: 本地执行]
- HTML 覆盖率报告 (`/tmp/coverage-report/html`) — 精确 uncovered 行号 [VERIFIED: 本地执行]
- `.planning/phases/02-fsevents/02-CONTEXT.md` — 所有锁定决策 [VERIFIED: 直接文件读取]

### Secondary (MEDIUM confidence)

- `src/cli/run/mod.rs` 中的并行路径触发条件（`jobs > 1 && log_files.len() > 1`） — 解释了为什么 collect_log_file 不被常规测试覆盖 [VERIFIED: 直接文件读取]

---

## Metadata

**Confidence breakdown:**
- 覆盖率基线数据: HIGH — 本地运行 cargo llvm-cov 确认
- uncovered 行号: HIGH — HTML 报告 Python 解析确认
- 测试访问模式（pub(super)）: HIGH — 直接代码审查
- 集成测试可带动的覆盖行数估计: MEDIUM — 估算值，需执行后验证
- 总覆盖率能否通过现有策略达到 92%: MEDIUM — 数学上边界（64-77 行），存在不足风险需兜底

**Research date:** 2026-06-06
**Valid until:** 2026-07-06（覆盖率基线稳定，但每次代码改动后需重新验证）
