# Phase 20: 测试覆盖深化 - Pattern Map

**Mapped:** 2026-05-18
**Files analyzed:** 10（7 × VERIFICATION.md + 1 × tests/integration.rs 新增函数 + 1 × src/pipeline/fingerprint.rs 新增 proptest + 1 × Cargo.toml dev-dep）
**Analogs found:** 10 / 10

---

## File Classification

| 新建/修改文件 | Role | Data Flow | 最近类似文件 | 匹配质量 |
|---|---|---|---|---|
| `.planning/milestones/v1.3-phases/12-sql/12-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `.planning/milestones/v1.3-phases/13-templateaggregator/13-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `.planning/milestones/v1.3-phases/14-exporter/14-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `.planning/milestones/v1.3-phases/15-svg/15-VERIFICATION.md`（补充） | doc | N/A | `.planning/milestones/v1.3-phases/15-svg/15-VERIFICATION.md` | exact |
| `.planning/milestones/v1.3-phases/16-remaining-charts/16-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `.planning/phases/17-filter-nesting/17-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `.planning/phases/18-template-chart-nesting/18-VERIFICATION.md` | doc | N/A | `.planning/phases/19-code-refactor/19-VERIFICATION.md` | exact |
| `tests/integration.rs`（追加端到端 + 边界测试） | test | request-response | `tests/integration.rs`（现有） | exact |
| `src/pipeline/fingerprint.rs`（追加 proptest） | test | transform | `src/pipeline/fingerprint.rs`（现有 `#[cfg(test)] mod tests`） | exact |
| `Cargo.toml`（新增 dev-dep proptest） | config | N/A | `Cargo.toml`（现有 dev-dependencies） | exact |

---

## Pattern Assignments

### VERIFICATION.md × 7（文档类）

**Analog:** `.planning/phases/19-code-refactor/19-VERIFICATION.md`（格式权威参照）

**Front-matter 块**（第 1-12 行）：
```yaml
---
phase: {nn}-{name}
verified: YYYY-MM-DDTHH:MM:SSZ
status: passed
score: N/M must-haves verified
overrides_applied: 0
gaps: []
deferred: []
---
```

**顶部标题与摘要**（第 14-19 行）：
```markdown
# Phase {N}: {名称} Verification Report

**Phase Goal:** {一句话目标}
**Verified:** YYYY-MM-DDTHH:MM:SSZ
**Status:** passed
**Re-verification:** No — initial verification
```

**Observable Truths 表格**（第 22-36 行）：
```markdown
## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | {可观察事实} | ✓ VERIFIED | {验证命令/输出摘要} |
...
```

**Required Artifacts 表格**（第 48-68 行）：
```markdown
### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `src/...` | {预期} | ✓ VERIFIED | {行号或 grep 结果} |
...
```

**Key Link Verification 表格**（第 70-80 行）：
```markdown
### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `src/a.rs` | `src/b.rs` | `pub mod b` | ✓ WIRED | 编译通过 |
...
```

**注意：Phase 15 VERIFICATION.md 已存在**（`.planning/milestones/v1.3-phases/15-svg/15-VERIFICATION.md`），内容仅覆盖 Wave 1（Plan 01/02）。需追加 Wave 2/3（Plan 03-05 的 SVG 渲染验证），而非重写。追加格式与 Phase 19 一致。

---

### `tests/integration.rs` — 端到端测试（TEST-02）

**Analog:** `tests/integration.rs` 第 850-881 行（`test_handle_run_with_filters_builds_pipeline`）

**Import 块**（文件第 1-17 行，现有，无需改动）：
```rust
use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::config::{
    Config, CsvExporterConfig, ExporterConfig, SqliteExporterConfig, SqllogConfig,
};
use dm_database_sqllog2db::pipeline::filters::{ExcludeFilters, IncludeFilters};
use dm_database_sqllog2db::pipeline::{FiltersFeature, NormalizeConfig};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
```

新增测试还需要导入 `OutputConfig` 和 `TemplateConfig`，在文件顶部 `use` 块追加（已有行旁）：
```rust
use dm_database_sqllog2db::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig, TemplateConfig};
```

