# Phase 70: Watch 增量处理与集成测试 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 70-watch
**Mode:** --auto (fully autonomous)
**Areas discussed:** 增量读取机制, 事件路由策略, SQLite 偏移持久化, handle_run 调用方式, 模块结构, 集成测试策略

---

## 增量读取机制（WATCH-03 核心）

| Option | Description | Selected |
|--------|-------------|----------|
| 临时文件方案 | 读取新字节 → NamedTempFile → LogParser → 处理 → 自动删除 | ✓ |
| 修改 LogParserBuilder | 上游 crate 新增 start_offset 参数 | |
| 内存 Buffer | 新字节存 Vec<u8>，自定义解析器遍历 | |

**自动选择：** 临时文件方案
**理由：** `LogParserBuilder::build()` 内部使用 `fs::read()` 全量读取，无 API 扩展点；临时文件方案零上游依赖，watch.rs 完全控制偏移逻辑。

---

## 事件路由策略

| Option | Description | Selected |
|--------|-------------|----------|
| Create→全文 + Modify→增量 | 按事件类型路由，Create 记录初始偏移 | ✓ |
| 所有事件走增量路径 | 统一处理，Create 时 start_offset=0 | |
| 仅处理 Create 事件（忽略 Modify） | Phase 70 退化到 Phase 69 | |

**自动选择：** Create→全文处理（Phase 69 行为）+ 记录偏移；Modify→增量处理（Phase 70 新增）
**理由：** 两种事件语义不同，需分开路由以保证正确性。

---

## SQLite 字节偏移持久化（WATCH-04 核心）

| Option | Description | Selected |
|--------|-------------|----------|
| SQLite 辅助表 _watch_offsets | 与导出 DB 同库，新开独立连接 | ✓ |
| 独立 JSON state 文件 | watch-state.json 存储偏移 | |
| 内存 only（不跨重启） | 不满足 WATCH-04 要求 | |

**自动选择：** SQLite 辅助表（STATE.md 既定决策）
**理由：** `.planning/STATE.md` §"Architecture Notes for Phases 69–70" 明确规定此方案；同库存储避免额外状态文件。

---

## handle_run 调用方式（增量路径）

| Option | Description | Selected |
|--------|-------------|----------|
| tmp_cfg + 强制 append=true | clone cfg，覆盖 inputs 和 sqlite.append | ✓ |
| 直接调用 handle_run + 依赖用户 config | 用户须自行设置 append=true | |
| 新建 exporter 实例 | 绕过 handle_run，直接调用 exporter | |

**自动选择：** tmp_cfg 方案，强制 `sqlite.append=true, overwrite=false`
**理由：** 增量触发必须 append，否则每次 Modify 事件会清空 SQLite 表。临时覆盖 config 不影响用户配置文件。

---

## 模块结构

| Option | Description | Selected |
|--------|-------------|----------|
| watch.rs → watch/mod.rs + watch/offsets.rs | Phase 69 预设的扩展路径 | ✓ |
| 保持 watch.rs 单文件 | 不拆分，offset 逻辑内联 | |
| 独立 src/offsets.rs | 跨模块共享（目前无需要） | |

**自动选择：** `watch/mod.rs + watch/offsets.rs`
**理由：** watch.rs 已 532 行；加入 offset 逻辑后将超过合理范围；Phase 69 D-13 预设了此扩展路径。

---

## 集成测试策略

| Option | Description | Selected |
|--------|-------------|----------|
| tempfile + 真实 SQLite DB | 端对端验证 append 幂等性 + 重启恢复 | ✓ |
| 纯单元测试（mock exporter） | 快但无法验证 SQLite 层行为 | |
| assert_cmd CLI 测试 | 需启动完整 watch 进程，超时控制复杂 | |

**自动选择：** tempfile + 真实 SQLite DB 集成测试
**理由：** WATCH-04 要求验证持久化和重启恢复，必须使用真实 SQLite；纯单元测试无法覆盖 SC3。

---

## Claude's Discretion

- 临时文件后缀 `.log`：确保 `dm-database-parser-sqllog` 的编码探测逻辑（基于文件内容采样）正常工作
- offsets.rs 连接策略：每次调用打开新连接（不长持），避免与 SqliteExporter 事务冲突
- 路径规范化：`file_offsets` 键使用 `canonicalize()` 避免相对/绝对路径不一致

## Deferred Ideas

- watch + CSV 增量插入 → Out of Scope
- watch 多目录监听（WATCH-F02）→ Future phase
- 内存 buffer 替代临时文件（大 append 性能优化）→ 后续 patch
