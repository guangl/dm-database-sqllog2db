# Roadmap: sqllog2db

## Milestones

- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- ✅ **v1.4 代码重构 & 质量深化** — Phases 17–20 (shipped 2026-05-18)
- ✅ **v1.5 文档完善 & 项目展示** — Phases 21–23 (shipped 2026-05-19)
- ✅ **v1.6 文档中文化 & 延后需求补全** — Phases 24–27 (shipped 2026-05-19)
- 🚧 **v1.7 项目精简** — Phases 28–33 (in progress)

## Phases

<details>
<summary>✅ v1.0 增强 SQL 内容过滤与字段投影 (Phases 1–2) — SHIPPED 2026-04-18</summary>

- [x] Phase 1: 正则字段过滤 (2/2 plans) — completed 2026-04-18
- [x] Phase 2: 输出字段控制 (4/4 plans) — completed 2026-04-18

Full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 性能优化 (Phases 3–6) — SHIPPED 2026-05-10</summary>

- [x] Phase 3: Profiling & Benchmarking (3/3 plans) — completed 2026-04-27
- [x] Phase 4: CSV 性能优化 (4/4 plans) — completed 2026-05-09
- [x] Phase 5: SQLite 性能优化 (3/3 plans) — completed 2026-05-10
- [x] Phase 6: 解析库集成 + 验收 (2/2 plans) — completed 2026-05-10

Full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2 质量强化 & 性能深化 (Phases 7–11) — SHIPPED 2026-05-15</summary>

- [x] Phase 7: 技术债修复 (1/1 plans) — completed 2026-05-10
- [x] Phase 8: 排除过滤器 (2/2 plans) — completed 2026-05-10
- [x] Phase 9: CLI 启动提速 (5/5 plans) — completed 2026-05-14
- [x] Phase 10: 热路径优化 (3/3 plans) — completed 2026-05-15
- [x] Phase 11: Nyquist 补签 (2/2 plans) — completed 2026-05-15

Full details: `.planning/milestones/v1.2-ROADMAP.md`

</details>

<details>
<summary>✅ v1.3 SQL 模板分析 & 可视化 (Phases 12–16) — SHIPPED 2026-05-17</summary>

- [x] Phase 12: SQL 模板归一化引擎 (3/3 plans) — completed 2026-05-15
- [x] Phase 13: TemplateAggregator 流式统计累积器 (2/2 plans) — completed 2026-05-15
- [x] Phase 14: Exporter 集成输出 (4/4 plans) — completed 2026-05-16
- [x] Phase 15: SVG 图表基础设施 + 前两类图表 (5/5 plans) — completed 2026-05-17
- [x] Phase 16: 剩余图表 (5/5 plans) — completed 2026-05-17

Full details: `.planning/milestones/v1.3-ROADMAP.md`

</details>

<details>
<summary>✅ v1.4 代码重构 & 质量深化 (Phases 17–20) — SHIPPED 2026-05-18</summary>

- [x] Phase 17: 过滤器配置嵌套化 (2/2 plans) — completed 2026-05-18
- [x] Phase 18: 模板 & 图表配置嵌套化 (3/3 plans) — completed 2026-05-18
- [x] Phase 19: 代码结构重构 (4/4 plans) — completed 2026-05-18
- [x] Phase 20: 测试覆盖深化 (3/3 plans) — completed 2026-05-18

Full details: `.planning/milestones/v1.4-ROADMAP.md`

</details>

<details>
<summary>✅ v1.5 文档完善 & 项目展示 (Phases 21–23) — SHIPPED 2026-05-19</summary>

- [x] Phase 21: README 全面更新 + 根文档补全 (2/2 plans) — completed 2026-05-19
- [x] Phase 22: GitHub Pages 落地页 + 部署流水线 (2/2 plans) — completed 2026-05-19
- [x] Phase 23: 补充文档 + CI 质量门禁 (4/4 plans) — completed 2026-05-19

Full details: `.planning/milestones/v1.5-ROADMAP.md`

</details>

<details>
<summary>✅ v1.6 文档中文化 & 延后需求补全 (Phases 24–27) — SHIPPED 2026-05-19</summary>

