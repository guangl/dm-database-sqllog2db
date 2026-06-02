# Phase 56: stats 模块清理与 benchmark 稳定化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 56-stats 模块清理与 benchmark 稳定化
**Areas discussed:** parse error 处理一致性, Phase 工作性质确认, Benchmark 文档补充

---

## parse error 处理一致性

| Option | Description | Selected |
|--------|-------------|----------|
| warn! 就够，保持现状 | stats 是分析命令，少量解析失败不影响统计结果 | |
| 对齐 run，写入 error log | 引入 ErrorStats 计数 + 写入 error log 文件 + 退出码 1 | 部分 ✓ |
| You decide | 让 planner 根据一致性原则确定 | |

**User's choice:** 对齐 run，写入 error log — 但通过**抽取公共文件扫描模块**实现，而不是直接复制 run 的代码

**Notes:** 用户明确提出"应该重构 stats，完全可以调用 run"的思路，最终决策是新建独立扫描模块（`src/scanner.rs` 或类似），run 和 stats 共用，Phase 56 范围相应扩大。退出码对齐未明确要求，暂时不纳入（parse error 写入 error log 但不改变退出码）。

---

## Phase 工作性质确认

**结论：** 有实际代码改动。公共文件扫描模块抽取是新增工作，不是纯验证 phase。原成功标准（warn! 清除、函数长度、benchmark 脚本）经代码审查已确认满足，planner 只需验证不需改动。

---

## Benchmark 文档补充

| Option | Description | Selected |
|--------|-------------|----------|
| 加一个短节说明如何下载/使用 JSON artifact | 说明 bench-results-*.json 的用途、下载、手动对比方法 | ✓ |
| 不需要，现有文档已足够 | bench.yml 注释已说明用途 | |

**User's choice:** 加一节 CI artifact 使用说明

**Notes:** 现有 BENCHMARKS.md 已有本地运行和 criterion baseline 对比说明，但没有 CI 收集的 `bench-results-*.json` artifact 的用途文档。

---

## Claude's Discretion

- 公共扫描模块的具体模块名（`src/scanner.rs` vs 其他）由 planner 根据项目结构确定
- parse error 在新模块中的具体行为（warn! 保留 vs info! vs 完全移到 error log）由 planner 根据 run 命令的现有模式确定

## Deferred Ideas

- benchmark CI 门控（自动回归检测）→ 未来 milestone，Phase 56 保持信息性收集
- parse error 影响退出码（退出码 1）→ 未明确要求，推迟到有需求时再加
