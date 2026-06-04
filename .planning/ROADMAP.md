# Roadmap: sqllog2db

## Milestones

- ✅ **v1.17 多文件并行提速** — Phases 64–66.1 (shipped 2026-06-04)
- ✅ **v1.16.0 工程质量深化** — Phases 59–63 (shipped 2026-06-03)
- ✅ **v1.15 工程质量全面提升** — Phases 55–58 (shipped 2026-06-02)
- ✅ **v1.14 stats 时间段过滤** — Phases 53–54 (shipped 2026-06-02)
- ✅ **v1.13 SQL 统计分析** — Phases 50–52 (shipped 2026-06-01)
- ✅ **v1.12 CLI 体验全面提升** — Phases 46–49 (shipped 2026-06-01)
- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- ✅ **v1.4 代码重构 & 质量深化** — Phases 17–20 (shipped 2026-05-18)
- ✅ **v1.5 文档完善 & 项目展示** — Phases 21–23 (shipped 2026-05-19)
- ✅ **v1.6 文档中文化 & 延后需求补全** — Phases 24–27 (shipped 2026-05-19)
- ✅ **v1.7 项目精简** — Phases 28–34 (shipped 2026-05-20)
- ✅ **v1.10 质量加固与体验优化** — Phases 35–40 (shipped 2026-05-21)
- ✅ **v1.11 性能深化与依赖适配** — Phases 41–45 (shipped 2026-05-25)

## Phases

<details>
<summary>✅ v1.0 增强 SQL 内容过滤与字段投影 (Phases 1–2) — SHIPPED 2026-04-18</summary>

- [x] Phase 1: 正则字段过滤 — completed 2026-04-18
- [x] Phase 2: 输出字段控制 — completed 2026-04-18

Full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 性能优化 (Phases 3–6) — SHIPPED 2026-05-10</summary>

- [x] Phase 3: Profiling & Benchmarking — completed 2026-04-27
- [x] Phase 4: CSV 性能优化 — completed 2026-05-09
- [x] Phase 5: SQLite 性能优化 — completed 2026-05-10
- [x] Phase 6: 解析库集成 + 验收 — completed 2026-05-10

Full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2 质量强化 & 性能深化 (Phases 7–11) — SHIPPED 2026-05-15</summary>

- [x] Phase 7: 技术债修复 — completed 2026-05-10
- [x] Phase 8: 排除过滤器 — completed 2026-05-10
- [x] Phase 9: CLI 启动提速 — completed 2026-05-14
- [x] Phase 10: 热路径优化 — completed 2026-05-15
- [x] Phase 11: Nyquist 补签 — completed 2026-05-15

Full details: `.planning/milestones/v1.2-ROADMAP.md`

</details>

<details>
<summary>✅ v1.3 SQL 模板分析 & 可视化 (Phases 12–16) — SHIPPED 2026-05-17</summary>

- [x] Phase 12: SQL 模板归一化引擎 — completed 2026-05-15
- [x] Phase 13: TemplateAggregator 流式统计累积器 — completed 2026-05-15
- [x] Phase 14: Exporter 集成输出 — completed 2026-05-16
- [x] Phase 15: SVG 图表基础设施 + 前两类图表 — completed 2026-05-17
- [x] Phase 16: 剩余图表 — completed 2026-05-17

Full details: `.planning/milestones/v1.3-ROADMAP.md`

</details>

<details>
<summary>✅ v1.4 代码重构 & 质量深化 (Phases 17–20) — SHIPPED 2026-05-18</summary>

- [x] Phase 17: 过滤器配置嵌套化 — completed 2026-05-18
- [x] Phase 18: 模板 & 图表配置嵌套化 — completed 2026-05-18
- [x] Phase 19: 代码结构重构 — completed 2026-05-18
- [x] Phase 20: 测试覆盖深化 — completed 2026-05-18

Full details: `.planning/milestones/v1.4-ROADMAP.md`

</details>

<details>
<summary>✅ v1.5 文档完善 & 项目展示 (Phases 21–23) — SHIPPED 2026-05-19</summary>

- [x] Phase 21: README 全面更新 + 根文档补全 — completed 2026-05-19
- [x] Phase 22: GitHub Pages 落地页 + 部署流水线 — completed 2026-05-19
- [x] Phase 23: 补充文档 + CI 质量门禁 — completed 2026-05-19

Full details: `.planning/milestones/v1.5-ROADMAP.md`

</details>

<details>
<summary>✅ v1.6 文档中文化 & 延后需求补全 (Phases 24–27) — SHIPPED 2026-05-19</summary>

- [x] Phase 24: 文档中文化 & 去 SVG 化 — completed 2026-05-19
- [x] Phase 25: 延后文档补全 — completed 2026-05-19
- [x] Phase 26: GitHub Pages 多页文档站 — completed 2026-05-19
- [x] Phase 27: 模板报告独立输出 — completed 2026-05-19

Full details: `.planning/milestones/v1.6-ROADMAP.md`

</details>

<details>
<summary>✅ v1.7 项目精简 (Phases 28–34) — SHIPPED 2026-05-20</summary>

- [x] Phase 28: 移除图表、自更新、补全 — completed 2026-05-19
- [x] Phase 29: 移除统计与摘要 — completed 2026-05-19
- [x] Phase 30: 移除模板分析 — completed 2026-05-20
- [x] Phase 31: 移除断点续传 — completed 2026-05-20
- [x] Phase 32: 项目结构清理 — completed 2026-05-20
- [x] Phase 33: 核心功能验证 — completed 2026-05-20
- [x] Phase 34: 审计遗留修复 — completed 2026-05-20

Full details: `.planning/milestones/v1.7-ROADMAP.md`

</details>

<details>
<summary>✅ v1.10 质量加固与体验优化 (Phases 35–40) — SHIPPED 2026-05-21</summary>

- [x] **Phase 35: CLI --help 增强** — completed 2026-05-21
- [x] **Phase 36: 错误处理体系重构** — completed 2026-05-21
- [x] **Phase 37: stdin 管道输入与错误实时输出** — completed 2026-05-21
- [x] **Phase 38: 进度显示与统计摘要** — completed 2026-05-21
- [x] **Phase 39: CSV/管道/参数核心验证** — completed 2026-05-21
- [x] **Phase 40: SQLite/并行/最终质量门禁** — completed 2026-05-21

</details>

<details>
<summary>✅ v1.11 性能深化与依赖适配 (Phases 41–45) — SHIPPED 2026-05-25</summary>

- [x] **Phase 41: 依赖升级与 Parser 库适配** — completed 2026-05-25
- [x] **Phase 42: Criterion 基准测试基础设施** — completed 2026-05-25
- [x] **Phase 43: Parser 新 API 适配与 Filter 重构** — completed 2026-05-24
- [x] **Phase 44: 热路径与内存优化** — completed 2026-05-24
- [x] **Phase 45: 并行扩展与 CI 基准集成** — completed 2026-05-25

Full details: `.planning/phases/41-parser/`, `.planning/phases/42-criterion/`, `.planning/phases/43-parser-api-filter/`, `.planning/phases/44-hotpath/`, `.planning/phases/45-ci/`

</details>

<details>
<summary>✅ v1.12 CLI 体验全面提升 (Phases 46–49) — SHIPPED 2026-06-01</summary>

