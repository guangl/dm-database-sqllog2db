# Phase 70: Watch 增量处理与集成测试 - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

为 `sqllog2db watch` 子命令实现已有文件追加内容的**增量处理**：收到 `Modify(Data(Content))` 事件时，仅读取文件自上次偏移后的新增字节并处理（WATCH-03）；SQLite 导出模式下通过辅助表持久化字节偏移，确保同一文件多次触发不产生重复行，且重启后可从上次记录的偏移恢复（WATCH-04）。

**前置条件：** Phase 69（Watch 模式核心框架）已完成 ✓（2026-06-06 确认）

</domain>

<decisions>
## Implementation Decisions

### 增量读取机制（WATCH-03 核心）

[auto] Q: "如何实现已有文件追加内容的增量读取（parse API 无 offset 支持）？" → Selected: "读取新增字节写入临时文件，从临时文件构建 LogParser，处理完后自动删除" (recommended)

- **D-01:** `LogParserBuilder::build()` 内部通过 `fs::read()` 全量读取文件，无 seek/offset API，无法直接跳过已处理字节。增量路径实现：
  1. 打开文件，`Seek::seek(SeekFrom::Start(start_offset))`
  2. `read_to_end()` 读取剩余字节到 `Vec<u8>`
  3. 写入 `tempfile::NamedTempFile`（处理完后自动 drop 删除）
  4. 以临时文件路径构建 `LogParser` 并正常处理
  5. 处理完成后用 `fs::metadata(original_path)?.len()` 更新偏移

- **D-02:** Modify 事件触发前检查 `new_size <= start_offset`，若无新字节则跳过触发（不调用 handle_run，不计入 trigger_count）。

### 事件路由策略

[auto] Q: "Modify 事件与 Create 事件如何区分路由？" → Selected: "Create→全文处理（复用 Phase 69 D-08 逻辑）并记录初始偏移为处理后文件大小；Modify→增量处理" (recommended)

- **D-03:** 事件路由在 `process_watch_event()` 函数内按 `EventKind` 分支：
  - `EventKind::Create(_)` — 调用现有 `trigger_full_file(path, ...)` 路径（Phase 69 行为），**处理完成后** 立即记录 `file_offsets[path] = fs::metadata(path)?.len()`，确保后续 Modify 事件不重复处理已有内容。
  - `EventKind::Modify(ModifyKind::Data(DataChange::Content))` — 调用新增 `trigger_incremental(path, ...)` 路径（Phase 70 行为）。
- **D-04:** 首次收到某路径的 Modify 事件但 `file_offsets` 中无记录时：设 `start_offset = fs::metadata(path)?.len()`（等于当前文件大小），跳过历史内容，不触发处理。这适用于 watch 启动前已存在的文件——只处理 watch 运行期间新追加的内容。

### SQLite 字节偏移持久化（WATCH-04 核心）

[auto] Q: "字节偏移跨重启持久化如何存储？" → Selected: "在 SQLite 导出 DB 中新建辅助表 _watch_offsets，与导出数据同库" (STATE.md 既定决策)

- **D-05:** 辅助表 DDL：
  ```sql
  CREATE TABLE IF NOT EXISTS _watch_offsets (
      path TEXT NOT NULL PRIMARY KEY,
      byte_offset INTEGER NOT NULL
  );
  ```
  使用导出配置中的同一 `database_url`，新开独立 `rusqlite::Connection`（不复用 SqliteExporter 的连接，避免事务干扰）。

- **D-06:** watch 启动时，若配置中有 SQLite 导出器：调用 `offsets::load_offsets(&database_url)` 读取 `_watch_offsets` 所有行，初始化 `WatchLoopState.file_offsets`。若表不存在（首次运行）则返回空 HashMap，不报错。

- **D-07:** 每次 Modify 触发成功处理后：调用 `offsets::save_offset(&database_url, &path, new_offset)` 持久化新偏移（`INSERT OR REPLACE INTO _watch_offsets`）。持久化失败时 `log::warn!` 但不中断 watch（非致命）。

- **D-08:** Create 触发成功后：同样持久化初始偏移（文件处理后的大小）到 `_watch_offsets`。

### handle_run 调用方式（增量路径）

[auto] Q: "增量路径调用 handle_run 时如何配置 exporter？" → Selected: "构建 tmp_cfg 覆盖 inputs + 强制 SQLite append=true/overwrite=false" (required for correctness)

