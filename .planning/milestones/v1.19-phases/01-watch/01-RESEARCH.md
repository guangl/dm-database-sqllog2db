# Phase 1: watch 功能完善 - Research

**Researched:** 2026-06-06
**Domain:** Rust CLI / watch 子命令修复（CSV 追加、error log 追加、退出码）
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** 所有触发路径（`trigger_full_file` 和 `build_incremental_cfg`）均设置 CSV append 模式：
```rust
if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
    csv_cfg.append = true;
    csv_cfg.overwrite = false;
}
```

**D-02:** CSV watch 不需要 offset/Seek 跟踪。`CsvExporter` 已有 `append=true` + 空文件才写 header 的 TOCTOU 安全逻辑（`src/exporter/csv/mod.rs` line 123–132），直接复用即可。

**D-03:** `WatchLoopState` 不需要新增 CSV 等价的 `csv_path` 字段；offset 管理仅 SQLite 需要。

**D-04:** 在 `Config`（`src/config/mod.rs`）中添加内部控制字段：
```rust
#[serde(skip)]
pub(crate) append_error_log: bool,
```
默认值 `false`（`Default::default()` 保持原覆盖写行为，`run` 路径不受影响）。

**D-05:** watch 触发时，在 `trigger_full_file` 和 `build_incremental_cfg` 中设置 `tmp_cfg.append_error_log = true`。

**D-06:** `write_error_log` 读取 `cfg.append_error_log`，为 true 时用 `OpenOptions::new().create(true).append(true).open(...)` 打开文件；为 false 时保持现有 `std::fs::File::create`（覆盖）行为。

**D-07:** 在 `handle_watch`（`src/cli/watch/mod.rs` line 73）的 `Ok(())` 前添加：
```rust
if interrupted.load(Ordering::Acquire) {
    return Err(Error::Interrupted);
}
```
`main.rs` 的 `Err(e) if matches!(e, Error::Interrupted)` 分支已处理（line 114–115），无需修改。

**D-08:** `print_final_summary` 仍在 interrupted 时调用（打印摘要后再返回错误），保持用户体验一致性。

### Claude's Discretion

- `build_incremental_cfg` 函数体将同时处理 SQLite 和 CSV 的 append 设置，可考虑是否提取成一个 `force_append_exporters(cfg)` 辅助函数——如果两处设置逻辑完全相同，提取可减少重复；否则保留内联即可。
- `append_error_log` 字段加入 `Config::validate()` 不做校验（它是运行时内部状态，非用户配置）。

### Deferred Ideas (OUT OF SCOPE)

- CSV watch 的 offset 跟踪（类似 SQLite 的 `_watch_offsets` 表）
- watch 支持多目录 glob 模式（WATCH-10）
- watch 远程推送 webhook（WATCH-11）
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WATCH-07 | watch 触发时将新增记录增量追加到 CSV 文件，多次触发累计记录 | `CsvExporter` 已支持 `append=true` 模式；在 `trigger_full_file` 和 `build_incremental_cfg` 中注入 `csv_cfg.append = true` |
| WATCH-08 | watch 期间产生的 parse error 追加写入 error log，不覆盖历史 | `write_error_log` 改用 `OpenOptions::append(true)`；通过 `Config.append_error_log` 字段区分 watch/run 路径 |
| WATCH-09 | 向 watch 进程发送 SIGINT 后退出码为 130 | `handle_watch` 尾部检查 `interrupted` 返回 `Err(Error::Interrupted)`；`main.rs` 已有 `exit(130)` 分支 |
</phase_requirements>

---

## Summary

本阶段是三个独立的 watch 子命令修复，互不依赖，可以任意顺序实现。

**WATCH-07（CSV 追加）**：`CsvExporter` 已通过 `append=true` 配置完整支持追加写模式，包含 TOCTOU 安全的"仅空文件写 header"逻辑。watch 路径唯一缺失的是在克隆 `Config` 时注入该标志。需要在 `trigger_full_file` 和 `build_incremental_cfg` 两处函数中，在调用 `handle_run` 前设置 `csv_cfg.append = true; csv_cfg.overwrite = false`。

**WATCH-08（error log 追加）**：`write_error_log` 目前用 `std::fs::File::create`（总是截断）。修复方式是在 `Config` 添加 `#[serde(skip)] pub(crate) append_error_log: bool` 字段，watch 触发时设为 `true`，`write_error_log` 读取该字段决定打开模式。`run` 命令路径默认 `false` 行为不变。

**WATCH-09（退出码 130）**：`main.rs` 的 `Err(Error::Interrupted) => exit(130)` 路径已存在。问题是 `handle_watch` 在 watch loop 结束后无论是否被 Ctrl+C 中断都返回 `Ok(())`，导致 `main` 不进入 130 分支。修复只需在 `handle_watch` 末尾、`Ok(())` 之前，检查 `interrupted.load(Ordering::Acquire)` 并条件返回 `Err(Error::Interrupted)`。

