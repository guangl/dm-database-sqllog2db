---
phase: 31-remove-resume
reviewed: 2026-05-20T00:00:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - benches/bench_csv.rs
  - benches/bench_filters.rs
  - benches/bench_sqlite.rs
  - Cargo.toml
  - docs/architecture.md
  - README.md
  - src/cli/init.rs
  - src/cli/opts.rs
  - src/cli/run/mod.rs
  - src/cli/run/parallel.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/config/mod.rs
  - src/config/validate.rs
  - src/error.rs
  - src/lang.rs
  - src/lib.rs
  - src/main.rs
  - tests/integration.rs
  - src/cli/mod.rs
  - src/cli/run/filter_processor.rs
  - src/cli/run/prescan.rs
  - src/pipeline/mod.rs
  - src/pipeline/filters/mod.rs
  - src/pipeline/filters/types.rs
  - src/pipeline/filters/serde_helpers.rs
  - src/pipeline/normalizer.rs
findings:
  critical: 1
  warning: 3
  info: 4
  total: 8
status: issues_found
---

# Phase 31: Code Review Report — Remove Resume/Checkpoint Feature

**Reviewed:** 2026-05-20T00:00:00Z
**Depth:** standard
**Files Reviewed:** 27 (including supporting modules for cross-file analysis)
**Status:** issues_found

## Summary

该阶段移除了 resume/checkpoint 特性（`--resume`、`--state-file`、`ResumeState`、`ResumeConfig` 及所有关联逻辑）。核心移除干净：无残留 resume 引用、`handle_run` 签名从 10 参数简化到 8 参数、所有 36 个测试通过、clippy 零警告。

然而，一次标准的 **cross-file 追踪** 发现现有快速审查遗漏了一个 **严重缺陷**：`bench_filters.rs` 中全部 6 个过滤基准测试场景使用了已被废弃的 `[features.filters]` 格式（而非当前的 `[filter]` 格式），serde 静默忽略后所有基准测试均测量了无过滤器的快速路径。**所有过滤器基准测试结果均无效。**

此外还有 3 个警告性缺陷和 4 个提示性问题。

## Critical Issues

### BL-01: bench_filters.rs 全部 6 个场景使用废弃格式，基准测试结果无效

**File:** `benches/bench_filters.rs:73,89,105,121,137,149`

**Issue:** `bench_filters.rs` 中的全部 6 个场景的配置函数均使用 `[features.filters]` 旧格式（被移除的 pre-v1.4 格式）。当前 `Config` 结构体不存在 `features` 字段，serde 默认行为（不存在 `deny_unknown_fields`）会使整个 `[features.filters]` 表被静默丢弃，且无任何警告或错误。

具体来说：
- `cfg_pipeline_passthrough` — `[features.filters]\nenable = true\nstart_ts = \"2000-01-01\"`
- `cfg_trxid_small` — `[features.filters]\nenable = true\ntrxids = [...]`
- `cfg_trxid_large` — `[features.filters]\nenable = true\ntrxids = [...]`
- `cfg_indicator_prescan` — `[features.filters]\nenable = true\nmin_runtime_ms = 2000`
- `cfg_exclude_passthrough` — `[features.filters]\nenable = true\nexclude_usernames = [...]`
- `cfg_exclude_active` — `[features.filters]\nenable = true\nexclude_usernames = [...]`

上述所有配置经 `toml::from_str` 反序列化后，`cfg.filter` 均为 `None`。这导致：
1. `cfg.validate_and_compile()` 返回 `Ok(None)`
2. `handle_run` 接收到 `None` 而非已编译的过滤器
3. `build_pipeline` 构造空管线
4. **6 个过滤器场景全部测量无过滤器的快速路径，结果完全相同**

此缺陷导致 `criterion` 报告的 7 个场景之间的差异全部来自测量噪音，而非真实的过滤器开销。

**Fix:** 将全部 6 个配置函数迁移到当前 `[filter]` 格式：

```rust
// cfg_pipeline_passthrough: 所有记录通过
fn cfg_pipeline_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.include]
start_ts = \"2000-01-01\"
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

// cfg_trxid_small: 精确匹配 10 个事务 ID
fn cfg_trxid_small(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let ids: Vec<String> = (0..10).map(|i: usize| format!("\"{i}\"")).collect();
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.include]
trxids = [{ids}]
",
        base = base_toml(sqllog_dir, bench_dir),
        ids = ids.join(", "),
    );
    toml::from_str(&toml).unwrap()
}

// cfg_indicator_prescan: 事务级指标过滤器
fn cfg_indicator_prescan(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.indicators]
min_runtime_ms = 2000
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

// cfg_exclude_active: 排除模式全部命中
fn cfg_exclude_active(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.exclude]
users = [\"BENCH\"]
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}
```

其余 2 个场景同理。迁移完成后运行 `cargo bench --bench bench_filters` 验证各场景结果产生显著差异。

## Warnings

### WR-03: `process_csv_parallel` 的 `_quiet` 参数已成为死代码

**File:** `src/cli/run/parallel.rs:77` — 函数签名中 `_quiet: bool`
**File:** `src/cli/run/mod.rs:107` — 调用处 `quiet,`

**Issue:** 参数 `quiet` 传入 `process_csv_parallel` 后以 `_quiet` 接收但从未在函数体中使用。原有的唯一消费者是 resume skip 消息日志（"skipped — already processed"），随 resume 特性被移除。下划线前缀抑制了 clippy 的警告，但这使参数成为留在函数签名中的死代码。

