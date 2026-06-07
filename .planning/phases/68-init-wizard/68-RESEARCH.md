# Phase 68: 交互式配置向导 - Research

**Researched:** 2026-06-06
**Domain:** Rust CLI 向导 / std::io 交互 / 字符串模板替换
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 在 `src/cli/opts.rs` 的 `Commands::Init` variant 新增 `#[arg(long = "interactive", short = 'i')] interactive: bool`。与现有 `--output` 和 `--force` flag 并列，clap 自动处理组合。
- **D-02:** `--output`（`-o`）在交互模式下控制 config 写入路径。向导内部只询问"配置内容"字段，不重新询问 config 写入路径。`--force` 仍然有效。
- **D-03:** `src/main.rs` dispatch：若 `interactive` 为 true，调用 `handle_init_interactive(output, force)`；否则调用现有 `handle_init(output, force)`。
- **D-04:** 向导步骤（顺序固定）：
  1. inputs 路径，提示 `"SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: "`
  2. 导出格式，提示 `"导出格式 (csv/sqlite) [default: csv]: "`，无效输入最多 3 次
  3a. csv 路径，提示 `"CSV 输出文件路径 [default: outputs/sqllog.csv]: "`
  3b. sqlite db 路径 + table_name，提示见 D-04
- **D-05:** logging、filter、stats、replace_parameters 段保持模板默认值不询问。
- **D-06:** 不引入新 crate。使用 `print!()` + `stdout().flush()` + `stdin.read_line()`。
- **D-07:** 签名：`pub fn handle_init_interactive(output: &str, force: bool) -> Result<()>`；核心逻辑提取为 `run_wizard(reader: impl BufRead, writer: impl Write) -> Result<WizardAnswers>`。
- **D-08:** 对 `CONFIG_TEMPLATE_EN` 做字符串替换生成最终内容，保留所有注释行。
- **D-09:** 替换策略：inputs 字段、csv/sqlite 路径字段、csv/sqlite 段的注释/激活切换。
- **D-10:** 生成后调用现有 `fs::write` 写入（复用目录创建 + 错误处理逻辑）。
- **D-11:** 测试用 `std::io::Cursor::new(b"sqllogs\ncsv\noutputs/test.csv\n")` 模拟 stdin。
- **D-12:** 测试覆盖：csv 默认、自定义 csv、sqlite、无效格式重试、空 inputs 使用默认。
- **D-13:** `writer` 在测试中传 `Vec<u8>` 丢弃提示输出，仅验证解析结果。

### Claude's Discretion

- 导出格式验证：无效输入最多 3 次后返回 `Err(Error::Config(ConfigError::InvalidValue {...}))`。
- sqlite 激活方式：`[exporter.csv]` 整段每行加 `# ` 注释掉；`[exporter.sqlite]` 段去掉 `# ` 激活。
- 向导完成时打印与非交互式 init 一致的 "Next steps" 格式，通过 `log::info!` 输出。

### Deferred Ideas (OUT OF SCOPE)

无——discussion 范围严格限定于本 phase。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INIT-01 | 用户可通过 `sqllog2db init --interactive` 启动对话式向导 | D-01/D-03：clap bool flag + dispatch 分支 |
| INIT-02 | 向导逐字段引导（输入路径、导出格式、输出路径），每步给出示例和默认值；Enter 接受默认，不因空输入崩溃 | D-04/D-06/D-07：step 设计 + read_line + WizardAnswers |
| INIT-03 | 向导生成的 config.toml 格式与非交互式 init 完全一致（含注释） | D-08/D-09/D-10：CONFIG_TEMPLATE_EN 字符串替换 |
</phase_requirements>

---

## Summary

Phase 68 在已有 `handle_init` 基础上叠加一个交互式向导分支，不引入新依赖，不修改配置文件格式。实现分三层：(1) clap 层添加 `--interactive` bool flag；(2) dispatch 层在 `main.rs` 增加分支；(3) 核心向导逻辑 `run_wizard(reader, writer)` 在 `src/cli/init.rs` 内实现，IO 可注入方便测试。

