# Phase 69: Watch 模式核心框架 - Context

**Gathered:** 2026-06-06 (updated 2026-06-06)
**Status:** Ready for planning

<domain>
## Phase Boundary

建立 `sqllog2db watch -c config.toml` 子命令骨架：监听 inputs 目录新增 `.log` 文件，触发完整处理并累计统计，实时刷新状态行（路径、上次触发时间、累计行数），Ctrl+C 优雅退出打印最终摘要。不包含增量处理（Phase 70）。

**前置条件：** Phase 68（init-wizard）已完成 ✓（2026-06-06 确认）

</domain>

<decisions>
## Implementation Decisions

### notify crate 版本与监听模式

[auto] Q: "notify API 选型？" → Selected: "notify = 6 + RecommendedWatcher + blocking mpsc::channel" (recommended default)

- **D-01:** `Cargo.toml` 新增依赖：`notify = "6"`。使用 `notify::RecommendedWatcher` + `std::sync::mpsc::channel`（blocking channel）。不引入 tokio，与现有单线程模型兼容。
- **D-02:** 监听模式：`RecursiveMode::Recursive`（递归监听 inputs 目录及其子目录）。只关注 `notify::EventKind::Create(_)` 事件，路径以 `.log` 结尾才触发处理。
- **D-03:** inputs 路径处理：`cfg.sqllog.inputs` 每个条目直接作为 `notify::Watcher::watch()` 的目标路径（目录）。glob 模式取父目录部分（`Path::new(glob_str).ancestors().find(|p| p.exists())`）。

### 状态行显示（WATCH-05）

[auto] Q: "实时状态行用 indicatif 还是手动 \\r？" → Selected: "indicatif::ProgressBar with spinner，set_message() 更新状态" (recommended default)

- **D-04:** 使用已有的 `indicatif::ProgressBar::new_spinner()`（不是 `new(len)`，因为触发次数不可预知）。Template: `"{spinner:.cyan} {wide_msg}"`，其中 `wide_msg` 格式：`"watching {paths} | triggers: {n} | processed: {rows} rows | last: {timestamp}"`。
- **D-05:** 状态行写入 stderr（`ProgressDrawTarget::stderr()`），不干扰 stdout/log 输出（SC3）。
- **D-06:** 程序启动时立即显示状态行："watching {paths} | waiting for new .log files..."，Ctrl+C 后清除状态行再打印最终摘要。

### 触发处理策略（WATCH-02）

[auto] Q: "是否需要 debounce？触发时处理范围？" → Selected: "直接触发，仅处理新增文件（inputs override）" (recommended default)

- **D-07:** 收到 Create 事件后直接触发（无 debounce）。满足 "2 秒内触发" 约束（SC2）。
- **D-08:** 触发时，将新文件路径作为 `inputs` 临时覆盖（`cfg.sqllog.inputs = vec![new_file_path]`），调用现有 `handle_run(&cfg, quiet, verbose, &interrupted, None)`。这样每次只处理该新文件的完整内容（Phase 70 负责追加增量逻辑）。
- **D-09:** 累计已处理行数追踪：`handle_run` 返回 `Result<ErrorStats>`，但 `ErrorStats` 当前无 `records_exported` 字段（实际行数在 `handle_run` 内部本地计算为 `total_records: usize`）。**需新增 `records_exported: usize` 字段到 `ErrorStats`（`src/error.rs:78`），`handle_run` 返回前赋值 `stats.records_exported = total_records`**。Watch 通过 `total_stats.merge(file_stats)` 自动累计（`ErrorStats::merge` 方法已存在于 `src/error.rs:118`）。

### Ctrl+C 退出（WATCH-06）

[auto] Q: "如何实现优雅退出？" → Selected: "复用 Arc<AtomicBool> + ctrlc::set_handler 模式（ctrlc dep 已在 Cargo.toml）" (recommended default)

- **D-10:** `let interrupted = Arc::new(AtomicBool::new(false));` + `ctrlc::set_handler(move || interrupted_flag.store(true, Relaxed))` — 与 `src/main.rs:166-169` Run 命令中完全相同的模式。
- **D-11:** main watch loop：`loop { match receiver.recv_timeout(Duration::from_millis(100)) { ... } if interrupted.load(Relaxed) { break; } }`。
- **D-12:** 退出时清除 ProgressBar（`pb.finish_and_clear()`），然后打印最终摘要（stderr）：`"Watch stopped. Triggers: {n}, total processed: {rows} rows, elapsed: {hh:mm:ss}"`，退出码 0（`return Ok(())`）。

### 模块结构

[auto] Q: "watch.rs 还是 watch/mod.rs？" → Selected: "src/cli/watch.rs 单文件（Phase 70 可扩展为 mod.rs）" (recommended default)