**Fix:** 从调用方和被调用方移除该参数：
- `parallel.rs:77`: 删除 `_quiet: bool,`
- `mod.rs:107`: 删除对应的 `quiet,` 实参

### WR-04: `process_log_file` 的 `limit` 参数名存在误导性

**File:** `src/cli/run/processor.rs:24` — 参数声明 `limit: Option<usize>`
**File:** `src/cli/run/mod.rs:137` — 调用处 `remaining,`

**Issue:** 参数在 `process_log_file` 中被命名为 `limit`，但其语义是"跨文件的剩余配额"（`remaining`）。调用方（`mod.rs:137`）已将其计算为 `limit.saturating_sub(total_records)` 后传入。而在 `process_log_file` 函数体（`processor.rs:123`）中：
```rust
if let Some(remaining) = limit {
    if records_in_file >= remaining {
        break 'outer;
    }
}
```
变量被重新绑定为 `remaining`，暴露了命名不一致。维护者可能误将其视为"此文件的绝对 limit"，引入配额计算错误。

**Fix:** 将 `processor.rs` 中该参数重命名为 `remaining` 以匹配其语义和调用方变量名。

### WR-05: `lang.rs` 模块级 `#![allow(dead_code)]` 抑制了合法的死代码检测

**File:** `src/lang.rs:11`

**Issue:** 模块顶部的 `#![allow(dead_code)]` 属性为整个模块的所有项抑制了死代码警告。说明注释指出某些函数"仅在 binary crate (main.rs) 中使用；lib crate 生产代码不调用"。然而 `src/lib.rs` 中的 `pub mod lang;` 将此模块编译为库 crate 的一部分，`pub(crate)` 及以下可见性的函数在库上下文中确实是死代码。当前的属性掩盖了这一点，使得任何未来在 lang 模块中引入的真正死代码都不会被编译器发现。

**Fix:** 更精确的方法是：
1. 将仅由 binary 使用的函数标记为 `#[cfg(not(tarpaulin_include))]` 或从 lib crate 的条件性暴露
2. 或使用 `#[allow(dead_code)]` 缩小作用域到单个项而非整个模块
3. 最小范围修复：验证当前模块中哪些函数在库上下文中真正存活，仅对确实死亡的项加 `#[allow(dead_code)]`，移除模块级属性

## Info

### IN-01: `FileError::ReadFailed` 死代码变体

**File:** `src/error.rs:58-59`

**Issue:** `ReadFailed` 是 `FileError` 枚举的一个变体，带有 `#[allow(dead_code)]` 和 TODO Phase 32 注释。该变体曾是 `resume.rs` 中 `mark_processed` 方法的错误映射目标。resume 移除后，此变体在代码库中没有任何构造点。

现有审查已将其识别为死代码。建议在 Phase 32 之前立即删除而非推迟清理。

### IN-02: 文档和 README 中存在过时的模块路径引用

**File:** `docs/architecture.md:50-51,60-63` — 引用 `src/features/` 和 `cli/run.rs`
**File:** `README.md:80-84` — 引用 `features/mod.rs`, `features/filters.rs`, `cli/run.rs`

**Issue:** pipeline 模块已从 `src/features/` 迁移到 `src/pipeline/`（`src/features/` 目录已不存在），`cli/run/` 从单文件重构为多文件模块（`mod.rs`, `parallel.rs`, `processor.rs`, `prescan.rs`, `filter_processor.rs`）。架构文档和 README 中的引用路径已过时，对新贡献者产生误导。

### IN-03: 基准测试 TOML 配置中的 `[error]` 段被静默忽略

**File:** `benches/bench_csv.rs:52` — `[error]\nfile = ".../errors.log"`
**File:** `benches/bench_filters.rs:52` — `[error]\nfile = ".../errors.log"`
**File:** `benches/bench_sqlite.rs:42` — `[error]\nfile = ".../errors.log"`

**Issue:** 基准测试的 `base_toml()` 和 `make_config()` 产生的 TOML 字符串中包含 `[error]` 段。当前 `Config` 结构体没有任何 `error` 字段，serde 默认行为使此段被静默丢弃。虽然这不影响基准测试结果（基准不依赖错误日志），但它反映了代码库中一段实际不再存在的配置模式，可能给维护者造成困惑。

### IN-04: `validate_filter()` 与 `validate_and_compile()` 编译逻辑重复

**File:** `src/config/validate.rs:67-78` — `validate_filter()` 方法

**Issue:** `validate()` 调用 `validate_filter()`，该函数内部执行 `CompiledMetaFilters::try_from_include_exclude()` 和 `CompiledSqlFilters::try_from_sql_filters()` 编译然后丢弃结果。而 `validate_and_compile()` 执行完全相同的编译并保留结果。考虑到代码库明确的目标是"消除 run 路径中的双重 regex 编译"，此重复的编译在 validate 路径中看似无害，但实际浪费了每次 validate 调用中的一次 regex 编译。

该问题的严重程度较低，因为在典型使用中 `validate` 和 `run` 是独立的 CLI 子命令，用户不会同时从同一个进程调用两者。但如果在库 API 层面连续调用 `validate()` 和 `validate_and_compile()`，会导致同一组过滤器的 regex 被编译两次。

---

_Reviewed: 2026-05-20T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
