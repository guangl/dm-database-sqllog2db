# Phase 2: 测试覆盖率与 FSEvents - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 02-fsevents
**Mode:** --auto (fully autonomous)
**Areas discussed:** FSEvents #[ignore] 落地方案, 覆盖率缺口目标文件策略, WATCH-07/08/09 集成测试层级

---

## FSEvents #[ignore] 落地方案 (QUAL-03)

| Option | Description | Selected |
|--------|-------------|----------|
| (a) `#[cfg(not(target_os = "macos"))]` | 条件编译跳过，macOS 上完全不编译此测试 | |
| (b) Mock 文件系统事件注入 | 在 watch/mod.rs 引入抽象层支持 notify mock | |
| (c) 保留 `#[ignore]` + 书面依据 | 维持现状，在 CONTEXT.md 记录决策理由 | ✓ |

**Auto-selected:** Option (c)
**Rationale:** notify crate 不提供 mock 层（option b 需重大架构改动）；cfg 跳过在 macOS 开发机上完全隐藏测试，比 ignore 更不透明（option a）；保留 ignore 使测试体仍可手动触发（`cargo test -- --ignored`）作为 smoke test。

---

## 覆盖率缺口目标文件策略 (QUAL-02)

| Option | Description | Selected |
|--------|-------------|----------|
| collector.rs + csv/mod.rs | 48+33=81 行 uncovered，半数即达目标，且两者因 watch CSV 集成测试关联度高 | ✓ |
| sqlite/mod.rs + filter_processor.rs | 50+X 行 uncovered，测试路径更复杂（事务错误路径） | |
| 全量扫描所有 <90% 文件 | 散弹式，可能不必要 | |

**Auto-selected:** collector.rs + exporter/csv/mod.rs
**Rationale:** watch 集成测试（WATCH-07/08/09）调用 `trigger_full_file` 会经过 csv exporter 的真实文件 I/O 路径，间接提升 exporter/csv/mod.rs 覆盖率；collector.rs 的 parse error 分支和 filtered PARAMS 分支可通过定向单元测试以低成本覆盖。

---

## WATCH-07/08/09 集成测试层级

| Option | Description | Selected |
|--------|-------------|----------|
| 单元测试已足够（Phase 1 已完成） | src/cli/watch/mod.rs 中的 test_watch_csv_append 等已覆盖 | |
| 在 tests/watch_incremental.rs 补充集成测试 | 对齐 success criteria「watch 集成测试」要求，带动 csv 覆盖率 | ✓ |
| CLI 级别 e2e 测试（assert_cmd） | 需要实际运行 watch 进程和信号，实现复杂度高 | |

**Auto-selected:** tests/watch_incremental.rs 集成测试
**Rationale:** Success criteria 明确要求「watch 集成测试」，在 tests/ 目录的测试符合 Rust 惯例。CLI e2e 测试需要进程级控制（信号发送），复杂度超出当前阶段。tests/watch_incremental.rs 与现有 test_watch_03_*/test_watch_04_* 模式一致，实现成本低。

---

## Claude's Discretion

- `collector.rs` 的 `process_record` 是私有函数，测试方式（间接调用 vs `#[cfg(test)]` 直接调用）由实现者决定
- 若集成测试已将整体行覆盖率推至 92%+，无需额外补充 sqlite/mod.rs 测试

## Deferred Ideas

- `exporter/sqlite/mod.rs` 错误路径测试（函数级 60%）——Phase 2 达标后若有余量再补
- FSEvents mock 注入架构方案的未来评估
