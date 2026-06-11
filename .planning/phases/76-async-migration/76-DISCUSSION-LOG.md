# Phase 76: 异步解析路径迁移 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-11
**Phase:** 76-async-migration
**Mode:** --auto (全自动模式)
**Areas discussed:** Tokio运行时接入策略, Rayon/Tokio混合路径桥接, 顺序路径与Scanner, 错误处理策略, Bench文件适配, 测试迁移策略

---

## 背景说明

本 phase 的实现在讨论前已通过独立提交完成：
- `65c24fd feat: complete async migration — replace async_rt bridge with AsyncLogParser`

--auto 模式下记录的决策均基于对该 commit 的代码审查，反映实际已实现的选择。

---

## Tokio 运行时接入策略

| 选项 | 描述 | 已选 |
|------|------|------|
| `#[tokio::main]` multi_thread | 主入口宏，支持 block_in_place | ✓ |
| `Runtime::new().block_on()` 手动 | 更精确控制，但模板代码多 | |
| single-thread runtime | 不支持 block_in_place，与 rayon 不兼容 | |

**选择：** `#[tokio::main]` multi_thread flavor
**理由：** multi_thread 是唯一支持 `block_in_place`（rayon 路径必须）的配置；`#[tokio::main]` 宏最简洁

---

## Rayon/Tokio 混合路径桥接

| 选项 | 描述 | 已选 |
|------|------|------|
| `block_in_place + Handle.block_on` | tokio 感知阻塞，安全 | ✓ |
| 在 rayon 任务内新建 Runtime | 会 panic（嵌套 runtime 禁止） | |
| `futures::executor::block_on` | 不集成 tokio reactor，会死锁 | |

**选择：** `block_in_place + Handle::current().block_on()`
**理由：** 唯一正确的 rayon 线程内驱动 tokio async 的方式；`block_in_place` 通知 tokio 当前线程将阻塞，防止线程池耗尽

---

## 顺序路径与 Scanner

| 选项 | 描述 | 已选 |
|------|------|------|
| native async fn + .await | 全链路 async，最自然 | ✓ |
| spawn_blocking 包装 | 额外线程开销，不必要 | |
| 保留同步、只在入口处 block_on | 丢失 async 传播优势 | |

**选择：** native async fn + .await
**理由：** 调用链全为 async 时 .await 最高效；`sequential.rs → processor.rs → AsyncLogParser` 全链路异步

---

## 错误处理策略

| 选项 | 描述 | 已选 |
|------|------|------|
| graceful warn + skip | 与 Phase 36 策略一致 | ✓ |
| 传播错误到 error log 文件 | AsyncLogParser 不支持此接口 | |
| 致命错误终止处理 | 用户体验差 | |

**选择：** graceful warn + skip
**理由：** AsyncLogParser 不暴露逐条解析错误（与旧 sync API 行为差异）；warn + skip 与项目既有错误处理策略一致

---

## Bench 文件适配

| 选项 | 描述 | 已选 |
|------|------|------|
| `Runtime::new().block_on()` per bench | bench 独立 runtime，最简单 | ✓ |
| 全局 `once_cell` runtime | 共享但需同步 | |
| `#[tokio::main]` 在 bench 入口 | criterion 不支持 async main | |

**选择：** `Runtime::new().block_on()` per bench
**理由：** criterion bench 回调为同步闭包，`Runtime::new().block_on()` 是标准适配模式；无需共享 runtime

---

## 测试迁移策略

| 选项 | 描述 | 已选 |
|------|------|------|
| `#[tokio::test(flavor = "multi_thread")]` for rayon | 支持 block_in_place | ✓ |
| `#[tokio::test]` 单线程 | 不支持 block_in_place | |
| `Runtime::new().block_on()` in test body | 啰嗦，不必要 | |

**选择：** 按需选择：rayon 测试用 multi_thread flavor，纯 async 测试用标准 `#[tokio::test]`
**理由：** 最小侵入；只有真正需要 `block_in_place` 的测试才开 multi_thread

---

## Claude's Discretion

- bench 文件 `Runtime::new().unwrap()` vs `expect("tokio runtime")`
- 各 async fn 签名上 `#[allow(clippy::too_many_arguments)]` 的清理时机

## Deferred Ideas

- flamegraph CPU 热点分析（PROF-01）— 待 async 迁移稳定后再 profile
- heaptrack 峰值内存 profiling（PROF-02）— 需要真实大文件环境
- AsyncLogParser 错误细节重新暴露（需 upstream 支持）
