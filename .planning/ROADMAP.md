# Roadmap: sqllog2db

## Milestones

- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- ✅ **v1.4 代码重构 & 质量深化** — Phases 17–20 (shipped 2026-05-18)
- ✅ **v1.5 文档完善 & 项目展示** — Phases 21–23 (shipped 2026-05-19)
- ✅ **v1.6 文档中文化 & 延后需求补全** — Phases 24–27 (shipped 2026-05-19)
- ✅ **v1.7 项目精简** — Phases 28–34 (shipped 2026-05-20)
- 🚧 **v1.10 质量加固与体验优化** — Phases 35–40 (planning)

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

<details open>
<summary>🚧 v1.10 质量加固与体验优化 (Phases 35–40) — PLANNING</summary>

- [ ] **Phase 35: CLI --help 增强** — zero-risk standalone change for DaMeng usage examples
- [ ] **Phase 36: 错误处理体系重构** — error type hierarchy, non-fatal continuation, context-rich messages
- [ ] **Phase 37: stdin 管道输入与错误实时输出** — /dev/stdin mapping, pre-scan skip, stderr output
- [ ] **Phase 38: 进度显示与统计摘要** — per-1024 progress spinners, completion summary
- [ ] **Phase 39: CSV/管道/参数核心验证** — end-to-end verification of CSV export, pipeline filters, normalization
- [ ] **Phase 40: SQLite/并行/最终质量门禁** — SQLite export, parallel CSV, full build/test/lint gate

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
  2. 并行 CSV（rayon）输出与顺序模式完全一致，多线程下无数据竞争或乱序
  3. `cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全部通过，无任何警告
  4. `cargo bench` 相比 v1.9 基线性能退化 < 5%
**Plans**: TBD

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

**15/15 requirements mapped — coverage: 100%**

## Progress

| Phase | Milestone | Status | Completed |
|-------|-----------|--------|-----------|
| 35. CLI --help 增强 | v1.10 | Not started | — |
| 36. 错误处理体系重构 | v1.10 | Not started | — |
| 37. stdin 管道输入与错误实时输出 | v1.10 | Not started | — |
| 38. 进度显示与统计摘要 | v1.10 | Not started | — |
| 39. CSV/管道/参数核心验证 | v1.10 | Not started | — |
| 40. SQLite/并行/最终质量门禁 | v1.10 | Not started | — |

---
*Created: 2026-05-21 for milestone v1.10*