- [x] **Phase 46: 错误信息优化** — completed 2026-05-31
- [x] **Phase 47: 配置文件体验** — completed 2026-05-31
- [x] **Phase 48: 日志级别与运行提示** — completed 2026-06-01
- [x] **Phase 49: Glob 输入支持** — completed 2026-06-01

Full details: `.planning/milestones/v1.12-ROADMAP.md`

</details>

<details>
<summary>✅ v1.13 SQL 统计分析 (Phases 50–52) — SHIPPED 2026-06-01</summary>

- [x] **Phase 50: SQL 标准化引擎** — 将字面量替换为 `?` 占位符的标准化模块 (completed 2026-06-01)
- [x] **Phase 51: stats 子命令 CLI 脚手架** — 新增 `stats` 子命令及 `--top N` 参数 (completed 2026-06-01)
- [x] **Phase 52: 统计输出与 Exporter 集成** — 慢 SQL / 高频 SQL TOP-N 通过现有 exporter 输出 (completed 2026-06-01)

</details>

<details>
<summary>✅ v1.14 stats 时间段过滤 (Phases 53–54) — SHIPPED 2026-06-02</summary>

- [x] **Phase 53: 时间段配置与 CLI 参数** — 扩展 StatsConfig、opts.rs 新增 --from/--to、格式验证与优先级合并 (completed 2026-06-01)
- [x] **Phase 54: StatsAccumulator 时间过滤** — 在聚合层按 ts 字段跳过时间段外的记录 (completed 2026-06-02)

</details>

<details>
<summary>✅ v1.15 工程质量全面提升 (Phases 55–58) — SHIPPED 2026-06-02</summary>

- [x] **Phase 55: CI/CD 基础设施修复** — 修正 workflow action 版本、修复 release 竞争条件、添加 Cross.toml (completed 2026-06-02)
- [x] **Phase 56: stats 模块清理与 benchmark 稳定化** — 删除遗留 warn! 占位符、检查函数长度、确认 benchmark 信息性运行 (completed 2026-06-02)
- [x] **Phase 57: e2e 测试扩展** — run/init/stats 子命令 CLI 全链路测试补全 (completed 2026-06-02)
- [x] **Phase 58: cli/run 函数清理** — 超 40 行函数提取为私有函数 (completed 2026-06-02)

Full details: `.planning/milestones/v1.15-ROADMAP.md`

</details>

<details>
<summary>✅ v1.16.0 工程质量深化 (Phases 59–63) — SHIPPED 2026-06-03</summary>

- [x] **Phase 59: cli/run 与 exporter/pipeline 结构整理** — 识别剩余超 40 行函数并拆分，消除 exporter/pipeline 模块内重复代码 (completed 2026-06-03)
- [x] **Phase 60: 错误处理路径统一** — 统一错误转换和传播路径，删除冗余 unwrap/expect (completed 2026-06-03)
- [x] **Phase 61: Cross.toml SHA 固定** — 将 edge 浮动标签替换为固定 SHA digest，提升构建可复现性 (completed 2026-06-03)
- [x] **Phase 62: 文档完善** — 更新 README、新建 CHANGELOG、补全 config.toml 模板注释 (completed 2026-06-03)
- [x] **Phase 63: 测试覆盖提升** — 运行覆盖率分析并按结果补全关键路径测试 (completed 2026-06-03)

Full details: `.planning/milestones/v1.16.0-ROADMAP.md`

</details>

---

<details>
<summary>✅ v1.17 多文件并行提速 (Phases 64–66.1) — SHIPPED 2026-06-04</summary>

- [x] **Phase 64: CSV 并行路径基础设施** — 建立多文件 rayon 并行解析 + temp-file 拼接架构 (completed 2026-06-04)
- [x] **Phase 65: 行为等价性保障** — 字段格式/过滤管道/输出控制与单线程路径完全对齐，verbose 透传 (completed 2026-06-04)
- [x] **Phase 66: 兼容性验证与测试** — 全量测试通过，新增多文件 CSV 集成测试，config 格式不变 (completed 2026-06-04)
- [x] **Phase 66.1: 修复并行集成测试覆盖** (INSERTED) — jobs_override 强制并行路径 + write_heterogeneous_log + 2 条强制并行测试 (completed 2026-06-04)

Full details: `.planning/milestones/v1.17-ROADMAP.md`

</details>

## Phase Details

### Phase 35: CLI --help 增强
**Goal**: 通过 clap `after_help` 在 `--help` 输出添加达梦场景的实用示例，提升新用户上手体验
**Depends on**: Nothing (zero-risk standalone change)
**Requirements**: UX-03
**Success Criteria** (what must be TRUE):
  1. `sqllog2db --help` 在尾部显示 3-4 个达梦场景实用示例（如：导出全部日志、按用户过滤、指定时间段）
  2. `sqllog2db run --help` 包含运行相关的示例（如：stdin 管道输入、自定义输出路径）
  3. `sqllog2db validate --help` 和 `sqllog2db init --help` 也包含相关示例
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过
**Plans**: TBD

### Phase 36: 错误处理体系重构
**Goal**: 将 Error 枚举细分为 IO/格式/配置/导出四类，每条错误包含文件路径和行号上下文，非致命错误继续处理
**Depends on**: Phase 35
**Requirements**: ERR-01, ERR-02, ERR-03
**Success Criteria** (what must be TRUE):
  1. IO 错误显示文件路径和 `No such file` 上下文，用户知道哪个文件缺失
  2. 解析错误显示行号和修复建议（"第 42 行：SQL 格式异常，建议检查是否包含不支持的语法"）
  3. 配置错误（如无效 TOML）明确指出来哪个字段和期望格式
  4. 一条损坏的日志记录被记录到 error log 并继续处理下一条，不会终止整个导出
  5. 导出错误（如磁盘满）正确报告且不会 panic 崩溃
**Plans**: TBD

### Phase 37: stdin 管道输入与错误实时输出
**Goal**: 通过 `/dev/stdin` 路径映射支持 `--input -` 管道输入，stdin 模式跳过 pre-scan，非致命错误实时输出到 stderr
**Depends on**: Phase 36 (依赖新错误体系)
**Requirements**: PIPE-01, PIPE-02, UX-04
**Success Criteria** (what must be TRUE):
  1. `cat log | sqllog2db run -c config.toml --input -` 完整执行成功，输出结果正确
  2. stdin 模式跳过文件发现和 pre-scan，无虚假的 "file not found" 错误
  3. stdin 模式下事务级过滤降级时在 stderr 打印清晰警告
  4. 非致命错误在 stderr 实时输出，不受进度显示干扰，格式统一（错误类型: 文件:行号: 原因）
**Plans**: TBD

### Phase 38: 进度显示与统计摘要
**Goal**: 引入基于 `indicatif` 的进度条（每 1024 条更新），完成后输出统计摘要（总记录数、成功/错误数、处理速率、总耗时）
**Depends on**: Phase 37
**Requirements**: UX-01, UX-02
**Success Criteria** (what must be TRUE):
  1. 处理过程中每 1024 条更新一次进度，显示已处理记录数和经过时间
  2. 非终端（管道输出）时进度条自动退化为静态文本，不输出 ANSI 控制码
  3. 完成后输出统计摘要：总记录数、成功导出数、错误数、处理速率（条/秒）、总耗时
  4. 摘要中成功数和错误数明确区分，一目了然
**Plans**: TBD

