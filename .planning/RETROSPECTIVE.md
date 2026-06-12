# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

---

## Milestone: v1.20 — 性能全面提升

**Shipped:** 2026-06-11  
**Phases:** 5 (72–76) | **Plans:** 9 | **Duration:** ~4 days | **Commits:** 87

### What Was Built

- hyperfine 冷启动基线建立（Phase 72）：`--version` 2.1ms、`validate` 2.2ms，Phase 72 段落写入 BENCHMARKS.md，benches/hyperfine-validate.toml 配置文件
- criterion v1.20 baseline 存档（Phase 72）：`CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20`，4 个 group baseline 文件完成
- SQLite multi-row batch INSERT（Phase 73）：`SqliteExporterConfig.multi_row_batch_size`（默认 64）、`row_buffer` + `flush_batch` + `sql_cache` 实现，`build_multi_row_insert_sql` SQL builder，benchmark 量化写入 BENCHMARKS.md
- ParamBuffer 二级化（Phase 74）：扁平 tuple key 重构为二级 HashMap，DML 查询路径改用 `Borrow<str>` 零分配 `&str` 查询（MEM-01）；`CsvExporter::new()` `Vec::with_capacity(4096)` 预热（MEM-02）
- record_iter 共享模块（Phase 75）：`parallel.rs` + `sqlite_parallel.rs` 重复迭代循环提取为 `record_iter.rs::iterate_records`，净消除 ~80 行重复代码
- tokio 异步迁移（Phase 76）：`#[tokio::main]` + `AsyncLogParser` 全解析路径迁移，503 tests 全绿，3.8MB release，clippy 零警告

### What Worked

- **Code Review 串联修复**：Phase 73/74/75/76 均通过 REVIEW.md → fix commit 循环修复发现的 bug（WR/IN/CR 类问题），4 个 phase 合计修复 ~15 处问题，没有带着已知 bug 进入下一个 phase
- **Phase 75 FnMut 闭包设计**：`iterate_records` 接受 FnMut 闭包而非泛型 trait，允许 CSV/SQLite 路径各自注入差异化写出逻辑，无需泛型约束爆炸
- **block_in_place 模式**：tokio 迁移后 rayon + BufWriter 阻塞操作用 `block_in_place` 包裹，避免 tokio 线程饥饿，性能无退化
- **Phase 73 完整 TDD**：`build_multi_row_insert_sql` 函数有独立单元测试（含边界：0 行、projection 模式），batch flush 有 drain 模式保证最后一批不满 N 条时正确处理

### What Was Inefficient

- **REQUIREMENTS.md checkbox 延迟更新**（再次出现）：7/8 需求实现后未及时勾选，里程碑审计时发现为文档技术债务，在 complete-milestone 流程中统一修复
- **Phase 73 Plans TBD**：ROADMAP.md Phase 73 段落 Plans 字段写的 "TBD"（阶段性记录习惯），归档时从 SUMMARY 重建了 plan 列表
- **5 个 VALIDATION.md 停留在 draft**：v1.20 全部 VALIDATION.md 未升级为 complete 状态，形成文档技术债务，VERIFICATION.md 弥补了实际验证缺口

### Patterns Established

- `block_in_place` 包裹 rayon + blocking I/O 是 tokio 异步路径的标准做法
- Code Review REVIEW.md → fix commit 循环已成为每个 phase 的标准收尾步骤
- `multi_row_batch_size` 配置字段范围限制（[1, 64]）防止 SQLITE_LIMIT_VARIABLE_NUMBER 溢出是 SQLite batch 的安全守卫模式

### Key Lessons

