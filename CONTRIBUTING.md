# 贡献指南

欢迎为 sqllog2db 做出贡献！本指南将帮助你搭建开发环境、了解编码规约、掌握 Pull Request（拉取请求）流程和 commit（提交）规范。

## 开发环境搭建

### 前置要求

- **Rust 工具链**：通过 [rustup](https://rustup.rs/) 安装。项目最低支持 Rust 版本（MSRV）为 1.85+。

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- **Git**：用于克隆仓库和版本控制。

### 克隆仓库

```bash
git clone https://github.com/guangl/sqllog2db
cd sqllog2db
```

### 构建与运行

```bash
# 构建（Release 模式）
cargo build --release

# 运行测试
cargo test

# 生成默认配置文件
cargo run -- init -o config.toml --force

# 验证配置
cargo run -- validate -c config.toml

# 运行导出
cargo run -- run -c config.toml
```

### 辅助工具

- **lychee**：用于检查文档中的死链（CI 中使用）。安装：`cargo install lychee`
- **rsvg-convert**：用于 SVG 渲染验证（macOS: `brew install librsvg`，Linux: `apt install librsvg2-bin`）

## 编码规约

### 命名规范

| 元素 | 命名风格 | 示例 |
|------|---------|------|
| 文件名 | `snake_case` | `replace_parameters.rs`、`sql_fingerprint.rs` |
| 结构体（Struct） | `PascalCase` | `SqllogParser`、`FieldMask` |
| 枚举（Enum） | `PascalCase` | `ExporterKind`、`ConfigError` |
| Trait | `PascalCase`（按能力命名） | `Exporter`、`LogProcessor` |
| 函数/方法 | `snake_case` | `handle_run`、`from_config` |
| 常量 | `UPPER_SNAKE_CASE` | `FIELD_NAMES`、`LOG_LEVELS` |
| 变量 | 描述性 `snake_case` | `overwrite`、`interrupted` |

### 代码风格

- **格式化**：使用 `cargo fmt`（rustfmt 默认配置），CI 中强制检查。
- **Lint**：必须通过 `cargo clippy --all-targets -- -D warnings`，零警告。
- **Import 顺序**：`std` → 外部 crate → 内部模块（`crate::`、`super::`）。
- **函数长度**：每个函数不超过 40 行。超过时拆分为更小的函数。
- **尾逗号**：多行 struct/enum 字面量使用尾逗号。

### 常用属性

- `#[inline]` — 用于热路径方法：`write_record_preparsed`、`export_one_preparsed`
- `#[must_use]` — 用于返回有意义值的纯函数：`new()`、`from_config()`
- `#[derive(Debug)]` — 所有公共类型必须实现
- `#[expect(clippy::lint_name, reason = "...")]` — 用于对特定 lint 做有理由的例外（优先于 `#[allow]`）

### 错误处理

- 使用 `thiserror` 派生宏定义类型化错误，错误变体包含路径和原因的上下文信息。
- 项目使用 `pub type Result<T> = std::result::Result<T, Error>` 统一错误类型。
- 解析错误是**非致命的**：写入错误日志文件后继续处理下一条记录。
- 使用 `?` 操作符在函数间传播错误。

### 注释

- 中文注释：用于达梦数据库相关的领域概念和性能优化说明。
- 英文文档注释（`///`）：用于公共 API（struct、trait、方法）。
- 性能相关的设计决策使用行内注释说明。

## PR 流程

1. **Fork** 本仓库，从 `main` 分支创建功能分支。
2. 在你的分支上进行修改。
3. 提交前确保通过以下所有检查：

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
cargo build --release
```

4. 如果是新功能或 bug 修复，建议在 `tests/` 目录或模块内联测试块中增加对应的测试用例。
5. 推送分支并发起 Pull Request。
6. 在 PR 描述中说明修改内容、动机和测试情况。
7. CI 将自动运行 clippy、fmt 检查、测试和文档死链检查（lychee），全部通过后方可合并。
8. 维护者审核后合并。

## Commit 规范

本项目使用 Conventional Commits（约定式提交）格式。CHANGELOG.md 遵循 Keep a Changelog 格式自动生成。

### 提交消息格式

```
<type>(<scope>): <简短描述>

<详细说明（可选）>
```

### Type（类型）

| 类型 | 用途 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档变更 |
| `refactor` | 重构（无功能变更） |
| `test` | 测试增补 |
| `chore` | 构建/工具/CI 维护 |
| `perf` | 性能优化 |

### Scope（范围）

使用受影响的模块或文件作为范围：`filters`、`exporter`、`config`、`cli`、`charts`、`pipeline`、`README` 等。

### 示例

```
feat(filters): add regex include for user field
fix(exporter): handle empty CSV output directory
docs(README): update installation instructions
refactor(config): extract validate_and_compile from from_config
test(charts): add edge case tests for empty histogram
chore(ci): add lychee link checker to CI pipeline
```

### 其他约定

- 提交消息使用英文撰写。
- 第一行（标题）不超过 72 个字符。
- 标题以动词原形开头（"add" 而非 "added"）。
- 关联 issue 时在正文中使用 `Closes #123` 或 `Refs #456`。

---

*本指南在 Phase 25 中编写，如有改进建议请通过 PR 提交。*