### Phase 39: CSV/管道/参数核心验证
**Goal**: 对 CSV 导出、Pipeline 过滤器、参数归一化三项核心功能进行端到端验证，确保质量加固后功能完整
**Depends on**: Phase 38
**Requirements**: VER-01, VER-03, VER-04
**Success Criteria** (what must be TRUE):
  1. CSV 导出 10,000 条记录文件与期望输出逐行匹配，空文件正确输出仅含表头的 CSV
  2. Pipeline 的 include/exclude/indicators/sql 四种过滤器各自产生正确的过滤结果
  3. 参数归一化在三种模式（`?` 占位符、`:num` 命名参数、`:name` 命名参数）下均正确替换
  4. 边界情况（超大值、空值、特殊字符）处理正确，不丢失或损坏数据
**Plans**: TBD

### Phase 40: SQLite/并行/最终质量门禁
**Goal**: SQLite 导出和并行 CSV 验证，全链路 cargo build/test/clippy/fmt 通过，benchmark < 5% 性能退化
**Depends on**: Phase 39
**Requirements**: VER-02, VER-05, VER-06
**Success Criteria** (what must be TRUE):
  1. SQLite 导出生成有效的 `.db` 文件，schema 正确（字段名、类型、约束），记录数与源文件一致
  2. 并行 CSV（rayon）输出与顺序模式完全一致,多线程下无数据竞争或乱序
  3. `cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部通过，无任何警告
  4. `cargo bench` 相比 v1.9 基线性能退化 < 5%
**Plans**: TBD

### Phase 41: 依赖升级与 Parser 库适配
**Goal**: 将所有 Cargo 依赖升级到最新兼容版本，`dm-database-parser-sqllog` 升级到最新版本，编译通过且无 deprecated 警告
**Depends on**: Phase 40
**Requirements**: REFACTOR-02, PARSER-01
**Success Criteria** (what must be TRUE):
  1. `cargo update` 后 `cargo test` 全部通过，无任何测试回归
  2. `cargo build --release` 输出无 `deprecated` 警告，无任何 `warning:` 行
  3. `cargo clippy --all-targets -- -D warnings` 通过，clippy 不报告任何新问题
  4. `Cargo.lock` 中 `dm-database-parser-sqllog` 版本号高于 v1.10 基线版本
**Plans**: 1 plan
- [ ] 41-01-PLAN.md — 升级 dm-database-parser-sqllog 到 2.0.0 + cargo update + 三道质量门禁验证 + 清理过时 v1.1.0 注释

### Phase 42: Criterion 基准测试基础设施
**Goal**: 建立覆盖 CSV 导出、SQLite 导出、filter 路径（启用/禁用）、parser 原始解析速度四大场景的 criterion benchmark 套件，`cargo bench` 可独立运行
**Depends on**: Phase 41
**Requirements**: BENCH-01
**Success Criteria** (what must be TRUE):
  1. `cargo bench` 独立运行成功，不依赖外部数据文件或环境变量
  2. benchmark 覆盖四大场景：CSV 导出吞吐量、SQLite 导出吞吐量、filter 启用时吞吐量、filter 禁用时吞吐量、parser 原始解析速度
  3. 每个 benchmark group 包含 baseline 标注，输出包含 throughput（records/sec 或 MB/s）指标
  4. benchmark 代码通过 `cargo clippy --all-targets -- -D warnings` 检查，无警告
**Plans**: 1 plan
- [ ] 42-01-PLAN.md — 新增 benches/bench_parser.rs（parser_throughput group，1K/10K/50K 三规模）+ Cargo.toml 注册 [[bench]] 条目 + BENCHMARKS.md 追加 Phase 42 baseline 段落

### Phase 43: Parser 新 API 适配与 Filter 重构
**Goal**: 利用新版 `dm-database-parser-sqllog` 的新 API 删除冗余的手动映射代码；重构 filter 模块，使 pre-scan 与 main-pass 逻辑边界清晰，代码复杂度降低
**Depends on**: Phase 42
**Requirements**: PARSER-02, REFACTOR-01
**Success Criteria** (what must be TRUE):
  1. 利用新 API（如 `from_reader` 或新字段）替换的代码行数可通过 `git diff` 验证减少，无冗余手动映射逻辑残留
  2. filter 模块中 pre-scan 逻辑与 main-pass 逻辑处于独立函数或子模块中，职责不交叉
  3. 重构后单元测试覆盖的场景数量不低于重构前（`cargo test` 过滤 filter 模块全部通过）
  4. `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过，无新增警告
**Plans**: 2 plans
- [x] 43-01-PLAN.md — IndicatorFilters::matches 签名改为 u32 + 消除 prescan.rs 中 i64::from(rowcount) + 修订过时的 v1.1.0 注释
- [x] 43-02-PLAN.md — compiled.rs 与 prescan.rs 添加 Pre-scan/Main-pass section 注释 + 全套质量门禁

### Phase 44: 热路径与内存优化
**Goal**: 通过 profiling 定位热路径瓶颈后实施优化，使单线程吞吐量超越 v1.10 基线（1.55M records/sec），同时减少处理 1GB+ 文件时的峰值堆分配
**Depends on**: Phase 43 (需要稳定的 API 和 benchmark 基础设施)
**Requirements**: PERF-01, PERF-02
**Success Criteria** (what must be TRUE):
  1. `cargo bench` 显示单线程 CSV 导出吞吐量高于 1.55M records/sec（相比 v1.10 基线有可量化提升）
  2. Heaptrack 或 jemalloc 统计显示处理 1GB+ 文件时峰值堆分配低于 v1.10 基线（减少量可 diff 对比）
  3. 所有现有测试（`cargo test`）仍全部通过，无功能回归
  4. 优化变更不引入新的 `unsafe` 代码，或新增 `unsafe` 有文档注释说明安全性
**Plans**: TBD

