# Phase 20: 测试覆盖深化 - Research

**Researched:** 2026-05-18
**Domain:** Rust 测试（proptest 属性测试、集成测试边界条件、VERIFICATION.md 文档补全）
**Confidence:** HIGH

## Summary

Phase 20 是纯测试与文档补全工作，不改动任何生产代码逻辑。工作分四块：

1. **VERIFICATION.md 补写（TEST-01，D-01 扩展为 Phase 12-18 全部七个）**：参照 Phase 19 VERIFICATION.md 格式，逐条验证各阶段 Goal 与 Success Criteria。各阶段目标来源于 `.planning/milestones/v1.3-ROADMAP.md`（Phase 12-16）和 `.planning/ROADMAP.md`（Phase 17-18）。

2. **端到端集成测试（TEST-02）**：在 `tests/integration.rs` 中新增三条测试，复用已有 `write_test_log()` + `make_run_config()` + `handle_run()` 模式，覆盖过滤器流水线、模板归一化、字段投影三条功能路径。

3. **边界条件测试（TEST-03）**：新增四条测试在 `tests/integration.rs`，覆盖空 log 文件、全部记录被过滤、格式错误行跳过、超长 SQL 字段四个场景。

4. **proptest 属性测试（TEST-04）**：在 `src/pipeline/fingerprint.rs` 的 `#[cfg(test)] mod tests` 中添加 `proptest` 依赖和两条属性测试，验证 `normalize_template` 的幂等性与字面量保护不变性。

**Primary recommendation:** proptest 添加为 `[dev-dependencies]`，版本锁定为 `1.6.0`（crates.io 最新稳定版）[VERIFIED: crates.io]。

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** VERIFICATION.md 补写范围 = Phase 12/13/14/15/16/17/18（七个阶段，TEST-01 原文 4 个 → 扩展为 7 个）
- **D-02:** 各 VERIFICATION.md 写入各阶段原目录（Phase 12-16 → `.planning/milestones/v1.3-phases/{nn}-*/`，Phase 17-18 → `.planning/phases/17-*/` 和 `.planning/phases/18-*/`）
- **D-03:** VERIFICATION.md 格式：UAT 标准 + 成功标准逐条验证 + 实际验证方法，参照 Phase 19 VERIFICATION.md 格式
- **D-04:** 端到端测试使用程序生成 log（延续 `write_test_log()` 模式），不建立 `tests/fixtures/` 目录
- **D-05:** 端到端测试验证 CSV 输出格式
- **D-06:** 三条端到端测试：(1) 带 include/exclude 过滤器、(2) 模板归一化（`template_key` 列非空）、(3) 字段投影（`ordered_fields` 控制 header/列顺序）
- **D-07:** 四个边界条件测试：空 log 文件、全部记录被过滤、格式错误行被跳过、超长 SQL（>1MB）字段
- **D-08:** 边界测试放 `tests/integration.rs`
- **D-09:** proptest 策略：`any::<String>()` 任意 ASCII 字符串
- **D-10:** proptest 仅覆盖 `normalize_template`
- **D-11:** proptest 测试放 `src/pipeline/fingerprint.rs` 的 `#[cfg(test)] mod tests`
- **D-12:** 两条属性测试：幂等性 + 字面量保护不变性

### Claude's Discretion

- TEST-03 各边界 case 的具体测试函数命名、arrange/act/assert 结构（按现有 integration test 风格）
- VERIFICATION.md 中各阶段实际运行命令的具体写法
- proptest `#[proptest]` 宏的参数（cases 数量等）——使用 proptest 默认即可

### Deferred Ideas (OUT OF SCOPE)

- fingerprint() 的属性测试（输出不含数字字面量等不变量）
- SQLite 输出的端到端验证
- `cargo llvm-cov` 覆盖率门控

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-01 | Phase 12/13/14/16 各补全 VERIFICATION.md，覆盖 UAT 标准与成功标准 | D-01 扩展范围至 12-18；Phase 19 VERIFICATION.md 格式已确认（observable truths + required artifacts + behavioral checks） |
| TEST-02 | 至少一条端到端集成测试：读取 fixture .log → 运行完整 pipeline → 验证 CSV 输出正确 | 三条端到端测试；`handle_run()` 签名确认（10 参数）；`make_run_config()` + `FiltersFeature` + `TemplateConfig` + `OutputConfig` 配置构造已确认 |
| TEST-03 | 边界条件覆盖：空 log 文件、全部过滤为空、格式错误行跳过并计入 error log、超长 SQL 字段 | 四个边界 case；processor.rs 的 error 处理（`Err(e)` 分支 `errors_in_file += 1`，不 panic）已确认 |
| TEST-04 | normalize_template 有 proptest 属性测试（幂等性 + 字面量保护不变性） | proptest 1.6.0 [VERIFIED: crates.io]；`normalize_template` 签名 `pub fn normalize_template(sql: &str) -> String` 已确认 |

