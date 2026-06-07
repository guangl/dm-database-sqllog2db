# Milestones: sqllog2db

## v1.19 — watch完善与文档对齐

**Shipped:** 2026-06-07  
**Phases:** 1–3, 71 | **Plans:** 16 | **Commits:** 96  
**Duration:** 2026-06-06 → 2026-06-07 (~2 days)  
**Code changes:** 125 files, +12,685 / -3,149 lines  
**Tests:** ~909 total (all passing, 2 ignored)  
**Coverage:** 92.06% line coverage

### Delivered

补完 watch 功能短板并完成大规模模块化重构：watch CSV 增量追加（WATCH-07）、error log 历史保留（WATCH-08）、Ctrl+C 退出码 130（WATCH-09）；测试覆盖率提升至 92.06%；macOS FSEvents 限制落地文档化方案；README 补充 watch/init --interactive/quiet+verbose 完整说明；10 个 mod.rs 文件全部拆分为命名子模块，mod.rs 仅保留 pub use 导入（Phase 71）。

### Key Accomplishments

1. watch 子命令支持 CSV 导出增量追加（WATCH-07）：`force_append_for_watch_trigger` 统一注入 `append=true`，多次触发行数正确累计，header 仅一次
2. error log 追加写入 + run 路径防回归（WATCH-08）：`write_error_log` OpenOptions 双分支；`test_write_error_log_run_still_truncates` 防回归测试
3. Ctrl+C 退出码修正为 130（WATCH-09）：`handle_watch` 返回 `Err(Error::Interrupted)`，main.rs signal-aware exit(130)
4. 行覆盖率 92.06%（QUAL-02）：collector.rs Group 1–4 + filter_processor 5 项字段过滤测试，超越 92% 目标
5. 4 份正式 VALIDATION.md（QUAL-01）：Phase 67/68/69/70 全部以 `status: complete` 落地，Per-Task Verification Map 转录完整
6. mod.rs 重构 Phase 71：10 个 mod.rs 共 ~2,600 行拆分为 >30 个命名子模块（watch/mod.rs 998 行拆为 11 个子文件），mod.rs 均精简为声明骨架

### Archives

- `.planning/milestones/v1.19-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.19-REQUIREMENTS.md` — 需求归档（8/8 complete）

---

## v1.18 — 用户体验全面升级

**Shipped:** 2026-06-06  
**Phases:** 67–70 | **Plans:** 12 | **Commits:** 130  
**Duration:** 2026-06-04 → 2026-06-06 (~2 days)  
**Code:** ~13,819 lines Rust  
**Tests:** ~880 total (376 lib + ~500 integration, 0 failed, 2 ignored)

### Delivered

全面改善用户交互体验：watch 实时监控目录（增量 SQLite 插入，字节偏移持久化跨重启恢复）、交互式配置向导（`init --interactive` 对话式引导，支持 CSV/SQLite 导出格式选择）、运行时错误诊断（parse error 按 ErrorKind 分类，error log 含行号与前 120 字符原文，摘要触发 encoding/field hint）、进度显示升级（[N/M] 文件计数器 + ETA + records/sec + 过滤率统计）。

### Key Accomplishments

1. 进度条升级为 `[N/M]` 文件计数器 + records/sec 吞吐显示 + indicatif 自动渲染 ETA（PROG-01/02）
2. ErrorStats 扩展：by_type HashMap + ParseErrorRecord 结构体 + 10k 上限收集；摘要按类型分组输出 + encoding/field hint（PROG-03/DIAG-01/02/03）
3. `init --interactive` 对话式配置向导：prompt_line 泛型 IO、str::replace 模板替换、6 个 e2e CLI 测试覆盖 INIT-01/02/03（含 SC4 validate 通过）
4. `watch` 子命令：notify RecommendedWatcher + mpsc channel + 500ms 路径防抖 + HumanDuration 动态状态行 + Ctrl+C 最终摘要（WATCH-01/02/05/06）
5. watch 增量处理：`_watch_offsets` SQLite 辅助表（独立 Connection），trigger_incremental Seek + NamedTempFile，跨重启 load_offsets 恢复，4 个集成测试全通过（WATCH-03/04）

### Known Deferred Items