### Phase 45: 并行扩展与 CI 基准集成
**Goal**: 扩展并行处理范围（SQLite 导出批量并行写入或多文件跨文件并行解析），并在 GitHub Actions CI 中集成 benchmark，每次 PR 自动导出基准报告供历史对比
**Depends on**: Phase 44
**Requirements**: PERF-03, BENCH-02
**Success Criteria** (what must be TRUE):
  1. SQLite 导出或多文件解析中至少一项支持并行处理，`cargo test` 包含并行路径的正确性验证（输出与顺序模式一致）
  2. GitHub Actions workflow 文件存在，PR 触发时自动运行 `cargo bench`，并将结果以 HTML 或 JSON 格式作为 artifact 上传
  3. CI benchmark artifact 包含足够信息（时间戳、commit SHA、各 benchmark 组的 mean/stddev）供历史趋势对比
  4. 全链路质量门禁通过：`cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部绿灯
**Plans**: 2 plans
- [ ] 45-01-PLAN.md — 新建 src/cli/run/sqlite_parallel.rs + SqliteExporter::set_wal_mode + mod.rs 路由扩展 + test_sqlite_parallel_matches_sequential
- [ ] 45-02-PLAN.md — 新建 .github/workflows/bench.yml + scripts/collect_bench_results.sh（PR + push to main 触发，artifact retention 60 天）

### Phase 46: 错误信息优化
**Goal**: 用户看到的每条错误都包含具体字段名/原因，并附带可操作的修复建议，让用户无需查阅文档就能自助解决配置和运行问题
**Depends on**: Phase 45
**Requirements**: ERROR-01, ERROR-02
**Success Criteria** (what must be TRUE):
  1. 配置错误（如字段类型错误、缺失必填项）在 stderr 显示出错字段名称和期望类型，例如 `error: field 'output.path' — expected string, got integer`
  2. 每条错误信息末尾附带 `hint:` 行，提供具体修复建议，例如 `hint: 将 output.path 设置为有效文件路径，如 "output/result.csv"`
  3. 运行时错误（文件不可读、磁盘满等）同样包含 hint，告知用户检查权限或磁盘空间
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，无性能退化
**Plans**: 1 plan
- [x] 46-01-PLAN.md — main.rs eprintln 前缀 Suggestion→hint + 抽取 format_error_output 辅助函数 + Error::Io hint 文本校验 + 单元测试 + tests/integration.rs 端到端 stderr 断言

### Phase 47: 配置文件体验
**Goal**: `init` 生成带注释的配置模板让用户一看即懂，`validate` 逐项显示每条校验结果让用户精确定位问题
**Depends on**: Phase 46
**Requirements**: CONFIG-01, CONFIG-02
**Success Criteria** (what must be TRUE):
  1. `sqllog2db init -o config.toml` 生成的文件中每个配置字段都有行内注释，说明用途和合法值示例（如 `# 输出路径，支持相对/绝对路径，例如 "output/result.csv"`）
  2. `sqllog2db validate -c config.toml` 对一个有效配置输出 `Configuration valid.`（CONTEXT D-03 静默通过，未输出 `[OK]` 列表——以 CONTEXT 决策为准；planning 计划已据此实现）
  3. `sqllog2db validate -c config.toml` 对一个包含错误的配置输出 `[FAIL] <字段>: <原因>` 与 `  hint: <修复建议>` 行（fail-fast 语义，首个失败即渲染并退出，符合 CONTEXT D-02）
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过
**Plans**: 2 plans
- [x] 47-01-PLAN.md — handle_validate 改 println! 静默通过 + main.rs Validate 分支失败渲染为 [FAIL] + tests/integration.rs 端到端 CLI 输出断言（CONFIG-02）
- [x] 47-02-PLAN.md — CONFIG_TEMPLATE_EN 补全 csv.{file,overwrite,append} 与 sqlite.{database_url,table_name,overwrite,append} 共 7 段行内注释 + tests/integration.rs 注释存在性断言（CONFIG-01）

### Phase 48: 日志级别与运行提示
**Goal**: 用户可通过 `--verbose` 和 `--quiet` 精确控制运行时输出的信息量，满足调试与静默脚本两种场景需求
**Depends on**: Phase 47
**Requirements**: LOG-01, LOG-02, LOG-03
**Success Criteria** (what must be TRUE):
  1. `sqllog2db run -c config.toml --verbose` 额外输出每个正在处理的文件名及过滤器匹配详情（每条匹配/跳过记录的原因）
  2. `sqllog2db run -c config.toml --quiet` 完全抑制进度条和运行摘要输出，stderr 仅在发生错误时才有内容
  3. 默认模式（不加标志）的运行结束摘要与加 `--verbose` 后的摘要内容不同，verbose 摘要包含更多字段（如每个文件的处理行数）
  4. `--verbose` 和 `--quiet` 互斥，同时指定时给出明确错误提示
  5. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过
**Plans**: 2 plans
- [x] 48-01-PLAN.md — opts.rs `-v` 改 bool + main.rs 移除 debug 映射并扩展 handle_run 调用 + cli/run/mod.rs 增 verbose 参数与 ProgressBar 条件化 + 35+ handle_run 调用点迁移 + 端到端 CLI 测试覆盖 LOG-01/LOG-02
- [x] 48-02-PLAN.md — sqlite_parallel 返回值对齐 csv_parallel 形态 + handle_run 顺序路径收集 per_file_counts + verbose 摘要前输出每文件 `Processed:` 明细 + 端到端测试覆盖 LOG-03
**UI hint**: yes

### Phase 49: Glob 输入支持
**Goal**: config.toml 的 input 字段和 CLI 的 --input 参数均支持 glob 模式，自动展开匹配文件列表，两种用法行为完全一致
**Depends on**: Phase 48
**Requirements**: INPUT-01, INPUT-02
**Success Criteria** (what must be TRUE):
  1. `input = ["sqllogs/*.log"]` 在 config.toml 中被解析后自动展开为所有匹配的 `.log` 文件,`cargo test` 包含此场景的单元测试
  2. `sqllog2db run -c config.toml --input 'logs/*.log'` 从命令行接收 glob 并展开，输出结果与手动列出所有文件一致
  3. 无匹配文件时给出明确错误（`error: glob pattern 'sqllogs/*.log' matched 0 files`），而非静默空输出
  4. glob 与直接路径混合使用时（如 `--input file1.log --input 'dir/*.log'`）均能正确处理
  5. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，不引入重量级依赖（使用 `glob` 或 `globset` crate）
**Plans**: 3 plans
- [x] 49-01-PLAN.md — SqllogConfig 改造 path→inputs: Vec<String> + path_deprecated 旧键检测 + ParserError::NoFilesFound 变体与 Error::suggestion 分支（schema + error 基础设施）
- [x] 49-02-PLAN.md — SqllogParser 改为 Vec<String> 多输入接口 + 调用方迁移（cli/run/mod.rs、preflight.rs、cli/validate.rs、config/mod.rs 与 config/validate.rs 内单元测试）+ handle_run 空列表抛 NoFilesFound
- [x] 49-03-PLAN.md — cli/opts.rs Run 增 --input/-i (ArgAction::Append) + main.rs apply_cli_inputs_to_config 注入 + CONFIG_TEMPLATE_EN [sqllog] 改为 inputs 数组 + tests/integration.rs 迁移与 4 个端到端 CLI 测试覆盖 INPUT-02

