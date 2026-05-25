# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.11] - 2026-05-25

### Added

- **Criterion benchmark suite** (`benches/bench_common.rs`): 共享 `synthetic_log` 辅助函数和 `bench_target_dir`，覆盖 CSV 导出、SQLite 导出、filter 启用/禁用、parser 原始吞吐四大场景。热循环用 `std::hint::black_box` 包装防止死代码消除。
- **GitHub Actions 基准 CI** (`.github/workflows/bench.yml`): PR 和 push to main 时运行 `cargo bench`，上传含时间戳、commit SHA、各组 mean/stddev 的 HTML/JSON artifact 供历史趋势对比。
- **SQLite 多文件并行解析路径** (`src/cli/run/sqlite_parallel.rs`): 基于 rayon 的并行记录收集，解析错误通过 `log::warn!` 上报，与顺序路径行为对齐。

### Changed

- **Parser API 适配**: `IndicatorFilters::matches` 签名更新为 `u32`，移除 `i64::from(rowcount)` 类型转换，清理过时的 v1.1.0 注释。
- **Filter 模块职责边界**: `compiled.rs` 和 `prescan.rs` 添加显式 Pre-scan / Main-pass 节注释，逻辑归属更清晰。

### Fixed

- **CSV 数据丢失** (`exporter/csv/writer.rs`): `has_metrics` 判断缺少 `|| sqllog.rowcount != 0`——`rowcount > 0` 但 `exec_id == 0` 且 `exectime == 0.0` 的记录在 CSV 中静默丢失，SQLite 导出器不受影响。
- **致命导出错误不中止循环**: 发生 `DatabaseFailed` 等致命错误后现在立即 `break`，不再继续向损坏连接发起导出。
- **记录计数器错误计数**: `records_in_file` 仅在导出成功时才递增，错误分支不再计数。
- **CSV 追加模式 TOCTOU 竞态** (`exporter/csv/mod.rs`): header 写入判断改为 open 文件后通过 `file.metadata().len() == 0` 确定，消除 `exists()` 预检查的竞态窗口。
- **不稳定的 jemalloc 测试** (`tests/jemalloc_peak.rs`): 移除暖运行时随机失败的 `heap_pressure > 0` 断言，测试改为纯基线测量。
- **死代码 GB18030 回退** (`pipeline/normalizer.rs`): 移除永远不可达的 GB18030 fallback 分支，同时从 `Cargo.toml` 删除 `encoding_rs` 依赖。
- **预扫描静默失败** (`cli/run/prescan.rs`): 文件打开错误和非 UTF-8 路径现在通过 `log::warn!` 记录，不再静默跳过。
- **预扫描进度日志**: `eprintln!` 替换为 `log::info!`，输出遵从 `RUST_LOG` 和 `--quiet` 设置。
- **基准死代码消除** (`benches/bench_parser.rs`): 热循环用 `std::hint::black_box` 包装，防止 LLVM 将整个循环判定为死代码。
- **基准 SQL 格式不一致**: 从合成日志模板移除多余的 `AND status='active'`，各基准间 records/sec 数字现在可直接比较。
- **SQLite 并行路径解析错误** (`cli/run/sqlite_parallel.rs`): 解析失败的行现在通过 `log::warn!` 上报，不再静默丢弃。
- **并行路径测试覆盖** (`cli/run/tests.rs`): `test_parallel_merge_consistent` 现在使用单文件目录正确触发顺序路径，不再仅测试两次并行运行。

---

## [1.10] - 2026-05-21

### Added

- **CLI `--help` 示例**: 所有三个子命令（`init`、`validate`、`run`）通过 `after_help` 添加达梦场景实用示例。
- **stdin 管道输入**: `--input -` 映射到 `/dev/stdin`，跳过文件发现和预扫描；事务级过滤降级时输出 stderr 警告。
- **进度显示**: 每 1024 条记录更新一次进度，非 TTY 模式降级为静态文本，不输出 ANSI 控制码。
- **运行摘要**: 完成后输出总记录数、成功导出数、错误数、处理速率（条/秒）和总耗时。

### Changed

- **错误体系重构**: `Error` 枚举拆分为 IO / 格式 / 配置 / 导出四类，每条错误包含文件路径和行号上下文。非致命错误继续处理，致命错误干净退出不 panic。
- **非致命错误实时输出**: 错误不再缓冲到运行结束，发生时立即写入 stderr。

---

## [1.9] - 2026-05-20

