# Phase 70: Watch 增量处理与集成测试 - Research

**Researched:** 2026-06-06
**Domain:** Rust incremental file I/O, rusqlite auxiliary table, tempfile-based parse adapter, integration test patterns
**Confidence:** HIGH

## Summary

Phase 70 在 Phase 69 已实现的 notify watcher 框架基础上，为 Modify 事件添加增量读取路径（WATCH-03）并通过 SQLite 辅助表持久化字节偏移实现跨重启幂等性（WATCH-04）。

所有关键技术决策已在 CONTEXT.md 中锁定（D-01 到 D-14）。核心挑战是：`dm-database-parser-sqllog 2.0.2` 的 `LogParserBuilder::build()` 内部执行 `fs::read(&self.path)` 全量读取，无 seek/offset API——这是临时文件方案的根本原因，已由 CONTEXT.md D-01 确认 [VERIFIED: codebase grep]。

本次研究重点在于：验证代码现状与 CONTEXT.md 描述的一致性、识别实现陷阱、确保测试策略可行。

**Primary recommendation:** 按 CONTEXT.md 锁定决策逐步实现，优先完成模块迁移（watch.rs → watch/mod.rs + watch/offsets.rs），再实现 trigger_incremental，最后补充集成测试。

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 增量路径：Seek 到 start_offset → read_to_end → NamedTempFile（.log 后缀）→ LogParser → 处理 → metadata().len() 更新偏移
- **D-02:** Modify 事件前检查 new_size <= start_offset，无新字节则跳过，不计 trigger_count
- **D-03:** Create → trigger_full_file（Phase 69 行为）+ 处理后记录初始偏移；Modify(Data(Content)) → trigger_incremental
- **D-04:** 首次 Modify 但无 file_offsets 记录时：start_offset = 当前文件大小，跳过处理（不触发）
- **D-05:** 辅助表 DDL：`CREATE TABLE IF NOT EXISTS _watch_offsets (path TEXT NOT NULL PRIMARY KEY, byte_offset INTEGER NOT NULL)`，同一 database_url 独立连接
- **D-06:** watch 启动时若有 SQLite 导出器：load_offsets 读取所有行，初始化 WatchLoopState.file_offsets
- **D-07:** 每次 Modify 成功处理后：save_offset 持久化（INSERT OR REPLACE），失败 log::warn! 不中断
- **D-08:** Create 成功后：同样持久化初始偏移到 _watch_offsets
- **D-09:** trigger_incremental 中 tmp_cfg：inputs=[temp_file]，sqlite.append=true，sqlite.overwrite=false
- **D-10:** Create 事件保持用户 config 的 overwrite/append 设置，不强制覆盖
- **D-11:** watch.rs → watch/mod.rs + 新建 watch/offsets.rs（三函数：load_offsets / save_offset / ensure_offset_table）
- **D-12:** WatchLoopState 新增 file_offsets: HashMap<PathBuf, u64> 和 sqlite_db_url: Option<String>
- **D-13:** 集成测试三场景：WATCH-03（追加不重复）、WATCH-04（重启后 offset 恢复，总行数 N+M）、新文件触发
- **D-14:** trigger_incremental / trigger_full_file 提取为 pub(crate)/pub(super) 可独立测试函数

### Claude's Discretion

- 临时文件：`tempfile::Builder::new().prefix("sqllog2db-watch-").suffix(".log").tempfile()?`
- offsets.rs 每次调用开新 rusqlite 连接（不长持）
- u64 偏移，与 fs::metadata().len() 类型一致
- 性能（大 append 场景）暂不优化

### Deferred Ideas (OUT OF SCOPE)

- watch + CSV 增量插入
- watch 多目录监听（WATCH-F02）
- watch --input CLI override
- 内存 buffer 代替临时文件
- 独立 state 文件（JSON）存储 offset
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WATCH-03 | 已有文件追加内容（文件变大）时触发增量处理 | D-01/02/03/04/09/14：Seek+tempfile 方案，handle_event 路由，trigger_incremental 函数 |
| WATCH-04 | SQLite 导出模式下仅插入新行（按字节偏移记录进度，避免重复） | D-05/06/07/08/11/12：_watch_offsets 辅助表，load/save_offset，WatchLoopState 新字段 |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 增量字节读取（Seek+Read） | CLI/Watch 层 | — | 文件 I/O 逻辑在 watch 模块内，不侵入 parser |
| 临时文件创建与 LogParser 适配 | CLI/Watch 层 | tempfile crate | 绕过 parser API 限制 |
| offset 内存缓存 | WatchLoopState | — | 运行时状态，随进程生命周期 |
| offset 持久化 | watch/offsets.rs | rusqlite | 独立 rusqlite 连接，与 SqliteExporter 隔离 |
| 事件路由（Create vs Modify） | handle_event() | — | 已有 EventKind 分支，Phase 70 扩展 |
| append 模式 SQLite 写入 | SqliteExporter | ExporterConfig | overwrite/append 字段已存在 |