- [x] Phase 24: 文档中文化 & 去 SVG 化 (3/3 plans) — completed 2026-05-19
- [x] Phase 25: 延后文档补全 (1/1 plan) — completed 2026-05-19
- [x] Phase 26: GitHub Pages 多页文档站 (1/1 plan) — completed 2026-05-19
- [x] Phase 27: 模板报告独立输出 (2/2 plans) — completed 2026-05-19

Full details: `.planning/milestones/v1.6-ROADMAP.md`

</details>

<details open>
<summary>🚧 v1.7 项目精简 (Phases 28–34) — IN PROGRESS</summary>

- [x] **Phase 28: 移除图表、自更新、补全** — 移除三个独立无依赖的外部功能模块
- [x] **Phase 29: 移除统计与摘要** — 移除 stats 和 digest 子命令及其依赖
- [x] **Phase 30: 移除模板分析** — 移除模板聚合、报告及相关配置
- [x] **Phase 31: 移除断点续传** — 移除 resume 模块、配置和 --resume 选项
- [x] **Phase 32: 项目结构清理** — 清理空目录、mod 声明、未使用代码
- [x] **Phase 33: 核心功能验证** — 验证精简后核心功能完整可用
- [ ] **Phase 34: 修复审计缺口** — 关闭 v1.7 审计发现的遗留问题

</details>

## Phase Details

### Phase 28: 移除图表、自更新、补全
**Goal**: 移除 SVG 图表、self-update 和 Shell 补全三个独立无依赖的外部功能模块
**Depends on**: Nothing (first removal phase)
**Requirements**: RM-01, RM-02, RM-07
**Success Criteria** (what must be TRUE):
  1. `src/charts/` 目录已移除，`plotters` 依赖从 Cargo.toml 中删除
  2. `src/cli/update.rs` 已移除，`self_update`/`reqwest`/`rustls` 依赖从 Cargo.toml 中删除
  3. `sqllog2db --help` 不再显示 `self-update`、`completions`、`man` 子命令
  4. `clap_complete`/`clap_mangen` 依赖从 Cargo.toml 中删除
  5. `[charts]` 配置段被移除，包含该配置段的旧文件在 `validate` 时不再被接受（或被忽略）
**Plans**: 3 plans
```
Plans:
- [x] 28-01-PLAN.md — 移除 SVG 图表模块（RM-01）
- [x] 28-02-PLAN.md — 移除 self-update 自更新（RM-02）
- [x] 28-03-PLAN.md — 移除 Shell 补全和 Man page（RM-07）
```

### Phase 29: 移除统计与摘要
**Goal**: 移除 stats 和 digest 两个子命令及其相关依赖和文件
**Depends on**: Phase 28
**Requirements**: RM-03, RM-04
**Success Criteria** (what must be TRUE):
  1. `sqllog2db --help` 不再显示 `stats` 子命令
  2. `sqllog2db --help` 不再显示 `digest` 子命令
  3. `src/cli/stats.rs` 和 `src/cli/digest.rs` 文件已移除
  4. `src/pipeline/fingerprint.rs` 已移除
  5. `serde_json` 依赖从 Cargo.toml 中删除
**Plans**: 2 plans
```
Plans:
- [x] 29-01-PLAN.md — 移除 stats 命令（RM-03）+ serde_json
- [x] 29-02-PLAN.md — 迁移 normalize_template + 移除 digest 命令 + 删除 fingerprint.rs（RM-04）
```

### Phase 30: 移除模板分析
**Goal**: 移除模板聚合器、模板报告器和相关配置段
**Depends on**: Phase 29
**Requirements**: RM-05
**Success Criteria** (what must be TRUE):
  1. `src/pipeline/aggregator.rs` 和 `src/pipeline/template_reporter.rs` 文件已移除
  2. `hdrhistogram` 依赖从 Cargo.toml 中删除
  3. `[template]` 和 `[template.report]` 配置段从 Config 结构体中移除
  4. 运行 `sqllog2db run` 时不再生成 `*_templates.csv` 或 SQLite 模板报告文件
  5. 核心 CSV/SQLite 导出在热循环中不受影响，`pipeline.is_empty()` 快路径保持零开销
