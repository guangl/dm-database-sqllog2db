# Phase 69: Watch 模式核心框架 - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

建立 `sqllog2db watch -c config.toml` 子命令骨架：监听 inputs 目录新增 `.log` 文件，触发完整处理并累计统计，实时刷新状态行（路径、上次触发时间、累计行数），Ctrl+C 优雅退出打印最终摘要。不包含增量处理（Phase 70）。

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
- **D-09:** 返回的 `ErrorStats` 通过 `total_stats.merge(file_stats)` 累计全局统计（`records_exported` 字段作为"累计已处理行数"）。

### Ctrl+C 退出（WATCH-06）

[auto] Q: "如何实现优雅退出？" → Selected: "复用 Arc<AtomicBool> + ctrlc::set_handler 模式（ctrlc dep 已在 Cargo.toml）" (recommended default)

- **D-10:** `let interrupted = Arc::new(AtomicBool::new(false));` + `ctrlc::set_handler(move || interrupted_flag.store(true, Relaxed))` — 与 `src/main.rs` Run 命令中完全相同的模式。
- **D-11:** main watch loop：`loop { match receiver.recv_timeout(Duration::from_millis(100)) { ... } if interrupted.load(Relaxed) { break; } }`。
- **D-12:** 退出时清除 ProgressBar（`pb.finish_and_clear()`），然后打印最终摘要（stderr）：`"watch stopped after {duration}. triggers: {n}, total processed: {rows} rows."`，退出码 0（`return Ok(None)`）。

### 模块结构

[auto] Q: "watch.rs 还是 watch/mod.rs？" → Selected: "src/cli/watch.rs 单文件（Phase 70 可扩展为 mod.rs）" (recommended default)

- **D-13:** 新建 `src/cli/watch.rs`，pub fn `handle_watch(cfg: &Config, quiet: bool, verbose: bool) -> Result<()>`。
- **D-14:** `src/cli/opts.rs` 新增 `Commands::Watch { config: String }` variant（与 `Commands::Run` 结构类似，仅 config 字段，无 --input override）。
- **D-15:** `src/cli/mod.rs` 新增 `pub mod watch;`。`src/main.rs` 新增 Watch arm：加载 config、validate、init logging、设置 ctrlc handler、调用 `handle_watch`。
- **D-16:** `src/main.rs` 中 `needs_simple_logging` 逻辑需排除 Watch（与 Run/Stats 一样使用完整 logging stack）。

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
- 状态行时间格式：`chrono` 已不在依赖中，使用 `std::time::SystemTime::now()` 转 local time 字符串（`format!("{:.19}", ...)`）或使用 `indicatif` 内置 `{elapsed}`。优先用 elapsed 避免引入 chrono。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 69: Watch 模式核心框架" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §WATCH-01、WATCH-02、WATCH-05、WATCH-06

### 核心实现文件（新建/修改）
- `src/cli/watch.rs` — 新建，`handle_watch()` 主逻辑
- `src/cli/opts.rs` — 新增 `Commands::Watch { config }` variant
- `src/cli/mod.rs` — 新增 `pub mod watch;`
- `src/main.rs` — 新增 Watch arm，复用 ctrlc + logging 初始化模式
- `Cargo.toml` — 新增 `notify = "6"`

### 参考实现模式
- `src/main.rs:160-168` — `Arc<AtomicBool>` + `ctrlc::set_handler` 模式（直接复用）
- `src/cli/run/mod.rs` — `handle_run()` 签名（`cfg, quiet, verbose, &interrupted, None`）
- `src/cli/run/processor.rs` — `make_progress_bar()` 模式（indicatif ProgressBar）
- `.planning/phases/68-init-wizard/68-CONTEXT.md` — Phase 68 决策（参考模式）
- `.planning/STATE.md` §"Architecture Notes for Phases 69–70"

### 外部依赖
- `notify = "6"` — 新增，`RecommendedWatcher` + `mpsc::channel`
- `ctrlc = "3"` — 已存在，无需新增
- `indicatif = "0.18"` — 已存在，`ProgressBar::new_spinner()`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/main.rs:160-168` — `Arc<AtomicBool>` + `ctrlc::set_handler` + `handle_run()` 调用序列，watch 直接复制该模式
- `src/cli/run/mod.rs` — `handle_run(cfg, quiet, verbose, &interrupted, None)` 签名，watch 触发时复用
- `indicatif::ProgressBar::new_spinner()` — 已在 `src/cli/run/mod.rs` 使用，spinner 模式直接适用
- `ErrorStats::merge()` — 累计跨触发统计（`total_stats.merge(file_stats)`）

### Established Patterns
- 新子命令添加：`Commands::Init` 的 clap 注解格式 → Watch 复用相同模式
- `needs_simple_logging` 排除模式：`src/main.rs:133-135` — Watch 需要加入排除列表（与 Run/Stats 一样使用完整 logging）
- `pb.finish_and_clear()` 退出时清除状态行 — indicatif 标准用法

### Integration Points
- `src/cli/mod.rs` — 需新增 `pub mod watch;`
- `src/main.rs:140-195` — match arm 新增 `Commands::Watch { config }` 分支
- `Cargo.toml [dependencies]` — 新增 `notify = "6"`

</code_context>

<specifics>
## Specific Ideas

- watch loop poll 使用 `receiver.recv_timeout(Duration::from_millis(100))`：100ms 间隔响应中断检查
- 最终摘要格式（stderr）：`"Watch stopped. Triggers: {n}, total processed: {rows} rows, elapsed: {hh:mm:ss}"`
- elapsed 用 `std::time::Instant::now()` 在 `handle_watch` 开始时记录，退出时 `start.elapsed()`
- notify `EventKind::Create(_)` 过滤：`path.extension().map_or(false, |e| e == "log")`

</specifics>

<deferred>
## Deferred Ideas

- watch 增量处理（文件追加）→ Phase 70（WATCH-03/04）
- SQLite 字节偏移去重 → Phase 70
- watch 路径 glob 展开 → Phase 70（Phase 69 直接用路径作目录）
- watch 支持 --input CLI override → 超出 Phase 69 范围

</deferred>

---

*Phase: 69-Watch 模式核心框架*
*Context gathered: 2026-06-05*
