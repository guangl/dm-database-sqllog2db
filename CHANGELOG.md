# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.15.0] - 2026-06-02

### CI/CD

- **GitHub Actions 版本统一**：修复 `.github/workflows/` 下 `ci.yaml`、`bench.yml`、`lychee.yml`、`pages.yml` 中误升级的 `actions/checkout@v6` 与 `actions/upload-artifact@v7`，全部回退到 `@v4`（CICD-01，Phase 55）
- **aarch64-linux 跨编译镜像配置**：新建 `Cross.toml` 锁定 `ghcr.io/cross-rs/aarch64-unknown-linux-gnu` 镜像，初期使用浮动 tag，Phase 61 进一步替换为 SHA256 摘要保证可复现构建（CICD-04 + CROSS-01，Phase 55 + Phase 61）
- **release workflow 竞争修复**：重构 `release.yaml`，拆出独立 `create-release` job 先于 4 个 matrix build job 运行，消除并行写入 release body 的竞争条件（CICD-02 + CICD-03，Phase 55）

### Changed

- **`cli/run/mod.rs` 拆分**：`handle_run` 提取为 7 个私有辅助函数（`resolve_input_files` / `merge_trxid_prescan` / `make_progress_bar` / `run_csv_parallel` / `run_sqlite_parallel` / `run_sequential` / `print_run_summary`），所有函数体 ≤40 行（CLEAN-02，Phase 58）
- **公共扫描模块**：新建 `src/scanner.rs` 抽取共享文件扫描逻辑，`stats` 模块与 `cli/run/processor.rs` 统一调用（Phase 56）

### Added

- **run / init 子命令端到端测试**：`tests/integration.rs` 新增 4 个 e2e 测试覆盖 TEST-01 + TEST-02——run 子命令 CSV 输出（字段名 + 记录数 + 退出码 0）、run 子命令 SQLite `sqllog_records` 表行数、init 子命令成功路径、init 子命令文件已存在时退出非零（Phase 57）
- **stats 时间范围跨字段校验**：`validate_stats_time_range` 新增 from ≤ to 检查 + 4 个单元测试 + 1 个 e2e 测试（TEST-03，Phase 57）

### Fixed

- **删除 stats 模块占位符**：移除 `src/cli/stats/mod.rs` 中遗留的 "not yet active" `warn!` 占位符调用（CLEAN-01，Phase 56）
- **benchmark 文档完善**：`benches/BENCHMARKS.md` 追加 CI Artifact 使用说明章节（命名规则、下载方式、JSON 结构、手动对比方法），benchmark workflow 设置 `continue-on-error: true` 不作为 merge 门控（BENCH-01，Phase 56）

---

## [1.14.0] - 2026-06-02

### Added

- **`stats` 命令时间范围过滤**：新增 `--from` / `--to` CLI 参数与 `config.toml` `[stats]` 节 `from` / `to` 字段，支持 `YYYY-MM-DD` 与 `YYYY-MM-DD HH:MM:SS` 两种格式；CLI 参数优先级高于 config（覆盖 STATS-07/08/09/11，Phase 53）
- **`StatsAccumulator` 时间过滤接入**：在聚合阶段按 `ts` 字段跳过窗口外记录，慢 SQL 与高频 SQL 两张表共享同一过滤逻辑，无 `--from`/`--to` 时行为与未过滤完全一致（覆盖 STATS-10，Phase 54）
- **`config.toml init` 模板**：追加 `[stats]` 节注释段，列出 `from`/`to`/`top` 三字段及格式示例与说明（Phase 53）
- **测试覆盖**：`tests/integration.rs` 新增 7 个 stats e2e 测试覆盖 STATS-07–11（Phase 53）+ 2 个 --from/--to 时间过滤效果测试（Phase 54）

### Changed

- **`opts.rs` Stats 变体**：`--top` 参数改为 `Option<u32>`，配合 CLI > config 优先级合并语义（Phase 53）

---

## [1.13.0] - 2026-06-01

### Added

- **`stats` 子命令**：`sqllog2db stats -c config.toml [--top N]` 流式扫描日志文件，输出慢 SQL 报告和高频 SQL 报告。
  - CSV 模式：在 `[exporter.csv].file` 的同级目录下生成 `slow_sql.csv`（字段：`sql_text,elapsed_ms,timestamp`）和 `frequent_sql.csv`（字段：`normalized_sql,call_count,avg_elapsed_ms,max_elapsed_ms`）。
  - SQLite 模式：在配置的数据库中写入 `slow_sql` 和 `frequent_sql` 表。
  - `--top N`（默认 20，最小 1）：每张表输出 Top N 条记录。
- **SQL 标准化引擎**（`src/stats/normalize.rs`）：状态机将 SQL 中的字面量（数字、字符串、绑定变量）替换为占位符，用于高频 SQL 聚合去重。
- **统计聚合器**（`src/stats/aggregate.rs`）：`StatsAccumulator` 持有固定大小的慢 SQL 堆和高频 SQL 频率表，流式扫描全程恒定内存。

---

## [1.12.0] - 2026-05-28

### Added