- **D-09:** 增量路径 `trigger_incremental` 中构建 `tmp_cfg`：
  ```rust
  let mut tmp_cfg = cfg.clone();
  tmp_cfg.sqllog.inputs = vec![temp_file_path.to_string_lossy().into_owned()];
  // 强制 append 模式：增量触发不能清空表
  if let Some(ref mut sqlite) = tmp_cfg.exporter.sqlite {
      sqlite.append = true;
      sqlite.overwrite = false;
  }
  ```
  调用 `crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None)`。
- **D-10:** 全文路径（Create 事件）保持 Phase 69 行为（尊重用户 config 的 overwrite/append 设置），不强制覆盖——用户初始化 watch 时可能需要 overwrite 清空旧数据。

### 模块结构（watch.rs → watch/ 目录）

[auto] Q: "Phase 70 offset 逻辑如何组织到模块中？" → Selected: "watch.rs → watch/mod.rs + watch/offsets.rs（Phase 69 预设的扩展路径）" (recommended)

- **D-11:** 将 `src/cli/watch.rs` 重命名为 `src/cli/watch/mod.rs`，新建 `src/cli/watch/offsets.rs`，包含：
  - `pub(super) fn load_offsets(database_url: &str) -> HashMap<PathBuf, u64>`
  - `pub(super) fn save_offset(database_url: &str, path: &Path, offset: u64)`
  - `pub(super) fn ensure_offset_table(database_url: &str) -> Result<()>`（建表 DDL）
- **D-12:** `WatchLoopState` 新增字段：
  ```rust
  file_offsets: HashMap<PathBuf, u64>,  // 运行时偏移缓存
  sqlite_db_url: Option<String>,        // 用于持久化，None 时不持久化
  ```
  `WatchLoopState::new()` 接受初始偏移和可选 DB URL 参数。

### 集成测试策略

[auto] Q: "Phase 70 集成测试如何验证 WATCH-03 和 WATCH-04？" → Selected: "tempfile + 真实 SQLite DB，测试追加处理和重启后 offset 恢复" (recommended)

- **D-13:** 在 `src/cli/watch/mod.rs` 或 `tests/` 中新增测试：
  - **WATCH-03 测试**：创建 log 文件写入 N 条记录，触发 `trigger_incremental`，追加 M 条记录，再次触发，断言 SQLite 中只有 M 条新行（不重复 N 条）。
  - **WATCH-04 测试**：写入 N 条，触发，持久化 offset；重建 `WatchLoopState` 时 `load_offsets` 恢复；追加 M 条，触发，断言 SQLite 总行数 N+M（不重复）。
  - 测试使用 `tempfile::NamedTempFile` 和 `tempfile::TempDir` 避免留痕。

- **D-14:** `trigger_incremental` 和 `trigger_full_file` 提取为可独立测试的 `pub(crate)` 或 `pub(super)` 函数，接受 `WatchLoopState` 可变引用（便于单元测试不启动完整 notify watcher）。

### Claude's Discretion

- 临时文件路径：使用 `tempfile::Builder::new().prefix("sqllog2db-watch-").tempfile()?`，自动 drop 时删除。
- offsets.rs 中的 rusqlite 连接：每次调用 `load_offsets`/`save_offset` 打开新连接（不长持），与 SqliteExporter 连接池隔离。
- 偏移值边界：`u64` 足够（文件大小最大 ~18EB），与 `fs::metadata().len()` 返回类型一致。
- 如果 Phase 70 的临时文件方案导致大文件（如 append 几十 MB）性能不佳，可后续优化为内存 buffer，但当前不做过早优化。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求与验收标准
- `.planning/ROADMAP.md` §"Phase 70: Watch 增量处理与集成测试" — Goal、Success Criteria（SC1–SC3）
- `.planning/REQUIREMENTS.md` §WATCH-03、WATCH-04

### Phase 69 上下文（必读）
- `.planning/phases/69-watch/69-CONTEXT.md` — Phase 69 所有决策（D-01 到 D-22），Phase 70 在此基础上扩展
- `.planning/STATE.md` §"Architecture Notes for Phases 69–70" — 字节偏移设计原则

### 核心实现文件（新建/修改）
- `src/cli/watch.rs` → 重命名为 `src/cli/watch/mod.rs`（不修改现有逻辑，只迁移文件）
- `src/cli/watch/offsets.rs` — 新建，offset 加载/保存/建表逻辑
- `Cargo.toml` — 确认 `tempfile` 在 dev-dependencies 中（集成测试需要）

