---
phase: 44-hotpath
plan: 02
subsystem: pipeline/exporter
tags: [arc, param-buffer, bufwriter, performance, hot-path, h3, h4]

# Dependency graph
requires:
  - phase: 44-hotpath
    plan: 01
    provides: Wave 0 baseline measurements (PERF-01/PERF-02)
provides:
  - H-3 Arc-based ParamBuffer eliminating Vec deep-copy on DML normalize path
  - H-4 BufWriter 16MB capacity reducing write syscall frequency
affects: [44-hotpath plan 03 (criterion comparison), PERF-01, PERF-02]

# Tech tracking
tech-stack:
  added:
    - std::sync::Arc (stdlib, no new dep)
  patterns:
    - Arc<Vec<T>> value in HashMap: O(1) clone via atomic ref-count instead of Vec deep-copy
    - Arc Deref coercion: &Arc<Vec<T>> auto-deref to &[T] at apply_params_into call site
    - BufWriter 16MB: reduces write() syscalls ~8x for 1GB+ exports

key-files:
  modified:
    - src/pipeline/normalizer.rs (use Arc; ParamBuffer type alias; buffer.insert wraps Arc::new)
    - src/exporter/csv/mod.rs (BufWriter::with_capacity 2MB -> 16MB)

key-decisions:
  - "H-1/H-2 key clone (sess_id + statement strings) intentionally NOT eliminated: HashMap borrow checker prevents borrowing key while inserting (Pitfall 3 per RESEARCH.md); accepted as-is"
  - "Arc chosen over Rc: single-threaded now but Arc costs only one atomic op per clone; safe for future Phase 45 parallel extension"
  - "BufWriter 16MB is the absolute-value constant (16*1024*1024) not bit-shift (1<<24) for grep readability"
  - "CLAUDE.md already documented 16MB; only code needed updating to match"

requirements-completed:
  - PERF-01
  - PERF-02

# Metrics
duration: ~4min
completed: 2026-05-24
---

# Phase 44 Plan 02: H-3 Arc ParamBuffer + H-4 BufWriter 16MB Summary

**Arc-based ParamBuffer 消除 H-3 Vec 深拷贝热点 + BufWriter 16MB 减少 write syscall（H-4）**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-24T14:07:39Z
- **Completed:** 2026-05-24T14:12:00Z
- **Tasks:** 2
- **Files modified:** 2 (src/pipeline/normalizer.rs, src/exporter/csv/mod.rs)

## Accomplishments

### Task 1: H-3 Arc<Vec<ParamValue>> 消除 DML 热路径深拷贝

`ParamBuffer` value 类型从 `Vec<ParamValue>` 改为 `Arc<Vec<ParamValue>>`。

三处精确修改：

1. **use 语句**（第 2 行，新增）：
   ```rust
   use std::sync::Arc;
   ```

2. **类型别名**（第 12 行）：
   ```rust
   // 改前：
   pub type ParamBuffer = HashMap<(String, String), Vec<ParamValue>>;
   // 改后：
   pub type ParamBuffer = HashMap<(String, String), Arc<Vec<ParamValue>>>;
   ```

3. **buffer.insert 调用**（第 354-357 行，cargo fmt 多行格式）：
   ```rust
   // 改前：
   buffer.insert((record.sess_id.clone(), record.statement.clone()), params);
   // 改后：
   buffer.insert(
       (record.sess_id.clone(), record.statement.clone()),
       Arc::new(params),
   );
   ```

**热路径行为变化（第 376 行，文字未变）：**
```rust
let params = buffer.get(&key)?.clone();
```
- 改前语义：深拷贝整个 `Vec<ParamValue>`（每个 `String` 都触发堆分配）
- 改后语义：复制 `Arc` 引用计数（单次原子操作 O(1)）

