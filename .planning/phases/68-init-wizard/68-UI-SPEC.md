---
phase: 68
slug: init-wizard
status: draft
shadcn_initialized: false
preset: none
created: 2026-06-05
ui_medium: cli-tty
---

# Phase 68 — CLI Interaction Design Contract

> 本 spec 描述的"UI"是终端文本向导的交互契约（stdout/stderr 输出文本、提示格式、输入验证、完成消息）。
> 不涉及 Web 前端。由 gsd-ui-researcher 生成，供 gsd-planner 和 gsd-executor 消费。

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none（纯 Rust std::io，无第三方 TUI 库）|
| Preset | not applicable |
| Component library | none |
| Icon library | none（使用 ASCII 符号：`?`、`>>`、`✓`）|
| Font | 终端默认等宽字体 |

---

## Prompt Format Contract

### 统一提示格式

每步提示遵循固定格式（来源：CONTEXT.md D-04）：

```
{字段描述}（{示例说明}）[default: {默认值}]: 
```

规则：
- 使用 `print!()` 而非 `println!()`，提示与用户输入在同一行
- `print!()` 后必须调用 `stdout().flush()` 确保提示可见
- 提示末尾有一个空格，冒号与用户光标之间留一个字符
- 光标跟在冒号+空格后，用户直接输入

---

## Wizard Steps Contract

### Step 1 — SQL 日志输入路径

**提示文本（精确字符串）：**

```
SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: 
```

**行为规则：**
- 输入为空（仅 Enter）→ 使用默认值 `sqllogs`
- 输入任意非空字符串 → 使用用户输入（原样接受，不做路径存在性验证）
- `.trim()` 后写入 `WizardAnswers.inputs`

**来源：** CONTEXT.md D-04 Step 1

---

### Step 2 — 导出格式选择

**提示文本（精确字符串）：**

```
导出格式 (csv/sqlite) [default: csv]: 
```

**行为规则：**
- 输入为空 → 使用默认值 `csv`
- 输入 `csv` → 选择 CSV 导出
- 输入 `sqlite` → 选择 SQLite 导出
- 其他输入 → 打印错误消息并重新提示（最多 3 次）：

```
无效格式"{用户输入}"，请输入 csv 或 sqlite: 
```

- 3 次失败后返回 `Err`（不无限循环）

**来源：** CONTEXT.md D-04 Step 2，Discretion

---

### Step 3a — CSV 输出路径（仅当选择 csv 时）

**提示文本（精确字符串）：**

```
CSV 输出文件路径 [default: outputs/sqllog.csv]: 
```

**行为规则：**
- 输入为空 → 使用默认值 `outputs/sqllog.csv`
- 输入任意非空字符串 → 使用用户输入
- 写入 `WizardAnswers.csv_file`

**来源：** CONTEXT.md D-04 Step 3a

---

### Step 3b — SQLite 数据库路径（仅当选择 sqlite 时）

**提示文本（精确字符串）：**

```
SQLite 数据库路径 [default: export/sqllog2db.db]: 
```

**行为规则：**
- 输入为空 → 使用默认值 `export/sqllog2db.db`
- 写入 `WizardAnswers.sqlite_db`

**来源：** CONTEXT.md D-04 Step 3b

### Step 3c — SQLite 表名（仅当选择 sqlite 时，紧跟 3b）

**提示文本（精确字符串）：**

```
表名（仅含字母/数字/下划线）[default: sqllog_records]: 
```

**行为规则：**
- 输入为空 → 使用默认值 `sqllog_records`
- 写入 `WizardAnswers.sqlite_table`
- 不做正则验证（由 `Config::validate` 在生成后检查）

**来源：** CONTEXT.md D-04 Step 3b（table_name 部分）

---

## Completion Output Contract

向导完成、文件写入成功后，打印与非交互式 `init` 完全一致的 Next steps 格式：