## Standard Stack

### Core（已在 Cargo.toml，无需新增）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.39.0 | _watch_offsets 辅助表读写 | [VERIFIED: codebase] 已是生产依赖，features=["bundled"] |
| tempfile | 3.27.0 | NamedTempFile 临时文件 | [VERIFIED: codebase] 已在 dev-dependencies |
| notify | 6.x | 文件系统事件（Phase 69 已用） | [VERIFIED: codebase] |
| std::io::Seek | std | SeekFrom::Start(offset) 跳字节 | [VERIFIED: codebase] std 内置 |

### 重要发现：tempfile 在 dev-dependencies

[VERIFIED: Cargo.toml 第 85 行] `tempfile = "3.27.0"` 位于 `[dev-dependencies]`，不在生产依赖。

**影响：** watch 模块的生产代码（`src/cli/watch/mod.rs`）中调用 `tempfile::NamedTempFile` 时，需要将 `tempfile` 移至 `[dependencies]`，否则 `cargo build --release` 失败（dev-dependencies 在生产构建中不可用）。

这是 **Planning 必须包含的显式任务**：在 Wave 0 或 Task 1 第一步修改 Cargo.toml。

### 无需新增依赖

所有实现所需 crate 均已存在，只需 tempfile 从 dev → 生产依赖。

## Package Legitimacy Audit

本 phase 不新增外部包，仅调整现有依赖的 dev/prod 分类。

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| tempfile | crates.io | 10+ yrs | 极高 | github.com/Stebalien/tempfile | N/A (known) | 从 dev-dep 提升至 dep |
| rusqlite | crates.io | 10+ yrs | 极高 | github.com/rusqlite/rusqlite | N/A (known) | 已在 dep，无需变更 |

**Packages removed due to slopcheck:** none
**Packages flagged as suspicious:** none

## Architecture Patterns

### System Architecture Diagram

```
notify event
    │
    ▼
handle_event()
    ├── EventKind::Create(_) ──────────────────► trigger_full_file()
    │                                                 │
    │                                                 ├── tmp_cfg (inputs=[path], user overwrite/append)
    │                                                 ├── handle_run(tmp_cfg)
    │                                                 └── save_offset(db_url, path, metadata().len())
    │
    └── EventKind::Modify(Data(Content)) ───────► trigger_incremental()
                                                      │
                                                      ├── start_offset = file_offsets[path]
                                                      │     (首次无记录 → start_offset=file_size → 跳过)
                                                      │
                                                      ├── new_size = metadata(path).len()
                                                      │     (new_size <= start_offset → 跳过)
                                                      │
                                                      ├── File::open → seek(SeekFrom::Start(offset))
                                                      ├── read_to_end → Vec<u8>
                                                      ├── NamedTempFile (prefix="sqllog2db-watch-", suffix=".log")
                                                      ├── write bytes → tempfile
                                                      │
                                                      ├── tmp_cfg (inputs=[tempfile_path], sqlite.append=true, overwrite=false)
                                                      ├── handle_run(tmp_cfg)
                                                      │
                                                      ├── file_offsets[path] = new_size  (内存更新)
                                                      └── save_offset(db_url, path, new_size)  (持久化)

watch 启动时（SQLite 导出器存在）:
    offsets::ensure_offset_table(db_url)
    offsets::load_offsets(db_url) → WatchLoopState.file_offsets
```

### Recommended Project Structure

```
src/cli/
├── watch/
│   ├── mod.rs          # 原 watch.rs 内容 + trigger_full_file / trigger_incremental
│   └── offsets.rs      # load_offsets / save_offset / ensure_offset_table
├── run/
│   └── mod.rs          # handle_run（不修改）
└── mod.rs              # pub mod watch（需更新 watch 引用）
```

