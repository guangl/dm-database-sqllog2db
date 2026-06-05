# Phase 67: 进度/摘要与诊断增强 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 67-进度/摘要与诊断增强
**Mode:** --auto (fully autonomous, no user interaction)
**Areas discussed:** 进度条升级, Error Log 写入, ErrorKind 分类, Hint 触发与摘要

---

## 进度条升级 (PROG-01/02)

| Option | Description | Selected |
|--------|-------------|----------|
| `ProgressBar::new(total_files)` + `{pos}/{len}` + records/sec in message | 切换为文件计数为长度的 bar，indicatif 自动计算 ETA，records/sec 嵌入 {msg} | ✓ |
| 保持 spinner，手动计算并嵌入全部指标 | 最灵活但 ETA 需手动实现 | |
| `MultiProgress` 每文件独立一栏 | 过度复杂，不适合当前 single-threaded 顺序路径 | |

**Auto-selected:** ProgressBar::new(total_files) + {pos}/{len} template + indicatif ETA + records/sec in message
**Notes:** [auto] 自动选择首选方案；并行路径不受影响（无进度条）

---

## Error Log 写入 (DIAG-01)

| Option | Description | Selected |
|--------|-------------|----------|
| `ErrorStats` 收集 `ParseErrorRecord`，完成后批量写入 | 与现有 ErrorStats 聚合模式一致，写入发生在 handle_run 末尾 | ✓ |
| 实时写入（Mutex<BufWriter>） | 需要跨线程共享，并行路径需要额外同步 | |
| 写入 app log（log::warn! 路径） | 无法满足 DIAG-01 对专用 error file 的要求 | |

**Auto-selected:** ErrorStats 收集 ParseErrorRecord，完成后批量写入 [error] file
**Notes:** [auto] Config.error 字段从 None → Some 开始生效；测试 TOML 中已有 [error] 段

---

## ErrorKind 分类 (DIAG-02)

| Option | Description | Selected |
|--------|-------------|----------|
| 启发式分类（FFFD → EncodingError；(EP[ 前缀 → FieldMissing；其他 → ParseFailed） | 零额外依赖，覆盖主要场景 | ✓ |
| 全部归类为 ParseFailed | 最简单，但 DIAG-02 分组统计没有意义 | |
| 正则精细分类 | 过度复杂，DM log 格式固定不需要复杂匹配 | |

**Auto-selected:** 启发式分类（含 \u{FFFD} → EncodingError；以 (EP[ 开头 → FieldMissing；其他 → ParseFailed）
**Notes:** [auto] ParseError 只有 InvalidFormat 单一变体，无内置分类

---

## Hint 触发与摘要 (DIAG-03/PROG-03)

| Option | Description | Selected |
|--------|-------------|----------|
| count > 0 即触发（任意同类错误） | 最简单，用户有错误时总能看到建议 | ✓ |
| count >= 5 AND 占比 >= 20% | 减少噪音，但小文件场景可能永远不触发 | |
| count >= 3（简单绝对阈值） | 中间方案，但阈值选择随意 | |

**Auto-selected:** count > 0 即触发 hint
**Notes:** [auto] 过滤率 filtered_out 字段同步加入 ErrorStats，摘要显示 filtered N records (X%)

---

## Claude's Discretion

- records/sec 显示格式：整数千位简写（1234 → "1234 rec/s"，12000 → "12k rec/s"）
- 并行路径不显示进度条（沿用现有行为）
- `parse_error_records` Vec 上限 10000 条（防 OOM 极端情况）

## Deferred Ideas

None
