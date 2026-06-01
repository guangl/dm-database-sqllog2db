---
phase: 49-glob
plan: 1
subsystem: config
tags: [serde, toml, config, error-handling, glob]

# Dependency graph
requires: []
provides:
  - "SqllogConfig.inputs: Vec<String> 替代 path: String，默认值 [\"sqllogs\"]"
  - "SqllogConfig.path_deprecated: Option<toml::Value> 旧键检测，validate() 返回迁移 hint"
  - "ParserError::NoFilesFound { inputs: Vec<String> } 变体 + Error::suggestion() 分支"
affects:
  - "49-02: parser 接口改造（接受 Vec<String>），调用 SqllogConfig.inputs"
  - "49-03: e2e 测试匹配 NoFilesFound Display 字符串与 hint: 前缀"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "旧键检测：#[serde(rename = \"旧键\", default)] Option<toml::Value> + validate() 中 is_some() 检测"
    - "字段废弃迁移 hint 通过 ConfigError::InvalidValue.reason 传递（而非 suggestion()）"

key-files:
  created: []
  modified:
    - "src/config/sqllog.rs — SqllogConfig 重写，4 个新单元测试"
    - "src/error.rs — ParserError::NoFilesFound 变体，3 个新单元测试"
    - "src/cli/init.rs — 模板更新为 inputs = [\"sqllogs\"]（含注释 SQL log path list）"
    - "src/cli/run/mod.rs — 临时使用 inputs.first() 兼容旧接口"
    - "src/cli/validate.rs — cfg.sqllog.path → cfg.sqllog.inputs"
    - "src/config/mod.rs — 测试更新为 inputs 字段"
    - "src/config/validate.rs — 测试 TOML 格式从 path 改为 inputs"
    - "src/exporter/tests.rs — SqllogConfig 构造从 path 改为 inputs"
    - "src/preflight.rs — 临时使用 inputs.first()，测试构造更新"
    - "tests/integration.rs — SqllogConfig 构造从 path 改为 inputs"
    - "tests/jemalloc_peak.rs — TOML 格式从 directory 改为 inputs"

key-decisions:
  - "SqllogConfig.inputs 字段级 #[serde(default)] 返回空 Vec，结构体 Default 实现返回 [\"sqllogs\"]；TOML 反序列化时若 inputs 键缺席，字段为空 Vec（非 [\"sqllogs\"]）——由 validate() 拒绝；完整 Default 通过 Config::default() 传播"
  - "ParserError::NoFilesFound 归入既有 Warning 严重性（不提升为 Error/Critical），与 D-07 示例 [ERROR] 前缀存在标签差异——已接受为偏差，SC3 退出码 + hint 要求由 Err 返回路径满足"
  - "添加 #[allow(dead_code)] 使 NoFilesFound 变体通过 clippy —— Plan 02 会在 parser.rs 构造该变体后可移除"
  - "所有调用方 cfg.sqllog.path 引用在 Plan 01 内一次性修复（偏差 Rule 3），以通过 pre-commit clippy hook"

patterns-established:
  - "旧键检测三要素：#[doc(hidden)] #[serde(rename=\"旧键\", default)] pub xxx_deprecated: Option<toml::Value>；validate() 开头检测 is_some()；取值用 .to_string() 填入 ConfigError::InvalidValue.value"
  - "迁移 hint 通过 reason 字段而非 suggestion() 传递，与 pipeline_deprecated 模式一致"

requirements-completed: [INPUT-01]

# Metrics
duration: 45min
completed: 2026-06-01
---

# Phase 49 Plan 01: Glob 输入数据模型基础 Summary

**SqllogConfig.inputs Vec<String> 替代 path: String，path_deprecated 旧键检测，ParserError::NoFilesFound 变体建立 Wave 2 所需数据模型基础**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-01T03:00:00Z
- **Completed:** 2026-06-01T03:45:00Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `SqllogConfig` 从 `path: String` 改造为 `inputs: Vec<String>` + `path_deprecated: Option<toml::Value>`，默认值 `["sqllogs"]` 兼容无配置场景
- `validate()` 三级校验：旧 `path` 键检测（返回含 `inputs = [\"...\"]` 迁移示例的错误）+ 空数组 + 纯空白条目
- `ParserError::NoFilesFound { inputs: Vec<String> }` 变体，Display 为 `"No log files found matching inputs: ..."`, suggestion 返回 glob 修复建议
- init 模板更新为 `inputs = ["sqllogs"]`，7 个新单元测试（4 + 3）全部通过，所有 254 个测试通过

## Task Commits

1. **Task 1: 重写 SqllogConfig 为 inputs + path_deprecated** - `43bda02` (feat)
2. **Task 2: 新增 ParserError::NoFilesFound 变体与 suggestion() 分支** - `0a88e2c` (feat)

## Files Created/Modified