### Pattern 1: Seek + NamedTempFile 增量读取

**What:** 利用 `std::io::Seek` 跳过已处理字节，将新增内容写入临时文件，绕过 LogParserBuilder 全量读取限制

**When to use:** 每次 `Modify(Data(Content))` 事件且 `new_size > start_offset`

```rust
// [VERIFIED: codebase] std::io::Seek + tempfile::NamedTempFile
use std::io::{Read, Seek, SeekFrom, Write};
use tempfile::Builder;

fn read_new_bytes(path: &Path, start_offset: u64) -> std::io::Result<tempfile::NamedTempFile> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start_offset))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let mut tmp = Builder::new()
        .prefix("sqllog2db-watch-")
        .suffix(".log")
        .tempfile()?;
    tmp.write_all(&buf)?;
    tmp.flush()?;
    Ok(tmp)
}
```

注意：`.suffix(".log")` 确保编码探测逻辑（按文件扩展名判断）正常工作 [ASSUMED：基于 builder.rs 编码探测检查 `str::from_utf8`，不依赖扩展名，但保持一致性]。

### Pattern 2: rusqlite 独立连接操作 offsets

**What:** 每次读写 `_watch_offsets` 表时开新连接，不持有长期连接

**When to use:** `load_offsets`（启动时一次）和 `save_offset`（每次触发后）

```rust
// [VERIFIED: codebase] rusqlite 0.39.0 API
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn ensure_offset_table(database_url: &str) -> rusqlite::Result<()> {
    let conn = Connection::open(database_url)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _watch_offsets (
             path TEXT NOT NULL PRIMARY KEY,
             byte_offset INTEGER NOT NULL
         );",
    )?;
    Ok(())
}

pub(super) fn load_offsets(database_url: &str) -> HashMap<PathBuf, u64> {
    let conn = match Connection::open(database_url) {
        Ok(c) => c,
        Err(e) => { log::warn!("load_offsets: {e}"); return HashMap::new(); }
    };
    let mut stmt = match conn.prepare(
        "SELECT path, byte_offset FROM _watch_offsets"
    ) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),  // 表不存在时正常返回空
    };
    stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let offset: i64 = row.get(1)?;
        Ok((PathBuf::from(path), offset as u64))
    })
    .into_iter()
    .flatten()
    .filter_map(|r| r.ok())
    .collect()
}

pub(super) fn save_offset(database_url: &str, path: &Path, offset: u64) {
    let conn = match Connection::open(database_url) {
        Ok(c) => c,
        Err(e) => { log::warn!("save_offset open: {e}"); return; }
    };
    let result = conn.execute(
        "INSERT OR REPLACE INTO _watch_offsets (path, byte_offset) VALUES (?1, ?2)",
        rusqlite::params![path.to_string_lossy().as_ref(), offset as i64],
    );
    if let Err(e) = result {
        log::warn!("save_offset write: {e}");
    }
}
```

### Pattern 3: WatchLoopState 扩展

```rust
// [VERIFIED: codebase] 现有结构体在 watch.rs:87-104
struct WatchLoopState {
    last_trigger_at: Option<Instant>,
    last_status_refresh: Instant,
    debounce_map: HashMap<PathBuf, Instant>,
    total_stats: ErrorStats,
    trigger_count: u64,
    // Phase 70 新增:
    file_offsets: HashMap<PathBuf, u64>,
    sqlite_db_url: Option<String>,
}
```

### Pattern 4: handle_event 路由扩展

**What:** 在现有 `is_relevant` 判断后，按 EventKind 分支路由到 full_file 或 incremental

```rust
// [VERIFIED: codebase] 现有 handle_event 在 watch.rs:232-273
// Phase 70 将 process_log_path 拆分为按 event.kind 分支的两条路径
let is_create = matches!(event.kind, EventKind::Create(_));
let is_content_modify = matches!(
    event.kind,
    EventKind::Modify(ModifyKind::Data(DataChange::Content))
);
```

### Anti-Patterns to Avoid

