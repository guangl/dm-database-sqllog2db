# Phase 57: e2e 测试扩展 - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

为 run/init/stats 子命令补全 CLI 全链路 assert_cmd 测试，涵盖正常路径、退出码、边界条件，为 Phase 58 的 cli/run 重构提供安全网。

同时包含一个小代码改动：在 `validate_stats_time_range` 加入 from > to 的跨字段验证（测试先于代码验证场景需要此改动）。

**不包括**：cli/run 函数拆分（Phase 58）、CI/CD 流程（Phase 59+）

</domain>

<decisions>
## Implementation Decisions

### from > to 验证逻辑（TEST-03 前提）

- **D-01:** 当 `stats.from` 晚于 `stats.to` 时，报错并非零退出——在 `src/stats/config.rs` 的 `validate_stats_time_range` 函数中加入跨字段字符串比较（YYYY-MM-DD 字典序 == 日期序，字符串比较可行）
- **D-02:** 错误格式遵循已有的 `ConfigError::InvalidValue` 模式，错误信息包含字段名 + 具体值，例如：`"stats.from (2024-01-31) must be <= stats.to (2024-01-01)"`
- **D-03:** 验证在 `validate_stats_time_range` 中执行（运行时立即报错），与现有 `validate_stats_time_range` 调用点（`Config::validate` 和 `run_stats`）保持一致

### run CLI 测试的 fixture 策略（TEST-01）

- **D-04:** 新增辅助函数 `write_run_config_toml(dir, log_dir, output_path) -> PathBuf`，风格参考 `make_stats_csv_config()`，生成临时 config.toml 文件供 assert_cmd CLI 测试使用
- **D-05:** CSV 内容验证层次：header 行完整匹配 `"ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql"` + 记录行数正确
- **D-06:** 测试数据复用现有 `write_test_log()` helper（生成真实格式的达梦 SQL 日志行）

### SQLite run 测试的验证深度（TEST-01）

- **D-07:** SQLite 输出验证：文件存在 + 用 rusqlite 查询 `sqllog` 表记录数等于写入行数（rusqlite 已是项目依赖，无需新 dep）

### init CLI 测试（TEST-02）

- **D-08:** 新增两个 assert_cmd 测试：
  1. `sqllog2db init -o <新路径>` 成功创建文件，exit 0
  2. 文件已存在 + 不加 `--force`，exit 非零 + stderr 包含错误信息
  （参考 `test_init_template_contains_stats_section` 的 assert_cmd 风格）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求定义
- `.planning/ROADMAP.md` §"Phase 57: e2e 测试扩展" — 成功标准（4 条）、requirements 映射（TEST-01, TEST-02, TEST-03）
- `.planning/REQUIREMENTS.md` §TEST-01, TEST-02, TEST-03 — 完整需求描述

### 待新增验证逻辑（必读）
- `src/stats/config.rs` — `validate_stats_time_range` 函数（第 17 行起）：需要在这里加 from ≤ to 跨字段检查；现有 `ConfigError::InvalidValue` 用法是改动参考
- `src/error.rs` — `ConfigError::InvalidValue` 变体定义，确认字段签名

### 现有测试基础设施（必读，测试风格参考）
- `tests/integration.rs` — 全部现有集成测试；assert_cmd 风格参考：`test_init_template_contains_stats_section`（行 1838）、`test_stats_from_to_filters_to_single_day`（行 1878）、`test_cli_stats_runtime_rejects_bad_cli_from_format`（行 1818）
- `tests/integration.rs` 行 1464 — `make_stats_csv_config()` 辅助函数，是新 `write_run_config_toml()` 的风格参考
- `tests/integration.rs` 行 16 — `write_test_log()` helper，复用生成测试日志数据

### 已覆盖（无需重写，仅供了解）
- `tests/integration.rs` 行 1878 — `test_stats_from_to_filters_to_single_day`（from == to 边界值 ✓）
- `tests/integration.rs` 行 1818 — `test_cli_stats_runtime_rejects_bad_cli_from_format`（无效日期格式 ✓）

</canonical_refs>

<code_context>
## Existing Code Insights

### 可复用资产
- `write_test_log(path, count)` — 生成真实格式的达梦 SQL 日志行，CLI 测试的输入数据来源
- `make_stats_csv_config(dir, log_path)` — 生成临时 config.toml 文件的风格模板，新 `write_run_config_toml()` 仿照此模式
- `assert_cmd::Command::cargo_bin("sqllog2db")` — 所有 CLI 测试的统一入口，已在现有测试中大量使用

### 改动目标
- `src/stats/config.rs:validate_stats_time_range`（行 17 起）— 加 from ≤ to 检查，新增一个 `ConfigError::InvalidValue` 返回路径
- `tests/integration.rs` — 新增约 5 个测试函数：2 个 run CLI（CSV + SQLite）、2 个 init CLI、1 个 stats from > to

### 现有字段顺序
- CSV header（无字段投影时）：`ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql`（来自 `src/pipeline/mod.rs:FIELD_NAMES`）

### 集成点
- rusqlite 已在 Cargo.toml 依赖中（SQLite exporter 使用），测试可直接 `use rusqlite::Connection`

</code_context>

<specifics>
## Specific Ideas

- D-02 错误信息示例：`"stats.from (2024-01-31) must be <= stats.to (2024-01-01)"`
- SQLite 验证：打开数据库，`SELECT COUNT(*) FROM sqllog` 应等于 `write_test_log()` 写入的行数 N
- init 测试的错误信息验证：用 `predicates::str::contains` 匹配 stderr 中包含 "already exists" 或类似提示（与 `test_cli_error_uses_hint_prefix` 风格一致）

</specifics>

<deferred>
## Deferred Ideas

- from > to 影响退出码（退出码 1 vs 2）细化 → 遵循现有的 `ConfigError` 映射规则，Phase 57 不改变退出码策略
- run CLI 测试的多平台矩阵（Windows + Linux）→ v1.15 后续 CI 阶段（REQUIREMENTS.md 已标记为 Future）

</deferred>

---

*Phase: 57-e2e 测试扩展*
*Context gathered: 2026-06-02*
