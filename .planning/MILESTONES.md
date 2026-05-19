# Milestones

## v1.5 — 文档完善 & 项目展示

**Shipped:** 2026-05-19
**Phases:** 21–23 | **Plans:** 8 | **Files changed:** 42 (+6,111 / -777 lines)

### Delivered

补全项目文档并建立 GitHub Pages 展示页面，零代码变更。README 从 395 行中英混排重写为 208 行纯英文骨架（覆盖 v1.3 模板分析 + v1.4 嵌套配置），CHANGELOG.md 补全 v1.0-v1.4 全部条目，docs/quickstart.md + docs/config-reference.md 全新创建，mdBook 落地页含 SVG Gallery，GitHub Actions 自动部署，lychee CI 防断链。

### Key Accomplishments

1. **README 全面重写** — 395 行混排 → 208 行纯英文骨架；6 枚 badges、Mermaid→ASCII 架构图、性能基准表、2 张 SVG PNG 截图、链接状态索引（DOC-01~09，Phase 21）
2. **CHANGELOG.md 完成** — Keep a Changelog 格式，v1.0–v1.4 五版本条目，v1.1 合并入 v1.2，0.x 405 行折叠为单段摘要（DOC-05，Phase 21）
3. **GitHub Pages 落地页** — mdBook 单页 7-section（Hero/Install/Features/Architecture/Performance/SVG Gallery/Links），4 类内联 SVG 图表，ayu 主题+自定义 CSS，GHA 自动部署（PAGES-01~05，SUPP-01，Phase 22）
4. **补充文档** — docs/quickstart.md（306 行，4 场景+故障排除）+ docs/config-reference.md（8 配置块 TOML 参考）（SUPP-02/03，Phase 23）
5. **Asciicast 演示** — site/src/asciicast/demo.cast（~30s）嵌入 Pages 交互播放器，README SVG badge 链接（SUPP-04，Phase 23）
6. **lychee 链接 CI** — .github/workflows/lychee.yml 内部严格+外部重试，防止文档断链回归（SUPP-05，Phase 23）

### Stats

- Timeline: 1 day (2026-05-18 → 2026-05-19)
- Files changed: 42 (+6,111 / -777 lines)
- Commits: ~20

### Archive

- `.planning/milestones/v1.5-ROADMAP.md`
- `.planning/milestones/v1.5-REQUIREMENTS.md`
- `.planning/milestones/v1.5-MILESTONE-AUDIT.md`

---

## v1.4 — 代码重构 & 质量深化

**Shipped:** 2026-05-18
**Phases:** 17–20 | **Plans:** 12 | **Commits:** ~90

### Delivered

系统性重构配置模型与代码结构：嵌套子表替代扁平字段（[filter.include]/[filter.exclude]、[template]、[charts]），5 个超大源文件模块化拆分，消除重复投影逻辑，统一 Exporter trait，收紧可见性。补全 7 份 VERIFICATION.md，新增 E2E/边界/proptest 测试。933 测试通过，cargo clippy 零警告。

### Key Accomplishments

1. 过滤器配置嵌套化 — 手写 `FiltersFeature::Deserialize` via `RawFiltersFeature` 中间结构，`IncludeFilters`/`ExcludeFilters` 替代 `MetaFilters`，serde alias 向后兼容旧扁平格式，`pipeline.is_empty()` 快路径不变（CONFIG-01/02/05，Phase 17）
2. 配置模型顶层化 — 删除 `PipelineConfig`，`Config` 5 个顶层 Option 字段，`pipeline_deprecated` 捕获旧 `[pipeline.*]` 并在 `validate()` 返回 5 条迁移映射，`TemplateConfig`/`OutputConfig` 独立 struct（CONFIG-03/04，Phase 18）
3. 代码结构重构 — filters.rs (1481→4 模块)、config/mod.rs (1418→3 模块)、csv.rs (1260→目录)、sqlite.rs (1302→目录)、run.rs (1281→4 模块)；`exporter/projection.rs` 消除投影 copy-paste；`DryRunExporter` → `ExporterKind::DryRun`；全 codebase 可见性收紧至 `pub(crate)`（REFACTOR-01/02/03/04，Phase 19）
4. 测试覆盖深化 — 补全 Phase 12/13/14/15/16/17/18 共 7 份 VERIFICATION.md；3 条 E2E 集成测试（过滤/归一化/投影）+ 4 条边界测试（空文件/全过滤/格式错误/1MB SQL）；2 条 proptest 属性测试（幂等性 + 字面量保护）（TEST-01/02/03/04，Phase 20）
5. 933 测试全通过，cargo clippy 零警告，性能基准编译无回归