### 参考实现模式
- `src/cli/watch.rs:275-316` — `trigger_and_update_stats()`（Phase 70 拆分为 trigger_full_file/trigger_incremental）
- `src/cli/run/mod.rs:29` — `handle_run()` 签名（增量路径复用）
- `src/exporter/sqlite/mod.rs` — SqliteExporter append/overwrite 字段控制（D-09 依赖）
- `src/cli/watch.rs:84-104` — `WatchLoopState` 结构体（D-12 扩展）

### 外部依赖（已存在，确认即可）
- `rusqlite` — 已在 `Cargo.toml` 依赖中（用于 offsets.rs）
- `tempfile` — 确认是否已在 `[dev-dependencies]`（集成测试 + 临时文件）

</canonical_refs>

<code_context>
## Existing Code Insights

### Phase 69 已实现（可直接扩展）
- `src/cli/watch.rs:275-316` — `trigger_and_update_stats()`：当前处理单个路径的完整文件，Phase 70 将其拆分为 `trigger_full_file`（Create 事件）和 `trigger_incremental`（Modify 事件）
- `src/cli/watch.rs:84-104` — `WatchLoopState`：`debounce_map`, `total_stats`, `trigger_count`；Phase 70 新增 `file_offsets: HashMap<PathBuf, u64>` 和 `sqlite_db_url: Option<String>`
- `src/cli/watch.rs:231-270` — `process_watch_event()`：路由逻辑，Phase 70 在此加入 Create vs Modify 分支判断

### LogParserBuilder API 限制（关键约束）
- `/Users/guang/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/dm-database-parser-sqllog-2.0.2/src/parser/builder.rs` — `build()` 内部执行 `fs::read(path)` 全量读取，无 offset 参数。这是临时文件方案的根本原因。

### SQLite Exporter 结构（append 模式控制）
- `src/exporter/sqlite/mod.rs:19-23` — `overwrite: bool, append: bool` 字段；`prepare_target_table()` 据此决定 DROP/DELETE/保留
- `src/config/mod.rs` — `SqliteExporterConfig` 包含 `append: bool, overwrite: bool` 字段，可通过 cfg.clone() + 字段覆盖修改

### 已有测试模式（集成测试参照）
- `src/cli/watch.rs:400-532` — 现有单元测试：`test_interrupted_flag_exits_immediately`, `test_collect_watch_dirs_*`, `test_should_trigger_*`
- `tests/` 目录 — assert_cmd / predicates / tempfile 集成测试模式

### 依赖确认
- `Cargo.toml:43` — `ctrlc = "3"`（已存在）
- `Cargo.toml:46` — `indicatif = "0.18"`（已存在）
- `Cargo.toml` — `rusqlite` 已存在（生产依赖）；`tempfile` 需确认是否在 dev-dependencies

</code_context>

<specifics>
## Specific Ideas

- 临时文件前缀：`tempfile::Builder::new().prefix("sqllog2db-watch-").suffix(".log").tempfile()?`，以 `.log` 结尾确保编码探测逻辑正常工作。
- 无新字节跳过：`if new_size <= start_offset { return Ok(()); }`，返回 `Ok(())` 不更新任何状态。
- offset 持久化 SQL：`INSERT OR REPLACE INTO _watch_offsets (path, byte_offset) VALUES (?1, ?2)`
- 建表 SQL：`CREATE TABLE IF NOT EXISTS _watch_offsets (path TEXT NOT NULL PRIMARY KEY, byte_offset INTEGER NOT NULL)`
- 路径规范化：`file_offsets` key 使用 `path.canonicalize().unwrap_or_else(|_| path.clone())`，避免相对路径与绝对路径不匹配。
- 成功标准：SC2 "2 秒内触发" — 临时文件方案在 Modify 事件到 handle_run 完成之间增加的 I/O 可忽略（新增字节量通常远小于原始文件大小）。

</specifics>

<deferred>
## Deferred Ideas

- watch + CSV 增量插入 → Out of Scope（CSV 不支持原位增量写）
- watch 多目录监听（WATCH-F02）→ Future phase
- watch 支持 --input CLI override → 超出范围
- 内存 buffer 代替临时文件（大 append 场景性能优化）→ 如有需要可在后续 patch 中实现
- offset 持久化使用独立 state 文件（JSON）→ 已被 STATE.md 决策排除，保持 SQLite 辅助表方案

</deferred>

---

*Phase: 70-Watch 增量处理与集成测试*
*Context gathered: 2026-06-06*