### Phase 50: SQL 标准化引擎
**Goal**: 用户可依赖一个内部 SQL 标准化模块，将 SQL 文本中的字符串和数字字面量替换为 `?` 占位符，从而把参数不同但模板相同的 SQL 调用归并为同一组
**Depends on**: Phase 49
**Requirements**: STATS-06
**Success Criteria** (what must be TRUE):
  1. `normalize_sql("SELECT * FROM t WHERE id = 42 AND name = 'alice'")` 返回 `"SELECT * FROM t WHERE id = ? AND name = ?"`
  2. 连续多个字面量（数字、带转义引号的字符串）均被替换，单次调用产生的占位符数量与字面量个数一致
  3. 不含字面量的 SQL（如 `SELECT 1` 或纯参数绑定查询）经过标准化后与输入相同，无误替换
  4. 标准化函数通过 `cargo test` 中的单元测试覆盖至少 5 种典型 SQL 模式（含边界情况）
  5. `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部通过
**Plans**: 1 plan
- [x] 50-01-PLAN.md — 新建 src/stats/{mod.rs,normalize.rs} + src/lib.rs 注册 pub mod stats + normalize_sql 字符扫描状态机 + 7 个单元测试（含 ROADMAP 5 模式 + 标识符/未闭合边界）

### Phase 51: stats 子命令 CLI 脚手架
**Goal**: 用户可运行 `sqllog2db stats -c config.toml [--top N]` 触发统计分析流程，CLI 参数被正确解析并传递到后续处理逻辑
**Depends on**: Phase 50
**Requirements**: STATS-01, STATS-02
**Success Criteria** (what must be TRUE):
  1. `sqllog2db stats --help` 显示 `stats` 子命令说明及 `-c/--config` 和 `--top` 参数的描述
  2. `sqllog2db stats -c config.toml` 在配置有效时不报错退出（即使统计输出暂为空也可），`--top` 缺省时使用默认值 20
  3. `sqllog2db stats -c config.toml --top 5` 将 TOP 数量限制为 5，`--top 0` 或负数给出明确错误提示
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过
**Plans**: 1 plan
- [x] 51-01-PLAN.md — opts.rs 新增 Commands::Stats { config, top } 变体 + cli/mod.rs 注册 stats 模块 + 新建 src/cli/stats/mod.rs handle_stats 桩函数（--top 0 校验 → ConfigError::InvalidValue）+ main.rs 分发分支（Config::from_file 不回落）+ needs_simple_logging 排除 Stats + tests/integration.rs 6 个端到端 CLI 测试覆盖 STATS-01/STATS-02

### Phase 52: 统计输出与 Exporter 集成
**Goal**: 用户运行 `stats` 后可在 config.toml 指定的 CSV 或 SQLite 文件中看到两张独立的统计表：慢 SQL TOP-N（按 elapsed 降序）和高频 SQL TOP-N（按调用次数降序）
**Depends on**: Phase 51
**Requirements**: STATS-03, STATS-04, STATS-05
**Success Criteria** (what must be TRUE):
  1. 慢 SQL 表包含字段：sql_text、elapsed（毫秒）、timestamp，记录按 elapsed 降序排列，行数不超过 `--top N`
  2. 高频 SQL 表包含字段：normalized_sql、call_count、avg_elapsed（毫秒）、max_elapsed（毫秒），记录按 call_count 降序排列，行数不超过 `--top N`
  3. 当 config.toml 配置 CSV exporter 时，输出为两个独立 CSV 文件（如 `slow_sql.csv` 和 `frequent_sql.csv`）；配置 SQLite 时，输出为同一 `.db` 文件中的两张独立表
  4. 对同一份日志文件，`--top 5` 输出的行数严格不超过 5 行（记录不足时按实际数量输出）
  5. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，不引入重量级新依赖

**Plans**: 1 plan
- [x] 52-01-PLAN.md — pub(crate) ensure_parent_dir/f32_ms_to_i64 + 新建 src/stats/{aggregate.rs,output.rs} StatsAccumulator (BinaryHeap<Reverse<SlowSqlEntry>> + HashMap<String,AggState>) + write_csv_stats/write_sqlite_stats 独立输出（DROP+CREATE）+ src/stats/mod.rs run_stats 编排（CSV 优先）+ src/cli/stats/mod.rs 接入 + 23 项单元/集成测试覆盖 STATS-03/STATS-04/STATS-05

### Phase 53: 时间段配置与 CLI 参数
**Goal**: 用户可通过 CLI 参数或 config.toml 为 `stats` 命令指定时间段过滤，格式被验证，优先级正确合并，为聚合层提供可用的时间范围值
**Depends on**: Phase 52
**Requirements**: STATS-07, STATS-08, STATS-09, STATS-11
**Success Criteria** (what must be TRUE):
  1. `sqllog2db stats -c config.toml --from "2024-01-01" --to "2024-01-31"` 不报错退出，`stats --help` 中可见 `--from` 和 `--to` 参数说明
  2. config.toml `[stats]` 节可配置 `from = "2024-01-01"` 和 `to = "2024-01-31"`，`sqllog2db validate -c config.toml` 通过验证
  3. CLI 参数存在时优先于 config 中的值；CLI 与 config 均未配置时，`stats` 命令正常运行且不做时间过滤
  4. `--from "not-a-date"` 或 `from = "20240101"` 等格式不合法的值给出明确错误提示（如 `error: --from 格式不合法，支持 "YYYY-MM-DD" 或 "YYYY-MM-DD HH:MM:SS"`）
  5. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过
**Plans**: 3 plans
- [x] 53-01-PLAN.md — 新建 src/stats/config.rs（StatsConfig + validate_time_str）+ stats/mod.rs 注册子模块 + Config 根结构追加 #[serde(default)] pub stats: StatsConfig
- [x] 53-02-PLAN.md — opts.rs Stats 变体新增 --from/--to + --top 改 Option<u32> + handle_stats 优先级合并（D-05）+ main.rs 分发分支接入新签名
- [x] 53-03-PLAN.md — Config::validate 与 run_stats 接入 validate_time_str + CONFIG_TEMPLATE_EN 追加 [stats] 注释段 + tests/integration.rs 新增 7 个端到端 stats 测试
- [x] 53-01-PLAN.md — 新建 src/stats/config.rs（StatsConfig + validate_time_str）+ stats/mod.rs 注册子模块 + Config 根结构追加 #[serde(default)] pub stats: StatsConfig
- [x] 53-02-PLAN.md — opts.rs Stats 变体新增 --from/--to + --top 改 Option<u32> + handle_stats 优先级合并（D-05）+ main.rs 分发分支接入新签名
- [x] 53-03-PLAN.md — Config::validate 与 run_stats 接入 validate_time_str + CONFIG_TEMPLATE_EN 追加 [stats] 注释段 + tests/integration.rs 新增 7 个端到端 stats 测试

### Phase 54: StatsAccumulator 时间过滤
**Goal**: `stats` 命令在聚合统计时自动跳过 `ts` 字段不在指定时间段内的记录，时间段过滤对慢 SQL 和高频 SQL 两张表均生效
**Depends on**: Phase 53
**Requirements**: STATS-10
**Success Criteria** (what must be TRUE):
  1. 对包含多日记录的日志，`--from "2024-01-15" --to "2024-01-15"` 只统计该日的记录，慢 SQL 表和高频 SQL 表的 timestamp 字段均在范围内
  2. `--from` 和 `--to` 均未设置时，聚合结果与未加时间过滤的结果完全一致（无行为变化）
  3. 只设置 `--from`（不设 `--to`）时，只过滤早于 `from` 的记录，晚于 `from` 的记录均被统计；只设置 `--to` 时，只过滤晚于 `to` 的记录
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，性能无明显退化（字符串前缀比较不引入额外分配）
**Plans**: 1 plan
- [ ] 54-01-PLAN.md — StatsAccumulator 新增 from/to 字段与 in_range 守卫 + update 范围外 return + run_stats 调用点接入 cfg.stats.from/to + 删除 cli/stats/mod.rs "not yet active" warn 占位 + 11 个单元测试（6 迁移 + 5 新过滤）+ 2 个端到端 stats --from/--to CLI 测试

### Phase 55: CI/CD 基础设施修复
**Goal**: CI/CD workflow 能够在三平台无错误运行，tag 推送触发 4 个平台二进制构建并正确创建 GitHub Release，aarch64-linux 跨编译通过 Cross.toml 配置无需手动干预
**Depends on**: Phase 54
**Requirements**: CICD-01, CICD-02, CICD-03, CICD-04
**Success Criteria** (what must be TRUE):
  1. 推送 PR 后，GitHub Actions CI 在 ubuntu/windows/macos 三平台自动运行 test/clippy/fmt 全部绿灯（使用正确的 `actions/checkout@v4` 和 `actions/upload-artifact@v4`）
  2. 推送 tag 后，CD workflow 成功构建 4 个平台（x86_64-linux、aarch64-linux、x86_64-windows、x86_64-macos）的二进制并在 GitHub Releases 中创建对应 release
  3. 4 个 matrix job 并行运行时 release body 内容完整、无重复条目，不存在因竞争写入导致的数据丢失（独立 create-release job 先于 upload-artifact job 运行）
  4. 项目根目录存在 `Cross.toml`，aarch64-linux 构建使用正确的 Docker 镜像，`cross build --target aarch64-unknown-linux-gnu` 无需手动配置即可执行
**Plans**: 2 plans
- [x] 55-01-PLAN.md — 修复 ci.yaml/bench.yml/lychee.yml/pages.yml 的 @v6/@v7 action 版本统一为 @v4（D-01/D-02）+ cargo 质量门禁（CICD-01）
- [x] 55-02-PLAN.md — 新建 Cross.toml（ghcr.io edge 镜像）+ 重构 release.yaml 为 artifact 暂存 + 独立 create-release job + 删除 publish job（CICD-02/03/04，D-04/D-05/D-06/D-07/D-08）

### Phase 56: stats 模块清理与 benchmark 稳定化
**Goal**: stats 模块代码整洁无遗留占位符，所有函数符合 40 行限制，benchmark 以信息性方式集成到 CI 并有配套采集脚本
**Depends on**: Phase 55 (CI 稳定后 benchmark workflow 才有意义)
**Requirements**: CLEAN-01, BENCH-01
**Success Criteria** (what must be TRUE):
  1. `src/cli/stats/mod.rs` 中不存在任何 `warn!` 占位符调用（已删除 "not yet active" 类占位符）
  2. `src/stats/output.rs` 中所有函数体不超过 40 行（可通过 `cargo clippy` 配合代码审查验证）
  3. `scripts/collect_bench_results.sh` 文件存在且可执行，脚本说明其用途
  4. `.github/workflows/bench.yml` 中 benchmark job 设置 `continue-on-error: true`，不作为 merge 门控
**Plans**: 2 plans
- [x] 56-01-PLAN.md — 新建 src/scanner.rs 公共扫描模块（D-01）+ src/lib.rs 注册 pub(crate) mod scanner + src/stats/mod.rs 重构 scan_files_into_accumulator 调用 scanner（D-02）+ grep/awk 验证 CLEAN-01 静态条件（cli/stats 无 warn!、output.rs 函数 ≤40 行）
- [x] 56-02-PLAN.md — src/cli/run/processor.rs 接入 scanner（D-03，限定 parser 创建+迭代循环范围，签名不变）+ benches/BENCHMARKS.md 追加 CI Artifact 使用说明章节（D-04，命名规则/下载方式/JSON 结构/手动对比方法）+ stat/grep 验证 BENCH-01 静态条件

### Phase 57: e2e 测试扩展
**Goal**: run/init/stats 子命令均有 CLI 全链路 assert_cmd 测试，涵盖正常路径、退出码、边界条件，为后续重构提供安全网
**Depends on**: Phase 56
**Requirements**: TEST-01, TEST-02, TEST-03
**Success Criteria** (what must be TRUE):
  1. `tests/integration.rs` 包含 `run` 子命令的端到端测试：给定真实输入文件，验证 CSV 输出内容（字段名+记录数）与退出码 0；给定真实输入文件，验证 SQLite 输出文件存在且退出码 0
  2. `tests/integration.rs` 包含 `init` 子命令的 assert_cmd 测试：`sqllog2db init -o /tmp/config.toml` 成功生成文件并退出码 0；文件已存在时不加 `--force` 退出码非零并输出错误信息
  3. `tests/integration.rs` 包含 `stats --from/--to` 边界条件测试：空时间范围（from > to）给出明确错误、边界值（from == to）正常运行、无效格式（非日期字符串）被拒绝并退出码非零
  4. `cargo test` 全部通过，新增测试不依赖外部服务或网络
**Plans**: 2 plans
- [x] 57-01-PLAN.md — validate_stats_time_range 新增 from ≤ to 跨字段检查（D-01/D-02）+ 4 个单元测试 + test_cli_stats_rejects_from_after_to e2e 测试覆盖 TEST-03
- [x] 57-02-PLAN.md — 新增 write_run_config_toml / write_run_sqlite_config_toml 两 helper + 4 个 e2e 测试（run CSV header+行数、run SQLite sqllog_records 表行数、init 新建成功、init 已存在退出非零）覆盖 TEST-01/TEST-02

### Phase 58: cli/run 函数清理
**Goal**: cli/run 模块中超过 40 行的函数被提取为私有辅助函数，代码可读性提升，已有 e2e 测试确认无行为变化
**Depends on**: Phase 57 (e2e 测试是重构的安全网，必须先于重构存在)
**Requirements**: CLEAN-02
**Success Criteria** (what must be TRUE):
  1. `src/cli/run/mod.rs` 中每个函数体不超过 40 行（以 `fn` 关键字开头计算）
  2. 提取出的私有函数命名清晰，反映其单一职责（不使用 `helper` 等无意义命名）
  3. `cargo test` 全部通过（包含 Phase 57 新增的 e2e 测试），无任何行为变化
  4. `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过，无新增警告
