# Phase 63: 测试覆盖提升 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 63-test-coverage
**Mode:** --auto (all choices auto-selected)
**Areas discussed:** 覆盖率工具选择, 覆盖率优先区域, 测试策略

---

## 覆盖率工具选择

| Option | Description | Selected |
|--------|-------------|----------|
| cargo-llvm-cov | LLVM instrumentation，已安装 0.8.5 | ✓ |
| cargo-tarpaulin | 基于 ptrace，已安装 0.35.2 | |

**Auto-selected:** cargo-llvm-cov (recommended default)
**Notes:** ROADMAP.md 明确提及 llvm-cov；两者都已安装，llvm-cov 精度更高

---

## 覆盖率优先区域

| Option | Description | Selected |
|--------|-------------|----------|
| 全模块均等提升 | 按总覆盖率提升 | |
| 关键路径优先 | 过滤器 + exporter + 错误路径 | ✓ |

**Auto-selected:** 关键路径优先 (recommended default)
**Notes:** 这些区域对功能正确性影响最大，与成功标准对齐

---

## 测试策略

| Option | Description | Selected |
|--------|-------------|----------|
| 集成测试优先 | 端到端 e2e 测试 | |
| 单元测试优先 | 模块内 mod tests | ✓ |
| 混合 | 两者均等 | |

**Auto-selected:** 单元测试优先 (recommended default)
**Notes:** 单元测试运行快、定位精准；集成测试作为端到端补充

---

## Claude's Discretion

- 具体补充哪 3+ 个区域由实际报告决定
- "难以测试"路径的文档化标准（OS 依赖、网络依赖等）

## Deferred Ideas

- CI 覆盖率自动发布（Codecov）— 后续里程碑
- 覆盖率门槛强制（--fail-under-lines）— 后续工程化
- Property-based testing — 超出范围