**核心模式：TempDir + write_test_log + make_run_config + handle_run + 断言**（第 146-173 行，`test_handle_run_real_csv_export`）：
```rust
#[test]
fn test_handle_run_real_csv_export() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    // header + 10 data rows
    assert!(content.lines().count() >= 10);
}
```

**过滤器配置模式**（第 856-881 行，`test_handle_run_with_filters_builds_pipeline`）：
```rust
let mut cfg = make_run_config(&log_dir, &csv_file);
cfg.filter = Some(FiltersFeature {
    enable: true,
    include: IncludeFilters {
        users: Some(vec!["TESTUSER".to_string()]),
        ..Default::default()
    },
    exclude: ExcludeFilters::default(),
    ..Default::default()
});
let compiled_filters = cfg.validate_and_compile().unwrap();
// ...
handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1, compiled_filters).unwrap();
```

**TemplateConfig 启用模式**（来自 `src/pipeline/mod.rs` 第 130-142 行）：
```rust
cfg.template = Some(TemplateConfig {
    enable: true,
    output_csv_path: String::new(),
    output_sqlite_table: String::new(),
});
```

**OutputConfig 字段投影模式**（来自 `src/pipeline/mod.rs` 第 180-186 行）：
```rust
cfg.output = Some(OutputConfig {
    fields: Some(vec!["ts".to_string(), "username".to_string(), "sql".to_string()]),
});
// 期望 CSV header: "ts,username,sql"
```

**CSV 行数断言模式**（带 header 修正，来自第 907-909 行）：
```rust
let content = std::fs::read_to_string(&csv_file).unwrap();
let data_lines = content.lines().count().saturating_sub(1); // minus header
assert_eq!(data_lines, N, "expected N records, got {data_lines}");
```

---

### `tests/integration.rs` — 边界条件测试（TEST-03）

**Analog:** `tests/integration.rs` 第 56-81 行（`test_handle_run_dry_run_empty_dir`）

**空目录/空文件模式**：
```rust
#[test]
fn test_handle_run_dry_run_empty_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // No log files → handle_run returns Ok early
    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, None, true, true, &interrupted, 80, false, None, 1, None).unwrap();
}
```

**关键注意事项**（来自 RESEARCH.md Pitfall 3）：

格式错误行的 `errors_in_file` 是 `processor.rs` 的局部变量，外部不可见。测试策略改为断言"正常行正确导出（CSV 数据行数 = 正常行数）"，而非访问内部 error 计数。

超长 SQL 字段必须保持达梦日志格式（来自 RESEARCH.md Pitfall 4）：
```
2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] <超长SQL>. EXECTIME: 13(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.
```
**不能**直接写入裸超长字符串，必须包裹在正确格式中。

---

### `src/pipeline/fingerprint.rs` — proptest 属性测试（TEST-04）

**Analog:** `src/pipeline/fingerprint.rs` 第 325-434 行（现有 `#[cfg(test)] mod tests`）

**现有 `#[cfg(test)] mod tests` 结构**（第 325-327 行）：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- fingerprint 原有测试（9 项，零回归） ---
    // ...

    // --- normalize_template 新增测试（8 项） ---
    // ...
```

**proptest 新增 import 写法**（放在 `mod tests` 的 `use super::*;` 下方）：
```rust
use proptest::prelude::*;
```

**幂等性测试写法**（来自 CONTEXT.md specifics）：
```rust
proptest! {
    #[test]
    fn prop_normalize_template_is_idempotent(s in any::<String>()) {
        let once = normalize_template(&s);
        let twice = normalize_template(&once);
        prop_assert_eq!(once, twice);
    }
}
```

**字面量保护不变性测试**（来自现有单元测试 `test_normalize_string_literal_hides_comment_marker`，第 423-430 行，提升为属性测试）：
```rust
// 现有单元测试（参照）
#[test]
fn test_normalize_string_literal_hides_comment_marker() {
    let result = normalize_template("WHERE col = '-- not a comment'");
    assert!(
        result.contains("'-- not a comment'"),
        "expected literal preserved in {result}"
    );
}
```

属性测试版本（来自 RESEARCH.md Pattern 4）：
```rust
proptest! {
    #[test]
    fn prop_normalize_template_literal_protection(
        prefix in "[A-Za-z0-9 ]{0,20}",
        inner in "[A-Za-z0-9 ]{0,50}"
    ) {
        let sql = format!("WHERE col = '{inner}-- not a comment{prefix}'");
        let result = normalize_template(&sql);
        prop_assert!(
            result.contains("-- not a comment"),
            "literal comment marker should survive in: {result}"
        );
    }
}
```

**关键规则（来自 RESEARCH.md Pitfall 1）：** `proptest!` 宏内部的 `#[test]` 由宏自动展开，不可在 `proptest! { }` 外部再加 `#[test]` 注解，否则 clippy 报 `unused attribute` 警告。

