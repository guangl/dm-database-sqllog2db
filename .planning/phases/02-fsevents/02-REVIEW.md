---
status: findings
phase: 02-fsevents
reviewed: 2026-06-07
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/cli/run/tests.rs
  - src/cli/run/filter_processor.rs
  - tests/watch_incremental.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
---

# Phase 02-fsevents: Code Review Report

**Reviewed:** 2026-06-07
**Depth:** standard
**Files Reviewed:** 3
**Status:** findings

## Summary

本 phase 仅新增测试代码，无生产代码改动。三个文件共增加：Group 1-4 collector 单元测试、5 个 filter_processor 字段过滤测试、3 个 watch 集成测试（WATCH-07/08/09）及若干辅助结构。

总体质量良好，隔离性到位（TempDir 使用正确，无共享可变状态）。发现 3 个 Warning 和 2 个 Info。

---

## Warnings

### WR-01: Group 4 测试对"过滤后 params_buf 是否被更新"无实质断言

**File:** `src/cli/run/tests.rs:693-718`

**Issue:**
`test_collector_filtered_params_normalize`（Group 4）的目标是验证：被过滤的 PARAMS 记录即使不进入 `rows`，也会更新 `params_buf`。但测试只断言了 `rows.is_empty()`，从未验证 `params_buf` 是否确实被写入。

对照 Group 1 的并联测试 `test_normalize_and_export_filtered_params_updates_buffer`（tests.rs:275-347），该测试通过 `params_buffer.contains_key(&buf_key)` 做了明确断言。Group 4 缺少等价检查，导致目标行为（`else` 分支更新 buffer）在实际上没被验证——即使 `collector::process_record` 的 `else` 分支被删除，此测试仍能通过。

**Fix:**
```rust
// collect_log_file 不直接暴露 params_buf，需通过间接方式验证。
// 最直接的方法：写两行日志，第一行 PARAMS（被 AlwaysFail 过滤），
// 第二行是引用该 stmt 的 DML 记录（pipeline 改为空，令其通过），
// 验证最终 rows[0] 的 normalized_sql 包含替换后的值。
// 若 params_buf 未更新，normalized_sql 仍为 None 或原始 '?'，断言失败。
```

---

### WR-02: WATCH-08 测试对"第二次触发的 error log 为追加"未显式验证追加语义

**File:** `tests/watch_incremental.rs:344-395`

**Issue:**
`test_watch_08_error_log_append` 只统计 `[ERROR]` 行总数 `>= 2`，但这个断言无法区分以下两种实现：
1. 正确实现：`append_error_log=true`，第二次触发以追加方式写，error log 包含来自两次触发的 `[ERROR]` 行。
2. 错误实现：每次触发覆盖写，恰好文件被覆盖为含有 1 条 `[ERROR]` 行（若解析器将两行无效日志合并为一条错误），仍为 `>= 1`，大于等于 2 才能触发失败。

更根本的问题：两个文件都只有 1 条非法行（`INVALID_LOG_LINE`），每次触发各产生 1 条 `[ERROR]`。如果第二次触发覆盖写（bug），则 error log 只会有 1 条 `[ERROR]`，此时断言 `>= 2` 确实会失败——所以测试的保护力度实际上足够。

**但存在隐患**：若未来 `INVALID_LOG_LINE` 被改为产生 2 条错误（例如多行），则覆盖写模式也能通过 `>= 2` 的断言，测试退化为无效。

**Fix:**
```rust
// 在第一次 trigger_full_file 之后、第二次之前记录文件大小，
// 第二次触发后验证文件大小增大（不是截断后重写）：
let size_after_first = std::fs::metadata(&error_log_path).unwrap().len();
trigger_full_file(&log_path_b, &cfg, ...);
let size_after_second = std::fs::metadata(&error_log_path).unwrap().len();
assert!(
    size_after_second > size_after_first,
    "error log 应追加（文件变大），而非截断重写"
);
```

---

### WR-03: `AlwaysFail` 未实现 `Debug`，但 `LogProcessor` trait 要求 `Debug` bound

**File:** `src/cli/run/tests.rs:598-604`