### Removed

- **6 non-essential dependencies**: removed `mimalloc` (custom allocator), `ahash` (custom hasher), `compact_str` (compact string), `smallvec` (small vector), `indicatif` (progress bar), and `chrono` (time formatting). These were unnecessary for the core streaming parse-export pipeline and their removal reduces binary size and compile time.
- **`S: BuildHasher` generic parameter**: removed from `compute_normalized()` to simplify the API surface.

### Changed

- **rusqlite feature trim**: reduced rusqlite features to `bundled` only, removing unnecessary optional features.
- **BufWriter capacity**: reduced from 16MB to 2MB for lower memory footprint without throughput regression.
- **Time handling**: replaced `chrono::Local` with `std::time::SystemTime` UTC computation, eliminating the chrono dependency.
- **Progress reporting**: replaced indicatif spinner with `eprintln!` output for simpler, dependency-free progress reporting.

---

## [1.7] - 2026-05-19

### Removed

- **Dead code elimination**: removed `show_config`, `stats`, `digest`, `update` CLI commands. Only `init`, `validate`, `run` remain.
- **Removed modules**: `color.rs`, `lang.rs`, `apply_one.rs`, `resume.rs`, `template_reporter.rs`, `fingerprint.rs`, `aggregator.rs`, `companion.rs`.
- **Removed test files**: `exporter/csv/tests.rs`, `exporter/sqlite/tests.rs`, `exporter/tests.rs` (consolidated into in-module tests).

### Changed

- **CLI slimmed**: command surface reduced to 3 subcommands (`init`, `validate`, `run`).
- **Config simplified**: removed legacy `apply_one` normalization, deprecated config fields.
- **Error module trimmed**: removed unused error variants.
- **Test consolidation**: tests migrated from standalone test modules into parent modules with `#[cfg(test)]`.
- **Dependency hygiene**: removed unused `proptest` dev-dependency.

---

## [1.6] - 2026-05-19

### Added

- **Template reporter** (`template_reporter.rs`): structured SQL template analysis with output aggregation.
- **Configuration enhancements**: extended validation and template-related config fields.

### Changed

- Updated `cli/init.rs` template generation.
- Improved `pipeline/mod.rs` with template reporter integration.
- Documentation site refinements (mdBook structure, architecture page).

---

## [1.5] - 2026-05-18

### Added

- **Documentation site**: mdBook-based site with architecture docs, config reference, quickstart guide, and security policy.
- **Chart gallery**: SVG chart examples for template frequency, latency histograms, and user distribution.
- **Quickstart guide** (`docs/quickstart.md`): step-by-step getting started with examples.
- **Config reference** (`docs/config-reference.md`): full configuration field documentation.

---

## [1.4] - 2026-05-18

### Changed

- **Nested sub-table config model**: `[filter.include]`, `[filter.exclude]`, `[template]`, `[charts]` moved to top-level TOML sections (v1.4 format). Old flat format supported through `RawFiltersFeature` intermediate struct and serde alias for backward compatibility.
- **Config validation**: `validate_and_compile()` validates the final form and rejects legacy layouts.
- **Module restructuring**: 5 large files split into focused modules for better maintainability.
- **ExporterManager tightened**: visibility reduced to `pub(crate)`.

### Added

- **Property-based testing**: `proptest` integration for filter pipeline and config validation.
- **Test coverage**: 933 tests, ~74% line coverage.

---

## [1.3] - 2026-05-17

### Added

- **SQL template normalization engine** (`normalize_template`): strips comments, folds IN-list values, uppercases keywords, collapses whitespace. Produces stable template keys from structurally identical queries with different parameter values.
- **TemplateAggregator**: streaming statistics engine that counts occurrences per template, accumulates execution time distribution via `hdrhistogram` (compact ~24 KB per template), and records first/last timestamps alongside a representative example SQL.
- **Dual-stat output**: aggregated template data written to both a CSV summary file and a dedicated SQLite table (`sql_templates`) in a single run.
- **Four SVG chart types**: frequency bar (top-N templates by occurrence), latency histogram (execution time distribution per template), trend line (normalized template frequency over time), and user pie (proportional share of queries by user). Rendered via plotters with SVG-only backend -- no system fonts or image libraries required.
- **8 new config fields**: `[template]` and `[charts]` TOML sections with per-chart type toggles and output directory.

### Performance