**apply_params_into 调用点（第 393 行，未修改）：**
```rust
apply_params_into(pm_sql, &params, colon_style, scratch);
```
`&Arc<Vec<ParamValue>>` 通过 `Deref` 自动转换为 `&[ParamValue]`，签名 `fn apply_params_into(sql: &str, params: &[ParamValue], ...)` 不变。

**compute_normalized 函数体行数：** 61 行（第 349-409 行），满足 ≤ 64 约束。

### Task 2: H-4 BufWriter 容量 2MB -> 16MB

`src/exporter/csv/mod.rs` 第 124 行：
```rust
// 改前：
let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file);
// 改后：
let mut writer = BufWriter::with_capacity(16 * 1024 * 1024, file);
```

CLAUDE.md 第 52 行已记录 "16MB `BufWriter`"——代码侧修复后两者一致，无需修改文档。

**syscall 收益分析：**
- 1GB 文件导出：write() 调用从 ~512 次降至 ~64 次（8x 减少）
- 进程峰值内存：单 BufWriter 实例增加 +14MB（从 2MB 到 16MB），单文件流式处理无放大

## Test Results

```
cargo test: 215 passed; 0 failed (lib tests + jemalloc_peak integration test)
cargo clippy --all-targets -- -D warnings: Finished (no warnings)
cargo fmt --check: OK
cargo build --release: OK
grep unsafe normalizer.rs: 0 (no unsafe code)
```

## Task Commits

1. **Task 1: ParamBuffer Arc 改造** - `31e724b`
   - `src/pipeline/normalizer.rs`
2. **Task 2: BufWriter 16MB** - `d7b97b2`
   - `src/exporter/csv/mod.rs`

## Acceptance Criteria Verification

| Criterion | Result |
|-----------|--------|
| `grep -c '^use std::sync::Arc;$' normalizer.rs` | 1 |
| `ParamBuffer = HashMap<..., Arc<Vec<ParamValue>>>` | matched |
| `grep -c 'Arc::new(params)' normalizer.rs` | 1 |
| `grep -c 'buffer.get(&key)?.clone()' normalizer.rs` | 2 (line 10 comment + line 376 code) |
| `grep -c 'apply_params_into(pm_sql, &params' normalizer.rs` | 1 |
| unsafe count (normalizer.rs) | 0 |
| compute_normalized body lines | 61 (≤ 64) |
| `BufWriter::with_capacity(16 * 1024 * 1024, file)` | 1 |
| `BufWriter::with_capacity(2 * 1024 * 1024` | 0 (replaced) |
| CLAUDE.md contains "16MB" | 1 |
| cargo build | OK |
| cargo test | 215 passed |
| cargo clippy | OK |
| cargo fmt --check | OK |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Style] cargo fmt 要求 buffer.insert 多行格式**
- **Found during:** Task 1 verify (`cargo fmt --check`)
- **Issue:** 单行 `buffer.insert((record.sess_id.clone(), record.statement.clone()), Arc::new(params));` 超过 rustfmt 行宽限制，fmt --check 返回 exit 1
- **Fix:** 将 buffer.insert 调用格式化为 4 行（函数名 + key 参数 + Arc::new(params) + 收尾 `;`）
- **Impact:** compute_normalized 函数体从原先 58 行增至 61 行，仍满足 ≤ 64 约束
- **Files modified:** src/pipeline/normalizer.rs
- **Committed in:** 31e724b（Task 1 commit，fmt 修复前提交前已修正）

### Not-done Items

- H-1/H-2（sess_id + statement String key clone）：受 HashMap borrow checker 限制，无法在保持安全 Rust 的前提下消除。计划已明确接受现状，未在本 plan 范围内处理。

## Known Stubs

None — 无占位符或 TODO 代码。

## Threat Flags

None — 纯内部实现优化，未引入新 I/O 边界、新 API 端点或外部输入路径。T-44-06/T-44-07 mitigations 已满足（测试全通过，unsafe 计数为 0）。

---
*Phase: 44-hotpath*
*Completed: 2026-05-24*
