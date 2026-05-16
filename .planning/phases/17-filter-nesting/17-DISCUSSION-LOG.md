# Phase 17: 过滤器配置嵌套化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 17-过滤器配置嵌套化
**Areas discussed:** 字段重命名, Phase 17 范围, init 命令格式

---

## 字段重命名

### [features.filter.include] 字段名风格

| Option | Description | Selected |
|--------|-------------|----------|
| 语义化短名 | users / ips / sessions / threads / statements / apps / tags。与 REQUIREMENTS 描述一致，嵌套上下文中不需要 exclude_ 前缀，语义更清晰 | ✓ |
| 保留旧名 | usernames / client_ips / sess_ids / thrd_ids。迁移成本低，exclude 子表字段名与 include 子表相同，只是位置不同 | |

**User's choice:** 语义化短名

---

### start_ts / end_ts / trxids 放置位置

| Option | Description | Selected |
|--------|-------------|----------|
| 放入 [filter.include] | 全部归入 include 子表；时间范围是"在内才保留"语义，算 include；少一层嵌套 | ✓ |
| 保留在 [features.filter] 层 | 这两个字段语义比较特殊，不属于 include/exclude 类别，直接居于 filter 层更直观 | |
| 单独 [filter.time] 子表 | 时间相关字段单独分组；但超出 Phase 17 范围 | |

**User's choice:** 放入 [filter.include]

---

## Phase 17 范围

### indicators / sql / record_sql 是否也进行位置调整

| Option | Description | Selected |
|--------|-------------|----------|
| 只动 include/exclude meta 字段 | indicators / sql / record_sql 保持在 [features.filter] 层不动。Phase 17 范围最小，风险最低 | |
| indicators 也嵌套化 | indicators 移入 [features.filter.indicators] 子表，sql / record_sql 不动 | |
| 全部嵌套 | include / exclude / indicators / sql / record_sql 全部成为子表，[features.filter] 层只保留 enable。最彻底，最清晰 | ✓ |

**User's choice:** 全部嵌套

---

### sql / record_sql 内字段名

| Option | Description | Selected |
|--------|-------------|----------|
| 保留现名 | include_patterns / exclude_patterns 不变，只重组 meta 字段 | |
| 语义化 | 改为 includes / excludes | ✓ |

**User's choice:** includes / excludes

---

## init 命令格式

| Option | Description | Selected |
|--------|-------------|----------|
| 新嵌套格式 | 生成 [features.filter.include] / [features.filter.exclude] 新格式，新用户直接得到最新用法，不带 include_ / exclude_ 混局 | ✓ |
| 保留旧格式 | 生成旧平铺格式，避免与现有 config.toml 不一致；不建议，会导致 init 生成的模板和新设计相违 | |

**User's choice:** 新嵌套格式

---

## Claude's Discretion

- **向后兼容实现方式**：serde alias 方案 vs 手写 Deserialize impl —— 由规划/实现阶段根据 toml crate 对 flatten+alias 的实际支持情况决定
- **indicators / sql / record_sql 旧格式兼容**：这些字段目前已是子表形式（无 flatten），旧格式 key 名不变；主要工作量在 meta 字段兼容

## Deferred Ideas

- `[template]` / `[charts]` 配置嵌套化 → Phase 18
- 代码结构拆分（filters.rs 行数过多）→ Phase 19
- TODO "调研 dm-database-parser-sqllog 1.0.0 新特性" → 已在 Phase 6 关闭（PERF-07），无需处理