**Plans**: 3 plans
```
Plans:
- [x] 30-01-PLAN.md — 配置层清理：删除 pipeline/mod.rs 中模块声明/TemplateConfig/辅助函数、config 层 template 引用、init/show_config template 段、Cargo.toml hdrhistogram
- [x] 30-02-PLAN.md — 运行时代码清理：删除 aggregator.rs/template_reporter.rs/companion.rs，清理 cli/run 热循环+导出器层的 write_template_stats
- [x] 30-03-PLAN.md — 测试清理+编译验证：删除 5 个测试文件中所有模板相关测试，全链路 cargo build+test+clippy+fmt 通过
```

### Phase 31: 移除断点续传
**Goal**: 移除 resume/checkpoint 模块及相关配置和 CLI 选项
**Depends on**: Phase 30
**Requirements**: RM-06
**Success Criteria** (what must be TRUE):
  1. `src/resume.rs` 文件已移除
  2. `[resume]` 配置段从 Config 结构体中移除
  3. `sqllog2db --help` 不再显示 `--resume` CLI 选项
  4. 运行 `sqllog2db run` 时不再读取或写入 checkpoint 状态文件
  5. `cargo build --release` 编译成功
**Plans**: 2 plans
```
Plans:
- [x] 31-01-PLAN.md — 删除 resume 源文件、清理 config/CLI 层、移除 run 循环中的 resume_state 逻辑
- [x] 31-02-PLAN.md — 更新测试文件、清理 init 模板和文档、全链路验证
```

### Phase 32: 项目结构清理
**Goal**: 清理之前移除操作遗留的空目录和未使用代码，简化项目结构
**Depends on**: Phase 31
**Requirements**: RM-08
**Success Criteria** (what must be TRUE):
  1. 不存在空目录（之前的移除操作后无残留空文件夹）
  2. 所有 `mod.rs` 和 `lib.rs`/`main.rs` 中不再包含已被移除模块的声明
  3. `Config` 结构体中不再包含 `[charts]`、`[template]`、`[resume]` 等已被移除的配置字段
  4. `cargo build` 编译成功且 `cargo clippy --all-targets -- -D warnings` 无警告
  5. Cargo.toml 中已清理所有未被使用的依赖
**Plans**: 3 plans
```
Plans:
- [x] 32-01-PLAN.md — 清理 stale mod 声明 + Config 结构体字段 + Cargo.toml 依赖
- [x] 32-02-PLAN.md — 清理 Exporter trait 死代码 + CLI opts 变体 + run 模块残留
- [x] 32-03-PLAN.md — 清理测试死代码 + init/show_config 模板 + 全链路验证
```

### Phase 33: 核心功能验证
**Goal**: 验证精简后所有核心功能完整可用，构建、测试、lint 全部通过
**Depends on**: Phase 32
**Requirements**: KEEP-01, KEEP-02, KEEP-03, KEEP-04, KEEP-05, KEEP-06
**Success Criteria** (what must be TRUE):
  1. `cargo build --release` 成功编译，无错误
  2. `cargo test` 全部测试通过（包括 CSV 和 SQLite 导出测试）
  3. `cargo clippy --all-targets -- -D warnings` 无警告
  4. CSV 导出功能正常工作，用户可以通过配置将日志导出为 CSV 文件
  5. SQLite 导出功能正常工作，用户可以通过配置将日志导出为 SQLite 数据库
  6. Pipeline 过滤器（include/exclude/indicators/sql）正常工作，过滤结果符合预期
  7. `cargo fmt` 格式检查通过
**Plans**: 3 plans (all Wave 1, parallel)
```
Plans:
- [x] 33-01-PLAN.md — 静态检查（cargo check + build --release + clippy + fmt）
- [x] 33-02-PLAN.md — 自动化测试（cargo test + cargo bench + baseline 对比）
- [x] 33-03-PLAN.md — CLI 冒烟验证（Shell 编排 + 数据校验 + VERIFICATION-CHECKLIST.md）
```