</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| VERIFICATION.md 补写 | `.planning/` 文档层 | — | 纯文档，不涉及代码 |
| 端到端集成测试 | `tests/integration.rs` | `src/cli/run/` | `handle_run` 是完整 pipeline 入口，integration test 调用它并验证文件输出 |
| 边界条件测试 | `tests/integration.rs` | `src/cli/run/processor.rs` | 边界行为（error 跳过、空文件）在 processor.rs 中实现，测试通过 integration test 层触发 |
| proptest 属性测试 | `src/pipeline/fingerprint.rs` | — | `normalize_template` 定义在此，测试与实现同文件 |

## Standard Stack

### Core（新增 dev-dependency）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| proptest | 1.6.0 | 属性测试：任意字符串生成 + 自动缩减（shrinking） | Rust 生态标准属性测试库 [VERIFIED: crates.io] |

### 已有 dev-dependencies（无需新增）

| Library | Version | Purpose |
|---------|---------|---------|
| tempfile | 3.27.0 | 临时目录/文件管理，所有集成测试复用 |
| criterion | 0.7 | bench（不用于本 Phase） |

**Installation:**
```toml
# Cargo.toml [dev-dependencies] 新增：
proptest = "1.6.0"
```

## Package Legitimacy Audit

> slopcheck 在此环境不可用，所有新增包标记为 `[ASSUMED]`，planner 需在实际安装前确认。

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| proptest | crates.io | ~8 yrs | 125M+ total [VERIFIED: crates.io] | github.com/proptest-rs/proptest | N/A (slopcheck unavailable) | [ASSUMED] — 需 `cargo add proptest --dev` 后通过 `cargo test` 验证编译 |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck 在本次研究中不可用；proptest 为 crates.io 知名属性测试库（1.25 亿总下载量），但 planner 应在 Wave 0 任务中通过 `cargo add proptest@1.6.0 --dev && cargo build --tests` 确认可编译。*

## Architecture Patterns

### System Architecture Diagram

```
VERIFICATION.md 工作流：
  v1.3-ROADMAP.md (Success Criteria) ──→ 手写 VERIFICATION.md ──→ 各阶段目录

测试数据流（端到端 + 边界测试）：
  write_test_log() ──→ TempDir/xxx.log
                           │
                    make_run_config() + 追加 filter/template/output 配置
                           │
                    handle_run(&cfg, None, false, true, ...)
                           │
               ┌───────────┴───────────────────────┐
          CSV 输出文件                   errors_in_file 计数 (processor.rs)
               │
       read_to_string → lines().count() / csv_headers 断言

proptest 数据流：
  any::<String>() ──→ normalize_template(s) ──→ once
                  └──→ normalize_template(once) ──→ twice
                                prop_assert_eq!(once, twice)
```

### Recommended Project Structure

```
tests/
└── integration.rs          # 新增 ~10 个测试函数（端到端 + 边界，追加到现有文件末尾）

src/pipeline/
└── fingerprint.rs           # #[cfg(test)] mod tests 末尾追加 proptest 两条

.planning/
├── milestones/v1.3-phases/
│   ├── 12-sql/12-VERIFICATION.md        # 新建
│   ├── 13-templateaggregator/13-VERIFICATION.md  # 新建
│   ├── 14-exporter/14-VERIFICATION.md   # 新建
│   ├── 15-svg/                          # 已有 15-VERIFICATION.md，可补充 Wave 2/3
│   └── 16-remaining-charts/16-VERIFICATION.md  # 新建
└── phases/
    ├── 17-filter-nesting/17-VERIFICATION.md     # 新建
    └── 18-template-chart-nesting/18-VERIFICATION.md  # 新建
```

### Pattern 1: proptest 属性测试写法

**What:** 使用 `proptest!` 宏定义属性测试，框架自动生成随机输入并在失败时缩减最小 case。

**When to use:** 验证纯函数的不变量（幂等性、单调性等）。