- **D-13:** 新建 `src/cli/watch.rs`，pub fn `handle_watch(cfg: &Config, quiet: bool, verbose: bool) -> Result<()>`（注：返回 `Result<()>` 而非 `Result<ErrorStats>`，摘要在内部打印）。
- **D-14:** `src/cli/opts.rs` 新增 `Commands::Watch { config: String }` variant（与 `Commands::Run` 结构类似，仅 config 字段，无 --input override）。
- **D-15:** `src/cli/mod.rs` 新增 `pub mod watch;`（当前有：init, opts, run, stats, validate）。
- **D-16:** `src/main.rs:130-132` 的 `needs_simple_logging` 逻辑需加入 `Commands::Watch { .. }` 到 `!matches!` 宏内（Watch 与 Run/Stats 一样使用完整 logging stack）。

### main.rs Watch arm 返回值

[auto] Q: "Watch arm 返回 Ok(None) 还是 Ok(Some(stats, quiet))？" → Selected: "Ok(None)（摘要在 handle_watch 内部打印，与 Init arm 一致）" (confirmed from codebase)

- **D-21:** `main.rs` Watch arm 调用 `cli::watch::handle_watch(&cfg, cli.quiet, cli.verbose)?;` 后返回 `Ok(None)`。不需要在外层 main.rs 中打印 ErrorStats（与 `Commands::Init` arm pattern 一致；`Commands::Run` 是例外，它在外层聚合 stats 并显示摘要）。
- **D-22:** Watch arm 需要包含 `preflight::check(&cfg)` —— 与 Run arm（`src/main.rs:161-164`）一致，确保 config 指向的 inputs 路径在监听启动前通过预检。

### watch 导出格式约束

[auto] Q: "watch 是否支持所有导出格式？" → Selected: "Phase 69 对所有格式透明（完整处理），Phase 70 增量逻辑仅限 SQLite" (recommended default)

- **D-17:** Phase 69 触发时调用 `handle_run` 处理完整新文件，CSV/SQLite 均可工作（无追加语义问题）。CSV append 模式（`append = true`）在 config 中控制，watch 不另行干预。
- **D-18:** watch 命令不强制要求 SQLite 配置（Out of Scope 限制针对 Phase 70 的增量写入语义）。

### 测试策略

[auto] Q: "Phase 69 如何测试？" → Selected: "集成测试：实际创建文件触发 handle_watch，assert 统计计数" (recommended default)

- **D-19:** 在 `tests/` 或 `src/cli/watch.rs` 内添加集成测试：用 `tempfile::TempDir` 创建监听目录，`thread::spawn` 在 50ms 后写入 `.log` 文件，在主线程 `handle_watch` 执行后检查累计行数 > 0。需要设置 timeout（约 3 秒）。
- **D-20:** Ctrl+C 测试用 `interrupted` flag 直接设置为 true，不依赖信号（单元测试可控性）。

### Claude's Discretion

- `recv_timeout(100ms)` poll 间隔：既能及时响应中断，又不忙等消耗 CPU。
- ProgressBar tick：watch loop 每次 iteration 调用 `pb.tick()`（spinner 动画）。
- 状态行时间格式：优先用 `indicatif` 内置 `{elapsed}` 避免引入 chrono；若需要时间戳，用 `std::time::SystemTime::now()` 转 local time 字符串。
- elapsed 用 `std::time::Instant::now()` 在 `handle_watch` 开始时记录，退出时 `start.elapsed()`。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 69: Watch 模式核心框架" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §WATCH-01、WATCH-02、WATCH-05、WATCH-06

### 核心实现文件（新建/修改）
- `src/cli/watch.rs` — 新建，`handle_watch()` 主逻辑（返回 `Result<()>`）
- `src/cli/opts.rs` — 新增 `Commands::Watch { config }` variant
- `src/cli/mod.rs` — 新增 `pub mod watch;`（当前模块：init, opts, run, stats, validate）
- `src/main.rs` — 新增 Watch arm，复用 ctrlc + logging + preflight 初始化模式
- `src/error.rs:78` — `ErrorStats` 新增 `records_exported: usize` 字段（D-09 要求）
- `Cargo.toml` — 新增 `notify = "6"`（ctrlc/indicatif 已存在）