输出一致性由字符串替换策略保证：向导用用户输入的值替换 `CONFIG_TEMPLATE_EN` 中的具体默认值，其余内容（注释、其他字段）原样保留。因此无需维护两套模板，INIT-03 天然满足。

测试路径无需 `#[cfg(test)]` 特殊处理——`run_wizard` 接受 `impl BufRead + impl Write`，直接用 `Cursor` 和 `Vec<u8>` 做单元测试；`handle_init_interactive` 用 assert_cmd 做端到端 CLI 测试。

**Primary recommendation:** 将全部向导逻辑集中在 `src/cli/init.rs`（无需新建子模块），文件写入复用 `handle_init` 中已提取的私有函数。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `--interactive` flag 解析 | CLI (clap) | — | opts.rs 管理所有 flag 定义，clap 负责解析 |
| Dispatch 分支 | CLI (main.rs) | — | main.rs 是所有子命令的调度中心 |
| 向导交互逻辑 | CLI (init.rs) | — | 与现有 handle_init 同层，同文件 |
| 字符串替换/生成配置内容 | CLI (init.rs) | — | CONFIG_TEMPLATE_EN 常量在 init.rs，替换逻辑就近放置 |
| 文件写入 | CLI (init.rs) | 错误处理(error.rs) | 复用现有 handle_init 路径 + 现有 FileError 类型 |

---

## Standard Stack

### Core（无新依赖）

| 组件 | 来源 | 用途 |
|------|------|------|
| `std::io::{BufRead, Write, stdin, stdout}` | Rust std | stdin 读取、stdout flush、泛型 IO 参数 |
| `std::io::Cursor` | Rust std | 测试中注入 mock stdin |
| `crate::error::{Error, ConfigError, Result}` | 项目内 | 错误传播，格式验证失败返回 ConfigError::InvalidValue |
| `crate::error::{FileError}` | 项目内 | stdin 读取失败 + 文件已存在错误 |
| `log::{info, error, warn, debug}` | log crate（已有） | 系统消息输出（非提示文本） |

所有组件均已在项目中存在，无需 `Cargo.toml` 修改。[VERIFIED: 项目现有 Cargo.toml + src/cli/init.rs 已使用相同 import 模式]

---

## Package Legitimacy Audit

> 本 Phase 不安装任何新 crate（D-06 锁定决策）。

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| (无新包) | — | — | — | — | — | N/A |

**Packages removed:** none  
**Packages flagged:** none

---

## Architecture Patterns

### System Architecture Diagram

```
用户键盘输入
    ↓ stdin.read_line()
run_wizard(reader: impl BufRead, writer: impl Write)
    ↓ 3–4 步交互
WizardAnswers { inputs, exporter, csv_file, sqlite_db, sqlite_table }
    ↓ apply_wizard_answers_to_template(&answers) → String
handle_init_interactive(output: &str, force: bool)
    ↓ 复用 write_config_file(path, content, force)  ← 从 handle_init 提取
Output: config.toml（格式与非交互式 init 完全相同）
```

### Recommended Project Structure

```
src/cli/
├── init.rs          ← 新增 handle_init_interactive + run_wizard + WizardAnswers + apply_wizard_answers_to_template
│                       现有 handle_init 提取 write_config_file 私有函数
├── opts.rs          ← Commands::Init 新增 interactive: bool
└── mod.rs           ← 无需修改（init 模块已注册）

main.rs              ← Commands::Init match arm 新增 interactive 分支
```

**注意：不新建 `wizard.rs` 子文件**——CONTEXT.md 提到 "或新建 src/cli/init/wizard.rs"，但 D-07 的函数签名只要求提取为 `run_wizard`，放在同一文件更符合项目"保持简单"的原则，且现有 `init.rs` 文件体量小（153 行），加入向导逻辑后不会超过合理复杂度。[ASSUMED]

### Pattern 1: clap bool flag 添加模式

`Commands::Init` 已有 `force: bool` 的完整注解模式，新增 `interactive: bool` 完全对称：

```rust
// Source: src/cli/opts.rs 现有 force flag 模式
#[arg(short = 'i', long = "interactive")]
interactive: bool,
```