**Plans**: 1 plan
- [x] 58-01-PLAN.md — handle_run 拆分为 7 个私有辅助函数 (resolve_input_files / merge_trxid_prescan / make_progress_bar / run_csv_parallel / run_sqlite_parallel / run_sequential / print_run_summary) + handle_run 本体改造为 D-04 模式 (merged.as_ref().unwrap_or(cfg)) + 全函数体 ≤40 行验证 + cargo clippy/test/fmt 三道质量门禁 (CLEAN-02)

### Phase 59: cli/run 与 exporter/pipeline 结构整理
**Goal**: cli/run 中所有超过 40 行的函数（handle_run 以外）被语义拆分，exporter/pipeline 模块内重复代码消除，模块职责边界清晰
**Depends on**: Phase 58
**Requirements**: STRUCT-01, STRUCT-02
**Success Criteria** (what must be TRUE):
  1. `src/cli/run/` 目录下所有函数体不超过 40 行，每个函数名称反映单一职责（可通过代码审查 + `cargo clippy` 验证）
  2. exporter 模块内的重复逻辑（如相同的字段序列化、路径创建逻辑）提取为共享函数，`git diff` 可见代码行数净减少
  3. pipeline 模块的子模块边界清晰：过滤器定义、编译逻辑、执行逻辑分属不同文件或清晰命名的子模块，不存在跨职责的函数
  4. `cargo test` 全部通过，无任何行为变化；`cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过
**Plans**: TBD

### Phase 60: 错误处理路径统一
**Goal**: 整个代码库的错误转换和传播路径统一，冗余的 unwrap/expect 被替换为显式错误传播，错误信息清晰可追溯
**Depends on**: Phase 59
**Requirements**: STRUCT-03
**Success Criteria** (what must be TRUE):
  1. `grep -r 'unwrap\(\)\|expect(' src/` 的结果中，每个 unwrap/expect 均有注释说明其不可失败的原因，或已被替换为 `?` 传播
  2. 错误从产生点到 main.rs 的传播路径一致：使用 `From` 自动转换而非手动 `.map_err`，且 From 实现位于 `src/error.rs`（或统一位置）
  3. `cargo test` 全部通过；`cargo clippy --all-targets -- -D warnings` 通过，clippy 不报告 `unwrap_used` 或 `expect_used` 相关警告
  4. 错误处理重构前后功能行为不变，已有 e2e 测试（Phase 57 新增）全部通过
**Plans**: 1 plan
  - [x] 60-01-PLAN.md — 为 logging.rs:60 与 parallel.rs:87 添加 infallible 注释，并交付全代码库 unwrap/expect/map_err 审计 + cargo clippy/test 兜底验证

### Phase 61: Cross.toml SHA 固定
**Goal**: Cross.toml 中 aarch64-linux 构建镜像的 edge 浮动标签被替换为固定 SHA digest，任意时刻执行 `cross build` 都使用相同的镜像层，构建结果可复现
**Depends on**: Phase 58 (不依赖代码重构，可独立执行；但按里程碑顺序排在 Phase 60 之后)
**Requirements**: CROSS-01
**Success Criteria** (what must be TRUE):
  1. `Cross.toml` 中镜像引用格式为 `image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu@sha256:<digest>"`，不含 `:edge` 或其他浮动标签
  2. `cross build --target aarch64-unknown-linux-gnu --dry-run`（或等效验证）可成功解析该镜像引用，无报错
  3. `Cross.toml` 中包含注释记录该 SHA 对应的镜像日期或版本，便于日后审计和更新
  4. `cargo clippy --all-targets -- -D warnings` + `cargo test` 通过（Cross.toml 变更不影响本地编译）
