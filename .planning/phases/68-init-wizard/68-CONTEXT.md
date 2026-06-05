# Phase 68: 交互式配置向导 - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

为首次使用者提供 `sqllog2db init --interactive` 对话式向导：逐字段提示（inputs 路径、导出格式、导出输出路径），每步显示示例和默认值，Enter 直接接受默认，向导完成后生成与非交互式 init 格式完全一致的 config.toml（含行内注释）。

</domain>

<decisions>
## Implementation Decisions

### `--interactive` Flag 结构

[auto] Q: "`--interactive` 应加入现有 Init variant 还是新建 subcommand？" → Selected: "加入现有 Commands::Init 为 bool flag" (recommended default)

- **D-01:** 在 `src/cli/opts.rs` 的 `Commands::Init` variant 新增 `#[arg(long = "interactive", short = 'i')] interactive: bool`。与现有 `--output` 和 `--force` flag 并列，clap 自动处理组合。
- **D-02:** `--output`（`-o`）在交互模式下控制 config 写入路径（即 config.toml 自身路径，不是导出输出路径）。向导内部只询问"配置内容"字段，不重新询问 config 写入路径。`--force` 仍然有效（写入前检查文件是否存在）。
- **D-03:** `src/cli/mod.rs` 或 `src/main.rs` 分发：若 `interactive` 为 true，调用 `handle_init_interactive(output, force)`；否则调用现有 `handle_init(output, force)`。

### 向导字段覆盖范围

[auto] Q: "向导应询问哪些字段？" → Selected: "3 个核心字段：inputs 路径、导出格式（csv/sqlite）、导出输出路径" (recommended default)

- **D-04:** 向导步骤（顺序固定）：
  1. `[sqllog] inputs` — 提示: `"SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: "`
  2. 导出格式选择 — 提示: `"导出格式 (csv/sqlite) [default: csv]: "`，仅接受 "csv" / "sqlite" / 空输入（默认 csv），其他输入重新提示
  3a. 若选 csv：`[exporter.csv] file` — 提示: `"CSV 输出文件路径 [default: outputs/sqllog.csv]: "`
  3b. 若选 sqlite：`[exporter.sqlite] database_url` — 提示: `"SQLite 数据库路径 [default: export/sqllog2db.db]: "`，再询问 `table_name` — 提示: `"表名（仅含字母/数字/下划线）[default: sqllog_records]: "`
- **D-05:** logging、filter、stats、replace_parameters 等段保持 `CONFIG_TEMPLATE_EN` 中的注释默认值，向导不询问。

### IO 实现与 stdin 读取

[auto] Q: "stdin 读取用原生 read_line 还是 dialoguer crate？" → Selected: "原生 std::io::stdin().read_line()，无新依赖" (recommended default)

- **D-06:** 不引入新 crate。每步使用 `print!()` 输出提示（需 `stdout().flush()`），`stdin.read_line(&mut buf)` 读取，`.trim()` 后判断是否空输入。
- **D-07:** 函数签名：`pub fn handle_init_interactive(output: &str, force: bool) -> Result<()>`，内部用 `std::io::stdin()` 直接读取（生产路径）。为支持测试，提取核心逻辑为 `run_wizard(reader: impl BufRead, writer: impl Write) -> Result<WizardAnswers>`，返回用户填写的字段值结构体。

### 配置文件生成方式

[auto] Q: "如何生成与非交互式 init 格式完全一致的配置文件（含注释）？" → Selected: "字符串替换 CONFIG_TEMPLATE_EN 中的默认值" (recommended default)

- **D-08:** 对 `CONFIG_TEMPLATE_EN` 做字符串替换，将模板中的具体默认值替换为用户输入的值。保留所有注释行，满足 INIT-03（格式与 `sqllog2db init -o config.toml` 完全一致）。
- **D-09:** 替换策略：
  - inputs 路径：替换 `inputs = ["sqllogs"]` → `inputs = ["{user_input}"]`
  - csv 路径：替换 `file = "outputs/sqllog.csv"` → `file = "{user_csv_path}"`，取消 sqlite 段注释
  - sqlite 模式：注释掉 `[exporter.csv]` 段，取消 `[exporter.sqlite]` 的注释并替换 `database_url` 和 `table_name`