**Example:**
```rust
// Source: proptest 官方文档 https://proptest-rs.github.io/proptest/proptest/getting-started.html
// [VERIFIED: crates.io + ASSUMED docs pattern]
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_normalize_template_is_idempotent(s in any::<String>()) {
        let once = normalize_template(&s);
        let twice = normalize_template(&once);
        prop_assert_eq!(once, twice);
    }
}
```

### Pattern 2: 端到端集成测试写法（过滤器路径）

**What:** 构造带过滤器的 Config，调用 `handle_run`，读取 CSV 验证行数和字段值。

**Example:**
```rust
// Source: 现有 tests/integration.rs 模式（直接观察）[VERIFIED: codebase]
#[test]
fn test_e2e_filter_pipeline() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 10);   // 10 条，user=TESTUSER

    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    // 配置 include.users = ["TESTUSER"]
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["TESTUSER".to_string()]),
            ..Default::default()
        },
        ..Default::default()
    });

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1, None).unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    let lines: Vec<_> = content.lines().collect();
    // header + 10 data rows（全部 TESTUSER）
    assert_eq!(lines.len(), 11);
}
```

### Pattern 3: 边界条件测试——空 log 文件

```rust
// Source: 现有 test_handle_run_dry_run_empty_dir 模式（直接观察）[VERIFIED: codebase]
#[test]
fn test_boundary_empty_log_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // 创建 0 字节 .log 文件
    std::fs::write(log_dir.join("empty.log"), b"").unwrap();

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1, None).unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    // 只有 header 行（1 行）
    assert_eq!(content.lines().count(), 1);
}
```

### Pattern 4: 字面量保护不变性（proptest 第二条测试）

**What:** 验证 `normalize_template` 对包含 `'-- not comment'` 字样的字符串不会错误去除字面量内注释。

**Invariant 表达方式（推荐）:** 若原字符串中存在完整的单引号包围字面量，其内容中的 `--` 序列不能消失于 normalized 结果中。具体可通过构造固定前缀的策略或在 `prop_assert` 中验证引号包围区域保持完整。

```rust
// [ASSUMED] — 具体策略由实现者根据 normalize_template 语义确定
proptest! {
    #[test]
    fn prop_string_literal_protects_comment_marker(
        prefix in "([A-Za-z0-9 ]{0,20})",
        inner in "([A-Za-z0-9 ]{0,50})"
    ) {
        // 构造 WHERE col = '<inner>-- not a comment'
        let sql = format!("WHERE col = '{inner}-- not a comment{suffix}'", suffix = prefix);
        let result = normalize_template(&sql);
        // 字面量内的 -- 序列应保留
        prop_assert!(
            result.contains("-- not a comment"),
            "literal comment marker should survive in: {result}"
        );
    }
}
```

### Anti-Patterns to Avoid

- **在热路径测试文件中引入 proptest**：`proptest!` 只放在 `#[cfg(test)]` mod 内，编译时通过 `dev-dependencies` 引入，不影响生产二进制大小。
- **用 `#[should_panic]` 断言错误路径**：项目惯例使用 `assert!(result.is_err())`，保持一致。
- **在 proptest 中调用 `handle_run`**：属性测试只针对纯函数 `normalize_template`，不涉及 I/O。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 随机输入生成 + 最小化缩减 | 手写循环 + 随机种子 | `proptest` | 自动缩减到最小失败 case，诊断友好 |
| 临时目录管理 | 手写 `mkdir`/`rm -rf` | `tempfile::TempDir` | 已在 dev-dependencies，RAII 自动清理 |

**Key insight:** proptest 的价值在于缩减（shrinking）——发现 panic 时自动找到最短触发字符串，极大降低调试成本。

## Common Pitfalls

### Pitfall 1: proptest 与 `#[test]` 混用导致属性测试不运行

**What goes wrong:** 在 `proptest!` 宏外部多加了 `#[test]`，或少了 `proptest!` 宏包裹。
**Why it happens:** proptest 宏自动展开为带 `#[test]` 的函数，手动加 `#[test]` 会冲突或产生 unused attribute 警告。
**How to avoid:** 只用 `proptest! { #[test] fn ... }` 写法，不在外层加 `#[test]`。
**Warning signs:** `cargo clippy` 报 `unused attribute` 或测试函数运行时输入始终为空字符串。

### Pitfall 2: 端到端测试断言 CSV 行数时忽略 header

**What goes wrong:** `assert_eq!(lines.count(), N)` 少算了 header 行，实际应为 `N+1`。
**Why it happens:** CSV 输出第 1 行为字段名 header，`write_test_log(path, N)` 写入 N 条记录。
**How to avoid:** 始终用 `header + N data rows = N+1`，或跳过第一行后计数 `lines().skip(1).count() == N`。
**Warning signs:** 断言值差 1，测试随机失败。

