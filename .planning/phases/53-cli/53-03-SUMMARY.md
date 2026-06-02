---
phase: 53-cli
plan: "03"
subsystem: config+stats+cli+tests
tags: [stats, validation, time-range, integration-tests, init-template]
dependency_graph:
  requires:
    - 53-01 (validate_time_str + StatsConfig + validate_stats_time_fields 提前实现)
    - 53-02 (handle_stats 优先级合并 + log::info! 输出 from/to 值)
  provides:
    - run_stats 入口防御性验证（validate_cfg_stats_time）
    - validate.rs 4 个单元测试（stats 时间格式）
    - stats/mod.rs 2 个单元测试（run_stats 拒绝非法格式）
    - init 模板 [stats] 注释段（from/to/top 三字段示例 + YYYY-MM-DD 格式说明）
    - 7 个 Phase 53 端到端集成测试（覆盖 SC#1-4）
  affects:
    - src/config/validate.rs（新增 4 个单元测试）
    - src/stats/mod.rs（新增 validate_cfg_stats_time 函数 + 2 个单元测试）
    - src/cli/init.rs（CONFIG_TEMPLATE_EN 追加 [stats] 注释段）
    - tests/integration.rs（新增 make_stats_config_with_section + 7 个集成测试）
tech_stack:
  added: []
  patterns:
    - 防御性验证：run_stats 入口在 log_files 查询前二次校验 from/to 格式
    - write!/writeln! 替代 format! + push_str（满足 clippy::format_push_string）
key_files:
  created: []
  modified:
    - src/config/validate.rs
    - src/stats/mod.rs
    - src/cli/init.rs
    - tests/integration.rs
decisions:
  - validate_cfg_stats_time 抽为私有函数，run_stats 主体保持在 40 行以内
  - "#[allow(unused_imports)] 保留在 pub use config::StatsConfig 行（bin target 不直接引用此路径；lib 消费者通过 crate::stats::StatsConfig 访问）"
  - Plan 01 已提前实现 validate_stats_time_fields，Task 1 的 RED 阶段测试立即通过（预期偏差）
metrics:
  duration: "约 30 分钟"
  completed_date: "2026-06-01"
  tasks_completed: 3
  files_changed: 4
---

# Phase 53 Plan 03: 端到端集成验证（D-08+D-09+init 模板+集成测试） Summary

## One-liner

`run_stats` 入口防御性验证（D-09）+ init 模板 [stats] 注释段 + 7 个端到端集成测试，闭合 Phase 53 全部 5 条 ROADMAP Success Criteria。

## What Was Built

### Task 1: src/config/validate.rs + src/stats/mod.rs（修改）

**validate.rs 新增 4 个单元测试：**
- `test_validate_rejects_invalid_stats_from`：cfg.stats.from="not-a-date"，断言错误含 stats.from + YYYY-MM-DD
- `test_validate_rejects_invalid_stats_to`：cfg.stats.to="20240101"，断言错误含 stats.to + YYYY-MM-DD
- `test_validate_accepts_valid_stats_time_strings`：from="2024-01-01", to="2024-01-31 23:59:59"，断言 Ok
- `test_validate_accepts_none_stats_time`：默认 Config（全 None），断言 Ok

**stats/mod.rs 新增 validate_cfg_stats_time 私有函数（D-09）：**
- 在 `run_stats` 入口的 `debug_assert!` 之后调用
- 对 cfg.stats.from/to 调用 config::validate_time_str
- 错误格式与 validate_stats_time_fields 相同：ConfigError::InvalidValue { field, value, reason }

**stats/mod.rs 新增 2 个单元测试：**
- `test_run_stats_rejects_invalid_from`：cfg.stats.from="bad"，断言 Err + field=="stats.from"
- `test_run_stats_rejects_invalid_to`：cfg.stats.to="20240101"，断言 Err + field=="stats.to"

### Task 2: src/cli/init.rs（修改）

在 `CONFIG_TEMPLATE_EN` 的 [filter.sql] 之后、[exporter.csv] 之前追加 [stats] 注释段：

```toml
# --- Stats subcommand time-range filter (optional) ---
[stats]
# from = "2024-01-01"   # Start of time range. Formats: "YYYY-MM-DD" or "YYYY-MM-DD HH:MM:SS"
# to   = "2024-01-31"   # End of time range. Same formats as from.
# top  = 20             # Default top-N. CLI --top overrides this value.
# CLI args --from / --to / --top override the values above. ...
```

生成的模板通过 `sqllog2db validate`（字段全为注释，不影响 serde 解析）。

### Task 3: tests/integration.rs（修改）

新增辅助函数 `make_stats_config_with_section(dir, from, to, top)` 和 7 个集成测试：