- **不要将 SqliteExporter 连接传入 offsets.rs：** SqliteExporter 持有事务（`BEGIN TRANSACTION`），共用连接会引发 SQLite locking 冲突。独立连接是 CONTEXT.md D-05 的原因。
- **不要在 trigger_incremental 中遗漏 tmp_cfg.sqlite.append=true：** 若 append=false，每次 Modify 触发都会 DELETE FROM 表，清空所有历史数据。
- **不要在首次 Modify 无 offset 记录时触发处理：** 应设 start_offset=file_size 并跳过，否则重复处理 watch 启动前的历史内容。
- **不要用相对路径作 file_offsets key：** 使用 `path.canonicalize().unwrap_or_else(|_| path.to_path_buf())` 确保路径规范化。
- **NamedTempFile 不要提前 drop：** 临时文件必须在 `handle_run` 调用完成后才可 drop，否则 LogParser 读取时文件已删除。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 临时文件创建与自动删除 | 手动 `fs::write` + `fs::remove_file` | `tempfile::NamedTempFile` | drop 时自动删除，panic 安全；已在 dev-dep，仅需提升 |
| SQLite 参数绑定 | 字符串拼接 SQL | `rusqlite::params![]` | 防止 SQL 注入，类型安全 |
| 文件大小查询 | 手动 read+count | `fs::metadata(path)?.len()` | 零开销，系统调用直接返回 |
| offset 类型 | i32/usize | u64 | 与 `metadata().len()` 返回类型一致，避免截断 |

## Common Pitfalls

### Pitfall 1: tempfile 在 dev-dependencies 导致生产构建失败

**What goes wrong:** `cargo build --release` 报 `use of undeclared crate or module 'tempfile'`

**Why it happens:** `tempfile = "3.27.0"` 目前在 `[dev-dependencies]`（Cargo.toml:85），生产代码无法使用

**How to avoid:** 在实现 trigger_incremental 之前，将 tempfile 移至 `[dependencies]`

**Warning signs:** `cargo build --release` 失败但 `cargo test` 通过

### Pitfall 2: rusqlite 整数存储 u64 溢出

**What goes wrong:** `byte_offset` 超过 i64::MAX（约 9.2EB）时 `as i64` 溢出为负数，load 时 `offset as u64` 恢复错误值

**Why it happens:** SQLite INTEGER 是 64-bit signed；rusqlite `get::<_, i64>()` 返回有符号整数

**How to avoid:** 文件大小实际不超过 i64::MAX（约 9.2EB 是不现实的），但仍应在 load_offsets 中过滤负值：`if offset >= 0 { Some((path, offset as u64)) } else { None }`

**Warning signs:** offset 读回为极大 u64 值，导致所有 Modify 事件跳过（new_size <= start_offset 恒真）

### Pitfall 3: NamedTempFile drop 时机过早

**What goes wrong:** `handle_run` 尝试打开临时文件时报 "No such file or directory"

**Why it happens:** 如果 `NamedTempFile` 在 `handle_run` 调用前就超出作用域（如被 `let _ = ...` 忽略）

**How to avoid:** 确保 `tmp_file` 变量持有到 `handle_run` 完成后，使用显式 `let tmp_file = ...;` 不要用 `let _ =`

**Warning signs:** `handle_run` 返回 `Err(Io(...: No such file))` 且只在 watch 路径触发

### Pitfall 4: SqliteExporter EXCLUSIVE locking 模式与辅助表冲突

**What goes wrong:** `save_offset` 开独立连接时报 `SQLITE_BUSY` 或 `database is locked`

**Why it happens:** `initialize_pragmas` 设置了 `PRAGMA locking_mode = EXCLUSIVE`（sqlite/mod.rs:36），SqliteExporter 持有独占锁期间其他连接无法写入

**How to avoid:** `save_offset` 应在 `handle_run` 完成（SqliteExporter.finalize() 调用后）再调用——`handle_run` 返回后 SqliteExporter 已 drop，连接已关闭，锁已释放。按照 trigger_incremental 的正常流程（先 handle_run，再 save_offset）这自然满足。

**Warning signs:** `save_offset` 偶发 `log::warn!("save_offset write: database is locked")`

### Pitfall 5: 模块迁移时 src/cli/mod.rs 未更新

**What goes wrong:** 编译报 `mod watch` 找不到模块

**Why it happens:** Rust 模块系统：`watch.rs` 改为 `watch/mod.rs` 后，`src/cli/mod.rs` 中的 `pub mod watch;` 声明无需修改（Rust 自动识别两种形式），但如果原来是 `pub mod watch` 且有 `use super::watch::handle_watch` 等引用，重命名后路径不变，不需要修改调用方。

