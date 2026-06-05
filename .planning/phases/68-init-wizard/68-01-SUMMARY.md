---
phase: 68-init-wizard
plan: "01"
subsystem: cli/init
tags: [wizard, interactive, config-generation, unit-tests]
dependency_graph:
  requires: []
  provides:
    - ExporterChoice (pub enum)
    - WizardAnswers (pub struct)
    - run_wizard (pub fn)
    - apply_wizard_answers_to_template (private fn)
    - write_config_file (private fn)
    - handle_init_interactive (pub fn)
  affects:
    - src/cli/init.rs
    - src/cli/opts.rs
    - src/main.rs
    - tests/integration.rs
tech_stack:
  added: []
  patterns:
    - "prompt_line/ask_exporter/build_*_answers helper decomposition (≤40 lines per fn)"
    - "str::replace chain for CONFIG_TEMPLATE_EN substitution"
    - "BufRead + Write generics for testable IO injection"
key_files:
  created: []
  modified:
    - src/cli/init.rs
    - src/cli/opts.rs
    - src/main.rs
    - tests/integration.rs
decisions:
  - "run_wizard decomposed into 4 private helpers (prompt_line, ask_exporter, build_csv_answers, build_sqlite_answers) to satisfy ≤40 lines per fn constraint"
  - "apply_sqlite_substitutions uses 4 precise str::replace calls for CSV section commenting to avoid regex dependency"
  - "handle_init_interactive and --interactive flag added in Plan 01 (instead of Plan 02) to resolve dead_code clippy warnings from binary target"
  - "Integration tests in tests/integration.rs reference run_wizard/ExporterChoice to satisfy library's public API usage requirement"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-06"
  tasks_completed: 3
  files_modified: 4
---

# Phase 68 Plan 01: Wizard Core Logic Summary

向导核心算法（IO 注入模式）+ 模板替换 + 文件写入复用，共新增 12 个单元测试，3 道质量门禁全绿。

## New Public API

| Signature | Location |
|-----------|----------|
| `pub enum ExporterChoice { Csv, Sqlite }` (Debug+PartialEq+Eq) | src/cli/init.rs |
| `pub struct WizardAnswers { inputs, exporter, csv_file, sqlite_db, sqlite_table }` (Debug+PartialEq+Eq) | src/cli/init.rs |
| `pub fn run_wizard(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<WizardAnswers>` | src/cli/init.rs |
| `pub fn handle_init_interactive(output_path: &str, force: bool) -> Result<()>` | src/cli/init.rs |

## Private Functions Added

| Signature | Purpose |
|-----------|---------|
| `fn write_config_file(path: &Path, content: &str, force: bool) -> Result<()>` | 文件写入（Task 1 重构） |
| `fn apply_wizard_answers_to_template(answers: &WizardAnswers) -> String` | 模板替换入口 |
| `fn apply_csv_substitutions(content: &str, answers: &WizardAnswers) -> String` | CSV 路径替换 |
| `fn apply_sqlite_substitutions(content: &str, answers: &WizardAnswers) -> String` | SQLite 段激活 |
| `fn prompt_line(reader, writer, prompt, default, buf) -> Result<String>` | 单步提示辅助 |
| `fn ask_exporter(reader, writer, buf) -> Result<ExporterChoice>` | 格式选择（≤3次重试） |
| `fn build_csv_answers(reader, writer, inputs, buf) -> Result<WizardAnswers>` | CSV 路径分支 |
| `fn build_sqlite_answers(reader, writer, inputs, buf) -> Result<WizardAnswers>` | SQLite 路径分支 |

## Refactored Function Sizes

| Function | Line Count (incl. signature + closing brace) |
|----------|----------------------------------------------|
| `handle_init` | 10 |
| `write_config_file` | 37 |
| `run_wizard` | 15 |
| `apply_wizard_answers_to_template` | 10 |
| `apply_csv_substitutions` | 7 |
| `apply_sqlite_substitutions` | 34 |

All functions satisfy the ≤40 lines constraint from CLAUDE.md.

## Unit Tests Added (12 new)

### Wizard tests (src/cli/init.rs)
1. `test_wizard_all_defaults` — Enter×3 返回全默认值 CSV 路径
2. `test_wizard_custom_csv_path` — 自定义 inputs 和 csv 路径
3. `test_wizard_sqlite_path` — sqlite 模式写入 sqlite_db 和 sqlite_table
4. `test_wizard_sqlite_defaults` — sqlite 模式全默认
5. `test_wizard_invalid_format_three_times_returns_err` — 3 次无效格式返回 ConfigError::InvalidValue { field: "exporter" }
6. `test_wizard_writer_receives_prompts` — writer 收到 3 段提示文本