**Plans**: 1 plan
- [ ] 61-01-PLAN.md — 获取 ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge 当前 SHA256 digest + 更新 Cross.toml 为 @sha256: 格式 + 添加源信息注释 + cargo clippy/test 验证 (CROSS-01)

### Phase 62: 文档完善
**Goal**: README 反映 v1.13–v1.15 全部新功能，CHANGELOG 完整覆盖 v1.0–v1.15 历史，config.toml init 模板每个字段均有内联注释
**Depends on**: Phase 61
**Requirements**: DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
  1. README.md 包含 `stats` 子命令的用法示例（含 `--from`/`--to` 参数）、v1.15 CI/CD 修复说明，功能列表与当前代码一致
  2. CHANGELOG.md 存在，采用 Keep a Changelog 格式（`## [Unreleased]`、`## [1.15.0]` 等标准节标题），覆盖 v1.0 至 v1.15 全部版本的 Added/Changed/Fixed 条目
  3. `sqllog2db init -o /tmp/test.toml` 生成的文件中，`[stats]` 节的 `from`/`to` 字段和 `[filters]` 各子字段均有行内注释，无任何字段缺少注释
  4. `cargo test` 中关于 `init` 模板注释存在性的断言（Phase 47 新增）仍全部通过
**Plans**: 3 plans
- [x] 62-01-PLAN.md — src/cli/init.rs CONFIG_TEMPLATE_EN 补全 [filter.include]/[filter.exclude]/[filter.indicators]/[filter.sql] 共 22 个示例字段行内注释（DOC-03）
- [x] 62-02-PLAN.md — README.md 追加 stats --from/--to 用法示例 + 新增 `## 版本亮点` 节描述 v1.15 CI/CD 修复 + 清理末尾遗留字符（DOC-01）
- [x] 62-03-PLAN.md — CHANGELOG.md 顶部新增 `## [Unreleased]` + 补全 `## [1.15.0]`/`## [1.14.0]` 章节 + 现有版本标题升级为 X.Y.Z 三段式 + 同步链接引用列表（DOC-02）

### Phase 63: 测试覆盖提升
**Goal**: llvm-cov 覆盖率报告生成完毕，关键路径（过滤器 edge case、exporter 单元逻辑、错误路径）的行覆盖率相比分析前有可量化提升
**Depends on**: Phase 62
**Requirements**: TEST-01, TEST-02
**Success Criteria** (what must be TRUE):
  1. `cargo llvm-cov --html`（或 tarpaulin 等效）成功生成覆盖率报告，报告文件保存在 `target/llvm-cov/` 或等效路径，整体行覆盖率数字被记录
  2. 覆盖率报告识别出至少 3 个覆盖不足区域（行覆盖率低于 60% 的函数或模块），在 Phase 计划文档中列出
  3. 按分析结果补全的测试使识别出的覆盖不足区域行覆盖率达到 80% 以上，或有文档说明为何该路径难以测试（如 OS 相关错误路径）
  4. `cargo test` 全部通过，新增测试不依赖外部服务或网络；`cargo clippy --all-targets -- -D warnings` 通过

**Plans**: 4 plans
- [x] 63-01-PLAN.md — baseline 覆盖率报告生成 + pipeline/filters/types.rs mod tests（间接覆盖 serde_helpers.rs + 旧格式/混合格式 + has_filters 分支）
- [x] 63-02-PLAN.md — exporter/csv/tests.rs has_metrics=false 与字段投影分支测试 + exporter/sqlite/tests.rs conn_ref Err / 字段投影 / pragma 间接验证测试
- [x] 63-03-PLAN.md — error.rs mod tests 末尾追加 12+ 个错误变体方法测试 + cli/run/prescan.rs 新建 mod tests 覆盖 build_indicator_filters/build_sql_exclude_filters 边界
- [x] 63-04-PLAN.md — 重新运行 cargo llvm-cov 采集 after 数字 + 生成 63-COVERAGE-REPORT.md 对比报告（baseline → after + 难以测试路径 D-04 文档化）+ 三道质量门禁验证

---

### Phase 64: CSV 并行路径基础设施
**Goal**: 用户输入多个文件且目标为 CSV 时，工具自动切换到多文件并行解析路径，各解析线程通过 channel 将记录传递给单一写入线程，内存占用不随文件数量线性增长
**Depends on**: Phase 63
**Requirements**: PARALLEL-01, PARALLEL-02
**Success Criteria** (what must be TRUE):
  1. 输入 2 个以上 .log 文件 + CSV 输出时，工具自动使用并行路径，无需修改 config.toml 任何字段
  2. 并行路径中每个 rayon 解析线程通过 channel 将记录发送给写入线程，写入线程持有唯一的 BufWriter，无全量内存缓冲
  3. 处理 3 个 300MB 文件时，进程峰值内存使用不超过单线程路径的 2 倍（channel back-pressure 生效）
  4. 输入仅 1 个文件时回退到单线程路径，行为与现有实现完全一致
**Plans**: 1 plan
- [x] 64-01-PLAN.md — 运行质量门禁（cargo test + clippy）核查 SC1–SC4 + 更新 REQUIREMENTS.md PARALLEL-02 描述与 temp-file 实现对齐（D-01）

### Phase 65: 行为等价性保障
**Goal**: 并行路径产生的 CSV 内容、过滤结果、输出控制与单线程路径在语义上完全等价，同时 BufReader 缓冲区扩容以减少大文件系统调用
**Depends on**: Phase 64
**Requirements**: PARALLEL-03, PARALLEL-04, PARALLEL-05, IO-01
**Success Criteria** (what must be TRUE):
  1. 对同一组输入文件，并行路径与单线程路径输出的 CSV 行集合完全相同（忽略文件间行顺序），字段值、转义、has_metrics 条件逐字节一致
  2. 启用 include/exclude/sql/indicators 任意组合过滤器时，并行路径过滤后的记录数与单线程路径完全一致
  3. `--verbose` 在并行路径下输出每个文件的处理进度，`--quiet` 完全抑制所有非错误输出,处理摘要（总行数/错误数）正确累加
  4. 读取 .log 文件的 BufReader 缓冲区大小 ≥ 64KB（代码可审查，或通过 strace 系统调用次数对比验证）