clap 自动处理 `--interactive` 和 `-i`，与 `-o`/`-f` 无冲突。[VERIFIED: 项目 opts.rs 现有 bool flag 模式]

### Pattern 2: 泛型 IO 向导函数

```rust
// Source: CONTEXT.md D-07 + D-11/D-13 测试策略
pub fn run_wizard(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<WizardAnswers> {
    // 生产路径：reader = stdin().lock(), writer = stdout()
    // 测试路径：reader = Cursor::new(b"...\n"), writer = Vec<u8>
}
```

`&mut impl Trait` 而非 `impl Trait` 是正确用法：`BufRead::read_line` 和 `Write::write_all` 需要可变引用。[VERIFIED: Rust std BufRead trait]

### Pattern 3: print! + flush 确保提示可见

```rust
// Source: CONTEXT.md D-06 + UI-SPEC.md Prompt Format Contract
use std::io::Write; // 引入 flush trait
write!(writer, "SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: ")?;
writer.flush()?;
let mut buf = String::new();
reader.read_line(&mut buf)?;
let value = buf.trim().to_owned();
```

注意：`write!` 宏（非 `print!`）在泛型 writer 上使用；在生产路径包装时可用 `print!`。[VERIFIED: Rust std Write trait]

### Pattern 4: CONFIG_TEMPLATE_EN 字符串替换策略

模板中精确的可替换字符串（来源：`src/cli/init.rs` 第 63-152 行）：

| 替换目标 | 模板中精确字符串 | 替换方式 |
|----------|----------------|---------|
| inputs 路径 | `inputs = ["sqllogs"]` | `.replace(r#"inputs = ["sqllogs"]"#, &format!(r#"inputs = ["{user_inputs}"]"#))` |
| CSV file | `file = "outputs/sqllog.csv"` | `.replace(...)` |
| SQLite 模式激活 | `# [exporter.sqlite]`（+ 后续注释行前缀 `# `）| 多行替换 |
| CSV 段禁用 | `[exporter.csv]\n# CSV output file path\nfile = ...` | 每行加 `# ` 前缀 |

**CSV → SQLite 模式的两步替换：**

1. 将 `[exporter.csv]` 整段注释掉（逐行加 `# `）
2. 将 `# [exporter.sqlite]` → `[exporter.sqlite]`，并去掉后续字段行的 `# ` 前缀

实现时需精确匹配模板格式，避免误替换。建议定义常量或精确搜索字符串，而非正则。[VERIFIED: 直接读取 src/cli/init.rs CONFIG_TEMPLATE_EN]

**实际模板中关键的精确字符串（逐字验证）：**

```
inputs = ["sqllogs"]                      # Step 1 替换目标
file = "outputs/sqllog.csv"              # Step 3a 替换目标（注意 [logging] 也有 file = 行，需区分）
# [exporter.sqlite]                       # SQLite 段激活（去掉 # ）
# database_url = "export/sqllog2db.db"   # SQLite db 路径
# table_name = "sqllog_records"          # SQLite 表名
```

**关键风险：** `[logging]` 段也有 `file = "logs/sqllog2db.log"` 行，与 CSV 的 `file = "outputs/sqllog.csv"` 相似，替换时必须包含足够上下文（如 `file = "outputs/sqllog.csv"` 字符串本身已唯一，不会冲突）。[VERIFIED: 直接读取模板内容确认唯一性]

### Pattern 5: 文件写入复用

`handle_init` 中的文件写入逻辑（第 7-58 行）可提取为：

```rust
// 私有辅助函数，handle_init 和 handle_init_interactive 共用
fn write_config_file(path: &Path, content: &str, force: bool) -> Result<()> {
    // 文件存在检查、目录创建、fs::write、info! 日志输出
}
```

这样两个 public 函数都在 40 行内，符合项目 CLAUDE.md 要求。[VERIFIED: 项目 CLAUDE.md 40 行限制规则]

### Anti-Patterns to Avoid