### Apply tests (src/cli/init.rs)
7. `test_apply_csv_default` — 默认值时输出与 CONFIG_TEMPLATE_EN 完全相同（零副作用）
8. `test_apply_csv_custom` — 自定义 inputs/csv_file 正确替换，[exporter.sqlite] 段保持注释
9. `test_apply_sqlite` — SQLite 模式激活 [exporter.sqlite]，注释掉 [exporter.csv]，database_url/table_name 替换
10. `test_apply_does_not_corrupt_logging_file` — CSV 和 SQLite 模式都不破坏 logging.file 行
11. `test_apply_output_parses_as_config_csv` — CSV 输出通过 toml::from_str + Config::validate
12. `test_apply_output_parses_as_config_sqlite` — SQLite 输出通过 toml::from_str + Config::validate

### Integration tests added (tests/integration.rs)
13. `test_wizard_integration_all_defaults` — 集成路径验证全默认
14. `test_wizard_integration_sqlite` — 集成路径验证 sqlite

## Template Substitution Edge Cases

**Pitfall 1 — logging.file 不被误替换：** 搜索键使用完整字符串 `file = "outputs/sqllog.csv"`（唯一），而非 `file =` 前缀，避免误改 `file = "logs/sqllog2db.log"`。

**Pitfall 4 — SQLite 段激活精确格式：** `apply_sqlite_substitutions` 使用 4 次精确 `.replace()` 链注释 CSV 段，再用 4 次精确替换激活 SQLite 段。`# overwrite = true\n# append = false` → `overwrite = true\nappend = false` 仅在 SQLite 段尾部出现，不与 CSV 段冲突。

**Dead code 解决：** binary 目标需要引用 `run_wizard`/`ExporterChoice`/`WizardAnswers`，通过提前添加 `handle_init_interactive` + opts.rs `--interactive` flag + main.rs dispatch 分支（原计划 Plan 02）解决。功能实现完整，Plan 02 无需再重复添加这些 flag 改动。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - 补全缺失集成] 提前实现 opts.rs/main.rs 的 --interactive flag dispatch**
- **Found during:** Task 2
- **Issue:** binary 目标对 pub ExporterChoice/WizardAnswers/run_wizard 报 dead_code，cargo clippy --all-targets -- -D warnings 失败
- **Fix:** 在 opts.rs 新增 `interactive: bool` flag，在 main.rs 中添加 if *interactive { handle_init_interactive } else { handle_init } dispatch，在 init.rs 中实现 handle_init_interactive
- **Files modified:** src/cli/opts.rs, src/main.rs, src/cli/init.rs
- **Commits:** 263d536

**2. [Rule 1 - Bug] run_wizard 超过 40 行限制**
- **Found during:** Task 2 验证
- **Issue:** 首次实现 run_wizard 函数体达 50+ 行，违反 CLAUDE.md ≤40 行规则
- **Fix:** 提取 prompt_line、ask_exporter、build_csv_answers、build_sqlite_answers 4 个私有辅助函数
- **Files modified:** src/cli/init.rs
- **Commits:** 263d536

**3. [Rule 1 - Bug] apply_csv_substitutions/apply_sqlite_substitutions 参数类型**
- **Found during:** Task 3 clippy
- **Issue:** `content: String` 参数 clippy 建议改为 `&str`（needless_pass_by_value）
- **Fix:** 改为 `content: &str`，调用处加 `&` 引用
- **Files modified:** src/cli/init.rs
- **Commits:** 8fe0f1f

## Commits

| Hash | Message |
|------|---------|
| d784e67 | refactor(68-01): extract write_config_file from handle_init |
| 263d536 | feat(68-01): add WizardAnswers, ExporterChoice, run_wizard with 6 unit tests |
| 8fe0f1f | feat(68-01): add apply_wizard_answers_to_template with 6 template tests |

## Quality Gates

- `cargo test --lib wizard`: 6 passed
- `cargo test --lib apply_`: 6 passed (20 total including normalizer tests)
- `cargo clippy --all-targets -- -D warnings`: 0 warnings
- `cargo fmt --check`: 0 diff

## Self-Check: PASSED

- src/cli/init.rs: FOUND
- Commits d784e67, 263d536, 8fe0f1f: FOUND
- 12 test functions in src/cli/init.rs: FOUND (grep -c "fn test_wizard_\|fn test_apply_" = 12)