- watch Ctrl+C 退出码 0 vs run 命令 130（非阻塞行为不一致）
- write_error_log 覆盖写（watch 长时间运行只保留最近一次触发的错误）
- VALIDATION.md 文件为草稿状态（Phases 67/68/69）；70-VALIDATION.md 缺失

### Archives

- `.planning/milestones/v1.18-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.18-REQUIREMENTS.md` — 需求归档（15/15 complete）
- `.planning/v1.18-MILESTONE-AUDIT.md` — 审计报告（tech_debt，无阻塞缺口）

---

## v1.17 — 多文件并行提速

**Shipped:** 2026-06-04  
**Phases:** 64–66.1 | **Plans:** 4 | **Commits:** 40  
**Duration:** 2026-06-04 → 2026-06-04 (~1 day)  
**Tests:** 780 total (all passing, 0 failed)

### Delivered

为 CSV 导出路径新增多文件 rayon 并行处理（temp-file 方案），对齐已有的 SQLite 并行路径；verbose 透传链保证并行路径与顺序路径输出行为完全一致；追加 Phase 66.1 修复单核 CI 上并行路径无法激活的测试盲点，引入 write_heterogeneous_log 异构测试数据 helper。

### Key Accomplishments

1. CSV 多文件并行处理（process_csv_parallel，rayon work-stealing + temp-file 拼接），2 个以上文件自动激活，1 个文件回退顺序路径（PARALLEL-01/02）
2. verbose 透传链：run_parallel_tasks → process_csv_parallel → run_csv_parallel → handle_run，逐文件 "Processing: {path}" 输出与顺序路径格式一致（PARALLEL-05）
3. 3 条兼容性集成测试（COMPAT-01/02/03）：并行与顺序路径 CSV 行集合相等（排序对比），过滤器等价性，init 模板格式无变化
4. jobs_override: Option<usize> 扩展 handle_run（36 处调用点同步），强制单核 CI 进入并行路径（PARALLEL-06）
5. write_heterogeneous_log helper（trxid_offset + username 两维度差异化），验证跨文件聚合正确性（PARALLEL-07）

### Archives

- `.planning/milestones/v1.17-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.17-REQUIREMENTS.md` — 需求归档（11/11 满足，含 PARALLEL-06/07）
- `.planning/v1.17-MILESTONE-AUDIT.md` — 审计报告（tech_debt，3 项轻微，Phase 66.1 关闭 WARNING-01/02）

---

## v1.16.0 — 工程质量深化

**Shipped:** 2026-06-03  
**Phases:** 59–63 | **Plans:** 15 | **Commits:** 109  
**Duration:** 2026-06-02 → 2026-06-03 (~2 days)  
**Code changes:** 72 files, +10,110 / -533 lines  
**Tests:** 740 total (320 lib + 351 bench + 68 integration + 1 jemalloc), all passing

### Delivered

全面提升工程基础：代码结构整理（cli/run + exporter/pipeline 函数拆分与重复消除）、全代码库 unwrap/expect 统一注释、Cross.toml SHA256 固定（消除 v1.15 遗留的构建不可复现问题）、README + CHANGELOG（v1.0–v1.15 历史）+ config 模板文档全面补全、行覆盖率从 90.68% 提升至 91.86%（+1.18 pp）。

### Key Accomplishments

1. process_log_file（152行）拆分为 45 行主体 + ExportAction 枚举 + 4 个辅助函数；全部超限函数完成语义拆分（STRUCT-01）
2. collector.rs 公共模块提取，sqlite_parallel.rs 行数从 225 降至 130，消除重复 collect_log_file/process_record 逻辑（STRUCT-02）
3. 全代码库 unwrap/expect 审计：生产代码全部标注 `// infallible` 或改为 `?` 传播，production_uncommented = 0（STRUCT-03）
4. Cross.toml SHA256 digest 固定（`de04c9cd...`），`:edge` 浮动标签移除，3 项自动化断言覆盖（CROSS-01）
5. README stats 示例 + CHANGELOG v1.0.0–v1.15.0 Keep a Changelog 格式 + config 模板 22 字段注释补全（DOC-01/02/03）
6. 51 项新测试（filters/csv/sqlite/error/prescan），行覆盖率 91.86% / 函数覆盖率 89.54%（TEST-01/02）

