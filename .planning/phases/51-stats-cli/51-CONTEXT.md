# Phase 51: stats 子命令 CLI 脚手架 - Context

**Gathered:** 2026-06-01
**Status:** Ready for planning

<domain>
## Phase Boundary

新增 `stats` 子命令，使用户可运行 `sqllog2db stats -c config.toml [--top N]`。CLI 参数正确解析并传递到后续处理逻辑（Phase 52 实现）。此阶段只构建 CLI 脚手架，不实现统计输出逻辑。

</domain>

<decisions>
## Implementation Decisions

### CLI 结构
- **D-01:** 在 `src/cli/opts.rs` 的 `Commands` enum 中新增 `Stats { config, top }` 变体，遵循现有 `Run`/`Init`/`Validate` 的模式。
- **D-02:** 参数：`-c/--config`（默认 `config.toml`，`env = "SQLLOG2DB_CONFIG"`），`--top N`（`u32`，默认 `20`）。`--top 0` 或负数给出明确错误提示。
- **D-03:** 新建 `src/cli/stats/mod.rs`（或 `src/cli/stats.rs`），`handle_stats` 函数负责编排：加载 config → 初始化日志 → 调用 Phase 52 统计逻辑 → 输出。

### Config 需求
- **D-04:** `stats` 读取**完整 config**（同 `run` 命令），复用 `load_config` 函数。读取 `[sqllog]`（输入路径）和 `[csv]`/`[sqlite]`（输出目标）。`[filter]` 节存在时静默忽略（stats 不使用过滤器）。
- **D-05:** config 文件不存在时**报错退出**（不回落默认配置）。理由：stats 需要知道读哪些文件、写到哪里，缺少 config 无法推断。

### 日志初始化
- **D-06:** `stats` 使用**完整日志栈**（`logging::init_logging`），支持 `--verbose`/`--quiet`（全局标志）。统计是流式处理操作，需要与 `run` 相同级别的可观察性。

### `--top` 验证
- **D-07:** `--top 0` 视为无效输入，在 CLI 层（clap validator 或 handle_stats 入口处）给出明确错误提示后退出，不继续处理。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 51: stats 子命令 CLI 脚手架" — Goal、Success Criteria（4 条）
- `.planning/REQUIREMENTS.md` §STATS-01、STATS-02

### 现有 CLI 模式（必读，以保持一致性）
- `src/cli/opts.rs` — 现有子命令结构（Run/Init/Validate 的参数定义方式）
- `src/main.rs` — 子命令分发逻辑（`match &cli.command`），日志初始化方式
- `src/cli/run/mod.rs` — handle_run 函数签名和编排模式

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `load_config(config_path: &str) -> Result<Config>`（`src/main.rs`）：直接复用
- `init_simple_logging(quiet: bool)`（`src/main.rs`）：**不**用于 stats，改用完整日志栈
- `logging::init_logging(&cfg.logging, false)` 模式：复用于 stats
- `apply_verbosity_to_config(&mut cfg, verbose, quiet)`：复用于 stats

### Established Patterns
- `clap` `#[command]` + `#[arg]`，`env = "SQLLOG2DB_CONFIG"`，`default_value`
- `after_help` 中提供使用示例（参照 `Run` 变体的格式）
- `--verbose`/`--quiet` 是全局标志（`global = true`），stats 自动继承

### Integration Points
- `main.rs` 的 `match &cli.command` 中增加 `Commands::Stats { .. }` 分支
- Phase 52 的 `handle_stats_output` 函数将在 `src/cli/stats/` 中调用

</code_context>

<specifics>
## Specific Ideas

- `stats --help` 的 after_help 示例参照 `Run` 命令格式（显示配置文件路径用法）

</specifics>

<deferred>
## Deferred Ideas

None — 讨论始终在阶段范围内。

</deferred>

---

*Phase: 51-stats 子命令 CLI 脚手架*
*Context gathered: 2026-06-01*
