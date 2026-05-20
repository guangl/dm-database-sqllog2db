# Phase 34: 修复审计缺口 - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

## Phase Boundary

Phase 34 关闭 v1.7-MILESTONE-AUDIT.md 发现的所有遗留问题。不涉及新功能开发，纯粹是代码清理 + 验证补全。

**In scope:** RM-05, RM-08, INT-01, INT-02, INT-03
**Out of scope:** 新功能；Phase 28/29/31/32 的 VALIDATION.md 补签（Nyquist MISSING 项延后）

## Implementation Decisions

### 死代码移除
- **D-01:** 移除 `normalize_template` 函数（normalizer.rs:462）— 被标记 `#[allow(dead_code)]`，Phase 30 移除模板分析后无调用者
- **D-02:** 移除 `normalize_template` 相关的所有单元测试和集成测试引用
- **D-03:** 移除后运行 `cargo test` 和 `cargo clippy` 确保无退化

### 配置验证
- **D-04:** `[template]` 配置段需显式拒绝 — 参照 `[pipeline]` 被拒绝的方式（Config 反序列化时拒绝未知/废弃 section）
- **D-05:** 移除 `[template]` 相关的测试 fixture 和配置引用（[template] 在 defaults.rs/handler 中）
- **D-06:** 拒绝行为：提供明确的错误消息告知用户 `[template]` 已废弃

### 错误清理
- **D-07:** 清理 `FileError::ReadFailed` 变体（error.rs:59）上的 "TODO: Phase 32 统一清理" 注释
- **D-08:** 评估 ReadFailed 是否可被其他变体替代，或确认其需要保留（移除 TODO 并说明原因）

### 验证补全
- **D-09:** 为 Phase 30 创建 VERIFICATION.md，验证模板分析及关联功能已完全移除
- **D-10:** VERIFICATION.md 基于 Phase 30 的 SUMMARY.md 文件验证 must_haves

### 计划组织
- **D-11:** 2 个 plan：
  - **Plan 1 (34-01):** 代码清理 — 移除 normalize_template（D-01/D-02）、拒绝 [template] 配置（D-04/D-05）、清理 FileError TODO（D-07/D-08）
  - **Plan 2 (34-02):** 验证补全 — cargo test/clippy 验证（D-03）、创建 Phase 30 VERIFICATION.md（D-09/D-10）、确认 RM-05/RM-08 满足
- **D-12:** Plan 2 依赖 Plan 1（代码清理完成后才能验证），串行执行

### 构建门禁
- **D-13:** 代码修改后必须通过：`cargo build --release` + `cargo clippy --all-targets -- -D warnings` + `cargo test` + `cargo fmt`

### Claude's Discretion

- `normalize_template` 移除的具体范围（仅函数体 vs. 完整的 import 清理）
- `[template]` 拒绝的精确实现方式（deny_unknown_fields vs. 自定义 deserialize）
- `FileError::ReadFailed` 是否保留（如保留则移除 TODO 并记录原因；如移除则替换调用者）
- Plan 1 内部的任务拆分粒度

## Canonical References

### 审计
- `.planning/v1.7-MILESTONE-AUDIT.md` — Phase 34 的所有缺口定义（RM-05, RM-08, INT-01, INT-02, INT-03）

### 需求
- `.planning/REQUIREMENTS.md` — RM-05（移除模板分析）、RM-08（项目结构清理）

### 相关源码
- `src/pipeline/normalizer.rs:462` — normalize_template 死代码位置
- `src/error.rs:59` — FileError::ReadFailed TODO 位置
- `src/config.rs` — Config 结构体，[template] 拒绝点
- `src/cli/run.rs` — 主编排逻辑

### 参考阶段
- `.planning/phases/30-remove-template-analysis/` — Phase 30 SUMMARY.md（用于创建 VERIFICATION.md）

## Specific Ideas

- INT-01/INT-02/INT-03 都是技术债清理，无歧义 — 审计已经精确描述了需要做什么
- D-04 参照 `[pipeline]` 的拒绝模式：`Config` 的 serde 反序列化使用 `#[serde(deny_unknown_fields)]` 或自定义 `Deserialize` 在发现废弃字段时给出明确错误
- Phase 30 VERIFICATION.md 不需要重新执行 Phase 30，只需基于 SUMMARY.md 和当前代码库状态验证移除完整性

## Deferred Ideas

- Phase 28/29/31/32 的 VALIDATION.md 补签（Nyquist MISSING 项）— 影响范围大，非阻塞，延后至后续版本
- REQUIREMENTS.md 中 RM-01~RM-08 的 checkbox 批量更新 — 由 complete-milestone 归档时处理

---

*Phase: 34-修复审计缺口*
*Context gathered: 2026-05-20*
