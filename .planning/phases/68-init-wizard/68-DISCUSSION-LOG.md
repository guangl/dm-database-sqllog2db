# Phase 68: 交互式配置向导 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 68-交互式配置向导
**Areas discussed:** `--interactive` flag 结构, 向导字段覆盖范围, IO 实现方式, 配置文件生成方式, 测试策略

---

## `--interactive` Flag 结构

| Option | Description | Selected |
|--------|-------------|----------|
| 加入现有 Init variant 为 bool flag | 与 `--output`/`--force` 并列，clap 处理组合 | ✓ |
| 新建独立 subcommand | 完全分离，但破坏 `init` 命令的整体性 | |

**User's choice:** [auto] 加入现有 `Commands::Init` 为 bool flag (recommended default)
**Notes:** `--output` 控制 config 写入路径，`--force` 仍有效；向导只询问配置内容字段。

---

## 向导字段覆盖范围

| Option | Description | Selected |
|--------|-------------|----------|
| 3 个核心字段（inputs、导出格式、导出输出路径） | 轻量，直接满足 INIT-02，其余段保持注释默认 | ✓ |
| 全字段向导（含 logging/filter/stats） | 完整但步骤过多，增加首次使用门槛 | |

**User's choice:** [auto] 3 个核心字段（inputs 路径、导出格式 csv/sqlite、导出输出路径）(recommended default)
**Notes:** logging/filter/stats/replace_parameters 段保持模板注释默认，无需向导引导。

---

## IO 实现方式

| Option | Description | Selected |
|--------|-------------|----------|
| 原生 `std::io::stdin().read_line()` | 无新依赖，轻量 | ✓ |
| `dialoguer` crate | 更丰富的 TUI 体验，但引入新依赖 | |

**User's choice:** [auto] 原生 `stdin().read_line()`，无新依赖 (recommended default)
**Notes:** 每步 `print!()` + `stdout().flush()` 确保提示可见。

---

## 配置文件生成方式

| Option | Description | Selected |
|--------|-------------|----------|
| 字符串替换 `CONFIG_TEMPLATE_EN` | 保留所有注释，格式与非交互式 init 完全一致 | ✓ |
| 独立 TOML 序列化 | 格式可能不一致，违反 INIT-03 | |

**User's choice:** [auto] 字符串替换 `CONFIG_TEMPLATE_EN` 中的默认值 (recommended default)
**Notes:** 满足 INIT-03 的核心约束；sqlite 模式需注释掉 csv 段并激活 sqlite 段。

---

## 测试策略

| Option | Description | Selected |
|--------|-------------|----------|
| `impl BufRead` 参数化，单元测试注入 `Cursor` | 快速、稳定，无进程开销 | ✓ |
| subprocess 集成测试（piped stdin） | 更接近真实，但慢且脆 | |

**User's choice:** [auto] `run_wizard(reader: impl BufRead, writer: impl Write)` 参数化，测试用 `Cursor` (recommended default)
**Notes:** 生产路径传入 `stdin()`/`stdout()`；测试传入 `Cursor`/`Vec<u8>`。

---

## Claude's Discretion

- 导出格式验证：无效输入循环提示最多 3 次，避免无限循环
- sqlite 模式的模板替换：按行处理，注释掉 csv 段，去注释 sqlite 段
- 向导结束后打印 "Next steps" 信息与非交互式 init 一致

## Deferred Ideas

None — discussion stayed within phase scope