### Pitfall 3: 格式错误行测试无法访问 errors_in_file 计数

**What goes wrong:** 期望断言 `errors_in_file == 1`，但该变量是 `processor.rs` 的局部变量，外部不可见。
**Why it happens:** 错误只写入 log（`log::warn!`）和进度条输出，不通过返回值暴露给调用方。
**How to avoid:** 通过检查 error log 文件（`cfg.logging.file`）内容或配置 error log 路径，读取文件行数来间接验证。或改为断言"正常记录被正确导出（行数 = 正常行数）"而非直接验证错误计数——参考 D-07 的表述。
**Warning signs:** 测试依赖内部变量，无法通过 public API 验证。

**补充：** processor.rs 的 `Err(e)` 分支（行 195）执行 `errors_in_file += 1` + `log::warn!`，不 panic、不中止处理。格式错误行被跳过，后续正常行继续导出。测试可验证"正常行已导出（CSV 有 N 行数据）"作为等价验证。

### Pitfall 4: 超长 SQL 字段测试不知道字段在日志行的位置

**What goes wrong:** 以为构造一个超长字符串直接写入 .log 文件即可，但日志格式不对导致解析失败而非"字段超长"。
**Why it happens:** 达梦日志格式有严格的结构：`<timestamp> (EP[0] sess:... user:...) [TAG] <SQL>. EXECTIME: ...`，SQL 文本在 `.` 之前的最后部分。
**How to avoid:** 用现有 `write_test_log` 的格式，把 `SELECT * FROM t WHERE id={i}` 替换为一个超长字符串，确保整行符合格式。

**参考 write_test_log 的日志格式（[VERIFIED: codebase]）：**
```
2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] <SQL>. EXECTIME: 13(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.
```

### Pitfall 5: VERIFICATION.md 中 Phase 15 已有文件的处理

**What goes wrong:** `.planning/milestones/v1.3-phases/15-svg/15-VERIFICATION.md` 已存在，若直接覆盖会丢失现有内容。
**Why it happens:** Phase 15 VERIFICATION.md 仅覆盖 Wave 1，Wave 2/3（SVG 渲染层）缺失。
**How to avoid:** D-01 范围包含 Phase 15，需要确认现有文件是否满足 Phase 15 的 Success Criteria，可直接补全而非重写。

## Code Examples

### handle_run 完整签名（[VERIFIED: codebase]）

```rust
// Source: src/cli/run/mod.rs:27
pub fn handle_run(
    cfg: &Config,
    limit: Option<usize>,
    dry_run: bool,
    quiet: bool,
    interrupted: &Arc<AtomicBool>,
    progress_interval: u64,
    resume: bool,
    state_file_override: Option<&str>,
    jobs: usize,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<()>
```

### TemplateConfig 配置启用模板归一化（[VERIFIED: codebase]）

```rust
// Source: src/pipeline/mod.rs:131
cfg.template = Some(TemplateConfig {
    enable: true,
    output_csv_path: String::new(),    // 不输出统计 CSV
    output_sqlite_table: String::new(), // 不输出统计表
});
```

### OutputConfig 字段投影配置（[VERIFIED: codebase]）

```rust
// Source: src/pipeline/mod.rs:181
// FIELD_NAMES: ["ts","ep","sess_id","thrd_id","username","trx_id","statement","appname",
//               "client_ip","tag","sql","exec_time_ms","row_count","exec_id","normalized_sql"]
cfg.output = Some(OutputConfig {
    fields: Some(vec!["ts".to_string(), "username".to_string(), "sql".to_string()]),
});
// 期望 CSV header: "ts,username,sql"
```

### FiltersFeature 过滤器配置（[VERIFIED: codebase]）

```rust
// Source: src/pipeline/filters/types.rs
use dm_database_sqllog2db::pipeline::filters::{ExcludeFilters, IncludeFilters};
use dm_database_sqllog2db::pipeline::FiltersFeature;

cfg.filter = Some(FiltersFeature {
    enable: true,
    include: IncludeFilters {
        users: Some(vec!["TESTUSER".to_string()]),
        ..Default::default()
    },
    exclude: ExcludeFilters::default(),
    ..Default::default()  // indicators, sql, record_sql
});
```

### template_key 列验证思路（[VERIFIED: codebase]）