**Primary recommendation:** 三个修改点高度局部化，总代码改动 < 30 行，无架构变化，无新依赖。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CSV 追加写入 | `exporter/csv/mod.rs` | `cli/watch/mod.rs` | CsvExporter 控制写模式；watch 层仅注入配置 |
| error log 追加 | `cli/run/mod.rs` | `config/mod.rs` | `write_error_log` 在 run 层；Config 传递模式标志 |
| 退出码 130 | `cli/watch/mod.rs` | `main.rs` | handle_watch 产生信号；main 处理 exit code |

---

## Standard Stack

本阶段无新依赖引入，全部使用已有 crate。

### 已有 Crate 复用

| Crate | 用途 | 使用位置 |
|-------|------|----------|
| `std::fs::OpenOptions` | error log 追加模式打开文件 | `write_error_log`（当前用 `File::create`，替换） |
| `std::sync::atomic::Ordering::Acquire` | 读取 interrupted 标志 | `handle_watch` 末尾检查 |
| `serde` `#[serde(skip)]` | Config 内部字段跳过序列化 | `Config.append_error_log` |

**安装命令：** 无需安装新包。

---

## Package Legitimacy Audit

本阶段不引入新外部包，无需执行 Package Legitimacy Gate。

---

## Architecture Patterns

### 数据流图（WATCH-07）

```
文件系统事件（Create/Modify）
    ↓ handle_event
    ↓ trigger_full_file / trigger_incremental
        └─ tmp_cfg = cfg.clone()
        └─ tmp_cfg.exporter.csv.append = true   ← 新增注入
        └─ tmp_cfg.exporter.csv.overwrite = false ← 新增注入
    ↓ handle_run(&tmp_cfg, ...)
    ↓ CsvExporter::from_config → WriteMode::Append
    ↓ initialize() → OpenOptions::append(true)
        └─ file_is_empty ? write_header : skip
    ↓ 追加记录行到已有 CSV
```

### 数据流图（WATCH-08）

```
handle_run(&tmp_cfg, ...) 内部
    ↓ 解析错误 → ErrorStats.parse_error_records
    ↓ write_error_log(&cfg, &stats)
        └─ cfg.append_error_log == true ?
            OpenOptions::append(true).open(...)  ← 追加
          : File::create(...)                     ← 覆盖（run 路径）
```

### 数据流图（WATCH-09）

```
run_watch_loop 退出（interrupted == true 或 Disconnected）
    ↓ handle_watch
        ↓ pb.finish_and_clear()
        ↓ print_final_summary(...)
        ↓ if interrupted.load(Acquire) { return Err(Error::Interrupted) }  ← 新增
        ↓ Ok(())  ← 仅未中断时到达
    ↓ main.rs: Err(Error::Interrupted) → exit(130)
```

### Recommended Project Structure

不需要新增模块或文件，修改点：
```
src/
├── cli/watch/mod.rs     — trigger_full_file, build_incremental_cfg, handle_watch
├── cli/run/mod.rs       — write_error_log
└── config/mod.rs        — Config 结构体新增字段
```

### Pattern 1: watch 触发时克隆并修改 Config

**What:** watch 触发不修改原始 cfg，而是克隆一份 tmp_cfg 后注入特定覆盖（inputs、exporter flags 等）
**When to use:** 所有触发路径，包括 trigger_full_file 和 build_incremental_cfg

现有模式（`build_incremental_cfg`，line 514–523）：
```rust
// Source: src/cli/watch/mod.rs line 514-523 [VERIFIED: codebase]
fn build_incremental_cfg(cfg: &Config, tmp_file: &tempfile::NamedTempFile) -> Config {
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![tmp_file.path().to_string_lossy().into_owned()];
    // D-09: 增量路径强制 append=true、overwrite=false，避免清空表
    if let Some(ref mut sqlite_cfg) = tmp_cfg.exporter.sqlite {
        sqlite_cfg.append = true;
        sqlite_cfg.overwrite = false;
    }
    tmp_cfg
}
```

WATCH-07 需在同函数中对 CSV 做相同处理（对称模式）：
```rust
// 新增（WATCH-07）
if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
    csv_cfg.append = true;
    csv_cfg.overwrite = false;
}
```

### Pattern 2: #[serde(skip)] 内部字段

**What:** Config 中的运行时状态字段用 `#[serde(skip)]` 标注，不参与 TOML 序列化/反序列化
**When to use:** WATCH-08 新增 `append_error_log` 字段

```rust
// Source: src/config/mod.rs（新增）[VERIFIED: codebase]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    // ... 已有字段 ...
    #[serde(skip)]
    pub(crate) append_error_log: bool,
}
```

