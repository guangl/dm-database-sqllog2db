# Phase 60: 错误处理路径统一 - Pattern Map

**Mapped:** 2026-06-03
**Files analyzed:** 6 (4 modified + 2 read-only reference)
**Analogs found:** 6 / 6

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/logging.rs` | utility | request-response | `src/stats/normalize.rs` | role-match (infallible `expect` pattern) |
| `src/cli/run/parallel.rs` | utility | batch | `src/cli/run/prescan.rs` | exact (same rayon map_err pattern) |
| `src/cli/run/sqlite_parallel.rs` | utility | batch | `src/cli/run/parallel.rs` | exact (identical rayon map_err pattern) |
| `src/cli/run/prescan.rs` | utility | batch | `src/cli/run/parallel.rs` | exact (same rayon map_err pattern) |
| `src/pipeline/normalizer.rs` | transform | transform | `src/stats/normalize.rs` | exact (same `# Panics` doc + `expect` pattern) |
| `src/error.rs` | utility | — | self (已是模式来源) | canonical reference only |

---

## Pattern Assignments

### `src/logging.rs` (utility, request-response)

**修改目标：** 第 60 行的 `.unwrap()` 加 `// infallible:` 注释。

**Analog:** `src/stats/normalize.rs`（相同的 infallible `expect` 文档模式）

**当前代码 — logging.rs 行 55-61：**
```rust
let mut buf = String::with_capacity(19);
write!(
    buf,
    "{y:04}-{m:02}-{d:02} {hours:02}:{mins:02}:{secs_part:02}",
)
.unwrap();
buf
```

**目标模式 — 参照 stats/normalize.rs 行 11-16 的文档风格：**
```rust
/// # Panics
///
/// 不会在实践中 panic：输出字节要么来自 UTF-8 输入的原样复制，要么是 ASCII
/// 字节 `b'?'`（单字节 ASCII 不会破坏多字节 UTF-8 序列）。`expect` 是内部
/// 一致性断言，正常情况下不会触发。
```

**修改方式（D-02）：** 在 `.unwrap();` 同行末加注释：
```rust
.unwrap(); // infallible: writing to a String never fails
```

**其余保留的 map_err — logging.rs 行 82-87（保留，携带上下文字段）：**
```rust
std::fs::create_dir_all(parent_dir).map_err(|e| {
    Error::File(FileError::CreateDirectoryFailed {
        path: parent_dir.to_path_buf(),
        reason: e.to_string(),
    })
})?;
```
这类 closure 构造了 `path` + `reason` 字段，属于 D-01 必须保留类型。

---

### `src/cli/run/parallel.rs` (utility, batch)

**修改目标 1：** 第 87 行的 `.expect(...)` 加 `// infallible:` 注释（D-02）。
**修改目标 2：** 第 117-120 行的 `rayon ThreadPoolBuilder.map_err(...)` 确认保留（D-01）。

**Analog:** `src/cli/run/prescan.rs`（相同的 rayon 线程池构建模式）

**expect 注释化 — parallel.rs 行 83-87：**
```rust
// 当前代码
let csv_cfg = cfg
    .exporter
    .csv
    .as_ref()
    .expect("parallel CSV requires CSV exporter");

// 修改后
let csv_cfg = cfg
    .exporter
    .csv
    .as_ref()
    // infallible: process_csv_parallel is only called when CSV exporter is present
    .expect("parallel CSV requires CSV exporter");
```

**rayon map_err 保留 — parallel.rs 行 117-120（D-01 不适用，保留原样）：**
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(e)))?;
```
原因：`rayon::ThreadPoolBuildError` 未实现 `From<...> for Error`，中间必须经过 `std::io::Error::other(e)` 构造，不可直接用 `?`。

---

### `src/cli/run/sqlite_parallel.rs` (utility, batch)

**修改目标：** 第 116-119 行的 `rayon ThreadPoolBuilder.map_err(...)` 确认保留（D-01），无需修改。

**Analog:** `src/cli/run/parallel.rs`（完全相同模式）

**当前代码 — sqlite_parallel.rs 行 116-119：**
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(e)))?;
```
与 `parallel.rs` 的 rayon 线程池构建完全对称，同样保留。

**携带上下文字段的 map_err（保留）— sqlite_parallel.rs 行 27-33：**
```rust
.map_err(|e| {
    crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
        path: file_str.into_owned().into(),
        reason: format!("{e}"),
        line_number: None,
    })
})?;
```
构造了 `path` + `reason` 字段，属于 D-01 必须保留类型。

---

### `src/cli/run/prescan.rs` (utility, batch)

**修改目标：** 第 114-117 行的 `rayon ThreadPoolBuilder.map_err(...)` 确认保留（D-01），无需修改。

**Analog:** `src/cli/run/parallel.rs`（完全相同模式）

