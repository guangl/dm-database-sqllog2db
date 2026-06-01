---
phase: 49-glob
plan: 2
subsystem: parser
tags: [parser, config, cli, error-handling, glob, multi-input]

# Dependency graph
requires: [49-01]
provides:
  - "SqllogParser.inputs: Vec<String> 多输入接口，log_files() 合并去重排序"
  - "handle_run 在非 stdin 空列表场景返回 Err(NoFilesFound)"
  - "preflight::check 遍历所有 cfg.sqllog.inputs"
  - "cli::validate::handle_validate 打印 inputs 列表"
  - "test_validate_rejects_legacy_sqllog_path_key 测试验证旧 path 键被拒"
affects:
  - "49-03: e2e 测试可直接验证 NoFilesFound 的 stderr 输出与 hint: 前缀"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SqllogParser 改为关联函数 expand_single/scan_glob（无 &self），通过多 input 遍历实现合并"
    - "handle_run 空列表分支从 warn+Ok 改为 Err(NoFilesFound)，保留 stdin pipe fallback 顺序"

key-files:
  created: []
  modified:
    - "src/parser.rs — SqllogParser 完整重写为 inputs: Vec<String>，2 个新多输入测试"
    - "src/cli/run/mod.rs — inputs.clone() 传入 parser，NoFilesFound 错误路径"
    - "src/preflight.rs — for loop 遍历 inputs，错误提示文本更新"
    - "src/cli/validate.rs — 打印 inputs 列表（枚举索引）"
    - "src/config/validate.rs — 重命名测试 + 新增 test_validate_rejects_legacy_sqllog_path_key"
    - "src/error.rs — 移除 #[allow(dead_code)]（NoFilesFound 现在被构造）"

key-decisions:
  - "expand_single 和 scan_glob 实现为关联函数而非 &self 方法，因为不再依赖 self.path"
  - "Task 1 的 cli/run/mod.rs 和 preflight.rs 临时调用方修复合并进 Task 1 提交（Rule 3 blocking fix），Task 2 再做完整迁移"

patterns-established:
  - "多输入合并：各 input 独立 expand_single，结果 append → sort → dedup"
  - "NoFilesFound 触发层：log_files() 只返回 Ok(空 Vec)，handle_run 决定语义"

requirements-completed: [INPUT-01]

# Metrics
duration: 25min
completed: 2026-06-01
---

# Phase 49 Plan 02: Parser 服务层迁移 Summary

**SqllogParser 改为 inputs: Vec<String> 多输入接口；handle_run 在非 stdin 空列表场景返回 Err(NoFilesFound)；所有调用方适配 cfg.sqllog.inputs**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-01T04:00:00Z
- **Completed:** 2026-06-01T04:25:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- `SqllogParser` 从 `path: PathBuf` 改造为 `inputs: Vec<String>`；`new()` 接受 owned `Vec<String>`；`log_files()` 遍历所有 inputs → `expand_single()` → 合并 → `sort()` → `dedup()`
- `expand_single` 和 `scan_glob` 提取为关联函数（无 `&self`），保留所有现有 glob/文件/目录/错误分类逻辑
- 新增 `test_log_files_multi_input_merge_and_dedup`（去重验证）和 `test_log_files_multi_input_mixes_file_dir_glob`（混合模式），连同原有 9 个测试共 11 个全通过
- `handle_run` 空文件列表分支从 `warn! + Ok(default)` 改为 `Err(NoFilesFound { inputs: cfg.sqllog.inputs.clone() })`，stdin pipe fallback 顺序保留
- `preflight::check` 从单路径改为 `for input in &cfg.sqllog.inputs` 循环，错误提示文本更新为 `[sqllog].inputs` 引用
- `cli::validate::handle_validate` 打印 inputs 条目数及各 input
- `config/validate.rs` 重命名 `test_validate_empty_sqllog_directory` → `test_validate_rejects_whitespace_input_entry`，新增 `test_validate_rejects_legacy_sqllog_path_key` 验证旧 `path = "..."` 键被 validate() 拒绝
- `error.rs` 移除 `#[allow(dead_code)]`（NoFilesFound 现在在 handle_run 中被构造）

## Task Commits

1. **Task 1: SqllogParser 改为多输入接口 + log_files 合并去重** — `772d27d` (feat)
2. **Task 2: 调用方迁移到 cfg.sqllog.inputs + handle_run 空列表抛 NoFilesFound** — `a452cdd` (feat)

## Files Created/Modified

- `src/parser.rs` — SqllogParser 完整重写，11 个单元测试（9 旧 + 2 新）
- `src/cli/run/mod.rs` — inputs.clone() 传入 parser，NoFilesFound 错误路径替代 warn+Ok
- `src/preflight.rs` — for loop 遍历 inputs，错误提示文本更新
- `src/cli/validate.rs` — 打印 inputs 列表
- `src/config/validate.rs` — 测试重命名 + 新增 test_validate_rejects_legacy_sqllog_path_key
- `src/error.rs` — 移除 #[allow(dead_code)]

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1 中同步修复调用方编译错误**
- **Found during:** Task 1 修改 parser.rs 后，cli/run/mod.rs 和 preflight.rs 立即编译失败（旧 `&first_input` 类型不匹配 `Vec<String>`）
- **Fix:** 在 Task 1 commit 中同时修复两个调用方的基础类型传递；Task 2 再做完整的语义迁移（NoFilesFound、循环、错误文本更新）
- **Files modified:** `src/cli/run/mod.rs`、`src/preflight.rs`（在 Task 1 commit 中）
- **Verification:** `cargo test --lib` 226 个测试全部通过

## 关键产出（供 Plan 03 引用）

### expand_single 是关联函数

```rust
fn expand_single(input: &str) -> Result<Vec<PathBuf>>
```

不依赖 `&self`，直接通过字符串参数驱动。Plan 03 的端到端测试若需要直接调用解析器，可参考此接口。

### handle_run NoFilesFound 错误的 stderr 格式

stderr 的格式由 `main.rs` 的错误处理层产生，结合 `Error::severity()` = `Warning` 以及 `Error::suggestion()`：

```
[WARNING] No log files found matching inputs: ["sqllogs"]
hint: Verify the glob/path entries exist; ensure patterns match .log files in the current directory.
```

（与 Plan 01 SUMMARY 确认的格式一致）

### preflight 错误提示文本最终形态

```
日志路径不存在: {path_str}  (检查 [sqllog].inputs 或 --input 标志)
```

（从 `--set sqllog.path=<path>` 更新为 `[sqllog].inputs 或 --input`）

### 测试覆盖总数

- lib 单元测试：226 个（全部通过）
- parser 模块：11 个（含 2 个新多输入测试）
- config::validate：包含 test_validate_rejects_legacy_sqllog_path_key

## Self-Check: PASSED

- FOUND: src/parser.rs
- FOUND: src/cli/run/mod.rs
- FOUND: src/preflight.rs
- FOUND: src/cli/validate.rs
- FOUND: src/config/validate.rs
- FOUND: src/error.rs
- FOUND commit: 772d27d
- FOUND commit: a452cdd
