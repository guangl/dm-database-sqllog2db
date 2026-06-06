# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

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