### 参考实现模式
- `src/main.rs:130-133` — `needs_simple_logging` 排除模式（Watch 需加入）
- `src/main.rs:151-175` — Run 命令完整 arm（Watch 复用相同 config 加载 + ctrlc + preflight 序列）
- `src/main.rs:139-149` — Init arm 返回 `Ok(None)` 模式（Watch 返回值同此）
- `src/main.rs:161-164` — `preflight::check` 模式（Watch 需复用）
- `src/main.rs:166-169` — `Arc<AtomicBool>` + `ctrlc::set_handler` 模式（直接复用）
- `src/cli/run/mod.rs` — `handle_run()` 签名（`cfg, quiet, verbose, &interrupted, None`）
- `src/error.rs:78-132` — `ErrorStats` 结构体定义 + `merge()` 方法（D-09/D-21 依赖）
- `src/cli/init.rs` — Phase 68 完成模式：`handle_init_interactive` 在单文件中实现（watch.rs 对应模式）
- `.planning/STATE.md` §"Architecture Notes for Phases 69–70"
- `.planning/phases/69-watch/69-UI-SPEC.md` — 终端 UI 设计合约（spinner、状态行、摘要格式锁定）

### 外部依赖
- `notify = "6"` — 新增，`RecommendedWatcher` + `mpsc::channel`
- `ctrlc = "3"` — 已存在（`Cargo.toml:43`），无需新增
- `indicatif = "0.18"` — 已存在（`Cargo.toml:46`），`ProgressBar::new_spinner()`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/main.rs:166-169` — `Arc<AtomicBool>` + `ctrlc::set_handler` + `handle_run()` 调用序列，watch 直接复制该模式
- `src/cli/run/mod.rs` — `handle_run(cfg, quiet, verbose, &interrupted, None)` 签名，watch 触发时复用
- `indicatif::ProgressBar::new_spinner()` — 已在 `src/cli/run/mod.rs` 使用，spinner 模式直接适用
- `src/error.rs:118` — `ErrorStats::merge()` 方法已存在，watch 用于跨触发累计统计
- `src/main.rs:161-164` — `preflight::check(&cfg)` 模式，Watch arm 需复用

### Key Modification: ErrorStats.records_exported
- `src/error.rs:78` — `ErrorStats` 当前字段：`total_errors`, `parse_errors`, `export_errors`, `fatal_error`, `by_type`, `filtered_out`, `parse_error_records`
- **需新增** `records_exported: usize` 字段，并在 `src/cli/run/mod.rs` 中 `handle_run` 返回前赋值 `run_stats.records_exported = total_records`（`total_records` 已在 `src/cli/run/mod.rs:132` 计算）
- `ErrorStats::merge()` 需同步更新以累计 `records_exported`

### Established Patterns
- 新子命令添加：`Commands::Init` / `Commands::Stats` 的 clap 注解格式 → Watch 复用相同模式
- `needs_simple_logging` 排除模式：`src/main.rs:130-133` — Watch 需要加入排除列表（与 Run/Stats 一样使用完整 logging）
- `pb.finish_and_clear()` 退出时清除状态行 — indicatif 标准用法
- Phase 68 模式：`handle_init_interactive` 位于 `src/cli/init.rs` 单文件 → `handle_watch` 在 `src/cli/watch.rs` 单文件
- Return value convention：Init/Watch → `Ok(None)`；Run → `Ok(Some((stats, quiet)))`

### Integration Points
- `src/cli/mod.rs` — 需新增 `pub mod watch;`（当前共 5 个 mod）
- `src/main.rs:138` — match arm 新增 `Commands::Watch { config }` 分支（在 Init arm 后）
- `Cargo.toml [dependencies]` — 新增 `notify = "6"`

</code_context>

<specifics>
## Specific Ideas

- watch loop poll 使用 `receiver.recv_timeout(Duration::from_millis(100))`：100ms 间隔响应中断检查
- 最终摘要格式（stderr）：`"Watch stopped. Triggers: {n}, total processed: {rows} rows, elapsed: {hh:mm:ss}"`
- elapsed 用 `std::time::Instant::now()` 在 `handle_watch` 开始时记录，退出时 `start.elapsed()`
- notify `EventKind::Create(_)` 过滤：`path.extension().map_or(false, |e| e == "log")`
- 状态行 wide_msg 格式：`"watching {dir} | triggers: {n} | processed: {rows} rows | last: {elapsed_since_last}"`
- `hh:mm:ss` 格式化：`let secs = elapsed.as_secs(); format!("{:02}:{:02}:{:02}", secs/3600, (secs%3600)/60, secs%60)` — 无需引入 chrono

</specifics>

<deferred>
## Deferred Ideas

- watch 增量处理（文件追加）→ Phase 70（WATCH-03/04）
- SQLite 字节偏移去重 → Phase 70
- watch 路径 glob 展开 → Phase 70（Phase 69 直接用路径作目录）
- watch 支持 --input CLI override → 超出 Phase 69 范围
- watch + CSV 增量插入 → Out of Scope（CSV 不支持原位增量写）

</deferred>

---

*Phase: 69-Watch 模式核心框架*
*Context gathered: 2026-06-06 (updated 2026-06-06)*