**Plans**: 1 plan
- [x] 65-01-PLAN.md — process_csv_parallel/run_parallel_tasks 新增 verbose: bool + 逐文件 eprintln + mod.rs 透传 + IO-01 mmap 注释 + 三道质量门禁（PARALLEL-03/04/05, IO-01）

### Phase 66: 兼容性验证与测试
**Goal**: 现有全量测试在并行路径引入后继续通过，新增集成测试验证多文件 CSV 内容一致性，config.toml 格式和 init 模板不受影响
**Depends on**: Phase 65
**Requirements**: COMPAT-01, COMPAT-02, COMPAT-03
**Success Criteria** (what must be TRUE):
  1. `cargo test` 运行全部 740+ 测试（lib/integration/benchmark）全部通过，无任何回归
  2. `tests/integration.rs` 包含至少 2 条新集成测试：多文件并行 CSV 输出与逐文件单线程合并结果的内容对比断言（行集合相等）
  3. `sqllog2db init -o /tmp/test.toml` 生成的 config.toml 与 v1.16 基线内容一致，不含并行相关新字段
  4. `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过，无新增警告
**Plans**: 1 plan
- [x] 66-01-PLAN.md — tests/integration.rs 新增 test_parallel_csv_content_matches_sequential + test_parallel_csv_filter_matches_sequential + test_init_no_parallel_fields + 全量 cargo test（COMPAT-01/02/03）

## Coverage Validation

| Requirement | Phase |
|-------------|-------|
| UX-03       | 35    |
| ERR-01      | 36    |
| ERR-02      | 36    |
| ERR-03      | 36    |
| PIPE-01     | 37    |
| PIPE-02     | 37    |
| UX-04       | 37    |
| UX-01       | 38    |
| UX-02       | 38    |
| VER-01      | 39    |
| VER-03      | 39    |
| VER-04      | 39    |
| VER-02      | 40    |
| VER-05      | 40    |
| VER-06      | 40    |
| REFACTOR-02 | 41    |
| PARSER-01   | 41    |
| BENCH-01    | 56    |
| PARSER-02   | 43    |
| REFACTOR-01 | 43    |
| PERF-01     | 44    |
| PERF-02     | 44    |
| PERF-03     | 45    |
| BENCH-02    | 45    |
| ERROR-01    | 46    |
| ERROR-02    | 46    |
| CONFIG-01   | 47    |
| CONFIG-02   | 47    |
| LOG-01      | 48    |
| LOG-02      | 48    |
| LOG-03      | 48    |
| INPUT-01    | 49    |
| INPUT-02    | 49    |
| STATS-06    | 50    |
| STATS-01    | 51    |
| STATS-02    | 51    |
| STATS-03    | 52    |
| STATS-04    | 52    |
| STATS-05    | 52    |
| STATS-07    | 53    |
| STATS-08    | 53    |
| STATS-09    | 53    |
| STATS-11    | 53    |
| STATS-10    | 54    |
| CICD-01     | 55    |
| CICD-02     | 55    |
| CICD-03     | 55    |
| CICD-04     | 55    |
| CLEAN-01    | 56    |
| TEST-01     | 57    |
| TEST-02     | 57    |
| TEST-03     | 57    |
| CLEAN-02    | 58    |
| STRUCT-01   | 59    |
| STRUCT-02   | 59    |
| STRUCT-03   | 60    |
| CROSS-01    | 61    |
| DOC-01      | 62    |
| DOC-02      | 62    |
| DOC-03      | 62    |
| TEST-01     | 63    |
| TEST-02     | 63    |
| PARALLEL-01 | 64    |
| PARALLEL-02 | 64    |
| PARALLEL-03 | 65    |
| PARALLEL-04 | 65    |
| PARALLEL-05 | 65    |
| IO-01       | 65    |
| COMPAT-01   | 66    |
| COMPAT-02   | 66    |
| COMPAT-03   | 66    |
| PARALLEL-06 | 66.1  |
| PARALLEL-07 | 66.1  |

**68/68 requirements mapped — coverage: 100%**

## Progress

| Phase | Milestone | Status | Completed |
|-------|-----------|--------|-----------|
| 35. CLI --help 增强 | v1.10 | Complete | 2026-05-21 |
| 36. 错误处理体系重构 | v1.10 | Complete | 2026-05-21 |
| 37. stdin 管道输入与错误实时输出 | v1.10 | Complete | 2026-05-21 |
| 38. 进度显示与统计摘要 | v1.10 | Complete | 2026-05-21 |
| 39. CSV/管道/参数核心验证 | v1.10 | Complete | 2026-05-21 |
| 40. SQLite/并行/最终质量门禁 | v1.10 | Complete | 2026-05-21 |
| 41. 依赖升级与 Parser 库适配 | v1.11 | Complete | 2026-05-25 |
| 42. Criterion 基准测试基础设施 | v1.11 | Complete | 2026-05-25 |
| 43. Parser 新 API 适配与 Filter 重构 | v1.11 | Complete | 2026-05-24 |
| 44. 热路径与内存优化 | v1.11 | Complete | 2026-05-24 |
| 45. 并行扩展与 CI 基准集成 | v1.11 | Complete | 2026-05-25 |
| 46. 错误信息优化 | v1.12 | 1/1 | Complete | 2026-05-31 |
| 47. 配置文件体验 | v1.12 | 2/2 | Complete | 2026-05-31 |
| 48. 日志级别与运行提示 | v1.12 | 2/2 | Complete | 2026-06-01 |
| 49. Glob 输入支持 | v1.12 | 3/3 | Complete | 2026-06-01 |
| 50. SQL 标准化引擎 | v1.13 | 1/1 | Complete | 2026-06-01 |
| 51. stats 子命令 CLI 脚手架 | v1.13 | 1/1 | Complete | 2026-06-01 |
| 52. 统计输出与 Exporter 集成 | v1.13 | 1/1 | Complete | 2026-06-01 |
| 53. 时间段配置与 CLI 参数 | v1.14 | 3/3 | Complete | 2026-06-01 |
| 54. StatsAccumulator 时间过滤 | v1.14 | Complete | 2026-06-02 |
| 55. CI/CD 基础设施修复 | v1.15 | 2/2 | Complete | 2026-06-02 |
| 56. stats 模块清理与 benchmark 稳定化 | v1.15 | 2/2 | Complete | 2026-06-02 |
| 57. e2e 测试扩展 | v1.15 | 2/2 | Complete | 2026-06-02 |
| 58. cli/run 函数清理 | v1.15 | 1/1 | Complete | 2026-06-02 |
| 59. cli/run 与 exporter/pipeline 结构整理 | v1.16.0 | Complete | 2026-06-03 |
| 60. 错误处理路径统一 | v1.16.0 | Complete | 2026-06-03 |
| 61. Cross.toml SHA 固定 | v1.16.0 | Complete | 2026-06-03 |
| 62. 文档完善 | v1.16.0 | Complete | 2026-06-03 |
| 63. 测试覆盖提升 | v1.16.0 | Complete | 2026-06-03 |
| 64. CSV 并行路径基础设施 | v1.17 | Complete | 2026-06-04 |
| 65. 行为等价性保障 | v1.17 | Complete | 2026-06-04 |
| 66. 兼容性验证与测试 | v1.17 | Complete | 2026-06-04 |
| 66.1. 修复并行集成测试覆盖 (INSERTED) | v1.17 | Complete | 2026-06-04 |

---
*Created: 2026-05-21 for milestone v1.10*
*Updated: 2026-06-04 — v1.17 milestone (Phases 64–66.1) shipped*