### Archives

- `.planning/milestones/v1.16.0-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.16.0-REQUIREMENTS.md` — 需求归档（9/9 满足）
- `.planning/v1.16-MILESTONE-AUDIT.md` — 审计报告（gaps_found → 关闭前修复）

---

## v1.15 — 工程质量全面提升

**Shipped:** 2026-06-02  
**Phases:** 55–58 | **Plans:** 7 | **Commits:** 32  
**Duration:** 2026-06-02 → 2026-06-02 (~1 day)  
**Code changes:** 47 files, +6,406 / -257 lines

### Delivered

补全测试覆盖、清理技术债务、建立 CI/CD 基础设施：GitHub Actions workflow action 版本修复 + release 竞争条件消除、Cross.toml aarch64-linux 跨编译配置、`scanner` 公共模块提取统一扫描逻辑、5 条 e2e CLI 全链路测试（run/init/stats）、cli/run handle_run 拆分为 7 个私有辅助函数。

### Key Accomplishments

1. GitHub Actions 4 个 workflow 文件中 6 处无效 @v6/@v7 action 版本修复为 @v4（CICD-01）
2. release.yaml 重构为 artifact 暂存 + 独立 create-release job，消除 4 并行 job 竞争条件（CICD-02/03）
3. Cross.toml 新建，aarch64-linux 跨编译 cross-rs edge 镜像配置（CICD-04）
4. `pub(crate) mod scanner` 新建，stats/run 共享同一文件扫描实现，CLEAN-01 静态断言通过（CLEAN-01）
5. 5 条 e2e CLI 测试新增（run CSV/SQLite、init 成功/冲突、stats from>to 拒绝），集成测试总数 69 条（TEST-01/02/03）
6. handle_run（234 行）拆分为 7 个语义清晰的私有辅助函数，逻辑语句数 ~37（CLEAN-02）

### Archives

- `.planning/milestones/v1.15-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.15-REQUIREMENTS.md` — 需求归档（10/10 满足，含 2 项 override）
- `.planning/milestones/v1.15-MILESTONE-AUDIT.md` — 审计报告（tech_debt，无阻塞性 gap）

### Known Gaps

- CICD-01/02：代码静态验证通过，GitHub Actions 运行时行为（三平台 CI + 四平台 CD）需人工推送 PR/tag 后确认
- Cross.toml 使用浮动 edge tag，构建不完全可复现（后续里程碑可固定 SHA digest）

---

## v1.12 — CLI 体验全面提升

**Shipped:** 2026-06-01  
**Phases:** 46–49 | **Plans:** 8 | **Commits:** 57  
**Duration:** 2026-05-31 → 2026-06-01 (~1 day)  
**Code changes:** 38 files, +3,307 / -432 lines  
**Tests:** 529 total (226 lib + 48+ integration + 1 jemalloc), all passing

### Delivered

全面改善用户与 sqllog2db 的交互体验：错误信息结构化并附带 `hint:` 修复建议、配置模板带注释、validate 命令输出清晰、`--verbose`/`--quiet` 运行控制、`inputs: Vec<String>` glob 展开支持。

### Key Accomplishments

1. 致命错误统一 `hint:` 前缀格式，`format_error_output` 纯函数可单元测试（ERROR-01/02）
2. `validate` 静默通过 `Configuration valid.` / 失败输出 `[FAIL] reason\n  hint: ...`（CONFIG-02）
3. `init` 模板所有 exporter 字段补全行内注释（CONFIG-01）
4. `--verbose` 逐文件 `Processing:` 输出，`--quiet` 完全抑制进度与摘要（LOG-01/02/03）
5. `inputs: Vec<String>` 替代 `path: String`，config 和 `--input` CLI flag 均支持 glob 展开（INPUT-01/02）
6. `assert_cmd`/`predicates` 加入 dev-dependencies，e2e CLI 测试覆盖率大幅提升

### Archives

- `.planning/milestones/v1.12-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.12-REQUIREMENTS.md` — 需求归档（9/9 满足）
- `.planning/milestones/v1.12-MILESTONE-AUDIT.md` — 审计报告

---

*Previous milestones tracked in git history. See `.planning/milestones/` for v1.7, v1.10, v1.11 archives.*
