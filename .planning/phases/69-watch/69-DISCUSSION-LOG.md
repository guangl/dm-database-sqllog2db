# Phase 69: Watch 模式核心框架 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 69-Watch 模式核心框架
**Areas discussed:** notify crate 选型, 状态行显示, 触发策略, Ctrl+C 退出, 模块结构, 导出格式约束, 测试策略

---

## notify crate 版本与监听模式

| Option | Description | Selected |
|--------|-------------|----------|
| notify = "6" + blocking mpsc::channel | 与现有单线程模型兼容，无需 tokio | ✓ |
| notify = "7" / async | 更新的 API，但需要 async runtime | |

**User's choice:** [auto] notify = "6" + RecommendedWatcher + blocking mpsc::channel (recommended default)
**Notes:** ctrlc dep 已在 Cargo.toml，无需新增信号处理 crate。

---

## 状态行显示方式（WATCH-05）

| Option | Description | Selected |
|--------|-------------|----------|
| indicatif ProgressBar（已有依赖） | 一致的 spinner 体验，set_message() 更新 | ✓ |
| 手动 \r 覆写 | 轻量但难以与 log 输出共存 | |

**User's choice:** [auto] indicatif::ProgressBar::new_spinner() + set_message() (recommended default)
**Notes:** stderr 输出，不干扰 stdout/log 输出（SC3）。

---

## 触发处理策略（WATCH-02）

| Option | Description | Selected |
|--------|-------------|----------|
| 直接触发，无 debounce | 最简单，满足 2 秒约束 | ✓ |
| debounce 延迟（如 500ms） | 防止多文件快速到来重复触发，增复杂性 | |

**User's choice:** [auto] 直接触发，仅处理新增文件（inputs override 到新文件路径）(recommended default)
**Notes:** Phase 69 处理完整新文件，Phase 70 负责追加增量。

---

## Ctrl+C 优雅退出（WATCH-06）

| Option | Description | Selected |
|--------|-------------|----------|
| 复用 Arc<AtomicBool> + ctrlc::set_handler | 与现有 Run 命令完全一致 | ✓ |
| tokio signal 或 signal-hook | 额外依赖，不一致 | |

**User's choice:** [auto] 复用现有 ctrlc 模式（src/main.rs:160-168）(recommended default)
**Notes:** 退出码 0，`pb.finish_and_clear()` 后打印最终摘要。

---

## 模块结构

| Option | Description | Selected |
|--------|-------------|----------|
| src/cli/watch.rs（单文件） | Phase 69 范围小，Phase 70 可扩展 | ✓ |
| src/cli/watch/mod.rs | 预先模块化，但过度设计 | |

**User's choice:** [auto] src/cli/watch.rs (recommended default)

---

## 导出格式约束

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 69 对所有格式透明 | 完整处理新文件，CSV/SQLite 均可 | ✓ |
| 仅支持 SQLite | Out of Scope 约束（实际适用于 Phase 70 增量）| |

**User's choice:** [auto] Phase 69 对所有格式透明，Phase 70 增量逻辑仅限 SQLite (recommended default)

---

## 测试策略

| Option | Description | Selected |
|--------|-------------|----------|
| 集成测试（实际创建文件触发） | 真实验证文件系统事件 | ✓ |
| 仅单元测试（mock watcher） | 快速但不验证 notify 集成 | |

**User's choice:** [auto] 集成测试 + interrupted flag 直接设置（单元可控）(recommended default)

---

## Claude's Discretion

- recv_timeout(100ms) poll 间隔
- ProgressBar tick 在每次 loop iteration
- elapsed 用 std::time::Instant，不引入 chrono

## Deferred Ideas

- watch 增量处理 → Phase 70
- SQLite 字节偏移去重 → Phase 70
