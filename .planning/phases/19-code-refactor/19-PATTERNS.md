# Phase 19: 代码结构重构 - Pattern Map

**Mapped:** 2026-05-18
**Files analyzed:** 18（5 个拆分目标 + 13 个新建子文件）
**Analogs found:** 18 / 18

---

## File Classification

| 新建/修改文件 | Role | Data Flow | 最近似 Analog | 匹配质量 |
|---|---|---|---|---|
| `src/pipeline/filters/mod.rs` | module-entry | transform | `src/config/mod.rs` (lines 1-9) | exact |
| `src/pipeline/filters/types.rs` | model | transform | `src/config/exporter.rs` | role-match |
| `src/pipeline/filters/compiled.rs` | service | transform | `src/pipeline/filters.rs:368-596` | exact |
| `src/pipeline/filters/serde_helpers.rs` | utility | transform | `src/pipeline/filters.rs:23-39` | exact |
| `src/config/mod.rs`（精简） | module-entry | request-response | 现有文件自身 lines 1-16 | self |
| `src/config/validate.rs` | service | request-response | `src/config/logging.rs:38-64` | role-match |
| `src/config/apply_one.rs` | service | request-response | `src/config/mod.rs:129-316` | exact |
| `src/exporter/projection.rs` | utility | transform | `src/pipeline/mod.rs:196-209` | role-match |
| `src/exporter/mod.rs`（清理后） | module-entry | request-response | 现有文件自身 lines 64-144 | self |
| `src/exporter/csv/mod.rs` | module-entry | file-I/O | `src/config/mod.rs` lines 1-9 | role-match |
| `src/exporter/csv/writer.rs` | service | file-I/O | `src/exporter/csv.rs:157-354` | exact |
| `src/exporter/csv/companion.rs` | utility | file-I/O | `src/exporter/csv.rs:25-93` | exact |
| `src/exporter/sqlite/mod.rs` | module-entry | file-I/O | `src/config/mod.rs` lines 1-9 | role-match |
| `src/exporter/sqlite/sql_builder.rs` | utility | transform | `src/exporter/sqlite.rs:71-115` | exact |
| `src/exporter/sqlite/write.rs` | service | file-I/O | `src/exporter/sqlite.rs:153-276` | exact |
| `src/cli/run/mod.rs` | module-entry | request-response | `src/config/mod.rs` lines 1-9 | role-match |
| `src/cli/run/processor.rs` | service | event-driven | `src/cli/run.rs:116-332` | exact |
| `src/cli/run/prescan.rs` | service | batch | `src/cli/run.rs:334-418` | exact |
| `src/cli/run/parallel.rs` | service | batch | `src/cli/run.rs:423-683` | exact |

---

## Pattern Assignments

### `src/pipeline/filters/mod.rs` (module-entry)

**Analog:** `src/config/mod.rs` lines 1-9

**re-export 模式**（lines 1-9）：
```rust
pub mod exporter;
pub mod logging;
pub mod resume;
pub mod sqllog;

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use resume::ResumeConfig;
pub use sqllog::SqllogConfig;
```

**实际应用（filters/mod.rs）：**
```rust
mod types;
mod compiled;
mod serde_helpers;   // 私有，无需 re-export

// 保持 crate::pipeline::filters::* 路径稳定
pub(crate) use types::{
    FiltersFeature, IncludeFilters, ExcludeFilters,
    RecordMeta, SqlFilters, IndicatorFilters,
};
pub(crate) use compiled::{CompiledMetaFilters, CompiledSqlFilters};
```

**可见性规则：** 原 `pub` 改 `pub(crate)`；binary crate 无外部消费者（D-10/D-11）。

---

### `src/pipeline/filters/types.rs` (model, transform)

**Analog:** `src/pipeline/filters.rs` lines 1-363（类型定义区）

**Imports 模式**（analog 文件 lines 1-4）：
```rust
use ahash::HashSet as AHashSet;
use compact_str::CompactString;
use regex::Regex;
use serde::{Deserialize, Deserializer};
```

