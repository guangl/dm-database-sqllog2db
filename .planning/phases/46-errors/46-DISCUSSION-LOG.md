# Phase 46: 错误信息优化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 46-错误信息优化
**Areas discussed:** 错误展示格式

---

## 错误展示格式

| Option | Description | Selected |
|--------|-------------|----------|
| 改为 'error:' + 'hint:' | 将 [CRITICAL] 改为 error: 前缀，Suggestion: 改为 hint: | |
| 保持 [SEVERITY] + 'Suggestion:' | 保持当前三级严重度格式不变 | |
| 保持 [SEVERITY]，只改 'Suggestion:' 为 'hint:' | 保留严重度分级，统一 hint 前缀 | ✓ |

**User's choice:** 保持 [SEVERITY]，只改 'Suggestion:' 为 'hint:'
**Notes:** 用户认为三级严重度（WARNING/ERROR/CRITICAL）有实际用途，不希望丢失。只做最小改动。

---

## 配置解析错误字段信息

| Option | Description | Selected |
|--------|-------------|----------|
| 依赖 TOML 解析器自带的字段信息 | toml::from_str 的错误已包含字段名和行号 | ✓ |
| 自定义 serde path 提取 | 更精确但实现复杂 | |
| 仅针对手动构建的 ConfigError 添加字段名 | 折中方案 | |

**User's choice:** 依赖 TOML 解析器自带的字段信息
**Notes:** 避免过度工程化，TOML 库自带的错误已够用。

---

## Error::Io hint 补全

| Option | Description | Selected |
|--------|-------------|----------|
| 添加通用 IO hint | 返回 "Check filesystem permissions and disk space." | ✓ |
| 不加 IO hint，保持空字符串 | Error::Io 已参含 std::io::Error 信息 | |

**User's choice:** 添加通用 IO hint
**Notes:** 与其他变体风格统一。

---

## Claude's Discretion

无

## Deferred Ideas

无