- **REQUIREMENTS.md 应在 phase 完成时同步勾选**（同 v1.19 教训重复）：此问题已在三个里程碑中连续出现，建议在 plan 执行阶段将勾选作为验收步骤之一
- **Phase 75 的 include_pm 硬编码是已知技术债务**：sqlite_parallel.rs 的 `include_pm: true` 硬编码与 CSV 路径动态读取不对称，Phase 75 SUMMARY 有记录，下个里程碑如需修复应作为独立 phase
- **VALIDATION.md draft 不阻断交付**：VERIFICATION.md 提供了等效的代码级验证，draft VALIDATION.md 是文档标记问题而非交付缺口

### Cost Observations

- Sessions: ~4 working sessions
- Notable: Phase 73 Code Review 发现最多 bug（4 处），其中 WR-01（record_success() 位置）是逻辑正确性问题，Code Review 流程有效捕获

---

## Milestone: v1.19 — watch完善与文档对齐

**Shipped:** 2026-06-07
**Phases:** 4 (1–3, 71) | **Plans:** 16 | **Duration:** ~2 days | **Coverage:** 92.06%

### What Was Built

- watch CSV 增量追加（WATCH-07）：`force_append_for_watch_trigger` 统一注入，trigger_full 和 trigger_incremental 共享同一注入函数（Phase 1）
- error log 追加写入（WATCH-08）：`write_error_log` OpenOptions 双分支，`append_error_log` config 字段带 `#[serde(skip)]` 防 TOML 污染（Phase 1）
- Ctrl+C 退出码 130（WATCH-09）：`handle_watch` 返回 `Err(Error::Interrupted)`，main.rs signal-aware exit(130)（Phase 1）
- 行覆盖率 92.06%（QUAL-02）：collector.rs 4 组单元测试 + filter_processor.rs 5 项字段过滤测试（D-05 补救）（Phase 2）
- 4 份正式 VALIDATION.md（QUAL-01）：Phase 67/68/69/70 全部以 `status: complete` 落地（Phase 3）
- README 5 命令 + 持续监听功能特性 + 3 段快速入门示例（DOC-04），watch/validate --help 各≥2 示例（DOC-05）（Phase 3）
- Phase 71：10 个 mod.rs 拆分为 >30 个命名子模块（watch/mod.rs 998 行拆为 11 个子文件）

### What Worked

- **Tech debt 闭环**：v1.18 遗留的 3 个 tech debt 项（Ctrl+C 退出码/error log 覆盖写/CSV watch 不支持）在 Phase 1 全部修复，milestone 无欠账
- **force_append_for_watch_trigger 抽取**：统一注入函数消除 trigger_full/trigger_incremental 重复注入逻辑，新增 CSV watch 支持时只改一处
- **D-05 补救策略**：Phase 2 覆盖率未达标时，识别出 filter_processor 为最短路径，追加 5 个字段过滤单元测试即跨越 92% 阈值，不需要大规模补测
- **Phase 71 wave 并行化**：10 个 mod.rs 分 Wave 1/2/3 执行，Wave 1 5 个独立模块并行无冲突，Wave 3 两个大文件（run/watch）依次完成，执行顺序清晰

### What Was Inefficient

- **macOS FSEvents #[ignore] 重新出现**：虽然评估了跨平台条件编译方案，但最终保留 #[ignore]，下个里程碑如需真正覆盖仍需 subprocess 方案
- **Phase 71 review-fix 循环**：71-REVIEW.md 发现 3 项问题（WR-01/02/03 + IN-03），需要额外 4 个 fix commit；mod.rs 拆分后 clippy 新规则暴露了之前隐藏的问题（prescan 内联测试迁移、函数长度）

### Patterns Established

- `#[serde(skip)]` 用于内部 flag 字段：防止 TOML 序列化污染，同时允许 pub 可见性供集成测试使用
- coverage gap 补救：先识别低覆盖率函数（llvm-cov），再针对性补单元测试，而非广撒网
- mod.rs 拆分后公开 API 验证：在每个 plan 结束后运行 `cargo test` 确认无 API 破坏，再进入下一个

### Key Lessons