**当前代码 — prescan.rs 行 114-117：**
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?;
```
注意：`prescan.rs` 的 closure 中额外包裹了格式化字符串 `format!("rayon thread pool: {e}")`，提供了更多上下文，是比 `parallel.rs` 和 `sqlite_parallel.rs` 更好的写法，可以作为参考模式。

---

### `src/pipeline/normalizer.rs` (transform, transform)

**修改目标：** D-03 — 保持不变，现有 `expect` 注释已符合成功标准。

**Analog:** `src/stats/normalize.rs`（完全相同的文档 + `expect` 组合模式）

**已有注释模式 — normalizer.rs 行 302-310（保持不变）：**
```rust
/// # Panics
///
/// Will not panic in practice: the output is valid UTF-8 (original SQL bytes plus
/// ASCII param literals). The `expect` is an internal consistency assertion.
#[cfg(test)]
fn apply_params(sql: &str, params: &[ParamValue], colon_style: bool) -> String {
    let mut buf = Vec::new();
    apply_params_into(sql, params, colon_style, &mut buf);
    String::from_utf8(buf).expect("apply_params produced invalid UTF-8")
}
```

**已有注释模式 — normalizer.rs 行 410-418（保持不变）：**
```rust
// ASCII literals used as delimiters ('?', ':', '\'') are single-byte and
// cannot appear in the interior of a multi-byte UTF-8 sequence, so no
// sequence is broken. The debug_assert guards this invariant cheaply in
// debug builds; the expect is a final consistency guard.
debug_assert!(
    std::str::from_utf8(scratch).is_ok(),
    "apply_params_into produced invalid UTF-8 — safety invariant violated"
);
Some(std::str::from_utf8(scratch).expect("apply_params_into produced invalid UTF-8"))
```

---

### `src/error.rs` (utility, canonical reference)

**修改目标：** 本阶段不修改 `src/error.rs`（D-04：现有 From 实现已完备，不新增变体）。

**已有 From impl 结构 — error.rs 行 70-89（只读参考）：**
```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("File error: {0}")]
    File(#[from] FileError),

    #[error("SQL log parser error: {0}")]
    Parser(#[from] ParserError),

    #[error("Export error: {0}")]
    Export(#[from] ExportError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Interrupted by user")]
    Interrupted,
}
```

**From 覆盖矩阵（验证 D-01 替换前提）：**
| 源类型 | 目标 Error 变体 | `?` 可用？ |
|--------|----------------|-----------|
| `ConfigError` | `Error::Config` | 是（已有 `#[from]`） |
| `FileError` | `Error::File` | 是（已有 `#[from]`） |
| `ParserError` | `Error::Parser` | 是（已有 `#[from]`） |
| `ExportError` | `Error::Export` | 是（已有 `#[from]`） |
| `io::Error` | `Error::Io` | 是（已有 `#[from]`） |
| `rayon::ThreadPoolBuildError` | — | **否**（无 From，必须手动构造） |

---

## Shared Patterns

### Pattern A: infallible unwrap/expect 注释格式

**来源：** `src/stats/normalize.rs` 行 11-16、`src/pipeline/normalizer.rs` 行 302-418

**适用文件：** `src/logging.rs:60`、`src/cli/run/parallel.rs:87`

**两种注释层次：**

1. **单行内联注释**（适用于 `unwrap()`，无需文档化）：
   ```rust
   .unwrap(); // infallible: writing to a String never fails
   ```

2. **前置注释块**（适用于 `expect()`，在调用处上方）：
   ```rust
   // infallible: process_csv_parallel is only called when CSV exporter is present
   .expect("parallel CSV requires CSV exporter");
   ```

3. **`# Panics` 文档注释**（适用于公开函数）：
   ```rust
   /// # Panics
   ///
   /// 不会在实践中 panic：输出字节来自 UTF-8 输入，ASCII 字节不破坏多字节序列。
   /// `expect` 是内部一致性断言。
   ```

### Pattern B: rayon ThreadPoolBuilder map_err（保留模式）

**来源：** `src/cli/run/prescan.rs` 行 114-117（最佳写法，含格式化上下文）

**适用文件：** `parallel.rs:120`、`sqlite_parallel.rs:119`（确认保留）

```rust
// 最佳写法参考（prescan.rs 版本，含额外上下文字符串）
.map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?;

// 当前两个文件的写法（语义等价，亦可接受）
.map_err(|e| Error::Io(std::io::Error::other(e)))?;
```
不可替换为 `?` 的理由：`rayon::ThreadPoolBuildError` 未实现 `From<...> for crate::error::Error`，需要先经 `std::io::Error::other()` 构造才能触发 `From<io::Error>`。

### Pattern C: 携带上下文字段的 map_err（必须保留）

**来源：** `src/logging.rs` 行 82-87、`src/cli/run/sqlite_parallel.rs` 行 27-33

**适用场景：** closure 构造 `{ path, reason }` 等结构体字段的变体

```rust
// 典型保留形式 — 注意 path: ... 和 reason: ... 字段
.map_err(|e| {
    Error::File(FileError::CreateDirectoryFailed {
        path: parent_dir.to_path_buf(),
        reason: e.to_string(),
    })
})?;
```
`From<io::Error>` 无法自动填充 `path` 字段，D-01 明确此类必须保留。

---

## No Analog Found

本阶段无文件缺少 analog。所有文件均可在代码库内找到结构相同的参照文件。

---

## Metadata

**Analog search scope:** `src/` 目录全量
**Files scanned:** 6（error.rs、logging.rs、parallel.rs、sqlite_parallel.rs、prescan.rs、normalizer.rs + stats/normalize.rs 作对比参照）
**Pattern extraction date:** 2026-06-03

**实际需要修改的行数汇总：**
| 文件 | 行号 | 操作 | 类型 |
|------|------|------|------|
| `src/logging.rs` | 60 | 行末加 `// infallible: writing to a String never fails` | 注释 |
| `src/cli/run/parallel.rs` | 86-87 | 在 `.expect(...)` 前一行加 `// infallible: ...` 注释 | 注释 |
| `src/cli/run/parallel.rs` | 117-120 | 确认保留，无需修改 | 审查 |
| `src/cli/run/sqlite_parallel.rs` | 116-119 | 确认保留，无需修改 | 审查 |
| `src/cli/run/prescan.rs` | 114-117 | 确认保留，无需修改 | 审查 |
| `src/pipeline/normalizer.rs` | 310、418 | D-03：保持不变 | 免修改 |