**Serde 类型定义模式**（lines 42-64）：
```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IncludeFilters {
    #[serde(default)]
    pub users: Option<Vec<String>>,
    // ...
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}
```

**手写 Deserialize 模式**（lines 185-263）：
```rust
impl<'de> Deserialize<'de> for FiltersFeature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawFiltersFeature::deserialize(d)?;
        Ok(FiltersFeature::from(raw))
    }
}
impl From<RawFiltersFeature> for FiltersFeature { ... }
```

**可见性收紧目标：**
- `pub struct RecordMeta` → `pub(crate) struct RecordMeta`
- 所有字段 `pub field: ...` → `pub(crate) field: ...`
- `pub struct IncludeFilters` → `pub(crate) struct IncludeFilters`
- `pub struct ExcludeFilters` → `pub(crate) struct ExcludeFilters`
- `pub struct FiltersFeature` → `pub(crate) struct FiltersFeature`
- `pub struct IndicatorFilters` → `pub(crate) struct IndicatorFilters`
- `pub struct SqlFilters` → `pub(crate) struct SqlFilters`
- `pub fn has_filters` → `pub(crate) fn has_filters`

---

### `src/pipeline/filters/compiled.rs` (service, transform)

**Analog:** `src/pipeline/filters.rs` lines 368-596（CompiledMetaFilters / CompiledSqlFilters 区域）

**Imports 模式：**
```rust
use super::types::{ExcludeFilters, IncludeFilters, RecordMeta, SqlFilters};
use super::serde_helpers::compile_patterns;
use ahash::HashSet as AHashSet;
use compact_str::CompactString;
use regex::Regex;
```

**Core 模式——CompiledMetaFilters**（lines 368-547）：
```rust
pub struct CompiledMetaFilters {
    pub usernames: Option<Vec<Regex>>,
    // ...（15 个字段）
}

impl CompiledMetaFilters {
    pub fn try_from_include_exclude(
        include: &IncludeFilters,
        exclude: &ExcludeFilters,
    ) -> crate::error::Result<Self> { ... }

    pub fn has_filters(&self) -> bool { ... }
    pub fn has_any_filters(&self) -> bool { ... }

    #[inline]
    pub fn should_keep(&self, meta: &RecordMeta) -> bool { ... }
}
```

**可见性收紧目标：**
- 所有 `pub struct CompiledMetaFilters` → `pub(crate) struct CompiledMetaFilters`
- 所有字段 `pub usernames: ...` → `pub(crate) usernames: ...`
- `pub fn try_from_include_exclude` → `pub(crate) fn try_from_include_exclude`
- `pub fn has_filters` → `pub(crate) fn has_filters`
- `pub fn has_any_filters` → `pub(crate) fn has_any_filters`
- `pub fn should_keep` → `pub(crate) fn should_keep`
- `pub struct CompiledSqlFilters` → `pub(crate) struct CompiledSqlFilters`

---

### `src/pipeline/filters/serde_helpers.rs` (utility, transform)

**Analog:** `src/pipeline/filters.rs` lines 23-39 + 333-363（私有辅助函数）

**模式：**
```rust
use ahash::HashSet as AHashSet;
use compact_str::CompactString;
use regex::Regex;
use serde::Deserializer;

// 私有模块，所有项不带 pub

type TrxidSet = AHashSet<CompactString>;

fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where D: Deserializer<'de> { ... }

fn vec_to_i64_hashset<'de, D>(deserializer: D) -> Result<Option<AHashSet<i64>>, D::Error>
where D: Deserializer<'de> { ... }

fn compile_patterns(
    field: &str,
    patterns: Option<&[String]>,
) -> crate::error::Result<Option<Vec<Regex>>> { ... }

fn match_any_regex(patterns: Option<&[Regex]>, val: &str) -> bool { ... }
```

**注意：** `TrxidSet` 类型别名也应迁移到此文件，供 `types.rs` 导入使用；或在 `types.rs` 中重定义。