- **不使用 `println!` 输出提示**：提示文本必须通过 `print!`/`write!` 确保同行显示，`println!` 加换行符会把光标移到下一行。
- **不对 inputs 路径做存在性验证**：向导接受任意非空字符串，与非交互式 init 一致（UI-SPEC.md Step 1 行为规则）。
- **不无限循环导出格式验证**：最多 3 次，第 3 次失败返回 `Err`（CONTEXT.md Discretion）。
- **不在 opts.rs 的 `-i` short flag 上与 `--input`（run 子命令）冲突**：`-i` 在 `Commands::Init` variant 内，`run` 的 `--input` 在 `Commands::Run` variant 内，clap subcommand 隔离无冲突。[VERIFIED: opts.rs 现有结构确认]
- **不新建多余模块**：`wizard.rs` 独立文件在当前代码量下会过度拆分。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| stdin 交互 TUI | 自制 ANSI 颜色/光标控制 | 纯 `print!` + `read_line` | D-06 无新依赖；项目简单场景不需要 dialoguer |
| 输入验证循环 | 复杂状态机 | 简单 for 循环最多 3 次 + `?` 返回 Err | 验证规则只有一条（csv/sqlite），无需 OOP |
| 模板引擎 | 手写占位符替换框架 | 直接 `str::replace` on 精确字符串 | 替换点固定（5 处），无条件表达式需求 |

---

## Common Pitfalls

### Pitfall 1: `[logging].file` 与 `[exporter.csv].file` 行冲突
**What goes wrong:** 两处都有 `file = "..."`，naive replace 会误改日志路径。
**Why it happens:** CONFIG_TEMPLATE_EN 中 `file = "logs/sqllog2db.log"` 和 `file = "outputs/sqllog.csv"` 共享 `file = ` 前缀。
**How to avoid:** 替换时使用完整字符串 `file = "outputs/sqllog.csv"` 作为搜索键（唯一），而非仅 `file = `。
**Warning signs:** `sqllog2db validate` 报 logging.file 路径异常。

### Pitfall 2: `write!` 宏需要 `use std::io::Write`
**What goes wrong:** 泛型 writer 调用 `.write_all()` 或 `write!()` 时编译错误 "the trait `std::io::Write` is not in scope"。
**Why it happens:** `Write` trait 方法需要 trait 在 scope 内。
**How to avoid:** 在函数体顶部或文件顶部 `use std::io::Write;`（但注意与 `std::fmt::Write` 的命名冲突，集成测试已有 `use std::fmt::Write as _` 示例）。
**Warning signs:** 编译时 "method not found in `impl Write`"。

### Pitfall 3: `read_line` 保留末尾 `\n`
**What goes wrong:** 用户输入 "csv" 后 `buf` 为 `"csv\n"`，直接比较 `buf == "csv"` 失败。
**Why it happens:** `read_line` 将换行符包含在缓冲区中。
**How to avoid:** `.trim()` 后再比较：`buf.trim() == "csv"`。CONTEXT.md D-06 已提及 `.trim()`。
**Warning signs:** 默认值不触发（空输入变成 `"\n"`，trim 后为 `""`，应判空）。

### Pitfall 4: SQLite 模式下激活段的精确格式
**What goes wrong:** 去掉 `# ` 前缀时，若模板换行格式不完全一致（如末尾空格），替换后文件无法被 toml 解析。
**Why it happens:** 字符串替换依赖精确匹配。
**How to avoid:** 在实现前用 `grep -n` 确认模板中各行的精确格式，写单元测试验证替换结果可被 `Config::from_str()` 解析。
**Warning signs:** `sqllog2db validate` 报 TOML 解析失败。

### Pitfall 5: `handle_init` 40 行限制
**What goes wrong:** 将 `handle_init_interactive` 和 `write_config_file` 逻辑全堆在 `handle_init` 中，导致函数超 40 行，违反 CLAUDE.md。
**Why it happens:** 代码共享路径导致函数膨胀。
**How to avoid:** 先提取 `write_config_file(path, content, force)` 私有函数，`handle_init` 和 `handle_init_interactive` 分别调用。
**Warning signs:** `cargo clippy` 不检查行数，需代码审查时手动验证。

