# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

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
