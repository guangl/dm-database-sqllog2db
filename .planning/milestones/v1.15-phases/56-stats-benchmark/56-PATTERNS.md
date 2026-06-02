# Phase 56: stats 模块清理与 benchmark 稳定化 - Pattern Map

**Mapped:** 2026-06-02
**Files analyzed:** 4 (new/modified files)
**Analogs found:** 4 / 4

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/scanner.rs` | utility/service | streaming (file-I/O) | `src/stats/mod.rs::scan_files_into_accumulator` + `src/cli/run/processor.rs` | role-match (composite) |
| `src/stats/mod.rs` | service | streaming (file-I/O) | `src/cli/run/processor.rs` | role-match |
| `src/cli/run/processor.rs` | service | streaming (file-I/O) | `src/stats/mod.rs` | role-match |
| `benches/BENCHMARKS.md` | config/doc | — | `benches/BENCHMARKS.md` 现有章节 | exact (append only) |

---

## Pattern Assignments

### `src/scanner.rs` (utility, streaming)

**Analog 1:** `src/stats/mod.rs` — `scan_files_into_accumulator`（第 38-68 行）
**Analog 2:** `src/cli/run/processor.rs` — parse error 处理（第 52-58 行 + 143-153 行）

**模块可见性模式** — 参考 `src/lib.rs`（第 6 行）和 `src/parser.rs`（第 1-5 行）：
```rust
// src/lib.rs 中新增一行（pub(crate) 与 parser.rs 保持一致）
pub(crate) mod scanner;
```

**Imports pattern** — 参考 `src/stats/mod.rs`（第 14-16 行）与 `src/cli/run/processor.rs`（第 1-9 行）：
```rust
use crate::error::{Error, ErrorStats, ParserError, Result};
use dm_database_parser_sqllog::LogParserBuilder;
use std::path::PathBuf;
```

**核心函数签名** — 基于 RESEARCH.md §Architecture Patterns：
```rust
// [ASSUMED: 基于 scan_files_into_accumulator + processor.rs 推断]
pub(crate) fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    stats: &mut ErrorStats,
) -> Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
```

**Parser 创建模式**（参考 `src/stats/mod.rs` 第 44-58 行）：
```rust
let file_path_str = file_path.to_str().ok_or_else(|| {
    Error::Parser(ParserError::InvalidPath {
        path: file_path.clone(),
        reason: "non-UTF8 path".to_string(),
        line_number: None,
    })
})?;
let parser = LogParserBuilder::new(file_path_str)
    .build()
    .map_err(|err| {
        Error::Parser(ParserError::InvalidPath {
            path: file_path.clone(),
            reason: format!("{err}"),
            line_number: None,
        })
    })?;
```

**Parse error 处理模式**（对齐 `src/cli/run/processor.rs` 第 143-153 行）：
```rust
for parse_result in parser.iter() {
    match parse_result {
        Ok(record) => on_record(&record),
        Err(err) => {
            stats.add_parse_error();            // 新增：ErrorStats 计数（对齐 run 命令）
            log::warn!("parse error in {}: {err}", file_path.display());  // 保留可观测性
        }
    }
}
```

**文件级 parse error 汇总日志**（参考 `src/cli/run/processor.rs` 第 151-153 行）：
```rust
// 建议：scanner 内部可选汇总，或由调用方在 scan_files 返回后检查 stats.parse_errors
if stats.parse_errors > 0 {
    log::warn!("{}: {} parse errors", file_path.display(), stats.parse_errors);
}
```

**测试模式**（参考 `src/stats/mod.rs` 第 102-212 行）：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorStats;

    #[test]
    fn test_scan_files_counts_parse_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("mixed.log");
        // 含 1 条非法行 + 1 条合法记录
        let content = "this is not a valid log line\n\
            2025-01-15 10:30:28.001 (EP[0] ...) [SEL] SELECT id FROM t. EXECTIME: 5(ms) ...\n";
        std::fs::write(&log_file, content).unwrap();
        let files = vec![log_file];
        let mut records_seen = 0usize;
        let mut stats = ErrorStats::default();
        scan_files(&files, &mut |_| records_seen += 1, &mut stats).unwrap();
        assert_eq!(stats.parse_errors, 1, "parse error should be counted");
        assert_eq!(records_seen, 1, "valid record should pass through");
    }

    #[test]
    fn test_scan_files_returns_err_on_invalid_path() {
        let files = vec![std::path::PathBuf::from("/nonexistent/path/test.log")];
        let mut stats = ErrorStats::default();
        let result = scan_files(&files, &mut |_| {}, &mut stats);
        assert!(result.is_err(), "invalid path should return Err");
    }
}
```

---

### `src/stats/mod.rs` (service, streaming) — 修改现有文件

**修改目标：** 将 `scan_files_into_accumulator`（第 38-68 行）的函数体替换为调用 `scanner::scan_files`，同时让 `run_stats` 能观察到 `ErrorStats`。

**重构后函数体模式**（替换第 41-67 行）：
```rust
fn scan_files_into_accumulator(
    log_files: &[std::path::PathBuf],
    accumulator: &mut StatsAccumulator,
) -> Result<()> {
    let mut scan_stats = crate::error::ErrorStats::default();
    crate::scanner::scan_files(
        log_files,
        &mut |record| accumulator.update(record),
        &mut scan_stats,
    )?;
    if scan_stats.has_errors() {
        log::info!(
            "stats: {} parse error(s) encountered during scan",
            scan_stats.parse_errors
        );
    }
    Ok(())
}
```