---

## Code Examples

### 完整 run_wizard 函数骨架

```rust
// Source: CONTEXT.md D-07/D-11/D-13 + UI-SPEC.md Data Model Contract
use std::io::{BufRead, Write};

#[derive(Debug)]
pub enum ExporterChoice { Csv, Sqlite }

#[derive(Debug)]
pub struct WizardAnswers {
    pub inputs: String,
    pub exporter: ExporterChoice,
    pub csv_file: Option<String>,
    pub sqlite_db: Option<String>,
    pub sqlite_table: Option<String>,
}

pub fn run_wizard(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::error::Result<WizardAnswers> {
    // Step 1
    write!(writer, "SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: ")?;
    writer.flush()?;
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let inputs = if buf.trim().is_empty() { "sqllogs".to_owned() } else { buf.trim().to_owned() };

    // Step 2 — 最多 3 次
    // ...

    Ok(WizardAnswers { inputs, exporter: ExporterChoice::Csv, csv_file: None, sqlite_db: None, sqlite_table: None })
}
```

### 测试：全默认值路径

```rust
// Source: CONTEXT.md D-11/D-12/D-13
#[test]
fn test_wizard_all_defaults() {
    let input = b"\n\n\n";  // Enter × 3
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let answers = run_wizard(&mut reader, &mut writer).unwrap();
    assert_eq!(answers.inputs, "sqllogs");
    assert!(matches!(answers.exporter, ExporterChoice::Csv));
    assert_eq!(answers.csv_file.as_deref(), Some("outputs/sqllog.csv"));
}
```

### 测试：无效格式三次失败

```rust
// Source: CONTEXT.md D-12 + UI-SPEC.md Step 2 行为规则
#[test]
fn test_wizard_invalid_format_three_times_returns_err() {
    let input = b"\nbad\nbad\nbad\n";
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let result = run_wizard(&mut reader, &mut writer);
    assert!(result.is_err());
}
```

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (内置) + assert_cmd (已有，tests/integration.rs) |
| Config file | none（内置） |
| Quick run command | `cargo test wizard` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INIT-01 | `init --interactive` flag 可解析并路由到向导 | e2e CLI | `cargo test test_cli_init_interactive` | ❌ Wave 0 |
| INIT-02 | 每步提示默认值，Enter 接受；不因空输入崩溃 | unit | `cargo test test_wizard_all_defaults` | ❌ Wave 0 |
| INIT-02 | 无效导出格式最多 3 次，第 4 次返回 Err | unit | `cargo test test_wizard_invalid_format_three_times_returns_err` | ❌ Wave 0 |
| INIT-02 | sqlite 路径询问 db + table_name 两步 | unit | `cargo test test_wizard_sqlite_path` | ❌ Wave 0 |
| INIT-03 | 生成内容可通过 Config::validate() | unit | `cargo test test_wizard_output_validates` | ❌ Wave 0 |
| INIT-03 | 生成文件与非交互式 init 包含相同注释 | e2e CLI | `cargo test test_cli_init_interactive_format_matches` | ❌ Wave 0 |

### Wave 0 Gaps

- [ ] `src/cli/init.rs` 中的 `run_wizard`、`WizardAnswers`、`ExporterChoice`、`apply_wizard_answers_to_template` — 覆盖 INIT-02/INIT-03
- [ ] `tests/integration.rs` 中 interactive CLI 测试 — 覆盖 INIT-01/INIT-03
- 无需新建测试文件，向导单元测试放在 `src/cli/init.rs` 的 `#[cfg(test)]` 块；e2e 测试追加到现有 `tests/integration.rs`

### Sampling Rate

