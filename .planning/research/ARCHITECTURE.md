# Architecture Research

**Domain:** Rust CLI 工具 — v1.15 CI/CD 基础设施 + 模块重构
**Researched:** 2026-06-02
**Confidence:** HIGH（基于直接代码阅读，无推断）

## 现状评估

### 已有 CI/CD 文件（不需要新建）

`.github/workflows/` 目录下已存在完整的 workflow 文件：

| 文件 | 用途 | 状态 |
|------|------|------|
| `ci.yaml` | test (3 平台矩阵) + lint + coverage | 已存在，功能完整 |
| `release.yaml` | tag 触发多平台构建 + crates.io 发布 | 已存在，功能完整 |
| `bench.yml` | PR/push 触发 criterion benchmark | 已存在，功能完整 |
| `pages.yml` | site/ 变更时部署 GitHub Pages | 已存在 |
| `lychee.yml` | 链接检查 | 已存在 |

v1.15 的 CI/CD 任务不是新建 workflow，而是验证和修复现有 workflow 的问题（如 `actions/checkout@v6` 使用了不存在的版本号，实际 latest 是 `v4`）。

### cli/run 模块现状

```
src/cli/run/
├── mod.rs               263 行  编排入口（handle_run），路由到三条路径
├── filter_processor.rs  300 行  FilterProcessor + build_pipeline
├── parallel.rs          225 行  CSV 并行路径 + concat_csv_parts
├── prescan.rs           137 行  事务 ID 预扫描
├── processor.rs         174 行  process_log_file（单文件处理核心）
├── sqlite_parallel.rs   225 行  SQLite 并行路径
└── tests.rs             253 行  mod.rs 的集成测试
```

`mod.rs` 当前 263 行，不是需要拆分的大文件。现有子模块划分已经合理——每个关注点已经独立成文件。

### stats 模块现状

```
src/stats/           业务逻辑层（数据域）
├── mod.rs           213 行  run_stats 编排 + scan_files + write_stats_output
├── aggregate.rs     388 行  StatsAccumulator（BinaryHeap + HashMap）
├── normalize.rs     159 行  SQL 标准化状态机
├── config.rs        240 行  StatsConfig + validate_time_str
└── output.rs        354 行  CSV/SQLite 输出

src/cli/stats/       CLI 适配层
└── mod.rs           147 行  handle_stats + merge_stats_options + 单元测试
```

stats 模块职责划分清晰，不需要重组，只需小幅清理。

## 系统概览

```
src/main.rs
    ↓ (dispatch by subcommand)
    ├── cli/run/mod.rs (handle_run)
    │       ↓ stdin/file discovery
    │       ├── run/prescan.rs         事务 ID 两阶段预扫描
    │       ├── run/filter_processor.rs  Pipeline 构建
    │       ├── run/processor.rs       单文件流式处理
    │       ├── run/parallel.rs        CSV 并行 + 合并
    │       └── run/sqlite_parallel.rs SQLite 并行
    │               ↓
    │           exporter/ (CSV / SQLite)
    │
    ├── cli/stats/mod.rs (handle_stats)
    │       ↓ CLI 参数合并
    │       └── stats/mod.rs (run_stats)
    │               ├── stats/aggregate.rs  StatsAccumulator
    │               ├── stats/normalize.rs  SQL 标准化
    │               └── stats/output.rs     CSV/SQLite 输出
    │
    ├── cli/init.rs (handle_init)
    └── cli/validate.rs (handle_validate)

.github/workflows/
    ├── ci.yaml       push/PR to main: test + lint + coverage
    ├── release.yaml  tag v*: 多平台构建 + GitHub Releases + crates.io
    └── bench.yml     push/PR: criterion benchmark (continue-on-error)

tests/
    ├── integration.rs    1940 行  assert_cmd e2e + handler 直调
    └── jemalloc_peak.rs   159 行  内存峰值回归
```

## 组件职责边界