### Anti-Patterns to Avoid

- **全局修改原始 cfg：** watch 是长进程，每次触发必须克隆 cfg；修改 &mut cfg 会污染后续触发
- **在 handle_watch 开头检查 interrupted：** 应在 print_final_summary 之后，保证退出前打印摘要（D-08）
- **write_error_log 用 BufWriter::new 后忘记 flush：** 现有代码已有 `writer.flush()` 处理，追加模式不变

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSV 追加写入 | 自定义 seek+write 逻辑 | `CsvExporter` 的 `WriteMode::Append` | 已内置 TOCTOU 安全的 header 判断（line 128–132） |
| 文件追加打开 | 手动 O_APPEND syscall | `OpenOptions::new().create(true).append(true)` | std 标准模式，原子追加 |
| 退出码传递 | 在 main 新增 match 分支 | 已有 `Err(Error::Interrupted) => exit(130)` | main.rs line 114–115 已就位 |

**Key insight:** 三项修复都是"激活已有功能"而非"实现新功能"——代码路径已存在，只需在正确位置注入参数或检查标志。

---

## Common Pitfalls

### Pitfall 1: trigger_full_file 忘记注入 CSV append

**What goes wrong:** 只修改了 `build_incremental_cfg`（增量路径），忘记在 `trigger_full_file` 的 inline tmp_cfg 构造中也注入 `csv_cfg.append = true`
**Why it happens:** 全量路径（Create 事件）没有走 `build_incremental_cfg`，是在 `trigger_full_file` 函数体内直接克隆 cfg
**How to avoid:** 修改两处：(1) `trigger_full_file` 中克隆后追加注入；(2) `build_incremental_cfg` 中追加注入
**Warning signs:** 只有 Modify 事件（增量）时 CSV 追加正常，Create 事件（全量）时 CSV 被截断

### Pitfall 2: append_error_log 默认值忘记设为 false

**What goes wrong:** 如果 `Default::default()` 的 `append_error_log` 为 `true`，`run` 子命令也会追加写 error log，破坏 run 的覆盖语义
**Why it happens:** Rust `bool` 的 `Default` 是 `false`，所以只要不手动 impl Default 就安全；但若通过 derive 且字段有默认值属性时需确认
**How to avoid:** 验证 `Config::default()` 后 `append_error_log == false`；`run` 路径不设置该字段
**Warning signs:** `cargo test` 中 `write_error_log` 相关测试行为异常

### Pitfall 3: handle_watch 退出码检查位置错误

**What goes wrong:** 若在 `print_final_summary` 之前检查 interrupted 并提前返回，用户 Ctrl+C 后看不到摘要统计
**Why it happens:** 控制流顺序：loop 结束 → pb.finish → print_summary → check interrupted（应如此）
**How to avoid:** 严格按 D-08：先 `print_final_summary`，再检查 `interrupted`，再 `Ok(())`
**Warning signs:** Ctrl+C 后终端没有打印 "Watch stopped. Triggers: ..." 这行

### Pitfall 4: OpenOptions::append 与 BufWriter flush 顺序

**What goes wrong:** 追加模式下文件已有内容，`BufWriter` 缓冲区如果没有 flush，错误记录可能丢失
**Why it happens:** 现有 `write_error_log` 已在末尾 `if let Err(e) = writer.flush()` 处理；追加模式改动不影响 flush 逻辑
**How to avoid:** 追加模式只改变文件打开方式，flush 逻辑保持不变即可

---

## Code Examples

### WATCH-07: trigger_full_file 中注入 CSV append

```rust
// Source: src/cli/watch/mod.rs trigger_full_file [VERIFIED: codebase - 现有模式的扩展]
pub fn trigger_full_file(
    path: &Path,
    cfg: &Config,
    // ...
) {
    // ...
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![path.to_string_lossy().into_owned()];
    // WATCH-07: 全量触发也追加写入 CSV（新增）
    if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
        csv_cfg.append = true;
        csv_cfg.overwrite = false;
    }
    // WATCH-08: watch 触发时 error log 追加写入（新增）
    tmp_cfg.append_error_log = true;
    match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
        // ...
    }
}
```

### WATCH-08: write_error_log 追加模式

```rust
// Source: src/cli/run/mod.rs write_error_log [VERIFIED: codebase - 修改现有函数]
fn write_error_log(cfg: &crate::config::Config, stats: &ErrorStats) {
    let Some(error_cfg) = cfg.error.as_ref() else { return; };
    if stats.parse_error_records.is_empty() { return; }
    use std::io::Write;
    let file = if cfg.append_error_log {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&error_cfg.file)
    } else {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&error_cfg.file)
    };
    // ... 后续 BufWriter + writeln! 逻辑不变
}
```

