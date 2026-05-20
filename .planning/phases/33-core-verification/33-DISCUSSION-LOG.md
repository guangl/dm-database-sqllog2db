# Phase 33: 核心功能验证 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-20
**Phase:** 33-核心功能验证
**Areas discussed:** 验证深度, 验证报告, 计划组织, 性能回归

---

## 验证深度

| Option | Description | Selected |
|--------|-------------|----------|
| 仅自动化检查 | build + test + clippy + fmt 全部通过即视为验证完成 | |
| 自动化 + CLI 冒烟测试 | 额外执行 cargo run 验证 CSV 和 SQLite 导出 | ✓ |
| 自动化 + 全面手动验证 | 针对每个 KEEP 需求分别准备测试场景 | |

**User's choice:** 自动化 + CLI 冒烟测试
**Notes:** 在自动化检查基础上增加 CLI 端到端验证

| Option | Description | Selected |
|--------|-------------|----------|
| CSV + SQLite 导出 | 分别执行 CSV 和 SQLite 导出验证 | |
| CSV + SQLite + 过滤器 | 增加带 include/exclude 过滤器的验证 | |
| 全功能覆盖 | CSV + SQLite + 过滤器 + 参数归一化 + 并行 CSV + 中文配置模板 | ✓ |

**User's choice:** 全功能覆盖

| Option | Description | Selected |
|--------|-------------|----------|
| 生成测试日志 | 使用代码生成测试日志（与集成测试模式一致） | |
| 使用真实日志 | 优先 sqllogs/ 目录下的真实达梦日志 | ✓ |

**User's choice:** 使用真实日志（不存在时回退到生成）

| Option | Description | Selected |
|--------|-------------|----------|
| 检查日志输出 | 通过日志确认并行路径生效 | |
| 检查输出 + 计时 | 确认输出正确性 + 处理时间对比 | ✓ |
| 不单独验证 | 并行 CSV 已有集成测试覆盖 | |

**User's choice:** 检查输出 + 计时

| Option | Description | Selected |
|--------|-------------|----------|
| 检查 CSV 输出 | 只检查 CSV 中参数替换结果 | |
| 检查 CSV + SQLite 双路输出 | 同时检查两种格式输出一致 | ✓ |
| 信任现有测试 | 参数归一化已有充分单元测试 | |

**User's choice:** 检查 CSV + SQLite 双路输出

| Option | Description | Selected |
|--------|-------------|----------|
| 单一组合验证 | 一个配置同时启用四类过滤器 | |
| 分项独立验证 | 每类过滤器单独准备配置和场景 | ✓ |

**User's choice:** 分项独立验证

| Option | Description | Selected |
|--------|-------------|----------|
| debug + release build | cargo check + cargo build --release | ✓ |
| 仅 release build | 仅 cargo build --release | |

**User's choice:** debug check + release build 两者都验证

| Option | Description | Selected |
|--------|-------------|----------|
| 需要验证 | 执行 init 生成配置然后 validate 确认 | ✓ |
| 跳过 | init 已有集成测试覆盖 | |

**User's choice:** 需要验证 init 中文模板

| Option | Description | Selected |
|--------|-------------|----------|
| 修复后重新验证 | 先修复问题然后重新执行完整验证 | ✓ |
| 记录并报告 | 在报告中记录问题作为后续 phase 输入 | |

**User's choice:** 修复后重新验证

| Option | Description | Selected |
|--------|-------------|----------|
| 行数对比 | 对比 CSV 和 SQLite 导出记录数 | |
| 行数 + 字段校验 | 行数对比 + 关键字段抽查 | ✓ |
| 仅文件存在检查 | 只检查 SQLite 文件生成且非空 | |

**User's choice:** 行数 + 字段校验

| Option | Description | Selected |
|--------|-------------|----------|
| 需要验证 | 确认错误日志文件功能正常 | ✓ |
| 跳过 | 错误日志已有单元测试覆盖 | |

**User's choice:** 需要验证错误日志输出

| Option | Description | Selected |
|--------|-------------|----------|
| 显式检查清单 | 每个 KEEP 项对应明确通过条件 | ✓ |
| 整体判断 | build+test+cli+fmt 通过即完成 | |

