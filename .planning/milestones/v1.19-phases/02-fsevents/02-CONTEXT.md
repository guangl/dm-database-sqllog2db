# Phase 2: 测试覆盖率与 FSEvents - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

两项质量目标：
1. **QUAL-02** — 整体行覆盖率从 91.41% 提升到 92%+（watch 模块 ≥ 80% 已达标，无需专项投入）
2. **QUAL-03** — macOS FSEvents `#[ignore]` 测试有书面落地决策，并补充 WATCH-07/08/09 对应的集成测试

</domain>

<decisions>
## Implementation Decisions

### QUAL-03: FSEvents #[ignore] 落地方案

[auto] Q: "macOS FSEvents `#[ignore]` 采用哪种方案？" → Selected: "Option (c) 保留 #[ignore] + 书面依据" (notify crate 无 mock 层；cfg 跳过会在 macOS CI 静默忽略；测试体保留 smoke test 价值)

- **D-01:** `tests/integration.rs:2917` 的 `test_watch_triggers_on_new_log_file` 保留 `#[ignore]` 注解，不做代码改动。该测试在 macOS `cargo test` 环境下因 FSEvents 事件合并（coalescing）延迟不可靠，但测试体本身有效，适合手动 smoke test 验证。
- **D-02:** 正式决策理由（书面依据）：
  1. `notify` crate 不提供可注入的事件流 mock — 实现 mock 需要在 `watch/mod.rs` 引入抽象层，涉及架构改动，超出本 Phase 范围。
  2. `#[cfg(not(target_os = "macos"))]` 会在 macOS 开发机和 macOS CI 上完全跳过测试，比 `#[ignore]` 更难发现平台差异。
  3. 保留 `#[ignore]` 使测试体仍可被 `cargo test -- --ignored` 手动触发，在 smoke test 环境下验证端到端行为。
- **D-03:** `tests/integration.rs:110` 的 `test_handle_run_empty_dir_unix_behavior`（stdin tty 行为）不在 QUAL-03 范围内，保持不变。

### QUAL-02: 覆盖率缺口填补策略

[auto] Q: "优先补测哪些文件来从 91.41% 提升到 92%？" → Selected: "collector.rs (48 lines) + exporter/csv/mod.rs (33 lines)" (合计 81 行 uncovered，覆盖一半即超所需 71 行，且两个文件关联度高——CSV watch 集成测试可同时带动两者)

- **D-04:** 主要目标文件：
  - `src/cli/run/collector.rs`：48 行 uncovered（函数级 66.67%）——未覆盖路径包括：parse error 累积分支、`process_record` 的 filtered PARAMS 分支（`do_normalize && record.tag.is_none()` 下 passes=false 的路径）。
  - `src/exporter/csv/mod.rs`：33 行 uncovered（函数级 72.22%）——通过 WATCH-07/08/09 集成测试间接带动。
- **D-05:** 次要目标文件（如主目标超额完成或未能覆盖足够行数，再补充）：
  - `src/cli/run/filter_processor.rs`（~75% fn）
  - `src/exporter/sqlite/mod.rs`（60% fn）
- **D-06:** watch 模块（`src/cli/watch/mod.rs`）当前行覆盖率 84.51%，高于 success criteria 的 80% 门槛，**无需专项测试**。WATCH-07/08/09 集成测试带来的覆盖率提升视为附加收益。
- **D-07:** 覆盖率验证命令：`cargo llvm-cov --summary-only`，以 `TOTAL` 行的 Line % 列为判据。

### WATCH-07/08/09 集成测试补充

[auto] Q: "Phase 1 已在 src/cli/watch/mod.rs 补充了单元测试，Phase 2 是否还需要 tests/ 级别的集成测试？" → Selected: "在 tests/watch_incremental.rs 补充集成测试" (对齐 success criteria 「watch 集成测试」要求，同时带动 exporter/csv/mod.rs 覆盖率)

- **D-08:** 在 `tests/watch_incremental.rs` 新增三个集成测试，遵循文件内现有的 `test_watch_03_*` / `test_watch_04_*` 模式（直接调用 `trigger_full_file` / `build_incremental_cfg`，使用 `tempfile::TempDir`）：
  - WATCH-07 (`test_watch_07_csv_append`): 两次 `trigger_full_file` 后验证 CSV 行累计、header 仅一行
  - WATCH-08 (`test_watch_08_error_log_append`): 两次带解析错误的触发后验证 error log 含历史记录
  - WATCH-09 (`test_watch_09_exit_code_130`): `interrupted=true` 时 `handle_watch` 返回 `Err(Error::Interrupted)`（匹配 `main.rs` exit 130 路径）