```rust
// FIELD_NAMES[14] = "normalized_sql"（Phase 12/13 使用的列名）
// 启用 template.enable=true 后，每条记录的 normalized_sql 列将被填充
// 断言方式：CSV 中 header 应包含 "normalized_sql"，且数据行该列非空
let content = std::fs::read_to_string(&csv_file).unwrap();
let header = content.lines().next().unwrap();
assert!(header.contains("normalized_sql"), "header should contain normalized_sql");
let data_line = content.lines().nth(1).unwrap(); // 第一条数据行
// normalized_sql 列非空（字段值长度 > 0）
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| quickcheck（早期 Rust 属性测试） | proptest | proptest 2018 年发布后成为主流 | proptest 有更强的 shrinking，且策略可组合 |

**Deprecated/outdated:**
- 无适用条目（本 Phase 为新增测试，无迁移）

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | proptest 1.6.0 为 crates.io 当前最新稳定版 | Standard Stack | 实际最新版可能更高；`cargo add proptest --dev` 会自动选最新版，影响极低 |
| A2 | proptest `any::<String>()` 生成任意 UTF-8 字符串（含非 ASCII） | Architecture Patterns | 若只生成 ASCII，覆盖面缩小；通过 `cargo test -- prop_` 手动验证策略行为 |
| A3 | 字面量保护不变性测试可用固定格式字符串策略表达 | Code Examples | 若 proptest 正则策略不支持某写法，可改为手写有限几个 case 的单元测试 |
| A4 | Phase 15 的 15-VERIFICATION.md 只需补充而非重写 | Common Pitfalls | 若文件结构不符合 D-03 格式要求，需整体重写 |

**If this table is empty:** N/A — 上述 4 条假设均需在实现阶段核实。

## Open Questions

1. **格式错误行的 error log 文件路径如何在测试中访问？**
   - What we know: `cfg.logging.file` 默认为 `"logs/sqllog2db.log"`，是相对路径；`processor.rs` 用 `log::warn!` 输出错误，不写入专用错误文件
   - What's unclear: `handle_run` 在测试临时目录下是否会实际创建 error log？
   - Recommendation: 测试策略改为验证"正常行已正确导出（CSV 有 expected 行数）"，不直接验证 error log 文件存在性

2. **template_key 在 CSV 中的列名是 `normalized_sql` 还是 `template_key`？**
   - What we know: `FIELD_NAMES[14] = "normalized_sql"`（`src/pipeline/mod.rs`），是固定常量；CONTEXT.md D-06 说"验证 `template_key` 列存在且非空"
   - What's unclear: D-06 的"template_key"是概念名称还是实际列名
   - Recommendation: 使用 `"normalized_sql"` 作为实际列名（与 `FIELD_NAMES` 一致），D-06 的"template_key"是语义描述

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | 所有 Rust 测试 | ✓ | 1.85 (rust-version) | — |
| proptest (dev-dep) | TEST-04 | ✗ 尚未加入 Cargo.toml | — | 需在 Wave 0 任务中 `cargo add proptest@1.6.0 --dev` |
| tempfile (dev-dep) | TEST-02/03 | ✓ | 3.27.0 | — |

**Missing dependencies with no fallback:**
- proptest — 需要在 Wave 0 任务中添加，否则 TEST-04 无法编译

**Missing dependencies with fallback:**
- 无

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (内置) + proptest 1.6.0 (属性测试) |
| Config file | 无（cargo test 不需要额外配置文件） |
| Quick run command | `cargo test -- --test-thread=1 2>&1 \| tail -5` |
| Full suite command | `cargo test --all-targets` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | 各阶段 VERIFICATION.md 存在且内容正确 | manual（文档检查） | `ls .planning/milestones/v1.3-phases/*/??-VERIFICATION.md .planning/phases/17-*/17-VERIFICATION.md .planning/phases/18-*/18-VERIFICATION.md` | ❌ Wave 0 新建 |
| TEST-02 | 端到端过滤器路径 | integration | `cargo test --test integration test_e2e_filter` | ❌ Wave 0 新增 |
| TEST-02 | 端到端模板归一化路径 | integration | `cargo test --test integration test_e2e_template` | ❌ Wave 0 新增 |
| TEST-02 | 端到端字段投影路径 | integration | `cargo test --test integration test_e2e_field_projection` | ❌ Wave 0 新增 |
| TEST-03 | 边界：空 log 文件 | integration | `cargo test --test integration test_boundary_empty` | ❌ Wave 0 新增 |
| TEST-03 | 边界：全部过滤为空 | integration | `cargo test --test integration test_boundary_all_filtered` | ❌ Wave 0 新增 |
| TEST-03 | 边界：格式错误行跳过 | integration | `cargo test --test integration test_boundary_malformed` | ❌ Wave 0 新增 |
| TEST-03 | 边界：超长 SQL 字段 | integration | `cargo test --test integration test_boundary_long_sql` | ❌ Wave 0 新增 |
| TEST-04 | normalize_template 幂等性 | property | `cargo test prop_normalize_template_is_idempotent` | ❌ Wave 0 新增 |
| TEST-04 | normalize_template 字面量保护 | property | `cargo test prop_normalize_template_literal_protection` | ❌ Wave 0 新增 |

### Sampling Rate

- **Per task commit:** `cargo test 2>&1 | tail -10`
- **Per wave merge:** `cargo test --all-targets && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** 全套绿色通过后才能进入 `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `proptest` dev-dependency — `cargo add proptest@1.6.0 --dev`
- [ ] `tests/integration.rs` 末尾新增 7 条函数（3 端到端 + 4 边界）
- [ ] `src/pipeline/fingerprint.rs` `#[cfg(test)] mod tests` 末尾新增 2 条 proptest 函数
- [ ] `.planning/milestones/v1.3-phases/{12,13,14,15,16}-*/??-VERIFICATION.md` — 7 个新文件（15 视情况补充）
- [ ] `.planning/phases/{17,18}-*/??-VERIFICATION.md` — 2 个新文件

