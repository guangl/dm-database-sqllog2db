# Phase 60 — 错误处理审计报告

生成时间：2026-06-03  
执行 Plan：60-01（两处 infallible 注释 + 审计验证）

---

## 1. grep 'unwrap()|expect(' 扫描结果分类

### 扫描命令

```
grep -rn 'unwrap()\|expect(' src/ --include="*.rs"
```

### unwrap/expect 分类表

所有命中行均属于以下四类之一：

#### production_commented（生产代码，已有 infallible 注释或 # Panics 文档）

| 文件 | 行 | 说明 |
|------|----|------|
| `src/logging.rs` | 60 | `write!(buf, ...).unwrap(); // infallible: writing to a String never fails` — 本 Plan 新增 |
| `src/cli/run/parallel.rs` | 281 | `.expect("parallel CSV requires CSV exporter")` 前一行有注释 `// infallible: process_csv_parallel is only called when CSV exporter is present` — 本 Plan 新增 |
| `src/stats/normalize.rs` | 56 | `normalize_sql` 函数头有完整 `# Panics` 文档注释（第 12-16 行）：输出字节来自 UTF-8 输入原样复制或 ASCII `b'?'`，不会破坏 UTF-8 序列 |
| `src/pipeline/normalizer.rs` | 418 | `apply_params_into` 函数中有 `debug_assert!` + 行内注释（第 410-413 行）说明 infallible 原因；`expect` 是最终一致性断言（D-03 保留） |

#### test_code（测试文件或 #[cfg(test)] 块内，per D-02 保留）

| 文件 | 行范围 | 说明 |
|------|--------|------|
| `src/logging.rs` | 212-270 | `#[cfg(test)]` 块从第 205 行开始，`.unwrap()` 均在测试辅助函数中 |
| `src/preflight.rs` | 162-250 | `#[cfg(test)]` 从第 110 行开始，该区间所有 unwrap 在测试块中 |
| `src/pipeline/normalizer.rs` | 310 | `apply_params` 函数被 `#[cfg(test)]` 标注（第 306 行），整体为测试辅助 |
| `src/cli/run/tests.rs` | 全文 | 独立测试文件，所有 unwrap 均为测试代码 |
| `src/exporter/sqlite/tests.rs` | 全文 | 独立测试文件，所有 unwrap 均为测试代码 |
| `src/exporter/csv/tests.rs` | 全文 | 独立测试文件，所有 unwrap 均为测试代码 |
| `src/exporter/tests.rs` | 全文 | 独立测试文件（89/92/97/99/132 行），所有 unwrap 均为测试代码 |

#### unwrap_or_family（误匹配的 *_or 系列，不在成功标准 1 范围内）

grep 模式 `unwrap()\|expect(` 包含了以下误匹配（均含括号，与 grep 字面匹配重叠）：

| 文件 | 行 | 调用形式 |
|------|----|---------|
| `src/cli/run/parallel.rs` | 80 | `.unwrap_or_default()` — *_or 系列 |
| `src/cli/run/parallel.rs` | 86 | `.unwrap_or(Path::new("."))` — *_or 系列 |
| `src/logging.rs` | 32 | `.unwrap_or_default()` — *_or 系列 |
| 多处 scanner/parser 等 | 各 | `.unwrap_or_else(...)` / `.unwrap_or_default()` — *_or 系列 |

（此类调用在 grep 输出中出现但不属于 `.unwrap()` 或 `.expect(...)` 裸调用，标记为 unwrap_or_family）

#### clippy_attribute（`#[expect(...)]` 属性，非方法调用，不计入成功标准 1）

| 文件 | 行 | 说明 |
|------|----|------|
| `src/exporter/mod.rs` | 291 | `#[expect(clippy::...)]` clippy lint 压制属性 |
| `src/stats/aggregate.rs` | 176, 180 | `#[expect(clippy::...)]` clippy lint 压制属性 |
| `src/stats/output.rs` | 149 | `#[expect(clippy::...)]` clippy lint 压制属性 |

### 结论

**production_uncommented（未注释生产代码 unwrap/expect）数量：0**

---

## 2. grep '.map_err' 扫描结果分类

### 扫描命令

```
grep -rn '\.map_err' src/ --include="*.rs"
```

### map_err 保留判定表

#### retained_with_context（closure 构造携带 path / reason / line_number 字段，per D-01 保留）

