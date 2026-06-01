---
phase: 52-exporter
reviewed: 2026-06-01T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/stats/aggregate.rs
  - src/stats/output.rs
  - src/stats/mod.rs
  - src/cli/stats/mod.rs
  - src/exporter/mod.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 52: Code Review Report

**Reviewed:** 2026-06-01T00:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

本次审查覆盖 Phase 52 新增的 stats 统计分析模块（`src/stats/`）、CLI 入口（`src/cli/stats/mod.rs`）、`ExporterManager` 工具函数（`src/exporter/mod.rs`）及集成测试（`tests/integration.rs`）。

整体实现思路清晰，数据流正确（最小堆维护 Top-N 慢 SQL，HashMap 聚合高频 SQL，单次扫描双侧收集）。`f32_ms_to_i64` 的边界处理、SQLite 事务回滚逻辑均可正常工作。发现 4 个 WARNING 级别问题，无 BLOCKER 级别问题。

---

## Warnings

### WR-01: `StatsAccumulator::new(0)` 在 release 模式下静默丢弃全部数据

**File:** `src/stats/aggregate.rs:71`

**Issue:** `top_n = 0` 的防御断言使用 `debug_assert!`，在 `--release` 编译下完全不触发。调用方传入 `0` 时，`push_slow` 函数的 `self.slow_heap.len() < self.top_n`（即 `0 < 0`）永远为 `false`，导致慢 SQL 堆始终为空；`build_freq_rows` 末尾 `truncate(0)` 也会清空高频 SQL 列表。最终两个输出均为空，无任何错误提示。

CLI 层已通过 `handle_stats` 拦截 `top == 0`，但库函数本身无法防止直接调用者传入 `0`。

**Fix:**
```rust
pub fn new(top_n: u32) -> Self {
    // 换用 assert! 使 release 模式同样报错，或直接返回 Result
    assert!(top_n >= 1, "top_n must be >= 1");
    Self {
        slow_heap: BinaryHeap::new(),
        freq_map: HashMap::new(),
        top_n: top_n as usize,
    }
}
```

---

### WR-02: `f32_ms_to_i64` 的边界注释与实际保证不符

**File:** `src/exporter/mod.rs:291-295`

**Issue:** 函数在 `else` 分支执行 `clamped as i64` 时添加了注释 `"value already clamped to i64 range"`，但这并不完全成立。`MAX_I64_F64` 常量（`9_223_372_036_854_775_807.0_f64`）由于 f64 精度，实际与 `i64::MAX as f64`（`9_223_372_036_854_775_808.0` 量级）相等；`ms_f64 > MAX_I64_F64` 使用的是严格大于，当 `ms_f64 == MAX_I64_F64` 时不进入 `i64::MAX` 分支，而是进入 else 分支后依赖 Rust saturating cast 才得到正确结果。

代码行为在运行时是正确的（Rust 1.45+ float-to-int 转换为饱和语义），但注释声称范围已确保，而实际上是靠 saturating cast 兜底。这与 `#[expect(clippy::cast_possible_truncation)]` 的意图相矛盾——如果真的已经 clamped，就不需要抑制该 lint。

**Fix:** 修正注释以如实说明：
```rust
#[expect(
    clippy::cast_possible_truncation,
    reason = "saturating float-to-int cast: values at boundary (== MAX_I64_F64) \
              are handled by Rust's saturating cast semantics (Rust 1.45+), \
              not solely by the if-condition above"
)]
{
    clamped as i64
}
```

---

### WR-03: `handle_stats` 接受 `quiet` 参数但完全忽略，无 API 约定

**File:** `src/cli/stats/mod.rs:8,17`

**Issue:** `pub fn handle_stats(cfg: &Config, top: u32, quiet: bool)` 接受 `quiet` 参数，函数体内以 `let _ = quiet;` 丢弃，注释只说"本命令不改变输出行为"。这造成以下问题：

1. 调用方（`main.rs:183`）传入 `cli.quiet`，期望该标志有效果，但实际上 `stats` 命令无论 `--quiet` 是否设置都输出相同内容（包括 `log::info!` 记录）。
2. 函数签名暗示 `quiet` 会影响行为，这是错误的契约。
3. 若将来有人要实现 quiet 模式，`let _ = quiet` 会被静默跳过而不是编译报错。