---

### `src/config/validate.rs` (service, request-response)

**Analog:** `src/config/mod.rs` lines 63-401（validate 方法区）

**Imports 模式：**
```rust
use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{ChartsConfig, FiltersFeature, NormalizeConfig, OutputConfig, TemplateConfig};
```

**Core 模式——validate 方法群**（lines 63-401）：
```rust
impl Config {
    pub fn validate(&self) -> Result<()> { ... }

    pub fn validate_and_compile(&self) -> Result<Option<(CompiledMetaFilters, CompiledSqlFilters)>> { ... }

    fn validate_filter(&self) -> Result<()> { ... }

    fn validate_output_fields(&self) -> Result<()> { ... }

    fn validate_template(&self) -> Result<()> { ... }

    fn validate_charts(&self) -> Result<()> { ... }
}
```

**可见性收紧目标：**
- `pub fn validate` → `pub(crate) fn validate`
- `pub fn validate_and_compile` → `pub(crate) fn validate_and_compile`
- 所有私有 `fn validate_*` 保持私有（无 pub）

**注意：** 89 个测试函数大部分测 validate 逻辑，随对应方法迁移到 `validate.rs` 的 `#[cfg(test)]` 块。测试中的 `use super::*` 需改为 `use super::Config; use crate::config::*;`。

---

### `src/config/apply_one.rs` (service, request-response)

**Analog:** `src/config/mod.rs` lines 129-316（apply_overrides/apply_one 区域）

**Core 模式**（lines 131-316）：
```rust
impl Config {
    pub fn apply_overrides(&mut self, overrides: &[String]) -> Result<()> {
        for item in overrides {
            let (key, value) = item.split_once('=').ok_or_else(|| {
                Error::Config(ConfigError::InvalidValue { ... })
            })?;
            self.apply_one(key, value)?;
        }
        Ok(())
    }

    fn apply_one(&mut self, key: &str, value: &str) -> Result<()> {
        let unknown = || { Error::Config(ConfigError::InvalidValue { ... }) };
        let parse_bool = |v: &str| -> Result<bool> { match v { ... } };
        match key {
            "sqllog.path" | ... => self.sqllog.path = value.to_string(),
            ...
            _ => return Err(unknown()),
        }
        Ok(())
    }
}
```

**可见性目标：**
- `pub fn apply_overrides` → `pub(crate) fn apply_overrides`
- `fn apply_one` 保持私有（`pub(super)` 仅当 validate.rs 需要访问时）

---

### `src/config/mod.rs`（精简后）

**精简后仅保留：**
```rust
pub mod exporter;
pub mod logging;
pub mod resume;
pub mod sqllog;
mod validate;   // 私有子模块，通过 mod.rs re-export Config 方法
mod apply_one;  // 私有子模块

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use resume::ResumeConfig;
pub use sqllog::SqllogConfig;

// Config struct + from_file（~80 行）
pub struct Config { ... }
impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> { ... }
}
```

**可见性目标：** `pub struct Config` → `pub(crate) struct Config`（binary crate 无外部访问）

---

### `src/exporter/projection.rs` (utility, transform)

**Analog:** `src/pipeline/mod.rs` lines 196-209（`ordered_field_indices` 的映射逻辑）

**Analog 代码**（lines 200-209）：
```rust
pub fn ordered_field_indices(&self) -> Vec<usize> {
    match &self.fields {
        None => (0..FIELD_NAMES.len()).collect(),
        Some(names) if names.is_empty() => (0..FIELD_NAMES.len()).collect(),
        Some(names) => names
            .iter()
            .filter_map(|name| FIELD_NAMES.iter().position(|&n| n == name.as_str()))
            .collect(),
    }
}
```

**新文件内容（来自 CONTEXT.md D-05）：**
```rust
// src/exporter/projection.rs
use crate::pipeline::FIELD_NAMES;

pub(crate) fn projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str> {
    ordered_indices.iter().map(|&i| FIELD_NAMES[i]).collect()
}
```