- `src/config/sqllog.rs` — SqllogConfig 完整重写，含 4 个单元测试
- `src/error.rs` — NoFilesFound 变体 + Display + suggestion() 分支，含 3 个单元测试
- `src/cli/init.rs` — CONFIG_TEMPLATE_EN 模板 `[sqllog]` 段更新为 `inputs = ["sqllogs"]`
- `src/cli/run/mod.rs` — 临时使用 `inputs.first().cloned().unwrap_or_default()` 兼容旧接口（TODO Plan 02）
- `src/cli/validate.rs` — `cfg.sqllog.path` → `cfg.sqllog.inputs`（打印所有输入）
- `src/config/mod.rs` — 测试 TOML 格式与字段断言更新
- `src/config/validate.rs` — 测试 TOML 和字段赋值更新（12 处批量替换 `path = "sqllogs"` → `inputs = ["sqllogs"]`）
- `src/exporter/tests.rs` — SqllogConfig 构造更新
- `src/preflight.rs` — 临时使用 `inputs.first()` + 测试构造更新
- `tests/integration.rs` — SqllogConfig 构造更新
- `tests/jemalloc_peak.rs` — TOML 格式从 `directory = "..."` 改为 `inputs = [...]`

## Decisions Made

- `NoFilesFound` 归入 `ParserError` 的 `Warning` 严重性（不提升为 `Critical`），接受 `[WARNING]` vs D-07 示例 `[ERROR]` 的标签差异；SC3 成功标准（退出码非零 + hint 行）由 `Err` 返回路径满足
- `#[allow(dead_code)]` 临时添加到 `NoFilesFound` 变体，避免 clippy dead_code 警告；Plan 02 在 `parser.rs` 构造该变体后可移除
- Plan 01 内一次性修复所有调用方（Rule 3），避免 pre-commit hook 阻塞提交

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 修复所有调用方 cfg.sqllog.path 引用**
- **Found during:** Task 1 提交时
- **Issue:** pre-commit hook 运行 clippy，发现 `cli/run/mod.rs`、`cli/validate.rs`、`config/mod.rs`、`config/validate.rs`、`exporter/tests.rs`、`preflight.rs`、`cli/run/tests.rs`、`tests/integration.rs`、`tests/jemalloc_peak.rs` 中共 8+ 处 `cfg.sqllog.path` 引用导致编译失败
- **Fix:** 逐一更新所有引用为 `cfg.sqllog.inputs`；`cli/run/mod.rs` 和 `preflight.rs` 中使用 `inputs.first()` 临时兼容（Plan 02 完整改造）；TOML 字符串中 `path = "sqllogs"` 批量替换为 `inputs = ["sqllogs"]`；`jemalloc_peak.rs` 中 `directory = "..."` 改为 `inputs = [...]`
- **Files modified:** 9 个文件
- **Verification:** `cargo test` 全部通过（220 lib + 33 integration + 1 jemalloc）
- **Committed in:** `43bda02` (Task 1 commit)

**2. [Rule 3 - Blocking] 添加 #[allow(dead_code)] 通过 clippy**
- **Found during:** Task 2 提交时
- **Issue:** `NoFilesFound` 变体在 Plan 01 阶段未被 `parser.rs` 构造，clippy `-D dead-code` 报告错误
- **Fix:** 添加 `#[allow(dead_code)]` 注释到 `NoFilesFound` 变体；注释说明 Plan 02 将构造该变体
- **Files modified:** `src/error.rs`
- **Verification:** `cargo clippy --lib --tests -- -D warnings` 无警告
- **Committed in:** `0a88e2c` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** 两处 Rule 3 修复均为必要的阻塞性修复（pre-commit hook 不可跳过）。Task 2 偏差范围完全在计划预期内（plan 已说明调用方会失败，但 clippy hook 触发了提前修复）。无范围蔓延。

## Issues Encountered

- `SqllogConfig` 字段级 `#[serde(default)]` 对 `Vec<String>` 返回空 Vec，不是 `["sqllogs"]`；未配置 `inputs` 键的 TOML（如只有 `directory = "..."` 的旧配置）会导致 `inputs` 为空，进而被 `validate()` 拒绝。这符合设计意图：旧配置格式不再被静默接受。

## 关键产出（供 Plan 02/03 引用）

### SqllogConfig 新字段（Plan 02 调用方更新参考）

```rust
pub struct SqllogConfig {
    #[serde(default)]
    pub inputs: Vec<String>,       // 替代 path: String
    #[doc(hidden)]
    #[serde(rename = "path", default)]
    pub path_deprecated: Option<toml::Value>,  // 旧键检测
}
// Default::default() 返回 inputs: vec!["sqllogs"]
```

### ParserError::NoFilesFound Display 字符串（Plan 03 e2e 测试匹配）

```
No log files found matching inputs: ["sqllogs/*.log"]
```

格式：`format!("No log files found matching inputs: {inputs:?}")`

### suggestion() 文本（Plan 03 断言 hint: 前缀）

```
Verify the glob/path entries exist; ensure patterns match .log files in the current directory.
```

### 严重性标签

实际输出：`[WARNING] No log files found matching inputs: [...]`
D-07 示例：`[ERROR] ...`
差异已接受为偏差。Plan 03 的 e2e 测试只断言 `"No log files found matching inputs"` 和 `"hint:"` 子串，不断言 `[ERROR]`/`[WARNING]` 标签。

## Next Phase Readiness

- Plan 02（parser 多输入接口）可以直接使用 `cfg.sqllog.inputs: Vec<String>` 字段，以及在 `parser.rs` 中构造 `ParserError::NoFilesFound { inputs }` 时移除 `#[allow(dead_code)]`
- Plan 03（e2e 验证）的测试断言格式已由本 SUMMARY 确认

---
*Phase: 49-glob*
*Completed: 2026-06-01*