**User's choice:** 显式检查清单

---

## 验证报告

| Option | Description | Selected |
|--------|-------------|----------|
| 生成检查清单报告 | VERIFICATION-CHECKLIST.md 逐项标记 | ✓ |
| 口头确认即可 | 不生成额外文档 | |
| CI 输出即报告 | 依赖 CI 输出作为证据 | |

**User's choice:** 生成检查清单报告

| Option | Description | Selected |
|--------|-------------|----------|
| KEEP 映射 + 通过/失败 + 证据 | 完整的通过条件、实际结果、证据 | ✓ |
| 简洁通过/失败表 | 仅 KEEP 编号和状态 | |

**User's choice:** KEEP 映射 + 通过/失败 + 证据

| Option | Description | Selected |
|--------|-------------|----------|
| phase 目录 | .planning/phases/33-core-verification/ | ✓ |
| 项目根目录 | 便于发现 | |
| 不需要单独文件 | 作为 CONTEXT.md 或 PLAN.md 的一部分 | |

**User's choice:** phase 目录

| Option | Description | Selected |
|--------|-------------|----------|
| 包含可复现步骤 | 每个 KEEP 项记录具体验证命令 | ✓ |
| 仅记录结果 | 只记录通过/失败和证据摘要 | |

**User's choice:** 包含可复现步骤

---

## 计划组织

| Option | Description | Selected |
|--------|-------------|----------|
| 按验证类型分组 | Plan 1 静态检查 / Plan 2 自动化测试 / Plan 3 手动冒烟 | ✓ |
| 按功能域分组 | Plan 1 导出器 / Plan 2 Pipeline / Plan 3 构建与报告 | |

**User's choice:** 按验证类型分组

| Option | Description | Selected |
|--------|-------------|----------|
| Plans 顺序执行 | Plan 1 → Plan 2 → Plan 3 逐步升级 | |
| Plans 可并行 | 三个 plan 独立执行 | ✓ |

**User's choice:** Plans 可并行

| Option | Description | Selected |
|--------|-------------|----------|
| Shell 脚本 | Bash 脚本执行所有冒烟测试 | |
| Rust 测试代码 | Rust 代码调用 CLI handler | |
| 混合方式 | Shell 编排 cargo run + Rust 做数据校验 | ✓ |

**User's choice:** 混合方式

---

## 性能回归

| Option | Description | Selected |
|--------|-------------|----------|
| 运行全部 benchmark | cargo bench 全部三个 benchmark | ✓ |
| 只运行集成测试吞吐量基准 | test_csv_throughput_baseline 足够 | |
| 跳过 | 精简操作不应影响性能 | |

**User's choice:** 运行全部 benchmark

| Option | Description | Selected |
|--------|-------------|----------|
| 与 baseline 对比 | 退化超过 10% 视为回归 | ✓ |
| 绝对阈值 | 不低于固定吞吐量阈值 | |
| 两者结合 | baseline 对比 + 绝对最小阈值 | |

**User's choice:** 与 baseline 对比，退化 >10% 为回归

| Option | Description | Selected |
|--------|-------------|----------|
| 分析原因并修复 | 在 Phase 33 中分析根因并修复 | ✓ |
| 记录但不阻塞 | 记录退化情况不阻塞完成 | |

**User's choice:** 分析原因并修复

| Option | Description | Selected |
|--------|-------------|----------|
| Plan 1（静态检查） | 构建验证后立即运行 benchmark | |
| Plan 2（自动化测试） | 与 cargo test 一起运行 | ✓ |
| Plan 3（冒烟验证） | 作为手动验证的一部分 | |

**User's choice:** Plan 2（自动化测试）

---

## Claude's Discretion

- VERIFICATION-CHECKLIST.md 的精确结构和字段定义
- 冒烟测试 Shell 脚本和 Rust 验证代码的具体实现
- 各 plan 内部的任务拆分细节
- benchmark baseline 更新策略（当前 baseline 过旧时的处理）

## Deferred Ideas

- "调研 dm-database-parser-sqllog 1.0.0 新特性" — 与本阶段验证范围无关，延后至未来版本