**调用方：**
- `sqlite/sql_builder.rs::build_create_sql` 的 `cols` 构造
- `sqlite/sql_builder.rs::build_insert_sql` 的 `cols` 构造
- **不在** `writer.rs::write_record_preparsed` 中调用（热路径，避免 Vec 分配）

---

### `src/exporter/csv/mod.rs` (module-entry, file-I/O)

**Analog:** `src/config/mod.rs` 的 re-export 模式 + `src/exporter/csv.rs` lines 97-152

**re-export 模式：**
```rust
mod writer;
mod companion;

pub(crate) use companion::write_companion_rows;

pub(crate) struct CsvExporter { ... }

impl CsvExporter {
    pub(crate) fn new(path: impl AsRef<Path>) -> Self { ... }
    pub(crate) fn from_config(config: &config::CsvExporterConfig) -> Self { ... }
}

impl Exporter for CsvExporter { ... }  // 委托到 writer::
```

**注意：** `write_record_preparsed` 是静态方法（analog lines 157-353），接受独立 `&mut` 参数而非 `&mut self`，是规避 Rust 借用检查的关键设计。拆分时**必须保持该签名不变**：
```rust
// src/exporter/csv.rs line 157-165（必须保持）
pub(crate) fn write_record_preparsed(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog<'_>,
    // ...
) -> Result<()>
```

---

### `src/exporter/csv/writer.rs` (service, file-I/O)

**Analog:** `src/exporter/csv.rs` lines 14-22（write_csv_escaped）+ lines 154-418

**Imports 模式：**
```rust
use super::CsvExporter;
use crate::error::{Error, ExportError, Result};
use crate::pipeline::FieldMask;
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};
use std::io::Write;
use std::path::Path;
```

**Core 写入模式**（lines 14-22，write_csv_escaped 内联辅助）：
```rust
#[inline]
fn write_csv_escaped(buf: &mut Vec<u8>, bytes: &[u8]) {
    let mut remaining = bytes;
    while let Some(pos) = memchr::memchr(b'"', remaining) {
        buf.extend_from_slice(&remaining[..=pos]);
        buf.push(b'"');
        remaining = &remaining[pos + 1..];
    }
    buf.extend_from_slice(remaining);
}
```

**静态方法签名（不可改变）**（lines 157-165）：
```rust
#[inline]
pub(crate) fn write_record_preparsed(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog<'_>,
    meta: &MetaParts<'_>,
    pm: &PerformanceMetrics<'_>,
    normalized: Option<&str>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) -> Result<()>
```

---

### `src/exporter/csv/companion.rs` (utility, file-I/O)

**Analog:** `src/exporter/csv.rs` lines 25-93（format_companion_row + write_companion_rows）

**Core 模式**（lines 69-93）：
```rust
pub(crate) fn write_companion_rows(
    path: &Path,
    stats: &[crate::pipeline::TemplateStats],
) -> Result<()> {
    ensure_parent_dir(path).map_err(|e| io_err(path, format!("create dir failed: {e}")))?;
    let file = File::create(path)
        .map_err(|e| io_err(path, format!("create companion failed: {e}")))?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"template_key,count,...\n").map_err(...)?;
    let mut itoa_buf = itoa::Buffer::new();
    let mut line_buf: Vec<u8> = Vec::with_capacity(512);
    for s in stats {
        format_companion_row(&mut line_buf, &mut itoa_buf, s);
        writer.write_all(&line_buf).map_err(...)?;
    }
    writer.flush().map_err(...)?;
    Ok(())
}
```

---

### `src/exporter/sqlite/mod.rs` (module-entry, file-I/O)

**Analog:** `src/exporter/sqlite.rs` lines 1-22（struct 定义）+ lines 38-128（Debug impl + new/from_config）