**How to avoid:** 直接 `git mv src/cli/watch.rs src/cli/watch/mod.rs` 后创建 `offsets.rs`，编译验证无报错

**Warning signs:** `error[E0583]: file not found for module`（实际上此错误不会出现，因为 Rust 两种方式都支持）

### Pitfall 6: 集成测试需要真实文件事件（无法用单元测试替代 SC4）

**What goes wrong:** 测试 WATCH-03/04 时直接调用 `trigger_incremental` 函数（不启动 notify watcher），避免了 macOS FSEvents 合并问题

**Why it happens:** Phase 69 已有 `test_watch_triggers_on_new_log_file` 被标记为 `ignored`（FSEvents 在 CI 环境合并事件不可靠）。Phase 70 的集成测试**不应**依赖 notify watcher 触发，而是直接调用 trigger_full_file / trigger_incremental

**How to avoid:** CONTEXT.md D-14 明确要求：将两个 trigger 函数提取为 `pub(crate)` 可独立调用的函数，测试直接调用这些函数，不启动 watcher

## Code Examples

### 集成测试框架（参考 tests/integration.rs 既有模式）

```rust
// [VERIFIED: codebase] tests/integration.rs 的 write_test_log helper 模式
// Phase 70 新增测试文件：tests/watch_incremental.rs 或在 tests/integration.rs 新增模块

#[cfg(test)]
mod watch_incremental_tests {
    use std::sync::{Arc, atomic::AtomicBool};
    use tempfile::{TempDir, NamedTempFile};

    // WATCH-03 测试：追加处理不重复
    #[test]
    fn test_incremental_appends_only_new_rows() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test.db");
        let log_path = tmp_dir.path().join("test.log");

        // 写入 N=10 条记录，全文触发，断言 DB 有 10 行
        write_test_log(&log_path, 10);
        let offset_after_full = std::fs::metadata(&log_path).unwrap().len();
        // trigger_full_file(...)

        // 追加 M=5 条记录，增量触发，断言 DB 有 15 行（不重复）
        append_test_log(&log_path, 5, 10);
        // trigger_incremental(start_offset=offset_after_full, ...)
        // assert DB row count == 15
    }

    // WATCH-04 测试：重启后 offset 恢复
    #[test]
    fn test_offset_persists_across_restart() {
        // 写 N 条 → trigger → load_offsets 验证 → 重建 WatchLoopState
        // → 追加 M 条 → trigger → 断言总行 N+M
    }
}
```

### 测试辅助函数 append_test_log