### Stats

- Rust LOC: ~14,763 (src/)
- Files changed: 58 Rust/TOML files (+8,298 / -7,287)
- Test suite: 933 tests passing (426 lib + 445 bin + 62 integration)
- Timeline: 2 days (2026-05-17 → 2026-05-18)
- Commits: ~90

### Archive

- `.planning/milestones/v1.4-ROADMAP.md`
- `.planning/milestones/v1.4-REQUIREMENTS.md`

---

## v1.3 — SQL 模板分析 & 可视化

**Shipped:** 2026-05-17
**Phases:** 12–16 | **Plans:** 19 | **Commits:** ~102

### Delivered

实现完整的 SQL 模板分析流水线：`normalize_template()` 归一化引擎、`TemplateAggregator` 流式统计累积器（hdrhistogram）、双路统计输出（SQLite 表 + CSV 伴随文件）、四类 SVG 图表（频率条形图、耗时直方图、时间趋势折线图、用户饼图）。全程恒定内存，热循环快路径零影响。418 项测试通过，cargo clippy 零警告。

### Key Accomplishments

1. `normalize_template()` 共享扫描引擎——注释去除、IN 折叠、关键字大写、字面量保护，`ScanMode` 枚举复用 `fingerprint()` 扫描基础（TMPL-01，Phase 12）
2. TemplateAggregator 侧路径聚合——`Option<&mut TemplateAggregator>` 绑路，hdrhistogram ~24KB/模板，rayon map-reduce 合并，`pipeline.is_empty()` 快路径不受影响（TMPL-02，Phase 13）
3. 双路统计输出——SQLite `sql_templates` 表（单事务批量 INSERT）+ CSV `*_templates.csv` 伴随文件（itoa 零分配序列化），写入在 `finalize()` 之后（TMPL-04，Phase 14）
4. SVG 图表基础设施——plotters SVG-only（无字体/图像系统依赖），Top N 频率横向条形图 + 对数轴耗时直方图，`generate_charts` 在 `finalize()` 前调用（CHART-01/02/03，Phase 15）
5. 时间趋势折线图 + 用户饼图——`hour_counts` BTreeMap 小时桶 + `user_counts` AHashMap 用户桶，HSL 颜色生成，"Others" 溢出聚合（CHART-04/05，Phase 16）

### Stats

- Rust LOC: ~14,164 (src/)
- Src files modified: 18 (+3,121 / -96 lines)
- Test suite: 418 tests passing
- Timeline: 3 days (2026-05-15 → 2026-05-17)
- Commits: ~102

### Known Deferred Items at Close

- VERIFICATION.md 缺失（Phases 12/13/14/16）— 文档差距，非功能差距（见 v1.3-MILESTONE-AUDIT.md）

### Archive

- `.planning/milestones/v1.3-ROADMAP.md`
- `.planning/milestones/v1.3-REQUIREMENTS.md`

---

## v1.2 — 质量强化 & 性能深化

**Shipped:** 2026-05-15
**Phases:** 7–11 | **Plans:** 13 | **Commits:** ~103

### Delivered

消灭已知技术债（SQLite 错误可观测性 + SQL 注入防护 + Nyquist 审计补签），上线排除过滤器 FILTER-03，以 validate_and_compile() 消除双重 regex 编译，并完成数据驱动的热路径门控分析（已达当前瓶颈）。

### Key Accomplishments

1. SQLite 双重技术债修复——`handle_delete_clear_result()` 软失败区分 + ASCII 白名单校验 + 5 处 DDL 双引号转义，11 个新测试（DEBT-01/02，Phase 7）
2. 排除过滤器 FILTER-03 上线——7 个 `exclude_*` 字段 OR-veto 语义，21 个新测试，`pipeline.is_empty()` 快路径零额外开销（Phase 8）
3. `validate_and_compile()` 统一接口——彻底消除 run 路径双重 Regex::new()，update check 后台化，hyperfine 基线记录（PERF-11，Phase 9）
4. 热路径 D-G1 门控——samply Top-10 src/ 最高 4.6% < 5% 阈值，记录"已达当前瓶颈"结论，729 测试全通过（PERF-10，Phase 10）
5. Nyquist 审计链全段闭合——Phase 3/4/5 回溯补签、Phase 6 从零创建 VALIDATION.md，DEBT-03 4/4 条件满足（Phase 11）

