# Phase 1: watch 功能完善 - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

三项 watch 子命令功能修复：
1. **WATCH-07** — CSV exporter 支持：watch 触发时增量追加到 CSV 文件（而非每次覆盖）
2. **WATCH-08** — error log 追加写入：watch 触发中产生的 parse error 追加到 error log，不覆盖历史
3. **WATCH-09** — Ctrl+C 退出码修正：watch 收到 SIGINT 后退出码为 130（与 `run` 保持一致）

</domain>

<decisions>
## Implementation Decisions

### WATCH-07: CSV watch 追加语义

[auto] Q: "全量触发（Create 事件）和增量触发（Modify 事件）是否都追加到 CSV？" → Selected: "两者都追加" (STATE.md 已决策：所有触发均追加)

- **D-01:** 所有触发路径（`trigger_full_file` 和 `trigger_incremental`/`build_incremental_cfg`）均设置 CSV append 模式。在两处函数中添加：
  ```rust
  if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
      csv_cfg.append = true;
      csv_cfg.overwrite = false;
  }
  ```
- **D-02:** CSV watch 不需要 offset/Seek 跟踪。`CsvExporter` 已有 `append=true` + 空文件才写 header 的 TOCTOU 安全逻辑（`src/exporter/csv/mod.rs` line 123–132），直接复用即可。
- **D-03:** `WatchLoopState` 不需要新增 CSV 等价的 `csv_path` 字段；offset 管理仅 SQLite 需要。

### WATCH-08: error log 追加传递方式

[auto] Q: "如何让 write_error_log 知道使用追加模式？" → Selected: "Config 内部字段 #[serde(skip)] append_error_log: bool" (不改 handle_run 签名，最小侵入)

- **D-04:** 在 `Config`（`src/config/mod.rs`）中添加一个内部控制字段：
  ```rust
  #[serde(skip)]
  pub(crate) append_error_log: bool,
  ```
  默认值 `false`（`Default::default()` 保持原覆盖写行为，`run` 路径不受影响）。
- **D-05:** watch 触发时，在 `trigger_full_file` 和 `build_incremental_cfg` 中设置 `tmp_cfg.append_error_log = true`。
- **D-06:** `write_error_log` 读取 `cfg.append_error_log`，为 true 时用 `OpenOptions::new().create(true).append(true).open(...)` 打开文件；为 false 时保持现有 `std::fs::File::create`（覆盖）行为。

### WATCH-09: 退出码 130 实现点

[auto] Q: "在哪里检查 interrupted 并返回 Err(Error::Interrupted)？" → Selected: "handle_watch 尾部 print_final_summary 之后" (与 run 路径对称，main.rs 无需修改)

- **D-07:** 在 `handle_watch`（`src/cli/watch/mod.rs` line 73）的 `Ok(())` 前添加：
  ```rust
  if interrupted.load(Ordering::Acquire) {
      return Err(Error::Interrupted);
  }
  ```
  `main.rs` 的 `Err(e) if matches!(e, Error::Interrupted)` 分支已处理（line 114–115），无需修改。
- **D-08:** `print_final_summary` 仍在 interrupted 时调用（打印摘要后再返回错误），保持用户体验一致性。

### Claude's Discretion

- `build_incremental_cfg` 函数体将同时处理 SQLite 和 CSV 的 append 设置，可考虑是否提取成一个 `force_append_exporters(cfg)` 辅助函数——如果两处设置逻辑完全相同，提取可减少重复；否则保留内联即可。
- `append_error_log` 字段加入 `Config::validate()` 不做校验（它是运行时内部状态，非用户配置）。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 1: watch 功能完善" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §WATCH-07、WATCH-08、WATCH-09

### 核心实现文件
- `src/cli/watch/mod.rs` — watch 主入口（`handle_watch`）、触发函数（`trigger_full_file`、`trigger_incremental`）、`build_incremental_cfg`；退出码和追加逻辑修改点
- `src/cli/run/mod.rs` — `handle_run`、`write_error_log`（line 425–461）；error log 追加模式修改点
- `src/config/mod.rs` — `Config` 结构体；新增 `append_error_log` 字段
- `src/exporter/csv/mod.rs` — `CsvExporter::from_config`（line 60–69）、`initialize`（line 93–144）；append 模式已实现，watch 只需将 config 的 append 设为 true
- `src/main.rs` — 退出码处理（line 113–115）；`Err(Error::Interrupted)` → exit 130 已就位

### 对齐参考
- `src/cli/watch/offsets.rs` — SQLite offset 管理（对照了解，CSV 不需要类似机制）
- `src/error.rs` — `Error::Interrupted` 变体定义

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CsvExporter::from_config`：已支持 `append` 模式（`config.append = true` → `WriteMode::Append`），只需在 watch 触发时将 `csv_cfg.append = true` 注入 tmp_cfg 即可
- `build_incremental_cfg`（`watch/mod.rs` line 514–523）：已有 SQLite append 注入模式，CSV 追加同样逻辑复制到此函数
- `Error::Interrupted`：`main.rs` 已有处理分支（line 114–115），`handle_watch` 只需返回该错误即可触发 exit 130

### Established Patterns
- watch 触发修改 Config 的模式：克隆 cfg → 修改 inputs（和 exporter flags）→ 调用 handle_run。CSV append 修改遵循相同模式。
- `#[serde(skip)]` 内部字段：项目中已有先例（如 `WatchLoopState` 的非序列化字段），`append_error_log` 同样处理。

### Integration Points
- `trigger_full_file`：需在克隆 tmp_cfg 后、调用 handle_run 前同时设置 `csv_cfg.append = true` 和 `tmp_cfg.append_error_log = true`
- `build_incremental_cfg`：与 `trigger_full_file` 同理
- `write_error_log`：从 `cfg.append_error_log` 读取模式，替换 `std::fs::File::create` 为 `OpenOptions`

</code_context>

<specifics>
## Specific Ideas

- STATE.md 明确：CSV watch 语义为"追加写入——每次触发向现有 CSV 文件追加，而非全量重写"
- STATE.md 明确：error log 追加使用 `OpenOptions::append(true)`
- STATE.md 明确：退出码通过传播 SIGINT 信号穿透 handle_watch 返回路径实现

</specifics>

<deferred>
## Deferred Ideas

- CSV watch 的 offset 跟踪（类似 SQLite 的 `_watch_offsets` 表）——当前 CSV append 追加全量记录不需要 offset，若将来需要精确增量可在 Phase 2 评估
- watch 支持多目录 glob 模式（WATCH-10）——Future requirement，不在本 milestone
- watch 远程推送 webhook（WATCH-11）——Future requirement，不在本 milestone

</deferred>

---

*Phase: 1-watch*
*Context gathered: 2026-06-06*
