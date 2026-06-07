# Phase 1: watch 功能完善 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 1-watch
**Mode:** --auto (fully autonomous, no interactive prompts)
**Areas discussed:** CSV watch 追加语义、error log 追加传递方式、退出码 130 实现点

---

## CSV watch 追加语义 (WATCH-07)

| Option | Description | Selected |
|--------|-------------|----------|
| 全量+增量都追加 | trigger_full_file 和 build_incremental_cfg 均设 csv_cfg.append=true | ✓ |
| 仅增量追加 | 全量触发覆盖写，增量追加 | |
| 不追加，每次覆盖 | 保持现有行为（CSV 不支持 watch） | |

**Auto-selected:** 全量+增量都追加
**Rationale:** STATE.md 已决策"所有触发均追加"；CsvExporter append 模式已内建 TOCTOU 安全 header 处理，无需额外工作。

---

## error log 追加传递方式 (WATCH-08)

| Option | Description | Selected |
|--------|-------------|----------|
| Config 内部字段 #[serde(skip)] | append_error_log: bool 字段，handle_run 签名不变 | ✓ |
| handle_run 新增参数 | 添加 append_errors: bool 参数，需更新所有调用点 | |
| write_error_log 直接参数 | 函数签名加 bool，从 handle_run 传入 | |

**Auto-selected:** Config 内部字段 #[serde(skip)]
**Rationale:** 最小侵入，handle_run 签名不变，与项目已有的非序列化内部字段惯例一致。

---

## 退出码 130 实现点 (WATCH-09)

| Option | Description | Selected |
|--------|-------------|----------|
| handle_watch 尾部检查 | print_final_summary 后检查 interrupted，返回 Err(Error::Interrupted) | ✓ |
| main.rs 单独处理 | watch 调用路径返回特殊状态，main 判断并 exit(130) | |
| run_watch_loop 内部传播 | loop 退出时直接返回 Err | |

**Auto-selected:** handle_watch 尾部检查
**Rationale:** 与 run 路径对称（handle_run 也是返回 Err(Error::Interrupted)），main.rs 已有处理分支无需修改，最小变更。

---

## Claude's Discretion

- `build_incremental_cfg` 同时处理 SQLite 和 CSV append 设置，可视代码重复程度决定是否提取辅助函数。
- `append_error_log` 字段不加入 Config::validate() 校验（内部运行时状态，非用户配置）。

## Deferred Ideas

- CSV offset 跟踪（类似 SQLite _watch_offsets）——当前 append 追加全量记录不需要，若将来需要精确增量可在后续 Phase 评估
- watch 多目录 glob 支持（WATCH-10）——Future requirement
- watch 远程推送 webhook（WATCH-11）——Future requirement