---

### `Cargo.toml` — 新增 proptest dev-dependency

**Analog:** `Cargo.toml` 现有 `[dev-dependencies]` 段（tempfile = "3.27.0" 是既有模式）

**新增一行**（追加到 `[dev-dependencies]` 段）：
```toml
proptest = "1.6.0"
```

Wave 0 验证命令：`cargo add proptest@1.6.0 --dev && cargo build --tests`

---

## Shared Patterns

### 测试函数命名约定
**来源：** `tests/integration.rs` 现有 55 个函数命名  
**规则：** `test_{handler}_{scenario}` 格式
- 端到端：`test_e2e_{feature_path}`（如 `test_e2e_filter_pipeline`、`test_e2e_template_normalization`、`test_e2e_field_projection`）
- 边界：`test_boundary_{condition}`（如 `test_boundary_empty_log_file`、`test_boundary_all_filtered`、`test_boundary_malformed_line`、`test_boundary_long_sql`）
- proptest：`prop_{function}_{invariant}`（如 `prop_normalize_template_is_idempotent`、`prop_normalize_template_literal_protection`）

### Arrange-Act-Assert 结构
**来源：** `tests/integration.rs` 第 146-173 行
```rust
// Arrange
let dir = tempfile::TempDir::new().unwrap();
let log_dir = dir.path().join("logs");
std::fs::create_dir_all(&log_dir).unwrap();
write_test_log(&log_dir.join("test.log"), N);
let csv_file = dir.path().join("out.csv");
let cfg = make_run_config(&log_dir, &csv_file);
// Act
let interrupted = Arc::new(AtomicBool::new(false));
handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1, None).unwrap();
// Assert
let content = std::fs::read_to_string(&csv_file).unwrap();
assert_eq!(content.lines().count().saturating_sub(1), N);
```

### handle_run 完整签名（10 个参数）
**来源：** `src/cli/run/mod.rs`（RESEARCH.md Code Examples）
```rust
pub fn handle_run(
    cfg: &Config,
    limit: Option<usize>,
    dry_run: bool,
    quiet: bool,
    interrupted: &Arc<AtomicBool>,
    progress_interval: u64,
    resume: bool,
    state_file_override: Option<&str>,
    jobs: usize,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<()>
```
常用调用形式（无 compiled_filters 时传 `None`）：
```rust
handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1, None).unwrap();
```

### 无 mock 原则
**来源：** CONTEXT.md Established Patterns  
所有测试通过 `tempfile::TempDir` + 真实 I/O，不引入 mock crate。错误路径用 `assert!(result.is_err())`，不用 `#[should_panic]`。

### CSV header 行偏移
**来源：** `tests/integration.rs` 第 907-909 行  
CSV 第 1 行为 header，`write_test_log(path, N)` 写入 N 条数据记录，断言格式：
```rust
let data_lines = content.lines().count().saturating_sub(1);
assert_eq!(data_lines, N);
// 或：
assert_eq!(content.lines().count(), N + 1); // header + N rows
```

---

## No Analog Found

无。所有文件均能在代码库中找到对应模式。

---

## Metadata

**Analog search scope:** `tests/`, `src/pipeline/`, `src/config/`, `.planning/phases/19-code-refactor/`
**Files scanned:** 5（integration.rs、fingerprint.rs、pipeline/mod.rs、config/mod.rs、19-VERIFICATION.md）
**Pattern extraction date:** 2026-05-18