```
Configuration file generated: {output_path}
Next steps:
  1. Edit configuration file: {output_path}
  2. Validate configuration: sqllog2db validate -c {output_path}
  3. Run export: sqllog2db run -c {output_path}
```

**规则：**
- 通过 `log::info!()` 输出（与 `handle_init` 保持一致）
- 不使用 `println!` 直接打印这段内容
- 若文件已存在且 `--force` 生效，第一行改为 `Configuration file overwritten: {output_path}`

**来源：** `src/cli/init.rs` handle_init 第 48–58 行，CONTEXT.md Discretion

---

## Error Messages Contract

### 输入验证失败（格式选择）

```
无效格式"{用户输入}"，请输入 csv 或 sqlite: 
```

- 在同一行等待重新输入（同样用 `print!` + flush）
- 第 3 次失败后返回：`Err(Error::Config(ConfigError::InvalidValue { ... }))`

### 文件已存在（未加 --force）

```
error: Configuration file already exists: {output_path}
hint: use --force to overwrite
```

- 通过 `log::error!` + `log::info!` 输出（复用现有 `handle_init` 逻辑）

### stdin 读取失败

```
error: Failed to read input: {io_error_description}
```

- `stdin.read_line()` 返回 `Err` 时，包装为 `Error::File(FileError::...)` 返回

### 来源：CONTEXT.md D-06/D-07，src/cli/init.rs 第 14–19 行

---

## Non-TTY / Piped Input Behavior

| 场景 | 行为 |
|------|------|
| stdin 是管道（非 TTY）| 正常运行；`read_line` 从管道读取；提示写到 stdout（可能被丢弃）|
| stdout 被重定向 | 提示文本仍通过 `print!` 写出，不做 TTY 检测 |
| stdin 立即 EOF（空管道）| 每步 `read_line` 返回空字符串，等同全程按 Enter，使用全部默认值 |
| CI/脚本环境 | 可通过 `echo -e "sqllogs\ncsv\n"` 管道方式驱动向导 |

**设计依据：** 无新依赖（CONTEXT.md D-06），不引入 `dialoguer` 的 TTY 检测，行为简单可预期。

---

## Template Substitution Contract

向导使用字符串替换操纵 `CONFIG_TEMPLATE_EN`（来源：CONTEXT.md D-08/D-09）：

| 替换目标 | 原始字符串 | 替换后字符串 |
|----------|----------|------------|
| inputs 路径 | `inputs = ["sqllogs"]` | `inputs = ["{user_inputs}"]` |
| CSV 文件路径 | `file = "outputs/sqllog.csv"` | `file = "{user_csv_file}"` |
| SQLite 激活 | `# [exporter.sqlite]` + 各字段前 `# ` | 去掉 `# `，激活 sqlite 段 |
| SQLite database_url | `# database_url = "export/sqllog2db.db"` | `database_url = "{user_db}"` |
| SQLite table_name | `# table_name = "sqllog_records"` | `table_name = "{user_table}"` |
| CSV 禁用（sqlite 模式）| `[exporter.csv]` + 各字段 | 每行前加 `# ` 注释掉整段 |

**不变部分：** 所有注释行（`# ...`）、`[logging]`、`[replace_parameters]`、`[filter]`、`[stats]` 段保持原样。

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| 主要操作完成 | `Configuration file generated: {path}` |
| 文件覆盖完成 | `Configuration file overwritten: {path}` |
| Step 1 提示 | `SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: ` |
| Step 2 提示 | `导出格式 (csv/sqlite) [default: csv]: ` |
| Step 3a 提示 | `CSV 输出文件路径 [default: outputs/sqllog.csv]: ` |
| Step 3b 提示 | `SQLite 数据库路径 [default: export/sqllog2db.db]: ` |
| Step 3c 提示 | `表名（仅含字母/数字/下划线）[default: sqllog_records]: ` |
| 格式验证失败 | `无效格式"{input}"，请输入 csv 或 sqlite: ` |
| Next steps 第 1 行 | `  1. Edit configuration file: {path}` |
| Next steps 第 2 行 | `  2. Validate configuration: sqllog2db validate -c {path}` |
| Next steps 第 3 行 | `  3. Run export: sqllog2db run -c {path}` |
| 文件已存在提示 | `use --force to overwrite` |