**struct 定义模式**（lines 9-22）：
```rust
pub(crate) struct SqliteExporter {
    database_url: String,
    table_name: String,
    insert_sql: String,
    overwrite: bool,
    append: bool,
    conn: Option<Connection>,
    stats: ExportStats,
    row_count: usize,
    batch_size: usize,
    pub(super) normalize: bool,         // 注意：仅 mod.rs 内部可见
    pub(super) field_mask: crate::pipeline::FieldMask,
    pub(super) ordered_indices: Vec<usize>,
}
```

**initialize_pragmas 模式**（lines 24-36）：
```rust
fn initialize_pragmas(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    conn.execute_batch("PRAGMA journal_mode = OFF; ...")?;
    Ok(())
}
```

**tech debt 修复机会**（CONCERNS.md）——`conn_ref()` 辅助函数替换 `self.conn.as_ref().unwrap()`：
```rust
// sqlite/mod.rs — RESEARCH.md 中已设计
fn conn_ref(&self) -> crate::error::Result<&rusqlite::Connection> {
    self.conn
        .as_ref()
        .ok_or_else(|| Self::db_err("not initialized"))
}
```

---

### `src/exporter/sqlite/sql_builder.rs` (utility, transform)

**Analog:** `src/exporter/sqlite.rs` lines 71-115（build_insert_sql + build_create_sql）

**Core 模式**（lines 71-115）：
```rust
fn build_insert_sql(table_name: &str, ordered_indices: &[usize]) -> String {
    use crate::pipeline::FIELD_NAMES;
    if ordered_indices.len() == FIELD_NAMES.len() {
        return format!("INSERT INTO \"{table_name}\" VALUES (?, ...)");
    }
    // 用 projection::projected_field_names() 替换原来的内联逻辑
    let cols = super::super::projection::projected_field_names(ordered_indices);
    let placeholders = vec!["?"; ordered_indices.len()].join(", ");
    format!("INSERT INTO \"{table_name}\" ({}) VALUES ({placeholders})", cols.join(", "))
}

fn build_create_sql(table_name: &str, ordered_indices: &[usize]) -> String {
    // cols 构造也改用 projected_field_names()
    use crate::pipeline::FIELD_NAMES;
    const COL_TYPES: &[&str] = &["TEXT NOT NULL", "INTEGER NOT NULL", ...];
    // 注意：create 需要 "field_name TYPE" 格式，不能直接用 projected_field_names()
    // 而是用 ordered_indices.iter().map(|&i| format!("{} {}", FIELD_NAMES[i], COL_TYPES[i]))
}
```

---

### `src/exporter/sqlite/write.rs` (service, file-I/O)

**Analog:** `src/exporter/sqlite.rs` lines 137-277（do_insert_preparsed + batch_commit_if_needed 等）

**Core 热路径模式**（lines 153-221）：
```rust
fn do_insert_preparsed(
    stmt: &mut rusqlite::CachedStatement<'_>,
    sqllog: &Sqllog<'_>,
    meta: &MetaParts<'_>,
    pm: &PerformanceMetrics<'_>,
    normalized_sql: Option<&str>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
) -> std::result::Result<(), rusqlite::Error> {
    // 全量掩码快速路径 → params![] 宏
    if field_mask == crate::pipeline::FieldMask::ALL {
        stmt.execute(params![...])?;
        return Ok(());
    }
    // 投影路径 → rusqlite::types::Value 数组
    use rusqlite::types::Value;
    let all: [Value; 15] = [...];
    let selected: Vec<&Value> = ordered_indices.iter().map(|&i| &all[i]).collect();
    stmt.execute(rusqlite::params_from_iter(selected))?;
    Ok(())
}
```

---

### `src/cli/run/mod.rs` (module-entry, request-response)

**Analog:** `src/config/mod.rs` re-export 模式 + `src/cli/run.rs` lines 685-992（handle_run）