```rust
// 追加写（不覆盖），用于 WATCH-03/04 测试
fn append_test_log(path: &std::path::Path, count: usize, start_id: usize) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap();
    for n in 0..count {
        let i = start_id + n;
        writeln!(
            file,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:TESTUSER trxid:{i} \
             stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. \
             EXECTIME: {}(ms) ROWCOUNT: {}(rows) EXEC_ID: {i}.",
            (i * 13) % 1000, i % 100,
        ).unwrap();
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| watch.rs 单文件 | watch/mod.rs + watch/offsets.rs | Phase 70（本次） | 关注点分离，offset 逻辑隔离 |
| 全量文件触发 | Create→全量 / Modify→增量 | Phase 70（本次） | 避免重复处理 |
| 无 offset 持久化 | _watch_offsets 辅助表 | Phase 70（本次） | 跨重启幂等 |

## Open Questions

1. **SqliteExporter 的 EXCLUSIVE locking 模式是否影响 ensure_offset_table**
   - What we know: initialize_pragmas 设 `PRAGMA locking_mode = EXCLUSIVE`；SqliteExporter 在 handle_run 内部创建并 drop
   - What's unclear: watch 启动时调用 `ensure_offset_table` 是否与已运行的 SqliteExporter 冲突
   - Recommendation: watch 启动时 SqliteExporter 尚未创建（handle_run 还未调用），所以 ensure_offset_table 在 watch 启动阶段调用是安全的。每次 save_offset 在 handle_run 返回后调用，此时 SqliteExporter 已 drop，也安全。规划任务时明确调用顺序即可。

2. **handle_event 签名需要传入 WatchLoopState 还是拆分字段**
   - What we know: 当前 handle_event 接受多个 `&mut` 字段（总 9 个参数），D-12 新增 file_offsets 和 sqlite_db_url
   - What's unclear: 是将新字段加入参数列表，还是重构为传入 `&mut WatchLoopState`
   - Recommendation: CLAUDE.md 要求函数 ≤ 40 行。当前 handle_event 已有 9 参数，加 2 个会超过 `too_many_arguments`（虽然已 allow）。建议将相关参数合并为传入 `&mut WatchLoopState`，同时简化 run_watch_loop。这也是更清晰的设计，与 D-14 的函数可测试性目标一致。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | 构建 | ✓ | 1.94.0 | — |
| rusqlite (bundled) | offsets.rs | ✓ | 0.39.0 | — |
| tempfile | trigger_incremental | ✓ (dev-dep) | 3.27.0 | 无（需提升至 dep） |

**Missing dependencies with no fallback:**
- tempfile 需从 dev-dependencies 提升至 dependencies（否则 release 构建失败）

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + integration tests |
| Config file | Cargo.toml（[[test]] 隐式） |
| Quick run command | `cargo test -p dm-database-sqllog2db watch` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WATCH-03 | Modify 触发仅处理新增字节，不重复 N 条 | integration | `cargo test test_incremental_appends_only_new_rows` | ❌ Wave 0 |
| WATCH-03 | new_size <= start_offset 时跳过 | unit | `cargo test test_trigger_incremental_skips_if_no_new_bytes` | ❌ Wave 0 |
| WATCH-04 | save_offset 后 load_offsets 返回正确值 | unit | `cargo test test_save_and_load_offset_roundtrip` | ❌ Wave 0 |
| WATCH-04 | 重启后 offset 恢复，追加不重复 | integration | `cargo test test_offset_persists_across_restart` | ❌ Wave 0 |
| WATCH-04 | ensure_offset_table 幂等（多次调用不报错） | unit | `cargo test test_ensure_offset_table_idempotent` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p dm-database-sqllog2db watch` （只跑 watch 相关测试）
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** 全套测试 + clippy 无 warning

### Wave 0 Gaps

- [ ] `src/cli/watch/offsets.rs` — load/save/ensure 函数及其单元测试
- [ ] `tests/watch_incremental.rs` 或 `tests/integration.rs` 内 watch_incremental 模块 — WATCH-03/04 集成测试
- [ ] `append_test_log` helper 函数（追加写，供集成测试使用）

## Security Domain

本 phase 不涉及网络、认证、用户输入验证或加密。唯一的安全相关点：

- **临时文件路径：** `tempfile::NamedTempFile` 使用 OS 临时目录，路径不可预测，无路径遍历风险 [VERIFIED: tempfile crate 设计]
- **SQLite 参数绑定：** 使用 `rusqlite::params![]` 而非字符串拼接，无 SQL 注入风险

ASVS V5（输入验证）：path 来自 notify 事件（OS 内核），不是外部用户输入，风险极低。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `.suffix(".log")` 对编码探测无影响（builder.rs 按内容探测，不依赖扩展名） | Code Examples | 影响极低：即使有影响，只需去掉 suffix 或改其他后缀 |
| A2 | macOS FSEvents 合并事件问题不影响直接调用 trigger_incremental 的集成测试 | Validation Architecture | 影响极低：测试绕过 notify watcher 直接调用函数 |

## Sources

### Primary (HIGH confidence)
- `src/cli/watch.rs` — 完整 Phase 69 实现，532 行，验证了所有 CONTEXT.md 代码引用
- `Cargo.toml` — 依赖版本确认（tempfile 在 dev-dep，rusqlite 0.39.0 在 dep）
- `src/exporter/sqlite/mod.rs` — SqliteExporter append/overwrite/locking 实现
- `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.2/src/parser/builder.rs` — 确认 `fs::read()` 全量读取，无 offset API
- `tests/integration.rs` — 集成测试既有模式（write_test_log, Config 构建方式）

### Secondary (MEDIUM confidence)
- `src/config/exporter.rs` — SqliteExporterConfig 字段结构（append/overwrite bool）

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 所有依赖已在 Cargo.toml 确认版本
- Architecture: HIGH — CONTEXT.md 锁定决策 + 代码现状完全吻合
- Pitfalls: HIGH — 基于实际代码路径（initialize_pragmas EXCLUSIVE 锁、NamedTempFile drop 语义、tempfile dev-dep 限制）

**Research date:** 2026-06-06
**Valid until:** 2026-07-06（依赖稳定，30 天有效）