- **D-10:** 生成后调用现有的 `fs::write(path, content)` 写入文件（复用 `handle_init` 中的目录创建 + 错误处理逻辑）。

### 测试策略

[auto] Q: "如何测试向导？" → Selected: "通过 impl BufRead + Write 参数化，单元测试注入 Cursor 模拟 stdin" (recommended default)

- **D-11:** `run_wizard(reader: impl BufRead, writer: impl Write) -> Result<WizardAnswers>` 接受可替换的 IO，单元测试用 `std::io::Cursor::new(b"sqllogs\ncsv\noutputs/test.csv\n")` 模拟用户输入全流程。
- **D-12:** 测试覆盖：csv 默认值路径（全 Enter）、自定义 csv 路径、sqlite 路径（含 table_name）、无效导出格式重新提示、空 inputs 接受默认。
- **D-13:** `writer` 参数在生产路径传入 `std::io::stdout()`，测试中传入 `Vec<u8>` 丢弃提示输出（仅验证解析结果）。

### Claude's Discretion

- 导出格式验证：输入不在 {"", "csv", "sqlite"} 时循环重新提示（最多 3 次后返回 Err），避免无限循环。
- sqlite 取消 csv 段注释的方式：将 `[exporter.csv]` 整段注释掉（每行加 `# `），激活 `[exporter.sqlite]` 段（去掉 `# `）。
- 向导结束时打印确认信息（与非交互式 init 一致的 "Next steps" 格式），由 `handle_init_interactive` 调用 `handle_init` 的写入逻辑完成后输出。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 68: 交互式配置向导" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §INIT-01、INIT-02、INIT-03

### 核心实现文件
- `src/cli/init.rs` — `handle_init(output, force)`（现有实现）、`CONFIG_TEMPLATE_EN` 常量（模板字符串，字段替换基准）
- `src/cli/opts.rs` — `Commands::Init { output, force }` variant（加 `interactive: bool` flag）
- `src/cli/mod.rs` 或 `src/main.rs` — dispatch 分支（interactive vs 非 interactive）

### 外部依赖
- 无新依赖；仅用 `std::io::{stdin, stdout, BufRead, Write}`

### 参考模式
- `.planning/phases/67-prog-diag/67-CONTEXT.md` — Phase 67 决策（ErrorStats、指示格式约定）
- `src/cli/run/mod.rs` — `handle_run()` 结构（dispatch → 具体 handler 的分发模式）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CONFIG_TEMPLATE_EN` (`src/cli/init.rs:63`) — 完整模板字符串，含所有字段默认值和注释；向导字符串替换的基准
- `handle_init(output: &str, force: bool)` (`src/cli/init.rs:7`) — 文件存在检查、目录创建、fs::write 逻辑可直接复用或提取为内部函数
- `Error::File(FileError::AlreadyExists)` / `WriteFailed` — 现有错误类型，新函数直接使用

### Established Patterns
- clap `Commands` enum variant — `Init { output, force }` 已有完整 clap 注解模式，加 bool flag 需复制注解格式
- `log::info!` / `log::warn!` — CLI 输出走 log crate（而不是 `println!`）；向导提示例外，必须用 `print!` + flush 直接写 stdout
- Result 类型：`crate::error::Result<()>`，错误用 `?` 传播

### Integration Points
- `src/main.rs` 或 `src/cli/mod.rs` — `Commands::Init { output, force, interactive }` match arm，根据 `interactive` 分发
- `src/cli/opts.rs` `Commands::Init` — 唯一需要修改 opts 的地方；clap 自动生成 `--interactive` flag 和 help 文本
- `--output` 默认值 `"config.toml"` — 在 opts.rs `default_value` 中定义，向导写入路径与此一致

</code_context>

<specifics>
## Specific Ideas

- 向导提示风格参考 STATE.md 中的 Architecture Notes：`print!` + `stdout().flush()` 确保提示在 stdin 之前可见
- `WizardAnswers` 结构体字段：`inputs: String, exporter: ExporterChoice, csv_file: Option<String>, sqlite_db: Option<String>, sqlite_table: Option<String>`
- 模板替换后最终结果必须通过 `sqllog2db validate` 验证（SC4），可在测试中直接调用 `Config::validate()` 验证生成内容

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 68-交互式配置向导*
*Context gathered: 2026-06-05*