**re-export + 公开接口模式：**
```rust
mod processor;
mod parallel;
mod prescan;

use prescan::recompile_meta_if_needed;
use processor::process_log_file;
use parallel::{process_csv_parallel, concat_csv_parts};

// 保持 build_pipeline 和 FilterProcessor 在 mod.rs（~50 行）
fn build_pipeline(cfg: &Config, compiled_meta: Option<CompiledMetaFilters>) -> Pipeline { ... }
struct FilterProcessor { ... }
impl FilterProcessor { ... }
impl LogProcessor for FilterProcessor { ... }

pub fn handle_run(...) -> Result<()> { ... }
```

**注意：** `FilterProcessor` 依赖 `processor.rs` 的逻辑同时被 `mod.rs` 调用，建议保留在 `mod.rs` 内，不再拆出。

---

### `src/cli/run/processor.rs` (service, event-driven)

**Analog:** `src/cli/run.rs` lines 116-332（process_log_file 函数）

**函数签名模式**（lines 116-132）：
```rust
fn process_log_file(
    file_path: &str,
    file_index: usize,
    total_files: usize,
    exporter_manager: &mut ExporterManager,
    pipeline: &Pipeline,
    pb: &ProgressBar,
    limit: Option<usize>,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    mut aggregator: Option<&mut TemplateAggregator>,
    placeholder_override: Option<bool>,
    params_buffer: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    reset_pb: bool,
    sql_record_filter: Option<&CompiledSqlFilters>,
) -> Result<usize>
```

**热循环模式**（lines 164-332）：
```rust
'outer: for result in parser.iter() {
    match result {
        Ok(record) => {
            let (passes, cached_meta) = if pipeline.is_empty() {
                (true, None)
            } else {
                let meta = record.parse_meta();
                let ok = pipeline.run_with_meta(&record, &meta);
                (ok, Some(meta))
            };
            // ...
        }
        Err(e) => { /* 写错误日志，继续 */ }
    }
}
```

---

### `src/cli/run/prescan.rs` (service, batch)

**Analog:** `src/cli/run.rs` lines 334-418（scan_log_file_for_matches + scan_for_trxids_by_transaction_filters）以及 lines 669-683（recompile_meta_if_needed）

**rayon 并行预扫描模式**（lines 334-401）：
```rust
fn scan_log_file_for_matches(file_path: &str, cfg: &Config) -> Vec<CompactString> {
    use rayon::prelude::*;
    // par_iter() + filter_map + collect → HashSet → Vec
}

fn scan_for_trxids_by_transaction_filters(
    log_files: &[std::path::PathBuf],
    cfg: &Config,
    jobs: usize,
) -> AHashSet<CompactString> {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build().unwrap();
    pool.install(|| log_files.par_iter().flat_map(...).collect())
}
```

---

### `src/cli/run/parallel.rs` (service, batch)

**Analog:** `src/cli/run.rs` lines 423-683（concat_csv_parts + process_csv_parallel）

**concat 模式**（lines 423-478）：
```rust
fn concat_csv_parts(
    parts: &[(PathBuf, usize)],
    output_path: &Path,
    overwrite: bool,
    append_to_existing: bool,
) -> Result<()> {
    use std::io::BufReader;
    // parts 为空时 early return，避免清空已有数据
}
```

**parallel 主函数模式**（lines 479-668）：
```rust
fn process_csv_parallel(
    cfg: &Config,
    log_files: &[PathBuf],
    // ...
) -> Result<usize> {
    use rayon::prelude::*;
    // 每线程独立 CsvExporter + ExporterManager::from_csv
    // rayon::ThreadPoolBuilder + pool.install
}
```

---

## Shared Patterns

### 可见性收紧（D-10/D-11）

**适用范围：** 所有新建和修改的文件

**决策树：**
```
pub 项评估：
  被 tests/ 外部测试访问？→ 不存在（已验证：src/lib.rs grep 确认无 tests/ 目录）
  被 crate 内跨模块访问？→ pub(crate)
  被父模块访问？→ pub(super)
  仅模块内使用？→ 去掉 pub（private）
  无任何访问者？→ 去掉 pub
```

**典型收紧清单：**

