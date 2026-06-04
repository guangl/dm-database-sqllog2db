# Phase 66: 兼容性验证与测试 - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

三项工作：(1) 确认 `cargo test` 全部 740+ 测试通过（无回归）；(2) 在 `tests/integration.rs` 新增 ≥2 条多文件并行 CSV 集成测试，验证并行与顺序路径输出内容一致；(3) 确认 config.toml 格式（`sqllog2db init` 生成内容）与 v1.16 基线一致。

</domain>

<decisions>
## Implementation Decisions

### 集成测试结构（COMPAT-02）

[auto] Q: "如何实现并行 vs 顺序 CSV 内容对比测试？" → Selected: "顺序拼合 vs 并行输出，按行集合对比" (recommended default)

- **D-01:** 测试策略：
  1. 用 `write_test_log` 写入 ≥2 个临时 .log 文件（各含不同数量记录）
  2. **顺序基线**：对每个文件单独运行（单文件 → 顺序路径），将各输出 CSV 的数据行（跳过 header）合并
  3. **并行路径**：将所有文件配置为 inputs，运行 `handle_run`（自动触发并行路径），读取输出 CSV 数据行
  4. 对两个行集合排序后断言相等（`assert_eq!(sorted_sequential, sorted_parallel)`）
  5. 同时断言并行输出有 header 行

- **D-02:** 测试用例 1（`test_parallel_csv_content_matches_sequential`）：2 个文件，各 10 条记录，无过滤器，验证行集合 + header。

- **D-03:** 测试用例 2（`test_parallel_csv_with_filter_matches_sequential`）：2 个文件，启用 include 过滤器（如按 user），验证过滤后行集合与顺序路径一致（PARALLEL-04）。

### 测试位置和辅助函数

[auto] Q: "集成测试放在哪里，如何复用现有辅助函数？" → Selected: "放在 tests/integration.rs，复用 make_run_config/write_test_log" (recommended default)

- **D-04:** 新测试追加到 `tests/integration.rs`（已有 `test_handle_run_multi_file` 为基础，但无内容验证）。复用 `write_test_log`（写合成 log）和 `make_run_config`（构建 Config）。
- **D-05:** 顺序基线需构建单文件 Config（`inputs` 仅包含一个文件），多次运行 `handle_run`，将各输出 CSV append 到基线文件，或各自读取再合并行。推荐各自读取再 Vec 合并（避免 append 模式复杂性）。

### config.toml 格式验证（COMPAT-03）

[auto] Q: "如何验证 config.toml 格式不变？" → Selected: "现有 test_init_template_has_csv_append_comment 测试已覆盖，补充字段数量断言" (recommended default)

- **D-06:** `tests/integration.rs` 中 `test_init_template_has_csv_append_comment`（line ~194）和 `test_init_template_has_csv_file_comment` 已验证 init 模板关键字段。Phase 66 在 plan 中确认此测试通过，若需精确对比可补充"无并行相关新字段"断言（grep 输出文件不含 "parallel" 或 "jobs" 字样）。

### Claude's Discretion

- 测试数据量：每文件 20 条（2×20=40 条）比 10 条更能暴露行顺序问题
- 排序策略：对 CSV 数据行（字符串排序）即可，无需解析字段

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 66: 兼容性验证与测试" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §COMPAT-01、COMPAT-02、COMPAT-03

### 关键文件
- `tests/integration.rs` — 现有测试集（~740 条），新增位置；`make_run_config`（line 31）、`write_test_log`（无明确行数，搜"fn write_test_log"）、`test_handle_run_multi_file`（line 74）
- `src/cli/init.rs` — init 模板字符串（config.toml 内容来源）

### 对齐参考
- `src/cli/run/parallel.rs` — Phase 64 实现（被测目标）
- `src/cli/run/mod.rs` — `use_csv_parallel` 条件（确保测试触发并行路径）

</canonical_refs>

<code_context>
## Existing Code Insights

### 现有测试基础
- `test_handle_run_multi_file`（line 74）：2 文件多文件测试，但只验证"不 panic"，无行内容断言
- `test_handle_run_real_csv_export`（line 89）：单文件，验证 header + 10 行
- 断言模式：`content.lines().count()` + 具体内容检查

### 并行路径触发条件
- `use_csv_parallel = jobs > 1 && log_files.len() > 1` — 集成测试环境 `available_parallelism()` 通常 > 1，测试可直接验证并行路径

### 辅助函数
- `write_test_log(path, n)` — 写 n 条合成记录
- `make_run_config(log_dir, csv_file)` — 返回 glob inputs 指向 log_dir

### 顺序基线构建技巧
- 需要临时构建"单文件 Config"，可复用 `make_run_config` 后覆盖 `inputs` 字段为单文件路径

</code_context>

<specifics>
## Specific Ideas

- 测试名称：`test_parallel_csv_content_matches_sequential`、`test_parallel_csv_filter_matches_sequential`
- 行集合对比：`let mut seq_lines: Vec<String> = ...; seq_lines.sort(); let mut par_lines: Vec<String> = ...; par_lines.sort(); assert_eq!(seq_lines, par_lines);`
- SC4 clippy/fmt 验证：在 PLAN 中作为收尾任务（`cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`）

</specifics>

<deferred>
## Deferred Ideas

- 内存基准测试（对比并行/顺序峰值内存）— 可选，不列入 Phase 66 必要工作
- property-based 测试（随机记录集并行 vs 顺序）— 超出范围

</deferred>

---

*Phase: 66-compat*
*Context gathered: 2026-06-04*