| 组件 | 职责 | 对外接口 |
|------|------|----------|
| `cli/run/mod.rs` | 路由到三条处理路径（CSV 并行/SQLite 并行/顺序），组装进度条与统计摘要 | `pub fn handle_run(cfg, quiet, verbose, interrupted) -> Result<ErrorStats>` |
| `cli/run/processor.rs` | 单文件流式处理：解析→过滤→标准化→导出，每 1024 条更新进度 | `pub(super) fn process_log_file(...)` |
| `cli/run/prescan.rs` | 事务过滤器的两阶段预扫描，收集事务 ID 集合 | `pub(super) fn scan_for_trxids_by_transaction_filters(...)` |
| `cli/run/filter_processor.rs` | 把 `FiltersFeature` 配置编译为 `FilterProcessor`，构建 `Pipeline` | `pub(super) fn build_pipeline(cfg)` |
| `cli/run/parallel.rs` | CSV 多文件 rayon 并行解析 + `concat_csv_parts` 合并 | `pub(super) fn process_csv_parallel(...)` |
| `cli/run/sqlite_parallel.rs` | SQLite 多文件 rayon 并行解析，各文件独立写入后合并 | `pub(super) fn process_sqlite_parallel(...)` |
| `stats/aggregate.rs` | `StatsAccumulator`：流式累积慢 SQL（BinaryHeap）和高频 SQL（HashMap），含时间过滤 | `pub StatsAccumulator::new/update/into_results` |
| `stats/normalize.rs` | SQL 字面量替换为 `?` 的状态机 | `pub fn normalize_sql(sql) -> String` |
| `stats/output.rs` | 将聚合结果写入 CSV（两个文件）或 SQLite（两张表） | `pub fn write_csv_stats / write_sqlite_stats` |
| `cli/stats/mod.rs` | CLI 层：合并 CLI 参数与 config 优先级，调用 `stats::run_stats` | `pub fn handle_stats(cfg, top, from, to)` |

## 推荐项目结构（v1.15 目标状态）

```
src/cli/run/
├── mod.rs              handle_run（编排，目标不超过 270 行）
├── filter_processor.rs FilterProcessor + build_pipeline（现状良好）
├── parallel.rs         CSV 并行路径（现状良好）
├── prescan.rs          事务 ID 预扫描（现状良好）
├── processor.rs        单文件处理核心（现状良好）
├── sqlite_parallel.rs  SQLite 并行路径（现状良好）
└── tests.rs            模块内集成测试

src/cli/stats/
└── mod.rs              handle_stats + merge_stats_options
                        （147 行，现状良好，确认删除遗留 warn! 占位符）

src/stats/
├── mod.rs              run_stats 编排（现状良好）
├── aggregate.rs        StatsAccumulator（现状良好）
├── config.rs           StatsConfig + validate_time_str（现状良好）
├── normalize.rs        SQL 标准化（现状良好）
└── output.rs           统计输出（354 行，检查子函数是否超 40 行）

tests/
├── integration.rs      扩展 e2e 覆盖（stats/run/validate/init edge case）
└── jemalloc_peak.rs    内存峰值（保持）

.github/workflows/
├── ci.yaml             修复 actions 版本号（checkout@v6 → @v4）
├── release.yaml        修复 actions 版本号
└── bench.yml           验证 scripts/collect_bench_results.sh 存在
```

## 架构模式

### 模式 1：pub(super) 内部模块隔离

**什么：** `cli/run/` 子模块全部使用 `pub(super)` 可见性，只有 `mod.rs` 暴露 `pub fn handle_run`。

**何时使用：** 当一个功能由多个协作文件组成，但对外只有一个入口时。

**优点：** 外部（tests/）只能通过 `handle_run` 测试，强制集成测试视角；内部重构不影响接口。

**v1.15 注意点：** `tests/integration.rs` 直接 `use dm_database_sqllog2db::cli::run::handle_run`，这依赖 `cli::run` 模块在 lib crate 中可见。验证 `src/lib.rs` 中的 `pub mod cli` 层级是否正确。

### 模式 2：双层 stats 分离（CLI 层 vs 业务层）

**什么：** `cli/stats/mod.rs` 只处理 CLI 参数优先级合并，`src/stats/` 只处理业务逻辑。

**何时使用：** 当 CLI 参数合并逻辑和业务聚合逻辑必须各自可独立测试时。

**现状：** 分离已经存在且正确。`merge_stats_options` 有完整单元测试，`run_stats` 有独立集成测试。不需要变动。

### 模式 3：handler 直调 + assert_cmd 双层 e2e

**什么：** `tests/integration.rs` 同时包含：
1. 直接调用 `handle_run/handle_init/handle_validate` 的 handler 集成测试（快速，无进程开销）
2. 通过 `assert_cmd::Command::cargo_bin("sqllog2db")` 的完整 e2e CLI 测试（慢，验证 clap 参数解析）

**v1.15 扩展策略：** 新增 e2e 测试时先判断——业务逻辑正确性用 handler 直调（快 10x）；CLI 参数格式、stderr 输出格式、exit code 用 assert_cmd。

## 数据流

### run 命令处理流

```
handle_run(cfg, quiet, verbose, interrupted)
    ↓
SqllogParser::log_files()  glob 展开，返回 Vec<PathBuf>
    ↓ (stdin 检测：is_empty && !is_terminal)
    ↓
has_transaction_filters? → scan_for_trxids_by_transaction_filters()
                           → merge_found_trxids() → 修改 cfg 副本
    ↓
build_pipeline(cfg)  编译 FilterProcessor

路由判断（互斥）：
  use_csv_parallel      → process_csv_parallel()    (rayon)
  use_sqlite_parallel   → process_sqlite_parallel()  (rayon)
  否则                  → 顺序：for each file { process_log_file() }

    ↓
摘要输出到 stderr (unless quiet)
```