| 位置 | 当前 | 目标 |
|------|------|------|
| `src/lib.rs:13` `pub use exporter::{...}` | `pub use` | `pub(crate) use` |
| `src/lib.rs` 所有 `pub mod` | `pub mod` | `pub(crate) mod` |
| `ExportStats` 所有字段 | `pub` | `pub(crate)` |
| `ExportStats::new/record_success/total` | `pub` | `pub(crate)` |
| `Exporter` trait | `pub` | `pub(crate)` |
| `ExporterManager` 所有方法 | `pub` | `pub(crate)` |
| `CsvExporter::new/from_config` | `pub` | `pub(crate)` |
| `SqliteExporter::new/from_config` | `pub` | `pub(crate)` |
| `CompiledMetaFilters` 所有字段 | `pub` | `pub(crate)` |
| `IncludeFilters/ExcludeFilters` 所有字段 | `pub` | `pub(crate)` |

---

### mod.rs re-export 模式（D-03）

**Source:** `src/config/mod.rs` lines 1-9（已有成熟的就地子模块模式）

**适用范围：** 所有 `filters/mod.rs`、`csv/mod.rs`、`sqlite/mod.rs`、`run/mod.rs`

**核心规则：**
```rust
// 子模块（私有）→ 通过 mod.rs pub(crate) use 保持调用路径稳定
mod child;
pub(crate) use child::TypeOrFn;

// 外部路径不变：crate::pipeline::filters::FiltersFeature 等保持不变
```

---

### Error 处理模式

**Source:** `src/exporter/csv.rs` lines 61-65

```rust
#[inline]
fn io_err(path: &Path, reason: String) -> Error {
    Error::Export(ExportError::WriteFailed {
        path: path.to_path_buf(),
        reason,
    })
}
```

**适用范围：** `csv/companion.rs`、`csv/writer.rs`、`sqlite/write.rs`（各自保留各自的 `io_err` / `db_err` 私有辅助函数）

---

### DryRunExporter 整合模式（D-08）

**Source:** `src/exporter/mod.rs` lines 66-143（ExporterKind + DryRunExporter）

**当前（analog 拆分前）：**
```rust
pub enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
    DryRun(DryRunExporter),  // 包裹独立 struct
}
pub struct DryRunExporter { stats: ExportStats }
impl Exporter for DryRunExporter { ... }  // 独立 impl（mod.rs lines 178-222）
```

**目标（D-08 整合后）：**
```rust
pub(crate) enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
    DryRun { stats: ExportStats },  // struct variant，删除独立类型
}
// ExporterKind 的 match arm 直接处理 DryRun { stats }
// 删除 DryRunExporter struct 和 impl Exporter for DryRunExporter
```

**测试更新：** `mod.rs` tests 中所有 `DryRunExporter::default()` 改为 `ExporterKind::DryRun { stats: ExportStats::default() }` 或 `ExporterManager::dry_run()`。

---

### 每步验证模式（RESEARCH.md 强调）

**适用范围：** 每个文件拆分完成后

```bash
# 每次拆分后立即运行（按顺序）
cargo build
cargo test
```

**最终验收：**
```bash
cargo test && cargo clippy --all-targets -- -D warnings
```

---

## No Analog Found

所有文件在现有代码库中都有直接对应的 analog。无需使用 RESEARCH.md 推断模式。

---

## Metadata

**Analog search scope:** `src/` 全目录（18 个 .rs 文件）
**Files scanned:** 18
**Pattern extraction date:** 2026-05-18

**拆分推荐顺序（来自 RESEARCH.md）：**
1. `src/pipeline/filters.rs` → `filters/`（独立性强，无下游依赖）
2. `src/config/mod.rs` → `config/validate.rs` + `config/apply_one.rs`（测试占主体）
3. `src/exporter/csv.rs` → `csv/`（同时建 `projection.rs`）
4. `src/exporter/sqlite.rs` → `sqlite/`（复用 `projection.rs`）
5. `src/cli/run.rs` → `run/`（依赖前四步路径稳定）
