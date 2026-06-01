# Phase 47: 配置文件体验 - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 47 有两个独立目标：

1. **`init` 模板**：`sqllog2db init -o config.toml` 生成的文件已有行内注释（`CONFIG_TEMPLATE_EN`），检查是否已满足 SC1，如不足则补充缺失字段的注释。
2. **`validate` 输出**：将当前走日志系统（`log::info!`）的 validate 展示改为结构化用户输出——全部通过时只显示一行 `Configuration valid.`，有失败时逐项输出 `[FAIL] <item>: <reason>`。

本 phase 不改变配置文件格式（TOML schema），不新增验证逻辑，不改变错误类型。

</domain>

<decisions>
## Implementation Decisions

### validate 输出机制
- **D-01:** 在 `src/logging.rs` 的 `SimpleLogger` 中添加对特定 log target（如 `validate_result`）的特殊格式处理：当 record.target() == `"validate_result"` 时，直接写 `[OK] ...` 或 `[FAIL] ...` 而不带时间戳前缀。在 `handle_validate()` 中用 `log::info!(target: "validate_result", ...)` 调用。
  
  **或者**（等效简化方案，更直接）：直接在 `handle_validate()` 中用 `println!` / `eprintln!` 输出结构化结果，不走日志路由。两种方式等效，planner 选简单实现。

### validate 展示粒度
- **D-02:** 静默通过策略：全部校验项通过时只输出单行：
  ```
  Configuration valid.
  ```
  有失败项时逐项输出失败原因：
  ```
  [FAIL] logging.level: 'verbose' — 无效值，合法值: trace, debug, info, warn, error
    hint: 将 logging.level 改为上述合法值之一
  ```
- **D-03:** 不输出 `[OK]` 条目——成功是静默的，只有失败需要可见。

### init 模板
- **D-04:** 检查 `CONFIG_TEMPLATE_EN` 中每个配置字段是否有行内注释。当前已有注释的字段保持不变，缺失注释的字段（如 `exporter.csv.append`、`exporter.sqlite.*`）补充说明用途和合法值示例。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 核心文件
- `src/cli/validate.rs` — `handle_validate()` 完整实现（当前用 log::info!）
- `src/cli/init.rs` — `CONFIG_TEMPLATE_EN` 常量（当前注释状态）
- `src/logging.rs` — `SimpleLogger` 实现（若选择走日志路由方案）
- `src/main.rs` — validate 子命令的调用路径（`logging::init_logging(..., true)`）

No external specs — requirements fully captured in decisions above

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/cli/validate.rs::handle_validate()` — 已遍历所有 Config 字段，只需改输出方式
- `src/cli/init.rs::CONFIG_TEMPLATE_EN` — 当前注释已覆盖大部分字段，需检查缺漏

### Established Patterns
- validate 子命令在 `main.rs` 中以 `logging::init_logging(&cfg.logging, true)` 初始化日志（log_to_stdout=true），所以 log::info! 确实会到 stdout，但格式含时间戳前缀不适合用户阅读

### Integration Points
- `handle_validate()` 在 config 已通过 `cfg.validate()` 之后调用，此时 config 字段保证有效
- validate 检查内容：sqllog.path、logging.*、filter.*、exporter.csv/sqlite

</code_context>

<specifics>
## Specific Ideas

- 成功只输出 `Configuration valid.` 一行，简洁
- 失败项格式参考 Phase 46 的 hint 风格：`[FAIL] field: reason \n  hint: ...`

</specifics>

<deferred>
## Deferred Ideas

无——讨论保持在 phase 边界内。

</deferred>

---

*Phase: 47-配置文件体验*
*Context gathered: 2026-05-31*
