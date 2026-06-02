# Phase 57: e2e 测试扩展 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 57-e2e 测试扩展
**Areas discussed:** from > to 验证逻辑, run CLI fixture 策略, SQLite 验证深度

---

## from > to 验证逻辑

| Option | Description | Selected |
|--------|-------------|----------|
| 报错，非零退出码 | 在 validate_stats_time_range 加跨字段比较，输出明确错误信息 | ✓ |
| 静默返回 0 条结果 | 不改代码，仅写测试验证当前静默行为 | |

**User's choice:** 报错，非零退出码

| 错误信息格式 | Description | Selected |
|-------------|-------------|----------|
| 字段名 + 具体值 + 格式提示 | 例如："stats.from (2024-01-31) must be <= stats.to (2024-01-01)"，与 ConfigError::InvalidValue 一致 | ✓ |
| 简短说明 | "--from 必须早于或等于 --to" | |

**User's choice:** 字段名 + 具体值 + 格式提示

---

## run CLI fixture 策略

| Option | Description | Selected |
|--------|-------------|----------|
| 辅助函数 write_run_config_toml() | 参考 make_stats_csv_config() 风格，多个测试可复用 | ✓ |
| 内联 TOML 字符串 | 每个测试直接内联，但 CSV + SQLite 两个测试会重复 | |

**User's choice:** 辅助函数 write_run_config_toml()

| CSV 验证层次 | Description | Selected |
|------------|-------------|----------|
| header 关键字段 + 行数 | header 包含 "ts" "sql" "exec_time_ms" + 行数正确 | |
| 全部 15 个字段名完整匹配 | header == "ts,ep,sess_id,...,normalized_sql" | ✓ |
| 仅验证行数 | 最简单，与现有 test_handle_run_real_csv_export 一致 | |

**User's choice:** 全部 15 个字段名完整匹配

---

## SQLite 验证深度

| Option | Description | Selected |
|--------|-------------|----------|
| 文件存在 + 退出码 0 | 成功标准原文描述 | |
| 文件存在 + 查询记录数 | 打开 SQLite 验证表记录数等于写入行数 | ✓ |

**User's choice:** 文件存在 + 查询记录数（用已有的 rusqlite dep，无需新依赖）

---

## Claude's Discretion

- init CLI 测试的错误信息具体措辞（stderr contains "already exists" 或类似提示）——与现有 `hint:` 前缀风格保持一致即可

## Deferred Ideas

- from > to 影响退出码细化 → 遵循现有 ConfigError 映射规则，不另立新策略
- run CLI 测试的多平台矩阵（Windows + Linux）→ v1.15 后续 CI 阶段