**语言一致性规则：** 提示文本使用中文（面向首次使用者），系统消息（Next steps、info log）与现有 `handle_init` 保持英文一致。

---

## Data Model Contract

`run_wizard` 返回以下结构体（来源：CONTEXT.md D-07/Specifics）：

```rust
pub struct WizardAnswers {
    pub inputs: String,
    pub exporter: ExporterChoice,
    pub csv_file: Option<String>,
    pub sqlite_db: Option<String>,
    pub sqlite_table: Option<String>,
}

pub enum ExporterChoice {
    Csv,
    Sqlite,
}
```

默认值填充规则（当用户按 Enter）：
- `inputs` → `"sqllogs"`
- `exporter` → `ExporterChoice::Csv`
- `csv_file` → `Some("outputs/sqllog.csv")`
- `sqlite_db` → `Some("export/sqllog2db.db")`（仅 sqlite 路径）
- `sqlite_table` → `Some("sqllog_records")`（仅 sqlite 路径）

---

## Spacing Scale

不适用于 CLI 文本 UI。终端输出无像素间距概念。

以下为终端"间距"约定（字符层面）：

| 位置 | 约定 |
|------|------|
| 提示文本末尾 | 冒号后跟一个空格（`: `），再接用户输入 |
| Next steps 缩进 | 2 个空格（与现有 `handle_init` 一致）|
| 步骤之间 | 无空行（连续提示，类似表单填写）|

---

## Typography

不适用于 CLI。终端使用系统等宽字体，无字号控制。

| 表现层 | 约定 |
|--------|------|
| 正常文本 | 终端默认前景色 |
| 错误消息 | 通过 `log::error!` 输出（env_logger 通常加红色 ERROR 前缀）|
| 提示文本 | 通过 `print!` 直接输出（无颜色修饰）|

---

## Color

不适用于 CLI。不使用 ANSI 颜色码，不引入 `colored` 或 `termcolor` 等 crate（CONTEXT.md D-06：无新依赖）。

日志级别颜色由 `env_logger` 或 `simplelog` 的现有配置决定，向导代码不直接控制颜色。

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| 无（纯 std::io）| none | not applicable |

不引入任何第三方 crate（CONTEXT.md D-06）。

---

## Pre-Population Source Audit

| 决策 | 来源 |
|------|------|
| 3 个向导步骤（inputs / exporter / output） | CONTEXT.md D-04 |
| 提示文本精确字符串 | CONTEXT.md D-04 |
| 默认值（sqllogs / csv / outputs/sqllog.csv 等）| CONTEXT.md D-04 + CONFIG_TEMPLATE_EN |
| `print!` + flush + `read_line` IO 模式 | CONTEXT.md D-06 |
| `run_wizard(reader, writer)` 可测试签名 | CONTEXT.md D-07 |
| CONFIG_TEMPLATE_EN 字符串替换策略 | CONTEXT.md D-08/D-09 |
| 格式验证最多 3 次循环 | CONTEXT.md Discretion |
| Next steps 完成输出格式 | CONTEXT.md Discretion + src/cli/init.rs:53-57 |
| `log::info!` 用于系统消息 | src/cli/init.rs 现有模式 |
| 非 TTY 降级行为（无检测，直接读取）| CONTEXT.md D-06（无新依赖约束）|

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Interaction: PASS
- [ ] Dimension 3 Error States: PASS
- [ ] Dimension 4 Default Values: PASS
- [ ] Dimension 5 Non-TTY Behavior: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