- **D-09:** Phase 1 在 `src/cli/watch/mod.rs::tests` 添加的 `test_watch_csv_append`、`test_watch_error_log_append`、`test_handle_watch_returns_interrupted` 保持原位，**不删除**。集成测试与单元测试互补：单元测试更快，集成测试覆盖真实文件 I/O 路径。

### Claude's Discretion

- `collector.rs` 的 `process_record` 是私有函数（`fn process_record`），需通过 `collect_log_file` 公开接口间接测试，或在 `#[cfg(test)]` 块中以 `super::process_record` 调用。
- 若集成测试已将整体行覆盖率推至 92%+，无需额外补充 `exporter/sqlite/mod.rs` 的测试。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 2: 测试覆盖率与 FSEvents" — Goal、Success Criteria（SC1–SC3）
- `.planning/REQUIREMENTS.md` §QUAL-02、QUAL-03

### 核心实现文件
- `tests/integration.rs:2917` — `test_watch_triggers_on_new_log_file`（`#[ignore]`，FSEvents 相关，QUAL-03 决策锚点）
- `tests/watch_incremental.rs` — watch 集成测试现有文件（WATCH-07/08/09 新测试写入此处）
- `src/cli/run/collector.rs` — 覆盖率主要目标（48 行 uncovered，parse error + filtered PARAMS 分支）
- `src/exporter/csv/mod.rs` — 覆盖率次要目标（33 行 uncovered，通过集成测试带动）
- `src/cli/watch/mod.rs::tests` — Phase 1 已有的 WATCH-07/08/09 单元测试（`test_watch_csv_append`、`test_watch_error_log_append`、`test_handle_watch_returns_interrupted`）

### 覆盖率基线（Phase 1 完成后）
- 整体行覆盖率：91.41%（目标 92%，缺口 ~71 行）
- watch/mod.rs 行覆盖率：84.51%（目标 80%，**已达标**）
- watch/offsets.rs 行覆盖率：92.63%（**已达标**）
- cli/run/collector.rs：57.89% 行（48 行 uncovered）
- exporter/csv/mod.rs：84.58% 行（33 行 uncovered）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tests/watch_incremental.rs`：现有 `test_watch_03_*` / `test_watch_04_*` 测试模式——`make_run_config` + `tempfile::TempDir` + 直接调用 `trigger_full_file`；WATCH-07/08/09 集成测试照此模式添加
- `src/cli/watch/mod.rs`：`force_append_for_watch_trigger`（Phase 1 新增）已在 `trigger_full_file` 和 `build_incremental_cfg` 中调用，集成测试只需构造适当 Config 即可触发 CSV append 路径
- `src/cli/run/collector.rs:collect_log_file`：`pub(super)` 函数，可在 `cli/run/` 子模块测试中以 `super::collect_log_file` 调用；parse error 路径通过注入包含非法行的日志文件触发

### Established Patterns
- watch 触发测试模式：`tempfile::TempDir` + `make_run_config(&log_dir, &output_file)` + `write_minimal_log` helper + 调用 `trigger_full_file` / `build_incremental_cfg` 两次 + 断言输出文件内容
- `collector.rs` 测试：需要真实 `.log` 文件路径；`LogParserBuilder::new(invalid_path)` 触发 `Error::Parser(InvalidPath)` 分支

### Integration Points
- `tests/watch_incremental.rs` 顶层 `use` 语句：`use sqllog2db::cli::watch::{trigger_full_file, build_incremental_cfg}`——新测试需相同引入
- `Config.append_error_log` 字段（Phase 1 新增）：`pub` 可见，可在集成测试中设置；`force_append_for_watch_trigger` 会自动设置该字段，测试无需手动赋值

</code_context>

<specifics>
## Specific Ideas

- STATE.md 明确：`#[ignore]` 的 FSEvents 测试为已知平台限制，保留作手工 smoke test 用途（QUAL-03 option c）
- Phase 1 SUMMARY 确认：WATCH-07/08/09 单元测试已全部绿色通过，集成测试为增量补充
- 覆盖率工具：`cargo llvm-cov --summary-only`（项目已配置，见 Phase 63 历史）

</specifics>

<deferred>
## Deferred Ideas

- `exporter/sqlite/mod.rs` 错误路径测试（函数级 60% 覆盖）——若 Phase 2 达标后仍有余量可补充，否则留 Phase 3 或后续 milestone
- `#[cfg(not(target_os = "macos"))]` 条件编译方案的重新评估——仅当 CI 切换到 macOS runner 且 FSEvents 测试稳定性问题解决后才有意义
- None — 其余讨论均在 Phase 2 范围内

</deferred>

---

*Phase: 2-fsevents*
*Context gathered: 2026-06-06*