### Stats

- Rust LOC: ~11,139 (src/)
- Test suite: 729 tests passing（673 → 729，增加 56 个测试）
- Timeline: 5 days (2026-05-10 → 2026-05-15)
- Commits: ~103

### Archive

- `.planning/milestones/v1.2-ROADMAP.md`
- `.planning/milestones/v1.2-REQUIREMENTS.md`

---

## v1.1 — 性能优化

**Shipped:** 2026-05-10
**Phases:** 3–6 | **Plans:** 12 | **Commits:** ~85

### Delivered

通过 profiling 定位热路径后，系统性提升 CSV 和 SQLite 导出性能，升级上游解析库至 1.0.0，651 测试全部通过。

### Key Accomplishments

1. Flamegraph + criterion benchmark 基础设施——定位 `parse_meta`/`LogIterator::next`/`_platform_memmove` 为主热路径（PERF-01）
2. CSV 条件 reserve + `bench_csv_format_only` 格式化层量化（~19.7M elem/s）；合成 benchmark 改善 -8.53%（PERF-02/03/08）
3. `include_performance_metrics=false` 配置项——完全跳过 `parse_performance_metrics()` 和 memrchr 扫描，热循环堆分配显著减少（PERF-08）
4. SQLite 批量事务 `batch_commit_if_needed()`——5x 性能差距（35.4ms vs 7.1ms/10k rows），WAL 模式用户决策移除（PERF-04）
5. SQLite `prepare_cached()`——三条导出路径复用已编译 statement，消除每行 `sqlite3_prepare_v3()` 开销（PERF-06）
6. dm-database-parser-sqllog 1.0.0 升级——mmap/par_iter/MADV_SEQUENTIAL 自动生效；index() API 不集成（流式无收益）；651 测试通过（PERF-07/09）

### Stats

- Rust LOC: ~9,889 (src/)
- Files modified: 165 (+11,746 / -5,638 lines)
- Test suite: 651 tests passing
- Timeline: 14 days (2026-04-26 → 2026-05-10)
- Performance: SQLite 5x batch improvement; CSV synthetic -8.53%; ~5.2M records/sec CSV synthetic

### Archive

- `.planning/milestones/v1.1-ROADMAP.md`
- `.planning/milestones/v1.1-REQUIREMENTS.md`

---

## v1.0 — 增强 SQL 内容过滤与字段投影

**Shipped:** 2026-04-18
**Phases:** 1–2 | **Plans:** 6 | **Commits:** ~33

### Delivered

让用户能够精确指定"导出哪些记录的哪些字段"——正则过滤 + AND 语义 + 输出字段控制全部上线。

### Key Accomplishments

1. Pre-compiled regex filter structs (`CompiledMetaFilters` + `CompiledSqlFilters`) with AND cross-field / OR intra-field semantics and startup validation via `Config::validate()`
2. `FilterProcessor` hot path integrated with compiled regex — regex filters on all 7 meta fields (usernames, client_ips, sess_ids, thrd_ids, statements, appnames, tags)
3. `FeaturesConfig::ordered_field_indices()` — returns user-specified field order as `Vec<usize>` for downstream projection
4. `CsvExporter` extended with `ordered_indices` — `build_header()` and `write_record_preparsed()` now project fields in user-specified order
5. `SqliteExporter` extended with `ordered_indices` — `build_create_sql()`, `build_insert_sql()`, `do_insert_preparsed()` all project by ordered index
6. End-to-end wiring through `ExporterManager::from_config()` and parallel CSV path `process_csv_parallel()`

### Stats

- Rust LOC: ~9,889
- Files modified: ~10 source files
- Test suite: 629+ tests passing
- Performance: zero-overhead fast path preserved (pipeline.is_empty() unchanged)

### Archive

- `.planning/milestones/v1.0-ROADMAP.md`
- `.planning/milestones/v1.0-REQUIREMENTS.md`