- **tech debt 项目应在下一 milestone Phase 1 优先关闭**：v1.19 的 WATCH-07/08/09 全部是 v1.18 deferred，Phase 1 一次性清掉，没有拖到 Phase 2/3
- **Phase 71 不应与文档/测试 milestone 混排**：mod.rs 重构是独立的代码质量工作，与 WATCH/QUAL/DOC 需求没有依赖关系，可以单独出一个里程碑
- **REQUIREMENTS.md 应在 phase 完成时同步勾选**：v1.19 关闭时需手动确认所有 checkbox，依赖 SUMMARY 文件做事后验证而非实时跟踪

### Cost Observations

- Sessions: ~2 working sessions
- Notable: Phase 71 的 10 个并行 worktree 执行大幅减少总时间，每个 plan ~5 分钟完成

---

## Milestone: v1.18 — 用户体验全面升级

**Shipped:** 2026-06-06
**Phases:** 4 (67–70) | **Plans:** 12 | **Duration:** ~2 days

### What Was Built

- 进度条升级为 [N/M] 文件计数器 + ETA + records/sec（Phase 67，indicatif 模板扩展）
- ErrorStats by_type HashMap + ParseErrorRecord（10k 上限）+ write_error_log + 摘要 hint（Phase 67）
- `init --interactive` 对话式向导（prompt_line 泛型 IO，str::replace 模板替换，6 个 e2e 测试）（Phase 68）
- `watch` 子命令：notify RecommendedWatcher + 500ms 路径防抖 + HumanDuration 状态行 + Ctrl+C 摘要（Phase 69，4 个 plans）
- watch 增量处理：`_watch_offsets` SQLite 辅助表（独立 Connection），trigger_incremental Seek + NamedTempFile，4 个集成测试（Phase 70）

### What Worked

- **阶段拆分有效**：PROG/DIAG 合并为 Phase 67（同一输出管道层），INIT 独立（stdin 测试策略不同），watch 按框架/增量两阶段拆分——各阶段的测试边界和实现风险均互相独立
- **独立 Connection 设计**：_watch_offsets 辅助表使用独立 rusqlite::Connection，彻底绕开 SqliteExporter EXCLUSIVE 锁冲突，无需修改 Exporter trait
- **save_offset 时序决策**：handle_run 返回后才写 offset（per Pitfall 4），避免在 exporter 持锁期间写 offset 造成死锁
- **4 个集成测试全通过（Phase 70）**：trigger_full_file + trigger_incremental + 重启恢复 + 无新字节跳过，覆盖 WATCH-03/04 完整场景

### What Was Inefficient

- **macOS FSEvents 限制导致 test_watch_triggers_on_new_log_file #[ignore]**：notify crate 在 cargo test piped stdin 环境下触发阻塞，e2e 级别只能用 #[ignore] 规避，需要 subprocess 方案才能真正覆盖
- **REQUIREMENTS.md checkbox 延迟更新**：12 个需求实现后未及时勾选，需在里程碑关闭时统一修复
- **watch Ctrl+C 退出码 0**：handle_watch 内部处理 interrupted=true 返回 Ok(()) 导致退出码 0，与 run 命令的 130 不一致，是设计时忽略的边界情况

### Patterns Established

- **watch 委托模式**：trigger_full_file / trigger_incremental 均委托 handle_run，用 tmp_cfg.sqllog.inputs 覆盖输入路径，无需重写处理逻辑
- **record_offset_after_trigger 防御调用**：绕过 handle_watch 启动路径时，调用 ensure_offset_table 保证辅助表存在，不假设外部已初始化
- **泛型 IO 注入（BufRead + Write）**：init 向导的 prompt_line 函数接受泛型 IO，在测试中注入 &[u8] stdin 和 Vec<u8> stdout，无需 PTY

### Key Lessons

- watch 类功能的集成测试用 trigger_* pub(crate) 函数直接调用，比 spawn_process + 等待文件系统事件更稳定、快速
- 独立数据库连接（per-operation）优于共享连接池，当主流程持有 EXCLUSIVE 锁时不会造成死锁
- macOS 下 notify 测试需要特殊处理（canonicalize 路径 + Modify(Data(Content)) 事件），不能假设 Create 事件一定优先触发

