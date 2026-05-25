# Phase 45: 并行扩展与 CI 基准集成 - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

将并行解析扩展到 SQLite 导出路径（多文件跨文件并行解析 + WAL 模式单线程写入）；在 GitHub Actions CI 中集成 benchmark，PR 触发时自动运行 `cargo bench` 并将 JSON 格式结果作为 artifact 上传。

</domain>

<decisions>
## Implementation Decisions

### SQLite 并行策略
- **D-01:** 实现多文件跨文件并行解析：rayon 并行解析各文件，结果在内存中合并（通过 channel 或 collect + merge），最终由单线程按批写入 SQLite（WAL 模式）。
- **D-02:** 避免多线程并发写入 SQLite，使用 WAL 模式 + 单 writer thread 策略（与现有 CSV 并行路径的 merge-then-write 模式一致）。
- **D-03:** SQLite 并行路径的正确性通过 `cargo test` 验证：并行输出与顺序模式输出一致（record 内容相同，顺序可不同）。

### CI Benchmark 集成
- **D-04:** GitHub Actions workflow 文件：`.github/workflows/bench.yml`（或加入现有 CI workflow），PR 触发时运行 `cargo bench`。
- **D-05:** benchmark 输出格式：JSON（critcmp 兼容格式）。通过 `cargo bench -- --output-format bencher | tee bench_output.json` 或保存 criterion 的 JSON 输出（`target/criterion/*/estimates.json`）。
- **D-06:** artifact 内容：时间戳、commit SHA、各 benchmark 组的 mean/stddev，文件名包含 commit SHA。
- **D-07:** CI artifact 使用 `actions/upload-artifact` 上传，retention 天数由 Claude 合理设置（建议 30-90 天）。

### 质量门禁
- **D-08:** 全链路质量门禁：`cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部绿灯（Phase 45 验收标准 #4）。

### Claude's Discretion
- CI workflow 触发条件（只 PR 还是也包含 push to main）
- artifact 的具体 JSON schema 格式（只要包含 timestamp、SHA、mean、stddev）
- critcmp vs 自定义比较脚本的选择

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 现有并行实现（必读，保持风格一致）
- `src/cli/run/parallel.rs` — `process_csv_parallel` 实现，SQLite 并行参照此模式
- `src/cli/run/mod.rs` — 并行/顺序路径判断逻辑（`jobs > 1` 判断）
- `src/exporter/mod.rs` — ExporterManager，SQLite exporter 接口

### CI 配置
- `.github/workflows/` — 现有 CI workflow，新 bench workflow 需与之协调
- `benches/BENCHMARKS.md` — benchmark 运行方式，artifact 存储说明

### Requirements
- `.planning/REQUIREMENTS.md` §PERF-03, §BENCH-02

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `process_csv_parallel` — 现有 CSV 并行实现，SQLite 并行路径的设计参考
- `rusqlite` with WAL：`JOURNAL_MODE=OFF SYNCHRONOUS=OFF` 已在 bench_sqlite.rs 中使用；生产路径需 WAL 模式而非关闭 journal
- rayon `par_iter` — 已在 prescan 和 CSV 并行路径中使用

### Established Patterns
- 多文件处理：`SqllogParser::log_files()` 返回排序后的文件列表，并行路径 `par_iter()` 遍历
- 错误处理：parse error 写入 error log 继续处理（不 fatal），并行路径需保持此行为

### Integration Points
- Phase 43 重构后的 filter 模块被并行路径使用，需确保线程安全（`CompiledMetaFilters` / `CompiledSqlFilters` 应为 `Send + Sync`）
- Phase 44 优化后的性能基线作为 Phase 45 CI 的初始基准值

</code_context>

<specifics>
## Specific Ideas

- SQLite WAL 模式开启：`PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`
- CI artifact JSON 可以直接解析 criterion 输出的 `target/criterion/*/estimates.json`，不需要额外工具

</specifics>

<deferred>
## Deferred Ideas

- critcmp PR comment bot（自动评论性能变化）→ 超出本 milestone 范围
- AsyncLogParser tokio 异步 SQLite 写入 → 过度工程

</deferred>

---

*Phase: 45-并行扩展与 CI 基准集成*
*Context gathered: 2026-05-24*
