# Phase 49: Glob 输入支持 - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 49 在 config.toml 和 CLI 两处添加多路径 + glob 输入支持：

1. **config schema 变更（破坏性）**：`[sqllog] path = "..."` 移除，改为 `inputs = ["sqllogs/*.log", "other.log"]` 数组字段。发现旧 `path` 键时返回 ConfigError，提示用户迁移。
2. **CLI `--input` 标志**：`run` 子命令添加可重复的 `--input` 标志，优先于 config `inputs`。

本 phase 不引入重量级依赖（仍用已有的 `glob` crate），不改变路径展开逻辑（已在 `parser.rs::scan_glob()` 实现）。

</domain>

<decisions>
## Implementation Decisions

### config schema 变更（破坏性）
- **D-01:** `src/config/sqllog.rs` 中 `SqllogConfig` 字段从 `path: String` 改为 `inputs: Vec<String>`。默认值改为 `vec!["sqllogs".to_string()]`（兼容无配置时的默认行为）。
- **D-02:** 添加 `path_deprecated: Option<toml::Value>` 旧键检测字段，类似现有的 `pipeline_deprecated`。当检测到 `path` 字段时，`validate()` 返回：
  ```
  ConfigError::InvalidValue {
    field: "sqllog.path".to_string(),
    value: "<旧值>".to_string(),
    reason: "此字段已移除，请改用 inputs = [\"...\"]".to_string(),
  }
  ```
  hint 指导用户迁移到新格式。
- **D-03:** config 模板（`CONFIG_TEMPLATE_EN`）中 `[sqllog]` 段改为：
  ```toml
  [sqllog]
  # 输入路径列表，支持目录、单文件或 glob 模式（如 "sqllogs/*.log"）
  inputs = ["sqllogs"]
  ```

### CLI --input 标志
- **D-04:** `src/cli/opts.rs` 中 `Run` 子命令添加：
  ```rust
  #[arg(long = "input", short = 'i', action = clap::ArgAction::Append)]
  pub input: Option<Vec<String>>,
  ```
  可重复使用：`--input f1.log --input 'dir/*.log'`
- **D-05:** 优先级：`--input` 有值时完全覆盖 config 的 `inputs`，两者不合并。在 `main.rs` 或 `handle_run()` 入口处替换 `cfg.sqllog.inputs`。

### 路径展开逻辑
- **D-06:** `src/parser.rs::SqllogParser::new()` 改为接受 `Vec<String>` 而不是单个路径，或者在 `log_files()` 中遍历所有 inputs 并合并结果（dedup + sort）。具体接口由 planner 决定，但语义是：每个 input 条目独立展开（文件/目录/glob），最终合并为一个去重排序的文件列表。
- **D-07:** 无匹配时行为：如果所有 inputs 展开后总文件数为 0，且不是 stdin 管道模式，则返回错误而不是静默空输出。错误格式：
  ```
  [ERROR] No log files found matching inputs: ["sqllogs/*.log"]
    hint: Verify the glob pattern matches existing .log files in the current directory.
  ```

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 核心文件
- `src/config/sqllog.rs` — `SqllogConfig` 结构体（当前 `path: String`）
- `src/parser.rs` — `SqllogParser::new()` 和 `scan_glob()` 实现
- `src/cli/opts.rs` — `Run` 子命令定义
- `src/main.rs` — `handle_run()` 调用路径，config 字段的传递方式
- `src/config/mod.rs` — `pipeline_deprecated` 模式参考（`Option<toml::Value>` 旧键检测）

No external specs — requirements fully captured in decisions above

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/parser.rs::scan_glob()` — 已实现单路径 glob 展开，可复用为多路径遍历的内部函数
- `src/config/mod.rs::pipeline_deprecated` — 旧键检测模式的参考实现（`#[serde(rename = "pipeline", default)] pub pipeline_deprecated: Option<toml::Value>`）
- `glob` crate — 已在 `Cargo.toml` 中，无需新增依赖

### Established Patterns
- `SqllogParser::new(path)` → `log_files()` 的接口模式需要调整为支持多路径
- 无匹配 glob 时当前只 `warn!`（非错误），Phase 49 要求改为错误

### Integration Points
- `cfg.sqllog.path` → `cfg.sqllog.inputs` 的所有引用点：`cli/run/mod.rs`（`SqllogParser::new(&cfg.sqllog.path)`）和 `cli/run/prescan.rs`
- `--input` CLI 值需要在 `main.rs` 中注入到 `cfg.sqllog.inputs` 之前处理

</code_context>

<specifics>
## Specific Ideas

- config 格式：`inputs = ["sqllogs"]` 而不是 `inputs = ["sqllogs/*.log"]`（默认匹配目录）
- 旧 `path` 键检测错误的 hint 要提供具体迁移示例

</specifics>

<deferred>
## Deferred Ideas

无——讨论保持在 phase 边界内。

</deferred>

---

*Phase: 49-Glob 输入支持*
*Context gathered: 2026-05-31*