---

## Milestone: v1.15 — 工程质量全面提升

**Shipped:** 2026-06-02
**Phases:** 4 (55–58) | **Plans:** 7 | **Duration:** ~1 day

### What Was Built

- GitHub Actions 4 个 workflow 文件 6 处无效 action 版本修复（@v6/@v7 → @v4）
- release.yaml 重构为 artifact 暂存 + 独立 create-release job，消除并行 job 竞争条件
- Cross.toml 新建，aarch64-linux 跨编译 cross-rs edge 镜像配置
- `pub(crate) mod scanner` 公共模块提取，stats/run 共享文件扫描逻辑（DRY）
- 5 条 e2e CLI 全链路测试（run CSV/SQLite、init 成功/冲突、stats from>to 拒绝）
- handle_run（234 行）拆分为 7 个语义清晰的私有辅助函数，逻辑语句数 ~37
- BENCHMARKS.md CI Artifact 使用指南

### What Worked

- **测试作为重构安全网**：Phase 57 先建立 e2e 测试，Phase 58 才做重构，完全消除回归风险
- **功能拆分原则**：Phase 55 先修 action 版本（小范围），Phase 55-02 再重构 release 架构（大范围），避免混杂
- **静态验证替代运行时**：CICD-01/02 用 grep/awk 静态断言替代实际 GitHub Actions 运行，快速验证代码逻辑正确性
- **公共模块提取时机**：Phase 56 在 Phase 55 稳定 CI 后提取 scanner，依赖关系清晰

### What Was Inefficient

- **Phase 53 SUMMARY 重复**：ROADMAP.md 中 Phase 53 计划条目重复（53-01/02/03 出现两次），之前里程碑留下的文档债务
- **REQUIREMENTS.md checkbox 同步延迟**：CICD-03/04 已实现但 checkbox 未勾选，在审计时才被发现，需要在完成时及时更新

### Patterns Established

- **测试先行的重构顺序**：先写覆盖当前行为的测试，再做结构性重构，pre-commit hook 保证无回归
- **`pub(crate)` 内部共享模块模式**：与 `pub(crate) mod parser` 一致，共享逻辑不暴露 pub API
- **Option<Config> D-04 模式**：预扫描结果的生命周期管理，避免借用/所有权冲突

### Key Lessons

1. **cargo fmt 展开多参数调用会大幅增加物理行数**：`handle_run` 逻辑语句数 ~37，但物理行数约 100（每个函数参数独占一行）。40 行限制应理解为逻辑语句数，而非 cargo fmt 后的物理行数。
2. **REQUIREMENTS.md checkbox 要在计划完成时即时更新**，不要等到里程碑归档时统一修正
3. **CI/CD workflow 测试本质上是外部依赖**：无法在本地验证 GitHub Actions 运行时行为，静态验证是合理的替代方案，但应在 VERIFICATION.md 中明确标注为 "PARTIALLY VERIFIED"

### Cost Observations

- 4 个 phase 全部在 2026-06-02 单天完成，执行效率高
- CI/CD phase（55）主要是文档阅读和配置修改，无复杂代码逻辑
- e2e 测试 phase（57）因 pre-commit hook 要求所有测试通过，迫使将 3 个任务合并为单次提交，节省了调试时间

---

## Cross-Milestone Trends

| 指标 | v1.15 | 说明 |
|------|-------|------|
| Phase 数 | 4 | 纯工程质量，无新功能 |
| 计划数 | 7 | 平均 1.75 计划/Phase |
| 总时长 | ~1 天 | 高效执行 |
| e2e 测试数 | 69 | Phase 57 新增 5 条 |
| 主要工作类型 | CI/CD 修复、重构、测试 | 工程基础设施为主 |
