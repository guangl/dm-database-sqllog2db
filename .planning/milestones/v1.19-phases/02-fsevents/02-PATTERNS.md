# Phase 02: 测试覆盖率与 FSEvents - Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 2（新增/修改文件）
**Analogs found:** 2 / 2

## File Classification

| 新增/修改文件 | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `tests/watch_incremental.rs`（追加 3 个测试函数） | test | file-I/O + event-driven | `tests/watch_incremental.rs` 现有 `test_watch_03_*` / `test_watch_04_*` | exact |
| `src/cli/run/tests.rs`（追加 collector 单元测试） | test | file-I/O | `src/cli/run/tests.rs` 现有 `test_parallel_merge_consistent` / `test_filter_path` | exact |

---

## Pattern Assignments

### `tests/watch_incremental.rs` — WATCH-07/08/09 集成测试

**Analog:** `tests/watch_incremental.rs`（现有 WATCH-03/04 测试函数）  
**Analog for helper structs:** `src/cli/watch/mod.rs::tests`（WATCH-07/08/09 单元测试，lines 890-998）

---

**Imports pattern（文件顶部 lines 4-13）:**
```rust
use dm_database_sqllog2db::cli::watch::{WatchLoopState, trigger_full_file, trigger_incremental};
use dm_database_sqllog2db::config::{Config, ExporterConfig, SqliteExporterConfig, SqllogConfig};
use indicatif::{ProgressBar, ProgressDrawTarget};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;
```

WATCH-09 额外需要：
```rust
use dm_database_sqllog2db::cli::watch::handle_watch;
use dm_database_sqllog2db::error::Error;
```

---

**Helper pattern（`build_pb`，lines 70-74）:**
```rust
fn build_pb() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_draw_target(ProgressDrawTarget::hidden());
    pb
}
```

---

**WATCH-07 核心模式（analog: `src/cli/watch/mod.rs` lines 896-936）:**

从 watch/mod.rs 的 `test_watch_csv_append` 提取：用 `toml::from_str` 构建 Config，两次 `trigger_full_file`，断言 CSV 行数 + header 唯一性。

集成测试版本改用 `write_test_log_records` helper（已存在），Config 构建改用 `build_csv_config`（新增 helper）：
```rust
// 新增 helper（watch_incremental.rs 内部）
fn build_csv_config(log_path: &Path, csv_path: &Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_path.to_string_lossy().into_owned()],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(dm_database_sqllog2db::config::CsvExporterConfig {
                file: csv_path.to_string_lossy().into_owned(),
                overwrite: true,
                append: false,
                include_performance_metrics: true,
            }),
            sqlite: None,
        },
        ..Config::default()
    }
}
```

WATCH-07 测试体模式（analog: lines 917-936）：
```rust
#[test]
fn test_watch_07_csv_append() {
    let tmp = TempDir::new().unwrap();
    let log_path_a = tmp.path().join("a.log");
    let log_path_b = tmp.path().join("b.log");
    let csv_path = tmp.path().join("out.csv");

    write_test_log_records(&log_path_a, 0, 3);
    write_test_log_records(&log_path_b, 3, 3);

    let cfg = build_csv_config(&log_path_a, &csv_path);
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(&log_path_a, &cfg, true, false, &interrupted, &mut state, &pb);
    trigger_full_file(&log_path_b, &cfg, true, false, &interrupted, &mut state, &pb);

    let content = std::fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 7, "应有 header + 6 rows，实际 {}", lines.len());
    let header_count = lines.iter().filter(|&&l| l == lines[0]).count();
    assert_eq!(header_count, 1, "header 应仅出现一次，实际 {header_count} 次");
}
```

---

**WATCH-08 核心模式（analog: `src/cli/watch/mod.rs` lines 938-978）:**

需新增常量（参照 watch/mod.rs line 893）：
```rust
const INVALID_LOG_LINE: &str = "this is not a valid dm sql log line at all\n";
```

WATCH-08 Config 构造要包含 `error` 字段，参照 watch/mod.rs lines 953-959：
```rust
// toml 字符串加入 [error] 段，或使用 Config 字段直接构建：
use dm_database_sqllog2db::config::ErrorLogConfig;
// cfg.error = Some(ErrorLogConfig { file: error_log_path.to_string_lossy().into_owned() });
```

断言模式（analog: lines 967-977）：
```rust
assert!(error_log_path.exists(), "error log 应在有解析错误时创建");
let error_content = std::fs::read_to_string(&error_log_path).unwrap();
let error_line_count = error_content.lines().filter(|l| l.starts_with("[ERROR]")).count();
assert!(error_line_count >= 2, "应含 2 条 [ERROR]，实际 {error_line_count}\n{error_content}");
```

---

**WATCH-09 核心模式（analog: `src/cli/watch/mod.rs` lines 981-997）:**

```rust
#[test]
fn test_watch_09_exit_code_130() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("out.csv");
    let cfg = build_csv_config(tmp.path(), &csv_path);
    let interrupted = Arc::new(AtomicBool::new(true)); // 预设已中断
    let result = handle_watch(&cfg, true, false, &interrupted);
    assert!(
        matches!(result, Err(Error::Interrupted)),
        "interrupted=true 时 handle_watch 应返回 Err(Error::Interrupted)，实际: {result:?}"
    );
}
```