**Issue:**
`Pipeline` 的 `processors: Vec<Box<dyn LogProcessor>>` 要求 `LogProcessor: std::fmt::Debug`（见 `src/pipeline/mod.rs:151`）。`AlwaysFail` 在 tests.rs 中标注了 `#[derive(Debug)]`（第 598-599 行），这是正确的。

然而，tests.rs 中另有 `AlwaysFail` 无 Debug 的情况吗？检查后确认仅一处定义，derive 存在。本条降为确认无误的细节——见 Info 区。

**实际 Warning**：`AlwaysFail` 被定义为模块级别（`tests` 模块之外，在 `mod.rs` 的 `#[cfg(test)] mod tests;` 引入的 `tests.rs` 顶层），这导致它对同 crate 内所有测试模块可见但不可复用（无法从集成测试引用）。这不是 bug，但若将来 `collector.rs` 的测试需要相同的 `AlwaysFail`，会有重复定义。

**Fix:** 将 `AlwaysFail` 移入 `tests.rs` 内部的某个 `mod helpers` 中或至少加 `#[cfg(test)]` 注释说明其性质（已隐含在 `tests.rs` 文件范围内，无需动作）。实际上此处风险较低，本条保留为 Warning 是因为它处于 `use super::*` 作用域外但与生产代码共享 `src/cli/run/mod.rs` 的编译单元，可能引起 clippy `dead_code` 警告（若未被所有测试分支引用）。

**验证命令：**
```bash
cargo clippy --all-targets -- -D warnings
```

---

## Info

### IN-01: `build_csv_config` helper 注释与实际行为存在描述偏差

**File:** `tests/watch_incremental.rs:272-290`

**Issue:**
注释说"`append=false, overwrite=true`：`trigger_full_file` 内的 `force_append_for_watch_trigger` 会在每次触发时将 append 覆盖为 true"。

这描述是准确的，但注释中的"初始值不影响最终行为（per Pitfall 3）"有误导风险：`overwrite=true` 初始值确实会在 `force_append_for_watch_trigger` 被调用后被覆盖为 `false`（强制追加）。但如果有读者直接用 `build_csv_config` 返回值去调用 `handle_run`（而非通过 `trigger_*`），将得到覆盖写语义，与注释暗示的"追加"相悖。

**Fix:** 注释补充说明：此 helper 仅供配合 `trigger_*` 函数使用，不应直接传入 `handle_run`。

---

### IN-02: `test_watch_07_csv_append` 假设 `trigger_full_file` 对不同文件路径的 CSV 使用同一输出文件，但 `build_csv_config` 的 `log_path` 参数在第二次触发时被忽略

**File:** `tests/watch_incremental.rs:293-341`

**Issue:**
两次 `trigger_full_file` 调用传入的 `cfg` 都是用 `log_path_a` 构建的（第 303 行 `let cfg = build_csv_config(&log_path_a, &csv_path)`），但第二次触发时传入的 `path` 参数是 `&log_path_b`（第 318 行）。

在 `trigger_full_file` 实现中（`watch/mod.rs:316`），`tmp_cfg.sqllog.inputs` 会被替换为 `path`（即 `log_path_b`），所以实际读取的是 `log_path_b` 的内容——行为是正确的。

但 `cfg` 的构造用的是 `log_path_a`，这造成了轻微的"入参与注释意图不一致"的代码气味，将来重构 `build_csv_config` 签名时可能引发误解。

**Fix:** 简化为不依赖 `log_path` 参数意义：
```rust
// cfg 中的 log_path 会被 trigger_full_file 内部覆盖，传任意有效路径即可
let cfg = build_csv_config(tmp.path(), &csv_path);
```

---

## Verdict

未发现 Critical（数据丢失、安全漏洞、逻辑错误）问题。3 个 Warning 中：

- **WR-01** 最实质：Group 4 测试对目标行为（`else` 分支更新 params_buf）缺乏断言，测试可能是一个"幽灵通过"——产品代码删除该分支测试仍绿。建议补充断言。
- **WR-02** 对 append 语义的验证可以加固，但当前保护力度在正常情况下已足够。
- **WR-03** 是 clippy 潜在警告风险，运行 `cargo clippy --all-targets -- -D warnings` 后确认实际结果。

`filter_processor.rs` 中新增的 5 个测试（sessions/apps/statements/threads/Debug）逻辑正确，断言清晰，隔离良好。

---

_Reviewed: 2026-06-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