| 测试函数 | 覆盖 SC | 验证内容 |
|----------|---------|---------|
| test_cli_stats_help_shows_from_and_to | SC#1 | --help 含 --from/--to/YYYY-MM-DD |
| test_cli_stats_with_cli_from_and_to_succeeds | SC#1+STATS-07 | CLI 参数传入，日志含 from=Some |
| test_cli_stats_validate_accepts_valid_config_stats_section | SC#2+STATS-08 | validate 通过含 from/to 的 config |
| test_cli_stats_validate_rejects_bad_config_from_format | SC#4+STATS-11 | validate 拒绝 from="20240101" |
| test_cli_stats_cli_overrides_config_from | SC#3+STATS-09 | CLI from 覆盖 config from |
| test_cli_stats_runtime_rejects_bad_cli_from_format | SC#4+STATS-11 CLI | stats --from not-a-date 退出非零 |
| test_init_template_contains_stats_section | init 模板 | 生成文件含 [stats]/from/to/top/YYYY-MM-DD HH:MM:SS |

## Deviations from Plan

### 自动修复（Rule 1 - clippy 门禁）

**1. [Rule 1 - Clippy] doc_markdown lint：`run_stats` 需用反引号包裹**
- **Found during:** Task 1 clippy 验证
- **Fix:** 将注释中 `run_stats` 改为 `` `run_stats` ``
- **Commit:** c96ae09

**2. [Rule 1 - Clippy] format_push_string + write_with_newline lint**
- **Found during:** Task 3 pre-commit hook
- **Fix:** `push_str(&format!(...))` → `writeln!(stats_section, ...)` + 导入 `std::fmt::Write`
- **Commit:** 4714c3e（修复后重新提交）

### 偏差说明

**Plan 01 提前实现了 validate_stats_time_fields：**
- Task 1 的 RED 阶段（4 个新单元测试）立即通过，未经历真正的 RED 状态
- 这是 Plan 01 SUMMARY 中明确记录的提前实现，属于预期偏差
- Plan 03 在此基础上直接补充了 run_stats 防御（Task 1 新增内容）和集成测试

### Commit 策略（同 Plan 01/02）

Task 3 因 clippy 检查需两次提交（第一次 pre-commit 失败，修复后重新提交）。最终三个任务各自独立提交，符合 task_commit_protocol。

## Verification Results

```
cargo test --lib config::validate::     -- 26/26 通过（含 4 个新 stats 测试）
cargo test --lib stats::                -- 46/46 通过（含 2 个新 run_stats 测试）
cargo test --test integration -- stats  -- 18/18 通过（S1-S6 + Phase 52 + 7 个新测试）
cargo test --release（全量）            -- 全部通过（零回归）
cargo clippy --all-targets -- -D warnings  -- 通过（零警告）
cargo fmt --check                       -- 通过
sqllog2db init + grep [stats]           -- 通过
sqllog2db validate <valid config>       -- exit 0, "Configuration valid."
```

## ROADMAP Phase 53 Success Criteria 验收

| SC | 描述 | 覆盖方式 |
|----|------|---------|
| SC#1 | stats --help 含 --from/--to | test_cli_stats_help_shows_from_and_to |
| SC#2 | validate 通过含 from/to 的 config.toml | test_cli_stats_validate_accepts_valid_config_stats_section |
| SC#3 | CLI 优先于 config，二者缺省不过滤 | test_cli_stats_cli_overrides_config_from + Plan 02 S1-S6 |
| SC#4 | 格式不合法明确错误 | test_cli_stats_validate_rejects_bad_config_from_format + test_cli_stats_runtime_rejects_bad_cli_from_format |
| SC#5 | clippy + test 全通过 | cargo clippy + cargo test --release |

## Threat Flags

无新增网络端点、认证路径或文件访问模式。仅在内存中进行格式字符串验证，写入了模板注释内容。

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/config/validate.rs 含 test_validate_rejects_invalid_stats_from | FOUND |
| src/config/validate.rs 含 test_validate_accepts_valid_stats_time_strings | FOUND |
| src/stats/mod.rs 含 validate_cfg_stats_time | FOUND |
| src/stats/mod.rs 含 test_run_stats_rejects_invalid_from | FOUND |
| src/cli/init.rs 含 [stats] 注释段 | FOUND |
| tests/integration.rs 含 test_cli_stats_help_shows_from_and_to | FOUND |
| tests/integration.rs 含 test_init_template_contains_stats_section | FOUND |
| commit c96ae09 存在 | FOUND |
| commit ed672b0 存在 | FOUND |
| commit 4714c3e 存在 | FOUND |
