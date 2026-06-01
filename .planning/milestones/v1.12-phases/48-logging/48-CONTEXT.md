# Phase 48: 日志级别与运行提示 - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 48 改造 `-v`/`-q` 标志的运行时行为：将 `-v` 重新定位为控制运行时展示内容（per-file 输出）的 `--verbose` 布尔标志，移除其现有的日志级别映射（debug/trace）功能；`--quiet` 通过在创建时跳过 ProgressBar 实例化来完全抑制进度条和运行摘要。

本 phase 不改变三级退出码逻辑，不改变日志文件输出机制，不引入 MultiProgress。

</domain>

<decisions>
## Implementation Decisions

### --verbose 标志重新定位
- **D-01:** 将 `opts.rs` 中 `-v` 的 `clap::ArgAction::Count` 改为普通布尔标志，从 `pub verbose: u8` 改为 `pub verbose: bool`。长标志名 `--verbose` 通过 `#[arg(long = "verbose")]` 显式设置。
- **D-02:** 移除 `main.rs::apply_verbosity_to_config()` 中 `verbose >= 1 → "debug"` 的日志级别映射。这个函数可能可以简化或删除（由 planner 评估）。
- **D-03:** `--verbose` 模式下，`handle_run()` 中每个文件开始处理时向 stderr 输出：
  ```
  Processing: sqllogs/2026-01-01.log
  ```
  过滤器匹配详情（每条匹配/跳过记录的原因）的输出粒度由 planner 根据性能影响决定（可能只输出文件级别，不输出每条记录级别）。

### verbose 与进度条的交互
- **D-04:** `--verbose` 时完全不实例化 ProgressBar。改用逐行 `eprintln!` 输出每个处理中的文件名。两者不共存，避免 stderr 内容互相覆盖。

### --quiet 模式
- **D-05:** `quiet: bool` 逻辑保持，但进度条处理改为：当 `quiet=true` 时，`handle_run()` 中不创建 `ProgressBar` 实例（不调用 `ProgressBar::new()`），完全跳过进度条相关代码路径。
- **D-06:** `--quiet` 同时抑制运行结束的摘要（`eprintln!("Completed with {N} error(s)...")`）——这部分摘要在 `main.rs` 而不在 `handle_run()` 内，需确认 quiet 信号能传到那里。

### 摘要内容差异化
- **D-07:** 默认模式摘要：`Processed {N} records in {t}s ({rate} records/sec). {errors} errors.`
  verbose 模式摘要：额外包含每个文件的处理记录数。具体格式由 planner 设计。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 核心文件
- `src/cli/opts.rs` — 当前 `-v` (Count) 和 `-q` (bool) 定义
- `src/main.rs` — `apply_verbosity_to_config()`、Run 子命令处理、摘要打印路径
- `src/cli/run/mod.rs` — `handle_run()` 签名、ProgressBar 创建位置

No external specs — requirements fully captured in decisions above

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/cli/run/mod.rs` — ProgressBar 当前在 `handle_run()` 内创建，受 `quiet` 参数控制（`let pb = if quiet { None } else { Some(ProgressBar::new(...)) }`——需确认当前实现是否已是 Option 模式）
- `indicatif::ProgressBar` — 已是依赖，无需新增

### Established Patterns
- `quiet: bool` 已作为参数传入 `handle_run()`，模式已建立
- `eprintln!` 用于错误实时输出（非致命错误已经走这条路），verbose 文件输出同样走 stderr

### Integration Points
- verbose 信号需要从 `main.rs::cli.verbose` 传入 `handle_run()`——当前签名是 `handle_run(cfg, quiet, interrupted, compiled_filters)`，需要添加 `verbose: bool` 参数
- quiet 的摘要抑制需要在 `main.rs` 的 `Err(stats)` 打印分支中加入 `if !quiet` 条件

</code_context>

<specifics>
## Specific Ideas

- verbose 的每文件输出用 `eprintln!("Processing: {}", path.display())` 格式
- `--verbose` 和 `--quiet` 互斥由 clap `conflicts_with` 保证，当前已有此约束

</specifics>

<deferred>
## Deferred Ideas

无——讨论保持在 phase 边界内。

</deferred>

---

*Phase: 48-日志级别与运行提示*
*Context gathered: 2026-05-31*
