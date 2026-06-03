# Milestones: sqllog2db

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
