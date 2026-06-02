# Phase 58: cli/run 函数清理 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 58-cli/run 函数清理
**Mode:** --auto (fully autonomous — no user interaction)
**Areas discussed:** 拆分方案、预扫描所有权、顺序路径行数控制

---

## 拆分方案

| Option | Description | Selected |
|--------|-------------|----------|
| 5 个私有函数按语义边界 | resolve_input_files / merge_trxid_prescan / make_progress_bar / run_sequential / print_run_summary | ✓ |
| 3 个粗粒度函数 | 仅拆出最大块，其余内联 | |
| 就地重构（无独立函数） | 通过 early return 缩短主函数 | |

**[auto] Selected:** 5 个私有函数按语义边界 (recommended default)
**Notes:** 每个函数对应 handle_run 中一个自然语义块，命名反映单一职责

---

## 预扫描所有权

| Option | Description | Selected |
|--------|-------------|----------|
| 返回 `Option<Config>` | None=用原始cfg, Some=合并后cfg；调用方 `.as_ref().unwrap_or(cfg)` | ✓ |
| 返回 `Cow<Config>` | 借用 vs 拥有的语义更明确，但引入 Cow 复杂度 | |
| 不提取，保持内联 | 避免所有权问题，但 handle_run 难以缩短 | |

**[auto] Selected:** 返回 `Option<Config>` (recommended default)
**Notes:** 简单直接，避免引入 Cow 或额外 trait bound

---

## 顺序路径行数控制

| Option | Description | Selected |
|--------|-------------|----------|
| 提取为 run_sequential，内联初始化代码 | 约 45 行，通过简化 for 内错误处理降至 ≤40 | ✓ |
| 用 ProcessingParams struct 缩短调用 | 打包 do_normalize/placeholder_override/field_mask/ordered_indices | |
| 保持内联 + early return | 不提取，用 early return 缩短 handle_run | |

**[auto] Selected:** 提取为 run_sequential，内联初始化代码 (recommended default)
**Notes:** process_log_file 调用有 14 个参数约占 16 行，可通过减少缩进或局部变量简化；不改子模块签名

---

## Claude's Discretion

- run_sequential 具体如何控制在 40 行以内：可根据实际代码情况选择最可读方案（简化 for 体 / 后置 finalize 调用 / 局部变量别名）

## Deferred Ideas

- ProcessingParams struct 封装多参数 → 超出本 phase 范围
- 子模块函数参数重构 → 未来独立 phase