---

### `src/cli/run/tests.rs` — collector.rs 单元测试

**Analog:** `src/cli/run/tests.rs`（现有测试，lines 1-170）  
**Analog for access pattern:** `src/cli/run/collector.rs`（pub(super) 函数签名）

---

**Imports pattern（文件 lines 1-2，已有）:**
```rust
use super::*;
use crate::config::Config;
```
collector 测试无需修改 use，`super::*` 已隐式包含 `collector` 子模块。

---

**核心访问模式（analog: tests.rs `test_parallel_merge_consistent`，多文件触发并行路径）:**

collector 单元测试直接调用 `collector::collect_log_file`（`pub(super)` 在此可见）：
```rust
// 签名（src/cli/run/collector.rs lines 18-24）：
pub(super) fn collect_log_file(
    file: &Path,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(Sqllog, Option<String>)>, ErrorStats)>
```

---

**Group 1 — InvalidPath 错误路径测试（collector.rs lines 26-34 对应）:**
```rust
#[test]
fn test_collector_invalid_path_returns_error() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = collector::collect_log_file(
        std::path::Path::new("/nonexistent/absolutely/not/there.log"),
        &pipeline,
        false,
        None,
        &interrupted,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::Error::Parser(crate::error::ParserError::InvalidPath { .. })
    ));
}
```

---

**Group 2 — parse error 累积路径测试（collector.rs lines 41-63 对应）:**

触发条件：日志文件含无效行，参照 watch/mod.rs line 893 `DM_LOG_LINE_GARBAGE`。
```rust
#[test]
fn test_collector_parse_error_accumulation() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("bad.log");
    std::fs::write(&log_path, "not a valid log line\nalso invalid\n").unwrap();
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = collector::collect_log_file(&log_path, &pipeline, false, None, &interrupted);
    let (rows, stats) = result.unwrap();
    assert!(rows.is_empty());
    assert!(stats.parse_error_count() > 0);
    assert!(!stats.parse_error_records.is_empty());
}
```

---

**Group 3 — !needs_processing 过滤分支（collector.rs lines 91-93 对应）:**

触发条件：非空 Pipeline（`!pipeline.is_empty()`）且 record 不是 PARAMS 行（`record.tag.is_some()`）且 `pipeline.run_with_meta` 返回 false，使 `passes=false` 且 `needs_processing=false`。

参照构造过滤器的模式，查阅 `src/pipeline/filters/mod.rs` 的 builder API（username 过滤器，用一个日志文件里用户名不匹配的记录）：
```rust
// 在 tests.rs 的已有 test_filter_path 基础上，改用 handle_run 的两文件目录
// 或直接调用 collector::collect_log_file + 带 username 过滤器的 Pipeline
```

---

**Group 4 — filtered PARAMS else 分支（collector.rs lines 109-119 对应）:**

触发条件：`passes=false` 且 `do_normalize=true` 且 `record.tag.is_none()`（PARAMS 行被过滤）。需要构造 `do_normalize=true` + 有效 PARAMS 行 + 不匹配过滤器的 Pipeline。

---

**error handling pattern（analog: tests.rs lines 67-68 的 `assert!(result.is_ok(...))` 模式）:**
```rust
// 成功路径
assert!(result.is_ok(), "应成功: {result:?}");
// 错误路径（Error::Parser 变体）
assert!(matches!(result.unwrap_err(), crate::error::Error::Parser(...)));
```

---

## Shared Patterns

### TempDir + 文件写入
**Source:** `tests/watch_incremental.rs` lines 19-36（`write_test_log_records`）
**Apply to:** 所有新测试函数
```rust
let tmp = TempDir::new().unwrap();
let log_path = tmp.path().join("some.log");
// TempDir drop 时自动清理，无需 manual cleanup
```

### AtomicBool interrupted 参数
**Source:** `tests/watch_incremental.rs` lines 102-103
**Apply to:** 所有调用 `trigger_full_file` / `collect_log_file` 的测试
```rust
let interrupted = Arc::new(AtomicBool::new(false));
```

### WatchLoopState 初始化
**Source:** `tests/watch_incremental.rs` lines 104 / `src/cli/watch/mod.rs` line 915
**Apply to:** 所有调用 `trigger_full_file` / `trigger_incremental` 的测试
```rust
let mut state = WatchLoopState::new(HashMap::new(), None); // CSV-only 时 sqlite_db_url=None
```

### toml::from_str Config 构建（watch/mod.rs 测试风格）
**Source:** `src/cli/watch/mod.rs` lines 907-912
**Apply to:** WATCH-08 的 error log 配置（需要 `[error]` 段）

---

## No Analog Found

无。所有目标文件均有高质量 analog。

---

## Metadata

**Analog search scope:** `tests/watch_incremental.rs`, `src/cli/run/tests.rs`, `src/cli/run/collector.rs`, `src/cli/watch/mod.rs`
**Files scanned:** 4
**Pattern extraction date:** 2026-06-07
