# Phase 33: 核心功能验证 - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

## Phase Boundary

Phase 33 对精简后的代码库进行全面验证，确保 Phase 28-32 的移除操作没有破坏任何核心功能。不涉及代码修改 — 纯粹是构建、测试、lint、benchmark 和端到端功能验证。

**In scope:** KEEP-01~06
**Out of scope:** 代码修改（验证通过即完成）；新功能开发；bug 修复（除非验证中发现精简引入的退化）

## Implementation Decisions

### 验证深度
- **D-01:** 验证方式 = 自动化检查（build/test/clippy/fmt）+ CLI 冒烟测试
- **D-02:** 冒烟测试覆盖全功能：CSV 导出 + SQLite 导出 + 四类过滤器（include/exclude/indicators/sql）+ 参数归一化 + 并行 CSV + 中文配置模板 + 错误日志
- **D-03:** 冒烟测试数据源优先使用真实日志（sqllogs/），不存在时生成测试日志
- **D-04:** 并行 CSV 验证需包含输出正确性检查 + 计时对比
- **D-05:** 参数归一化验证需同时检查 CSV 和 SQLite 双路输出
- **D-06:** 四类过滤器分项独立验证，每类单独准备配置和场景
- **D-07:** 构建验证含 debug check + release build 两者
- **D-08:** 需要验证 `cargo run -- init` 生成的中文配置模板可用
- **D-09:** 冒烟测试发现问题时：先修复，然后重新执行完整验证
- **D-10:** SQLite 验证深度：行数对比（CSV vs SQLite）+ 关键字段抽查
- **D-11:** 错误日志输出需验证（配置 [error] file 后确认错误被写入）
- **D-12:** 每个 KEEP 项使用显式检查清单判定通过/失败

### 验证报告
- **D-13:** 生成 VERIFICATION-CHECKLIST.md 到 phase 目录
- **D-14:** 报告格式：KEEP 需求映射 + 通过/失败 + 证据 + 可复现步骤

### 计划组织
- **D-15:** 3 个 plan 按验证类型分组：
  - **Plan 1 (33-01):** 静态检查 — cargo check + cargo build --release + cargo clippy + cargo fmt
  - **Plan 2 (33-02):** 自动化测试 — cargo test + cargo bench（含 baseline 对比）
  - **Plan 3 (33-03):** 手动冒烟验证 — Shell 编排 CLI 命令 + Rust 代码校验数据，生成 VERIFICATION-CHECKLIST.md
- **D-16:** 三个 plan 可并行执行，互不依赖

### 性能回归
- **D-17:** 运行全部 benchmark（bench_csv + bench_sqlite + bench_filters），与 benches/baselines/ 既定基线对比
- **D-18:** 退化超过 10% 视为回归，需分析根因并修复
- **D-19:** Benchmark 放在 Plan 2（自动化测试）中

### Claude's Discretion

- VERIFICATION-CHECKLIST.md 的精确结构和字段
- 冒烟测试 Shell 脚本和 Rust 验证代码的具体实现
- 各 plan 内部的任务拆分细节
- benchmark baseline 更新策略（如果当前 baseline 过旧）

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求文档
- `.planning/ROADMAP.md` §Phase 33 — 阶段目标、Success Criteria、KEEP-01~06 需求映射
- `.planning/REQUIREMENTS.md` §v1.7 Requirements — KEEP-01~06 的完整定义

### 项目上下文
- `.planning/PROJECT.md` — 项目架构、Key Decisions、Constraints
- `.planning/codebase/TESTING.md` — 测试框架、测试文件组织、fixture 模式、覆盖率

### 核心代码（Plan 3 冒烟验证需要）
- `src/exporter/csv.rs` — CSV 导出器实现
- `src/exporter/sqlite.rs` — SQLite 导出器实现
- `src/features/filters.rs` — 四类过滤器实现
- `src/features/replace_parameters.rs` — 参数归一化
- `src/features/mod.rs` — Pipeline + LogProcessor trait
- `src/cli/run.rs` — 主编排逻辑（并行 CSV 路径）

### 性能基线
- `benches/bench_csv.rs` — CSV 吞吐量 benchmark
- `benches/bench_sqlite.rs` — SQLite 吞吐量 benchmark
- `benches/bench_filters.rs` — 过滤器 pipeline benchmark
- `benches/baselines/` — 既定性能基线数据

## Existing Code Insights

（本阶段不涉及代码修改，以下仅用于 Plan 3 冒烟验证的参考）

### Reusable Assets
- `tests/integration.rs` 中的 `write_test_log()` — 测试日志生成器，可在无真实日志时复用
- `tests/integration.rs` 中的 `make_run_config()` — 测试配置工厂模式
- 现有的 `test_csv_throughput_baseline` — 已有吞吐量基准测试
- `handle_run` / `handle_init` / `handle_validate` — 所有 CLI handler 均通过 integration tests 验证

### Established Patterns
- `cargo test` 运行所有单元测试 + 集成测试（36 个集成测试当前全部通过）
- `cargo clippy --all-targets -- -D warnings` 零警告门禁
- `cargo fmt --check` 格式检查
- Benchmarks 使用 criterion 框架，baseline 比较用 `cargo bench`

### Integration Points
- 冒烟测试 Shell 脚本通过 `cargo run -- <subcommand>` 调用 CLI
- Rust 数据校验代码可复用现有 CSV/SQLite 读取模式

## Specific Ideas

- 冒烟测试 Shell 脚本建议按 KEEP 需求编号组织，每个 KEEP 项一个验证函数
- Rust 数据校验部分建议复用 `csv` 和 `rusqlite` crate 的读取 API
- Plan 3 的 VERIFICATION-CHECKLIST.md 应在所有验证步骤完成后由脚本自动生成（而非手动撰写）

## Deferred Ideas

### Reviewed Todos (not folded)
- "调研 dm-database-parser-sqllog 1.0.0 新特性" — 与本阶段验证范围无关，延后至未来版本

---

*Phase: 33-核心功能验证*
*Context gathered: 2026-05-20*