**当前现有 imports**（第 14-16 行，无需改动）：
```rust
use crate::config::Config;
use crate::error::{Error, ParserError, Result};
use aggregate::StatsAccumulator;
```

**ErrorStats 引入**（需将 `ErrorStats` 加入 use 声明）：
```rust
use crate::error::{Error, ErrorStats, ParserError, Result};
```

---

### `src/cli/run/processor.rs` (service, streaming) — 可选修改

**修改范围：** 仅内部 parser 创建+迭代循环部分（第 52-153 行），**不改函数签名**（避免触碰 ProgressBar、parallel、normalize 等参数）。

**重构前现有模式**（`src/cli/run/processor.rs` 第 52-58 行，parser 创建）：
```rust
let parser = LogParserBuilder::new(file_path).build().map_err(|e| {
    crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
        path: file_path.into(),
        reason: format!("{e}"),
        line_number: None,
    })
})?;
```

**重构前现有模式**（`src/cli/run/processor.rs` 第 143-153 行，parse error 处理）：
```rust
Err(e) => {
    errors_in_file += 1;
    file_stats.add_parse_error();
    log::warn!("{file_path} | {e:?}");
}
// ...
if errors_in_file > 0 {
    log::warn!("{file_path}: {errors_in_file} parse errors");
}
```

**注意：** RESEARCH.md §Common Pitfalls Pitfall 1 明确指出：若改动超过 `processor.rs` 第 52-68 行范围，应停下重新评估。D-03 属于可选优化，不改函数签名。

---

### `benches/BENCHMARKS.md` (doc) — 追加新节

**追加位置：** 在文件末尾（当前最后一节为"Phase 44"），新增"CI Artifact 使用说明"节。

**现有文档风格**（`benches/BENCHMARKS.md` 第 1-40 行节头格式）：
```markdown
## How to reproduce

```bash
cargo bench --bench bench_csv
```

## How to compare against this baseline

baseline JSON 数据存档在 `benches/baselines/`...
```

**bench.yml artifact 配置**（`.github/workflows/bench.yml` 第 41-46 行）：
```yaml
- name: Upload benchmark artifact
  uses: actions/upload-artifact@v4
  with:
    name: bench-results-${{ github.sha }}
    path: bench-results-*.json
    retention-days: 60
```

**新节应包含内容：**
- artifact 名称格式：`bench-results-{full_sha}`（注意：GitHub Actions 使用完整 SHA，scripts/ 脚本使用 8 位 SHA8）
- 从 GitHub Actions UI 下载方式
- `gh` CLI 下载命令（`gh run download` 方式）
- JSON 文件结构说明（参考 `scripts/collect_bench_results.sh` 输出格式）
- 手动对比历史数据的方法

---

## Shared Patterns

### ErrorStats 计数
**Source:** `src/error.rs` — `ErrorStats::add_parse_error()`（第 46-49 行）
**Apply to:** `src/scanner.rs`（新建时直接使用）
```rust
pub fn add_parse_error(&mut self) {
    self.total_errors += 1;
    self.parse_errors += 1;
}
```

### log::warn! 可观测性
**Source:** `src/cli/run/processor.rs`（第 146 行）和 `src/stats/mod.rs`（第 63 行）
**Apply to:** `src/scanner.rs`（parse error 时保留 warn!）
```rust
// processor.rs 风格（含文件路径前缀）
log::warn!("{file_path} | {e:?}");
// stats 风格（含显示路径）
log::warn!("parse error in {}: {err}", file_path.display());
// 新 scanner 应选择其中一种保持一致，建议使用 stats 风格（更易读）
```

### parse error 不终止语义
**Source:** `src/stats/mod.rs` 第 60-64 行（`match parse_result { Err => warn!, Ok => continue }`）
**Apply to:** `src/scanner.rs`
```rust
for parse_result in parser.iter() {
    match parse_result {
        Ok(record) => on_record(&record),
        Err(err) => {
            stats.add_parse_error();
            log::warn!("parse error in {}: {err}", file_path.display());
            // 不 return / break — 继续处理下一条
        }
    }
}
```

### Error 类型构造（InvalidPath）
**Source:** `src/stats/mod.rs` 第 44-50 行（non-UTF8 path）、第 53-58 行（build 失败）
**Apply to:** `src/scanner.rs`（文件打开/路径错误）
```rust
Error::Parser(ParserError::InvalidPath {
    path: file_path.clone(),
    reason: "non-UTF8 path".to_string(),
    line_number: None,
})
```

### 测试辅助函数模式
**Source:** `src/stats/mod.rs` 第 126-137 行（`write_test_log` helper）
**Apply to:** `src/scanner.rs` 测试中需要临时日志文件时
```rust
fn write_test_log(path: &std::path::Path, count: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 180);
    for idx in 0..count {
        writeln!(buf, "2025-01-15 10:30:28.001 (EP[0] ...) [SEL] SELECT * FROM t. ...",).unwrap();
    }
    std::fs::write(path, buf).unwrap();
}
```

---

## No Analog Found

所有文件均有明确的 analog。无缺失项。

---

## Metadata

**Analog search scope:** `src/stats/`, `src/cli/run/`, `src/`, `benches/`, `.github/workflows/`
**Files scanned:** 7（`src/stats/mod.rs`, `src/cli/run/processor.rs`, `src/error.rs`, `src/lib.rs`, `src/parser.rs`, `benches/BENCHMARKS.md`, `.github/workflows/bench.yml`）
**Pattern extraction date:** 2026-06-02