- **Per task commit:** `cargo test wizard && cargo test init`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`

---

## Security Domain

> `security_enforcement` 未显式设置 false，检查适用性。

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | 无认证逻辑 |
| V3 Session Management | no | 无会话 |
| V4 Access Control | no | 本地 CLI |
| V5 Input Validation | yes（有限）| 导出格式枚举验证（csv/sqlite），路径不验证 |
| V6 Cryptography | no | 无加密 |

**Threat patterns 说明：** 向导接受用户输入的路径和格式，路径不做存在性验证（INIT-02 设计：接受任意字符串）。生成的配置由后续 `sqllog2db validate` 验证。格式输入（csv/sqlite）白名单枚举验证，避免意外注入（虽然仅是字符串替换，枚举验证仍是最佳实践）。

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dialoguer` crate（交互式 CLI 库）| 纯 `std::io::stdin().read_line()` | D-06 锁定 | 零新依赖，可测试性由泛型 IO 参数保证 |
| 独立 wizard.rs 子文件 | 同文件（init.rs 内）| CONTEXT.md + 简洁原则 | 减少模块跳跃，代码组织更集中 |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 将向导逻辑放在 `init.rs` 同文件（不新建 wizard.rs）更简洁 | Architecture Patterns | 如果 init.rs 体量超出预期需拆分，但拆分不影响接口 |

---

## Open Questions

1. **`apply_wizard_answers_to_template` 中 CSV → SQLite 段切换的精确替换字符串**
   - What we know: 模板第 143-151 行是注释掉的 sqlite 段，每行以 `# ` 开头
   - What's unclear: 行与行之间是否有空行会影响多行替换逻辑
   - Recommendation: 执行者在实现前用 `src/cli/init.rs` 实际内容写测试确认替换后 TOML 可解析

2. **`io::Error` wrapping：`read_line` 失败应包装为哪个 Error 变体**
   - What we know: `Error::Io(io::Error)` 和 `Error::File(FileError::WriteFailed)` 均可用
   - What's unclear: stdin 读取失败更语义上接近 Io 还是 File
   - Recommendation: 使用 `Error::Io` via `?` 自动转换（`From<io::Error> for Error` 已实现），最简洁

---

## Environment Availability

> 本 Phase 为纯代码修改，无外部工具依赖。

Step 2.6: SKIPPED — 无外部依赖，仅修改 `src/` 内 Rust 源码和 `tests/integration.rs`。

---

## Project Constraints (from CLAUDE.md)

| 约束 | 来源 | 影响 |
|------|------|------|
| 函数体不超过 40 行 | CLAUDE.md | `handle_init_interactive` 需提取 `write_config_file` 和 `run_wizard`，否则超限 |
| 描述性变量名，不用单字母 | CLAUDE.md | `r` / `w` 等短参数名禁止；用 `reader` / `writer` |
| `cargo clippy --all-targets -- -D warnings` 必须通过 | CLAUDE.md | 需在每次提交前运行 |
| `cargo fmt` 格式化 | CLAUDE.md | 代码提交前必须 fmt |
| `cargo test` 全量通过 | CLAUDE.md | 新测试和既有测试均不能回归 |

---

## Sources

### Primary (HIGH confidence)

- `src/cli/init.rs` — CONFIG_TEMPLATE_EN 完整内容，handle_init 实现，可替换字符串精确验证
- `src/cli/opts.rs` — Commands::Init 现有 bool flag 模式（force），`-i` short flag 无冲突确认
- `src/main.rs` — dispatch 模式，Commands::Init match arm 修改点
- `src/error.rs` — ConfigError::InvalidValue、Error::Io From<io::Error> 自动转换确认
- `.planning/phases/68-init-wizard/68-CONTEXT.md` — 所有锁定决策 D-01 到 D-13
- `.planning/phases/68-init-wizard/68-UI-SPEC.md` — 精确提示字符串、完成输出格式、数据模型契约

### Secondary (MEDIUM confidence)

- `tests/integration.rs` — 现有 handle_init 测试模式，assert_cmd 端到端测试写法参考

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 无新依赖，全部使用已验证的项目内组件
- Architecture: HIGH — 决策全部锁定，代码变更点精确（4 处：opts.rs / main.rs / init.rs / tests）
- Pitfalls: HIGH — 从模板精确内容和项目已有代码中直接派生，无假设

**Research date:** 2026-06-06
**Valid until:** 2026-07-06（模板字符串稳定，30 天内有效）