## Security Domain

> security_enforcement 未在 config.json 中显式设置为 false，按默认启用处理。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | 无用户认证逻辑 |
| V3 Session Management | no | 无 |
| V4 Access Control | no | 无 |
| V5 Input Validation | yes（边界测试） | 超长字段 + 格式错误行——测试验证不 panic，即安全边界满足 |
| V6 Cryptography | no | 无 |

### Known Threat Patterns for Rust test/proptest stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| 超长字符串导致 Vec 分配耗尽内存 | Denial of Service | normalize_template 流式处理，`Vec::with_capacity(sql.len())`——不额外扩容；proptest 默认 cases=256，字符串长度有上限 |
| proptest 生成包含特殊字节的字符串导致 UTF-8 错误 | Tampering | `scan_sql_bytes` 末尾有 `String::from_utf8(...).expect(...)` 守护；proptest `any::<String>()` 只生成有效 UTF-8 |

## Sources

### Primary (HIGH confidence)

- `src/pipeline/fingerprint.rs` — normalize_template 实现、现有单元测试 [VERIFIED: codebase]
- `src/cli/run/mod.rs` — handle_run 签名（10 参数） [VERIFIED: codebase]
- `src/cli/run/processor.rs` — 错误行处理逻辑（Err 分支 errors_in_file += 1，不 panic）[VERIFIED: codebase]
- `src/pipeline/mod.rs` — FIELD_NAMES、TemplateConfig、OutputConfig、FiltersFeature [VERIFIED: codebase]
- `tests/integration.rs` — write_test_log、make_run_config、55 条现有测试模式 [VERIFIED: codebase]
- `Cargo.toml` — 现有 dev-dependencies（tempfile 3.27.0）[VERIFIED: codebase]
- `.planning/phases/19-code-refactor/19-VERIFICATION.md` — VERIFICATION.md 标准格式 [VERIFIED: codebase]
- crates.io proptest — `1.6.0`，125M+ 总下载量 [VERIFIED: crates.io]

### Secondary (MEDIUM confidence)

- `.planning/milestones/v1.3-ROADMAP.md` — Phase 12-16 的 Goal + Success Criteria（VERIFICATION.md 写作依据）[VERIFIED: codebase]
- `.planning/ROADMAP.md` Phase 17/18 — Phase 17/18 的 Goal + Success Criteria [VERIFIED: codebase]

### Tertiary (LOW confidence)

- proptest 属性测试字面量保护不变量表达方式 — 基于 normalize_template 实现推断 [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — proptest 版本从 crates.io 直接确认
- Architecture: HIGH — 所有关键签名、文件路径、现有模式均从代码库直接读取
- Pitfalls: MEDIUM — 错误 log 可访问性问题需实现阶段验证
- VERIFICATION.md 内容: MEDIUM — 依赖对 v1.3-ROADMAP.md 中 Success Criteria 的正确理解

**Research date:** 2026-05-18
**Valid until:** 2026-06-18（代码库变动低，30 天有效期合理）