- **Parallel CSV map-reduce merge**: `merge()` function eliminates lock contention by merging per-thread CSVs in a single pass.
- Benchmark: ~5.2M records/sec CSV (synthetic), ~1.55M records/sec on a real 1.1 GB file.

---

## [1.2.1] - 2026-05-15

### Fixed

- Minor bug fixes and improvements discovered during v1.2 deployment.

---

## [1.2] - 2026-05-15

### Added

- **Exclude filters** (FILTER-03): record-level exclude filters with OR-veto semantics integrated into `CompiledMetaFilters`.
- **Unified filter interface**: `validate_and_compile()` eliminates double `Regex::new()` calls.

### Changed

- **Cargo features removed**: `csv`, `jsonl`, `sqlite`, `filters`, `replace_parameters`, `full` features removed. All functionality always compiled into the binary.
- **CLI startup optimization**: parallel feature compilation via `rayon` removed from prescan; `binrw` dependency removed.

### Performance

- Hot-path optimization based on PERF-10 gate analysis. No blind optimization -- every change verified by criterion benchmarks.
- CSV: ~2.13M records/sec, SQLite: ~1.11M records/sec.

### Fixed

- **SQLite technical debt** (DBUS-01/02/03): prepared statement caching, transaction handling, and PRAGMA tuning.
- **Nyquist audit remediation**: improved test coverage and validation gaps.

> **Note:** v1.1 profiling and benchmarking work laid the foundation for v1.2 performance improvements. v1.1 functionality is fully merged into v1.2.

---

## [1.0.0] - 2026-04-18

### Migration Note

This is the first stable release. Upgrading from 0.x:

- Configuration format changed from array `[[exporter.*]]` to single `[exporter.*]` sections.
- JSONL exporter removed; migrate to CSV or SQLite.
- Error logging changed from separate error log file to `log::trace!()` in application log.
- `--set` flag replaces direct config file edits for one-off overrides.

### Added

- **Regular expression multi-field filtering**: AND semantics for include + OR-veto for exclude.
- **Field projection via `ordered_indices`**: exact column order and subset selection.
- **FieldMask-based output field control**.
- **Initial CLI**: `init`, `validate`, `run`, `stats`, `digest`, `show-config`, `man`, `completions`.
- **690+ tests** with comprehensive coverage.

---

## [0.x] - 2025-11 to 2026-04

The 0.x series (0.1.0 through 0.10.7) covered the initial development of sqllog2db:

- Streaming SQL log parsing with multi-file, directory, and glob input modes
- Multi-exporter architecture (CSV, JSONL, SQLite, DuckDB, PostgreSQL, Oracle, DM) -- later simplified to CSV + SQLite
- Feature flags for conditional compilation (`[csv]`, `[jsonl]`, `[sqlite]`, `[filters]`) -- later removed for unified binary
- CLI commands: `init`, `validate`, `run`, `stats`, `digest`, `show-config`, `completions`, `man`
- GB18030 encoding support, `replace_parameters` SQL normalization
- Error logging, progress bar, exit codes, graceful shutdown (SIGINT)
- Performance optimization: mmap + SIMD parser, `itoa` zero-alloc CSV formatting, batch writer, pipeline fast path, SQLite prepared statement caching
- Architecture simplification: single-exporter mode, JSONL removal, Cargo features removal, module restructuring
- Peak memory reduced from 2.42GB to ~179MB (-92.6%) through streaming and zero-copy design
- Extensive test coverage, CI with clippy, coverage gates, and performance benchmarks

See git history for full details.

[1.11]: https://github.com/guangl/sqllog2db/releases/tag/v1.11
[1.10]: https://github.com/guangl/sqllog2db/releases/tag/v1.10
[1.9]: https://github.com/guangl/sqllog2db/releases/tag/v1.9
[1.7]: https://github.com/guangl/sqllog2db/releases/tag/v1.7
[1.6]: https://github.com/guangl/sqllog2db/releases/tag/v1.6
[1.5]: https://github.com/guangl/sqllog2db/releases/tag/v1.5
[1.4]: https://github.com/guangl/sqllog2db/releases/tag/v1.4
[1.3]: https://github.com/guangl/sqllog2db/releases/tag/v1.3
[1.2.1]: https://github.com/guangl/sqllog2db/releases/tag/v1.2.1
[1.2]: https://github.com/guangl/sqllog2db/releases/tag/v1.2
[1.0.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.0
[0.x]: https://github.com/guangl/sqllog2db/releases/tag/v0.10.7