| 文件 | 行 | 上下文字段 |
|------|----|-----------|
| `src/logging.rs` | 82 | `path` + `reason` |
| `src/logging.rs` | 112 | `path` + `reason` |
| `src/cli/init.rs` | 31 | `path` + `reason` |
| `src/cli/init.rs` | 40 | `path` + `reason` |
| `src/parser.rs` | 58 | `path` + `reason` |
| `src/parser.rs` | 66 | `path` + `reason` |
| `src/parser.rs` | 108 | `path` + `reason` |
| `src/cli/run/collector.rs` | 25 | `path` + `reason` |
| `src/config/mod.rs` | 38 | `path` + `reason` |
| `src/config/mod.rs` | 45 | `path` + `reason` |
| `src/exporter/csv/mod.rs` | 95 | `path` + `reason` |
| `src/exporter/csv/mod.rs` | 116 | `path` + `reason` |
| `src/exporter/csv/mod.rs` | 134 | `path` + `reason` |
| `src/exporter/csv/mod.rs` | 207 | `path` + `reason` |
| `src/exporter/csv/writer.rs` | 203 | `path` + `reason` |
| `src/exporter/sqlite/mod.rs` | 118 | reason（db 错误描述） |
| `src/exporter/sqlite/mod.rs` | 127 | reason |
| `src/exporter/sqlite/mod.rs` | 147 | reason |
| `src/exporter/sqlite/mod.rs` | 167 | reason |
| `src/exporter/sqlite/mod.rs` | 171 | reason |
| `src/exporter/sqlite/mod.rs` | 173 | reason |
| `src/exporter/sqlite/mod.rs` | 185 | reason |
| `src/exporter/sqlite/mod.rs` | 188 | reason |
| `src/exporter/sqlite/mod.rs` | 215 | reason |
| `src/exporter/sqlite/mod.rs` | 224 | reason |
| `src/exporter/sqlite/mod.rs` | 234 | reason |
| `src/scanner.rs` | 21 | `path` + `reason` |
| `src/stats/config.rs` | 19 | reason（时间格式校验） |
| `src/stats/config.rs` | 28 | reason |
| `src/stats/output.rs` | 84 | reason |
| `src/stats/output.rs` | 86 | reason |
| `src/stats/output.rs` | 93 | reason |
| `src/stats/output.rs` | 103 | reason |
| `src/stats/output.rs` | 105 | reason |

#### retained_rayon（rayon::ThreadPoolBuildError 的 std::io::Error::other(e) 中转模式，per D-01 保留）

| 文件 | 行 | 模式 |
|------|----|------|
| `src/cli/run/parallel.rs` | 149 | `.map_err(\|e\| Error::Io(std::io::Error::other(e)))?` |
| `src/cli/run/sqlite_parallel.rs` | 26 | `.map_err(\|e\| Error::Io(std::io::Error::other(e)))?` |
| `src/cli/run/prescan.rs` | 117 | `.map_err(\|e\| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?` |

注：`rayon::ThreadPoolBuildError` 无 `From<...> for Error` 实现，无法使用 `?` 直接传播，必须通过 `std::io::Error::other` 中转。

#### replaceable_with_question_mark（未列出）

**replaceable_with_question_mark 数量：0**

审计结论：所有 `.map_err` 均携带不可由 `From` 自动填充的上下文字段（path/reason）或属于 rayon 错误中转。没有可简化为 `?` 的残留 map_err。

---

## 3. cargo clippy 输出摘录（关键行）

```
cargo clippy --all-targets -- -D warnings 2>&1
```

关键输出：
```
Checking dm-database-sqllog2db v1.15.0 (...)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.93s
```

退出码：**0**（无警告，无错误）

clippy 完整输出中不含 `unwrap_used` 或 `expect_used` 关键字（clippy 内置 lint 未触发）。

---

## 4. cargo test 输出摘录（pass/fail 计数）

```
cargo test 2>&1 | grep 'test result'
```

输出：
```
test result: ok. 269 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 300 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 68 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

总计：**638 个测试通过，0 个失败**

---

## 5. ROADMAP Phase 60 成功标准核对

### 标准 1：每个 unwrap/expect 可解释

- [x] grep 扫描结果中所有命中行均已分类：
  - production_commented：4 处（logging.rs:60、parallel.rs:281、stats/normalize.rs:56、pipeline/normalizer.rs:418）
  - test_code：大量（均在 tests.rs 文件或 #[cfg(test)] 块内）
  - unwrap_or_family：误匹配，不计入
  - clippy_attribute：4 处 `#[expect(...)]` 属性，不计入
  - **production_uncommented 数量：0**

### 标准 2：From 集中且 map_err 已审计

- [x] `git diff --stat src/error.rs` 输出为空（D-04：src/error.rs 未被修改）
- [x] 所有保留的 `.map_err` 均携带上下文字段（path/reason）或属于 rayon 错误中转（无法由 `From` 自动填充）
- [x] **replaceable_with_question_mark 数量：0**

### 标准 3：clippy + test 绿

- [x] `cargo clippy --all-targets -- -D warnings` 退出码 0
- [x] clippy 输出中不含 `unwrap_used` 或 `expect_used` 关键字
- [x] `cargo test` 退出码 0（638 个测试全部通过）

### 标准 4：行为不变

- [x] cargo test 既有套件（含 Phase 57 e2e）100% 通过
- [x] src/ 下仅新增两处 `// infallible` 行内注释，非注释代码字节零变化
- [x] `git diff --stat src/pipeline/normalizer.rs` 输出为空（D-03）
- [x] `git diff --stat src/cli/run/sqlite_parallel.rs src/cli/run/prescan.rs` 输出为空（D-01 三处 rayon 保留）

---

**总结：Phase 60 四条成功标准全部通过。无 BLOCKER 项。**