### WATCH-09: handle_watch 尾部退出码检查

```rust
// Source: src/cli/watch/mod.rs handle_watch [VERIFIED: codebase - 修改现有函数]
pub fn handle_watch(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    // ... 已有逻辑不变 ...
    pb.finish_and_clear();
    print_final_summary(
        &start,
        state.trigger_count(),
        state.total_stats().records_exported,
        quiet,
    );
    // WATCH-09: 若被中断则返回 Err(Error::Interrupted)，main.rs 处理 exit(130)（新增）
    if interrupted.load(std::sync::atomic::Ordering::Acquire) {
        return Err(crate::error::Error::Interrupted);
    }
    Ok(())
}
```

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test（内置）|
| Config file | Cargo.toml（无独立 test config）|
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WATCH-07 | CSV watch 追加：多次触发后文件包含所有记录，无重复 header | integration | `cargo test test_watch_csv_append` | ❌ Wave 0 新建 |
| WATCH-08 | error log 追加：两次触发后 error log 包含所有历史错误 | integration | `cargo test test_watch_error_log_append` | ❌ Wave 0 新建 |
| WATCH-08 | run 路径覆盖行为不变：`append_error_log` 默认 false | unit | `cargo test test_write_error_log_run_still_truncates` | ❌ Wave 0 新建 |
| WATCH-09 | Ctrl+C 后退出码 130：handle_watch 返回 Err(Interrupted) | unit | `cargo test test_handle_watch_returns_interrupted` | ❌ Wave 0 新建 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Phase gate:** 全套绿色后执行 `/gsd:verify-work`

### Wave 0 Gaps

新增测试应添加到 `src/cli/watch/mod.rs` 的 `#[cfg(test)]` 块或独立集成测试文件：

- [ ] `test_watch_csv_append` — 验证 WATCH-07：连续两次调用 `trigger_full_file` 后 CSV 包含两批记录、仅一个 header
- [ ] `test_watch_error_log_append` — 验证 WATCH-08：两次带错误记录的触发后 error log 包含所有错误行
- [ ] `test_write_error_log_run_still_truncates` — 验证 `append_error_log=false`（run 路径）仍覆盖写
- [ ] `test_handle_watch_returns_interrupted` — 验证 `interrupted=true` 时 `handle_watch` 返回 `Err(Error::Interrupted)`

---

## Environment Availability

Step 2.6: SKIPPED — 本阶段为纯代码修改，无外部工具或服务依赖。`cargo`、`rustc` 在当前环境已就绪（测试通过确认）。

---

## Open Questions

1. **`force_append_exporters` 辅助函数是否提取？**
   - What we know: `trigger_full_file` 和 `build_incremental_cfg` 两处代码完全相同
   - What's unclear: 是否值得为 2 处调用点提取函数（YAGNI vs DRY 的权衡）
   - Recommendation: Claude 自行决定（在 Claude's Discretion 范围内）；若提取，函数签名为 `fn force_append_exporters(cfg: &mut Config)`，调用方 `force_append_exporters(&mut tmp_cfg)`

2. **`write_error_log` 中 `File::create` 如何等价替换？**
   - What we know: `File::create` 等价于 `OpenOptions::new().create(true).write(true).truncate(true).open()`
   - What's unclear: 无歧义，直接替换
   - Recommendation: 使用 `OpenOptions` 统一两个分支，消除原 `File::create` 依赖

---

## Sources

### Primary (HIGH confidence)

- `src/cli/watch/mod.rs` — 完整读取，理解触发路径、WatchLoopState、handle_watch 控制流
- `src/cli/run/mod.rs` — 完整读取 write_error_log（line 423–461）
- `src/config/mod.rs` — 完整读取，理解 Config 结构体和现有 `#[serde(skip)]` 模式
- `src/exporter/csv/mod.rs` — 读取 line 1–144，理解 WriteMode::Append 和 TOCTOU 安全 header 逻辑
- `src/main.rs` — 完整读取，确认 `Err(Error::Interrupted) => exit(130)` 在 line 114–115
- `src/error.rs` — 完整读取，确认 `Error::Interrupted` 变体存在
- `src/config/exporter.rs` — 完整读取，确认 `CsvExporterConfig.append` 和 `overwrite` 字段

### Secondary (MEDIUM confidence)

- `cargo test` 输出 — 确认当前测试全部通过，无回归基线

---

## Metadata

**Confidence breakdown:**
- 修改点定位: HIGH — 直接读取源文件确认行号和函数签名
- 修改方式: HIGH — CONTEXT.md 已有明确 D-01 到 D-08 决策
- 测试策略: HIGH — 项目现有测试模式清晰，新测试可照抄 `tests.rs` 格式

**Research date:** 2026-06-06
**Valid until:** 无外部依赖变化，无有效期限制