### stats 命令处理流

```
handle_stats(cfg, top, from, to)
    ↓ merge_stats_options  CLI > config > default
    ↓
run_stats(merged_cfg, effective_top)
    ↓ validate_stats_time_range
    ↓ SqllogParser::log_files()
    ↓
for each file { LogParserBuilder → iter() → accumulator.update(record) }
    ↓ StatsAccumulator::into_results()  BinaryHeap + HashMap → Vec
    ↓
write_stats_output(cfg, slow_rows, frequent_rows)
    CSV 优先 → write_csv_stats (slow_sql.csv + frequent_sql.csv)
    SQLite   → write_sqlite_stats (slow_sql table + frequent_sql table)
```

## CI/CD 集成点

### 现有 ci.yaml 三个 job

| Job | 触发 | 运行环境 | 关键命令 |
|-----|------|----------|---------|
| `test` | push/PR to main | ubuntu + windows + macos | `cargo test` + release 性能基线 |
| `lint` | push/PR to main | ubuntu-latest | `cargo fmt --check` + `cargo clippy` + `cargo doc` + `cargo bench --no-run` |
| `coverage` | push/PR to main | ubuntu-latest | `cargo llvm-cov --fail-under-lines 70` |

### 当前 workflow 的版本问题（阻塞性）

所有 workflow 文件使用 `actions/checkout@v6`，但 GitHub Actions `checkout` 最新版本是 `v4`。如果这些 workflow 从未实际运行过，`v6` 会导致立即失败：

```yaml
# 当前（错误）
- uses: actions/checkout@v6

# 应改为
- uses: actions/checkout@v4
```

同样问题存在于 `release.yaml`（含 `taiki-e/install-action@v2`、`softprops/action-gh-release@v3`）和 `bench.yml`（含 `actions/upload-artifact@v7`）。

### release.yaml 多平台矩阵

| Target | OS | 工具 | 注意 |
|--------|-----|------|------|
| x86_64-unknown-linux-gnu | ubuntu | cargo | 无 |
| aarch64-unknown-linux-gnu | ubuntu | cross | 需要 Docker 守护进程 |
| x86_64-pc-windows-msvc | windows | cargo | 无 |
| aarch64-apple-darwin | macos | cargo | 无 |

CHANGELOG.md 必须存在，release.yaml 用 `awk "/## \[${VERSION}\]/,/## \[/"` 从中提取 release notes。

### bench.yml 依赖

```bash
scripts/collect_bench_results.sh   # bench.yml 第 32 行调用此脚本
```

如果此脚本不存在，bench job 会失败（但 `continue-on-error: true` 所以不阻塞 PR）。

## 重构边界分析

### cli/run/mod.rs 拆分评估

**结论：不需要拆分。** 当前 263 行，已经是经过多轮重构后的精简形态：

- `handle_run` 本身约 230 行（含注释），主要是三条路径的路由逻辑
- 所有可提取的业务函数已经在子模块中（`processor.rs`、`prescan.rs` 等）
- 剩余逻辑是高度相关的条件分支（stdin 检测、并行路由判断、进度条管理、摘要输出），强行拆分会增加复杂度

如果需要清理：唯一合理的提取是将"摘要输出"（约 30 行）提取为私有函数，但这不改变模块结构。

### stats 模块整理评估

**结论：结构已合理，只需删除遗留代码。** 根据 ROADMAP Phase 54 计划备注：

- `cli/stats/mod.rs`：删除 "not yet active" warn! 占位符
- `src/stats/output.rs`（354 行）：检查 `write_csv_stats` 和 `write_sqlite_stats` 各自是否超过 40 行，必要时提取子函数

## v1.15 变更分类（新建 vs 修改）

### 仅修改的文件（不新建模块）

| 文件 | 变更类型 | 原因 |
|------|----------|------|
| `.github/workflows/ci.yaml` | 修改 | 修复 `checkout@v6` → `v4` |
| `.github/workflows/release.yaml` | 修改 | 修复 actions 版本，验证多平台构建 |
| `.github/workflows/bench.yml` | 修改 | 确认 `scripts/collect_bench_results.sh` 路径 |
| `tests/integration.rs` | 扩展 | 新增 edge case：stats --from/--to、run 中断、validate 错误 |
| `src/cli/stats/mod.rs` | 小幅清理 | 删除遗留 warn! 占位符 |
| `src/stats/output.rs` | 可选重构 | 如子函数超过 40 行则提取 |

### 可能新建的文件

| 文件 | 条件 | 用途 |
|------|------|------|
| `scripts/collect_bench_results.sh` | 如果不存在 | bench.yml 第 32 行依赖此脚本 |
| `CHANGELOG.md` | 如果不存在 | release.yaml 从中提取 release notes |
| `tests/e2e_stats.rs` | 可选（integration.rs 超 2000 行时） | 拆分 stats e2e 测试 |