**Fix:** 若确认 stats 命令永远不需要 quiet 行为，应移除该参数并更新调用方：
```rust
// stats/mod.rs
pub fn handle_stats(cfg: &Config, top: u32) -> Result<()> { ... }

// main.rs
cli::stats::handle_stats(&cfg, *top)?;
```
若将来可能需要 quiet，至少应加文档注释说明当前行为和计划。

---

### WR-04: `avg_elapsed_ms` 使用截断（trunc）而非四舍五入，导致可见精度损失

**File:** `src/stats/aggregate.rs:152,156`

**Issue:** `avg_f32` 由 `(total_elapsed / call_count as f64) as f32` 得到（f64→f32 精度舍入），然后经 `f32_ms_to_i64` 调用 `.trunc()` 向零截断后写入输出。例如平均值为 `2.9ms` 会输出 `2`，而非用户期望的 `3`。

对于统计场景，截断误差最大达 1ms，可能导致用户误读数据（"平均 2ms" 实际是 "平均 2.9ms"）。更关键的是：相同的截断逻辑也适用于 `max_elapsed_ms`，`5.9ms` 的最大值会被输出为 `5ms`。

**Fix:** 在 `f32_ms_to_i64` 中改用四舍五入，或在调用前手动对 avg 值做 round：
```rust
// 方案一：f32_ms_to_i64 改为四舍五入
let clamped = ms_f64.round(); // 而不是 trunc()

// 方案二：调用前 round
let avg_f32 = (state.total_elapsed / state.call_count as f64) as f32;
avg_elapsed_ms: crate::exporter::f32_ms_to_i64(avg_f32.round()),
max_elapsed_ms: crate::exporter::f32_ms_to_i64(state.max_elapsed.round()),
```

注意：若选择方案一（修改 `f32_ms_to_i64`），需评估对 CSV 导出路径的影响，因为该函数同样被 `exporter/csv/writer.rs` 使用。

---

## Info

### IN-01: `pub use normalize_sql` 是死的公开 API 导出

**File:** `src/stats/mod.rs:7-8`

**Issue:** `#[allow(unused_imports)] pub use normalize::normalize_sql;` 将 `normalize_sql` 重新导出为库公开 API，但整个代码库中无任何地方通过 `stats::normalize_sql` 路径引用它（内部使用 `crate::stats::normalize::normalize_sql`）。`allow(unused_imports)` 的存在本身就说明这个导出从未被使用。

**Fix:** 若该导出无意成为公开 API，应删除这两行：
```rust
// 删除以下两行：
#[allow(unused_imports)]
pub use normalize::normalize_sql;
```
若确实需要公开，应移除 `allow` 属性并确保有外部用例验证。

---

### IN-02: `test_write_sqlite_stats_drop_recreates` 测试断言不完整

**File:** `src/stats/output.rs:322-335`

**Issue:** 该测试第二次调用 `write_sqlite_stats(&make_slow_rows(1), &[], &db_url)` 时传入空 `frequent` 列表，但断言只验证了 `slow_sql` 行数为 1，未验证 `frequent_sql` 表被正确清空（应为 0 行）。

**Fix:** 补充对 `frequent_sql` 表的断言：
```rust
let freq_count: i64 = conn
    .query_row("SELECT COUNT(*) FROM frequent_sql", [], |row| row.get(0))
    .unwrap();
assert_eq!(freq_count, 0, "DROP+CREATE should empty frequent_sql table");
```

---

### IN-03: `make_stats_config_file` 中 `log_path` 命名与用途混淆

**File:** `tests/integration.rs:1353`

**Issue:** 测试辅助函数 `make_stats_config_file` 中，`log_path = dir.join("test.log")` 实际上是**应用程序日志文件**（对应 `[logging] file`），但其命名 `log_path` 很容易被误读为"SQL 输入日志文件路径"（另一个变量叫 `input_log`）。S3/S4 测试也各自重新声明了 `let log_path = dir.path().join("test.log")`，造成重复和混淆。

**Fix:** 将变量重命名以明确用途：
```rust
let app_log_path = dir.join("test.log");  // 应用程序日志，不是 SQL 输入
let input_log = dir.join("input.log");
```
并同步更新 TOML 模板中对 `app_log_path` 的引用。

---

_Reviewed: 2026-06-01T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
