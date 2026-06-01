---
phase: 47-config
fixed_at: 2026-06-01T00:00:00Z
review_path: .planning/phases/47-config/47-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 7
skipped: 1
status: partial
---

# Phase 47: Code Review Fix Report

**Fixed at:** 2026-06-01T00:00:00Z
**Source review:** .planning/phases/47-config/47-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8
- Fixed: 7
- Skipped: 1

## Fixed Issues

### CR-01: Config::from_file 将所有 IO 错误均映射为 NotFound

**Files modified:** `src/config/mod.rs`
**Commit:** edf882c
**Applied fix:** 添加 `use std::io`，将 `map_err(|_| ...)` 改为检查 `e.kind() == io::ErrorKind::NotFound`。文件不存在时映射为 `ConfigError::NotFound`，其他 IO 错误（权限拒绝、路径为目录等）映射为 `Error::Io(e)`，防止 `load_config` 对权限错误静默降级。

### CR-02: apply_verbosity_to_config 不设置 verbose → debug

**Files modified:** `src/main.rs`
**Commit:** 27e233a
**Applied fix:** Phase 46 移除了 verbose 参数导致 verbose 时文件日志级别不被更新。恢复 `verbose: bool` 参数：verbose=true 设置 `logging.level = "debug"`，quiet=true 设置 `"error"`，两者都不时保持配置值不变。更新调用处传入 `cli.verbose`，并添加 `test_apply_verbosity_verbose_sets_debug` 测试。注：当前 CLI 中 verbose 是 bool 类型（不是 u8），所以 `-vv` 不存在，fix 用 bool 语义。

### WR-01: handle_validate 集成测试注释描述不存在的分支

**Files modified:** `tests/integration.rs`
**Commit:** 8d4fb29
**Applied fix:** 将 6 处 "hits X branch" 注释替换为准确描述 "validate called without panic (context)"。handle_validate 实际只打印 inputs 列表和 "Configuration valid."，没有任何分支逻辑。

### WR-02: test_init_generated_zh_template_passes_validate 是重复测试

**Files modified:** `tests/integration.rs`
**Commit:** e06eef3
**Applied fix:** 删除 `test_init_generated_zh_template_passes_validate` 函数。该测试与 `test_init_generated_en_template_passes_validate` 逐字逐行相同，仅名称和断言消息中的 ZH/EN 不同。代码库中只有一份 `CONFIG_TEMPLATE_EN`，不存在中文模板。

### WR-03: None 命令分支退出逻辑错误

**Files modified:** `src/main.rs`
**Commit:** ab08209
**Applied fix:** 将 `try_parse_from(["sqllog2db", "--help"])` + `exit(1)` 替换为 `Cli::command().print_help().ok()` + `exit(0)`。前者 exit code 1 是死代码（try_parse_from 已以 0 退出），后者明确且正确。同时移除不再需要的 `Parser` import。

### IN-02: init.rs 中双重 path.exists() 检查

**Files modified:** `src/cli/init.rs`
**Commit:** 09fcfa7
**Applied fix:** 在函数开始处捕获 `let file_existed = path.exists()`，两处条件检查改用 `file_existed`。同时修正最后的日志消息：改为基于 `file_existed` 判断 "overwritten" vs "generated"，而不是基于 `force`（原来用 `force=true` 时即使文件不存在也会说 "overwritten"）。

### IN-03: 重复的 init 模板测试

**Files modified:** `tests/integration.rs`
**Commit:** b0d32c1
**Applied fix:** 将 `test_handle_init_en_template` 和 `test_handle_init_template_is_english` 合并为一个综合测试 `test_handle_init_en_template`，保留并扩展所有断言：[sqllog] 存在、"SQL log path" 存在、"log path" 存在、不含 "日志路径"。

## Skipped Issues

### IN-01: 遗留 TODO 注释出现在用户可见的帮助文本中

**File:** `src/cli/opts.rs:52`
**Reason:** skipped: code context differs from review
**Original issue:** REVIEW.md 描述的是 TODO 注释和 `--input -` 示例（`Pipe log data via stdin (requires --input flag): sqllog2db run -c config.toml --input -`）出现在帮助文本中。但当前 opts.rs 中不含任何 TODO 注释，第 53-54 行是 `Pipe log data via stdin: / cat access.log | sqllog2db run -c config.toml`，这描述的是已实现的 Unix stdin pipe 功能（无需 `--input` flag），不是未实现的 `--input -` 功能。该问题已在早期 phase 中被修复，当前状态正确无需改动。

---

_Fixed: 2026-06-01T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