### Phase 34: 修复审计缺口
**Goal**: 关闭 v1.7-MILESTONE-AUDIT.md 发现的所有遗留问题：死代码移除、配置验证、缺失的 VERIFICATION.md
**Depends on**: Phase 33
**Requirements**: RM-05, RM-08
**Success Criteria** (what must be TRUE):
  1. `[template]` 配置段被显式拒绝（与 `[pipeline]` 行为一致）
  2. Phase 30 创建 VERIFICATION.md 验证移除结果
  3. RM-05 和 RM-08 要求全部满足
  4. `cargo build --release` 通过
  5. `cargo clippy --all-targets -- -D warnings` 零警告
  6. `cargo test` 全部通过
**Plans**: 2 plans
```
Plans:
- [x] 34-01-PLAN.md -- 添加 [template] 配置段显式拒绝逻辑
- [x] 34-02-PLAN.md -- 创建 Phase 30 VERIFICATION.md，确认所有审计缺口关闭
```

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. 正则字段过滤 | v1.0 | 2/2 | Complete | 2026-04-18 |
| 2. 输出字段控制 | v1.0 | 4/4 | Complete | 2026-04-18 |
| 3. Profiling & Benchmarking | v1.1 | 3/3 | Complete | 2026-04-27 |
| 4. CSV 性能优化 | v1.1 | 4/4 | Complete | 2026-05-09 |
| 5. SQLite 性能优化 | v1.1 | 3/3 | Complete | 2026-05-10 |
| 6. 解析库集成 + 验收 | v1.1 | 2/2 | Complete | 2026-05-10 |
| 7. 技术债修复 | v1.2 | 1/1 | Complete | 2026-05-10 |
| 8. 排除过滤器 | v1.2 | 2/2 | Complete | 2026-05-10 |
| 9. CLI 启动提速 | v1.2 | 5/5 | Complete | 2026-05-14 |
| 10. 热路径优化 | v1.2 | 3/3 | Complete | 2026-05-15 |
| 11. Nyquist 补签 | v1.2 | 2/2 | Complete | 2026-05-15 |
| 12. SQL 模板归一化引擎 | v1.3 | 3/3 | Complete | 2026-05-15 |
| 13. TemplateAggregator 流式统计累积器 | v1.3 | 2/2 | Complete | 2026-05-15 |
| 14. Exporter 集成输出 | v1.3 | 4/4 | Complete | 2026-05-16 |
| 15. SVG 图表基础设施 + 前两类图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 16. 剩余图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 17. 过滤器配置嵌套化 | v1.4 | 3/3 | Complete | 2026-05-18 |
| 18. 模板 & 图表配置嵌套化 | v1.4 | 3/3 | Complete | 2026-05-18 |
| 19. 代码结构重构 | v1.4 | 4/4 | Complete | 2026-05-18 |
| 20. 测试覆盖深化 | v1.4 | 3/3 | Complete | 2026-05-18 |
| 21. README 全面更新 + 根文档补全 | v1.5 | 2/2 | Complete | 2026-05-19 |
| 22. GitHub Pages 落地页 + 部署流水线 | v1.5 | 2/2 | Complete | 2026-05-19 |
| 23. 补充文档 + CI 质量门禁 | v1.5 | 4/4 | Complete | 2026-05-19 |
| 24. 文档中文化 & 去 SVG 化 | v1.6 | 3/3 | Complete   | 2026-05-19 |
| 25. 延后文档补全 | v1.6 | 1/1 | Complete   | 2026-05-19 |
| 26. GitHub Pages 多页文档站 | v1.6 | 1/1 | Complete   | 2026-05-19 |
| 27. 模板报告独立输出 | v1.6 | 2/2 | Complete   | 2026-05-19 |
| 28. 移除图表、自更新、补全 | v1.7 | 3/3 | Complete    | 2026-05-19 |
| 29. 移除统计与摘要 | v1.7 | 2/2 | Complete    | 2026-05-19 |
| 30. 移除模板分析 | v1.7 | 3/3 | Complete    | 2026-05-20 |
| 31. 移除断点续传 | v1.7 | 2/2 | Complete    | 2026-05-20 |
| 32. 项目结构清理 | v1.7 | 3/3 | Complete    | 2026-05-20 |
| 33. 核心功能验证 | v1.7 | 3/3 | Complete    | 2026-05-20 |
| 34. 修复审计缺口 | v1.7 | 2/2 | Complete    | 2026-05-20 |