## 构建顺序（依赖顺序）

**Phase A：CI/CD 修复（无代码依赖，优先执行）**
1. 修复所有 workflow 文件中的 actions 版本号（`checkout@v6` → `v4` 等）
2. 确认 `scripts/collect_bench_results.sh` 存在，否则新建占位
3. 验证 CHANGELOG.md 格式符合 release.yaml 的 awk 提取模式

**Phase B：代码清理（独立，与 A 并行可进行）**
4. 删除 `cli/stats/mod.rs` 中的遗留 warn! 占位符
5. 检查 `stats/output.rs` 函数长度，超 40 行则拆分子函数

**Phase C：测试扩展（依赖 A+B 完成后功能稳定）**
6. 扩展 `tests/integration.rs`：stats 子命令 edge case（--from/--to 边界、无匹配记录、无效格式）
7. 扩展 `tests/integration.rs`：run 子命令 edge case（中断标志预置、多文件并行一致性）
8. 运行 `cargo llvm-cov --fail-under-lines 70` 验证覆盖率门禁通过

**Phase D：性能基准稳定化（依赖 C 通过）**
9. 确认 `cargo bench --no-run` 编译通过（ci.yaml lint job 已包含此步骤）
10. 验证 `cargo bench` 在本地可完整运行（bench.yml 的 continue-on-error 不是应对编译失败的借口）

## 反模式

### 反模式 1：在 handle_run 中内联业务逻辑

**错误做法：** 将过滤、标准化、并行合并等逻辑直接写在 `mod.rs` 的 `handle_run` 中。

**为何有问题：** `mod.rs` 膨胀，子模块独立测试性下降。

**正确做法：** 现有结构已经正确——业务逻辑在 `pub(super)` 子模块，`handle_run` 只做路由。

### 反模式 2：stats CLI 层直接访问 StatsAccumulator

**错误做法：** `cli/stats/mod.rs` 直接创建 `StatsAccumulator` 并调用 `update()`。

**为何有问题：** CLI 层应只关心参数合并，业务细节泄漏会破坏分离。

**正确做法：** 现有结构正确——`cli/stats/mod.rs` 调用 `crate::stats::run_stats`，不直接操作 accumulator。

### 反模式 3：e2e 测试全部使用 assert_cmd

**错误做法：** 所有新测试都用 `Command::cargo_bin("sqllog2db")` 调用进程。

**为何有问题：** assert_cmd 测试每次都启动新进程，比 handler 直调慢 5-10 倍，CI 时间显著增加。

**正确做法：** 业务逻辑测试用 `handle_run/handle_stats` 直调；只有 CLI 参数格式、exit code、stderr 格式用 assert_cmd。

### 反模式 4：用 continue-on-error 掩盖 workflow 问题

**错误做法：** `bench.yml` 设置 `continue-on-error: true` 后就认为 benchmark 脚本不需要维护。

**为何有问题：** `scripts/collect_bench_results.sh` 缺失时 benchmark 数据不会被上传，CI artifact 一直为空。

**正确做法：** `continue-on-error` 只用于"性能回退不阻塞 merge"的语义，脚本本身必须存在且可运行。

## 集成点总结

| 边界 | 通信方式 | 注意事项 |
|------|----------|----------|
| `ci.yaml` ↔ Cargo | `cargo test/clippy/fmt/doc/bench` | `bench --no-run` 验证编译，不运行 benchmark |
| `release.yaml` ↔ cross | `cross build --release --target aarch64` | cross 用 Docker，CI runner 需要 Docker 守护进程 |
| `bench.yml` ↔ `scripts/` | `bash scripts/collect_bench_results.sh` | 脚本必须存在 |
| `release.yaml` ↔ CHANGELOG.md | `awk` 提取版本段落 | 格式必须是 `## [VERSION]`，否则 release notes 为空 |
| `tests/integration.rs` ↔ lib | `use dm_database_sqllog2db::cli::run::handle_run` | lib.rs 必须 `pub mod cli` |
| `assert_cmd` ↔ binary | `Command::cargo_bin("sqllog2db")` | 需要 `[[bin]] name = "sqllog2db"` 已注册在 Cargo.toml |

## 来源

- 直接代码阅读：`src/cli/run/mod.rs`、`src/cli/stats/mod.rs`、`src/stats/mod.rs`（HIGH）
- `.github/workflows/` 全部文件直接阅读（HIGH）
- `Cargo.toml` 直接阅读（HIGH）
- `.planning/PROJECT.md` + `.planning/ROADMAP.md`（HIGH）

---
*Architecture research for: sqllog2db v1.15 CI/CD + module refactoring*
*Researched: 2026-06-02*