- **glob 输入支持**：`[sqllog].inputs` 接受文件路径、目录路径或 glob 模式（如 `./logs/2025-*.log`），支持多条目数组。
- **`--input` CLI 标志**：`sqllog2db run -c config.toml --input ./logs/*.log` 在命令行覆盖配置文件中的输入列表；`--input -` 映射到 stdin（跳过文件发现和预扫描）。
- **`-v`/`--verbose` 标志**：在 stderr 输出每文件处理详情；默认只在完成后输出汇总行。
- **`validate` 结构化输出**：通过校验时静默退出（exit 0），失败时输出 `[FAIL] <字段>: <原因>` 并退出非零码。
- **配置模板内联注释**：`sqllog2db init` 生成的模板为 `[exporter.csv]` 和 `[exporter.sqlite]` 所有字段添加了说明注释。

### Changed

- **`[sqllog].path` 已弃用**：改为 `inputs = ["sqllogs"]`（数组格式）。旧的 `path` 字段保留兼容性检测，使用时报错提示迁移。
- **错误信息**：所有错误提示统一加 `hint:` 前缀，可读性更好；`Config::from_file` 区分文件未找到与 IO 错误。

### Fixed

- **None 命令分支**：不带子命令直接运行 `sqllog2db` 现在打印帮助并以 exit 0 退出（之前报错）。

---

## [1.11.0] - 2026-05-25

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

## [1.10.0] - 2026-05-21

### Added

- **CLI `--help` 示例**: 所有三个子命令（`init`、`validate`、`run`）通过 `after_help` 添加达梦场景实用示例。
- **stdin 管道输入**: `--input -` 映射到 `/dev/stdin`，跳过文件发现和预扫描；事务级过滤降级时输出 stderr 警告。
- **进度显示**: 每 1024 条记录更新一次进度，非 TTY 模式降级为静态文本，不输出 ANSI 控制码。
- **运行摘要**: 完成后输出总记录数、成功导出数、错误数、处理速率（条/秒）和总耗时。

### Changed

- **错误体系重构**: `Error` 枚举拆分为 IO / 格式 / 配置 / 导出四类，每条错误包含文件路径和行号上下文。非致命错误继续处理，致命错误干净退出不 panic。
- **非致命错误实时输出**: 错误不再缓冲到运行结束，发生时立即写入 stderr。

---

## [1.7.0] - 2026-05-19

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

## [1.6.0] - 2026-05-19

### Added

- **Template reporter** (`template_reporter.rs`): structured SQL template analysis with output aggregation.
- **Configuration enhancements**: extended validation and template-related config fields.

### Changed

- Updated `cli/init.rs` template generation.
- Improved `pipeline/mod.rs` with template reporter integration.
- Documentation site refinements (mdBook structure, architecture page).

---

## [1.5.0] - 2026-05-18

### Added

- **Documentation site**: mdBook-based site with architecture docs, config reference, quickstart guide, and security policy.
- **Chart gallery**: SVG chart examples for template frequency, latency histograms, and user distribution.
- **Quickstart guide** (`docs/quickstart.md`): step-by-step getting started with examples.
- **Config reference** (`docs/config-reference.md`): full configuration field documentation.

---

## [1.4.0] - 2026-05-18

### Changed

- **Nested sub-table config model**: `[filter.include]`, `[filter.exclude]`, `[template]`, `[charts]` moved to top-level TOML sections (v1.4 format). Old flat format supported through `RawFiltersFeature` intermediate struct and serde alias for backward compatibility.
- **Config validation**: `validate_and_compile()` validates the final form and rejects legacy layouts.
- **Module restructuring**: 5 large files split into focused modules for better maintainability.
- **ExporterManager tightened**: visibility reduced to `pub(crate)`.

### Added

- **Property-based testing**: `proptest` integration for filter pipeline and config validation.
- **Test coverage**: 933 tests, ~74% line coverage.

---

## [1.3.0] - 2026-05-17

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

## [1.2.0] - 2026-05-15

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
- Feature flags for conditional compilation (`[csv]`, `[jsonl]`, `[sqlite]`, `[filters]`, `replace_parameters`, `full`) -- later removed for unified binary
- CLI commands: `init`, `validate`, `run`, `stats`, `digest`, `show-config`, `completions`, `man`
- GB18030 encoding support, `replace_parameters` SQL normalization
- Error logging, progress bar, exit codes, graceful shutdown (SIGINT)
- Performance optimization: mmap + SIMD parser, `itoa` zero-alloc CSV formatting, batch writer, pipeline fast path, SQLite prepared statement caching
- Architecture simplification: single-exporter mode, JSONL removal, Cargo features removal, module restructuring
- Peak memory reduced from 2.42GB to ~179MB (-92.6%) through streaming and zero-copy design
- Extensive test coverage, CI with clippy, coverage gates, and performance benchmarks

See git history for full details.

[Unreleased]: https://github.com/guangl/sqllog2db/compare/v1.15.0...HEAD
[1.15.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.15.0
[1.14.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.14.0
[1.13.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.13.0
[1.12.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.12.0
[1.11.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.11.0
[1.10.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.10.0
[1.7.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.7.0
[1.6.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.6.0
[1.5.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.5.0
[1.4.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.4.0
[1.3.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.3.0
[1.2.1]: https://github.com/guangl/sqllog2db/releases/tag/v1.2.1
[1.2.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.2.0
[1.0.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.0.0
[0.x]: https://github.com/guangl/sqllog2db/releases/tag/v0.10.7
