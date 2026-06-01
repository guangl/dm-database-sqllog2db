---
phase: 49-glob
verified: 2026-06-01T05:30:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
gaps: []
---

# Phase 49: Glob 输入支持 验证报告

**Phase Goal:** Support multiple input sources (files, directories, globs) via Vec<String> inputs field and --input CLI flag
**Verified:** 2026-06-01T05:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | SqllogConfig.inputs 字段（Vec<String>）替代 path: String，旧 path 键被 path_deprecated 捕获 | ✓ VERIFIED | `src/config/sqllog.rs` 第 8 行 `pub inputs: Vec<String>`，第 13 行 `pub path_deprecated: Option<toml::Value>` |
| 2  | 对旧 [sqllog] path = "..." 配置，validate() 返回 ConfigError::InvalidValue，reason 包含迁移示例 inputs = ["..."] | ✓ VERIFIED | `src/config/sqllog.rs` 第 29–33 行；unit test `test_validate_rejects_legacy_path_key` 通过 |
| 3  | 对 inputs 为空数组或任意条目为纯空白，validate() 返回 ConfigError::InvalidValue | ✓ VERIFIED | 第 36–52 行；`test_validate_rejects_empty_inputs` + `test_validate_rejects_whitespace_entry` 通过 |
| 4  | ParserError 新增 NoFilesFound { inputs: Vec<String> } 变体，Error::suggestion() 为该变体返回非空 hint | ✓ VERIFIED | `src/error.rs` 第 216 行变体定义，第 147 行 suggestion() 分支；3 个单元测试通过 |
| 5  | SqllogParser::new 接受 Vec<String>，log_files() 遍历所有 inputs，结果合并去重排序 | ✓ VERIFIED | `src/parser.rs` 第 11 行 `inputs: Vec<String>`，第 16 行 `new(inputs: Vec<String>)`；`test_log_files_multi_input_merge_and_dedup` + `test_log_files_multi_input_mixes_file_dir_glob` 通过 |
| 6  | `sqllog2db run -c config.toml --input file.log --input 'dir/*.log'` 可重复使用 --input/-i，CLI 值完全覆盖 config inputs | ✓ VERIFIED | `src/cli/opts.rs` 第 75 行 `ArgAction::Append`，第 78 行 `input: Option<Vec<String>>`；`src/main.rs` 第 49 行 `apply_cli_inputs_to_config` 实现完全替换语义；e2e 测试 C1/C2 通过 |
| 7  | init 模板 [sqllog] 段使用 inputs = ["sqllogs"] 数组语法 + 旧 path 键被拒绝（legacy 端到端验证） | ✓ VERIFIED | `src/cli/init.rs` 第 66 行 `inputs = ["sqllogs"]`；C3 `test_cli_legacy_path_key_rejected` 使用 validate 子命令，assert failure + stderr 含 sqllog.path / inputs / hint: |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/config/sqllog.rs` | SqllogConfig with inputs: Vec<String> + path_deprecated + validate() | ✓ VERIFIED | 56 行，完整实现含 4 单元测试 |
| `src/error.rs` | ParserError::NoFilesFound 变体 + suggestion() 分支 | ✓ VERIFIED | 第 216 行变体，第 147 行 suggestion，3 单元测试 |
| `src/parser.rs` | SqllogParser with inputs: Vec<String>; log_files() 多路径合并去重 | ✓ VERIFIED | 第 11 行字段，第 16 行 new，expand_single 关联函数，dedup() 存在 |
| `src/cli/run/mod.rs` | handle_run 使用 cfg.sqllog.inputs.clone(); 空列表抛 NoFilesFound | ✓ VERIFIED | 第 41 行 SqllogParser::new(cfg.sqllog.inputs.clone())，第 53–54 行 NoFilesFound |
| `src/preflight.rs` | check_log_paths 遍历 cfg.sqllog.inputs | ✓ VERIFIED | 第 13 行 `for input in &cfg.sqllog.inputs` |
| `src/cli/validate.rs` | handle_validate 打印 inputs 列表 | ✓ VERIFIED | 第 5–6 行打印 inputs 枚举 |
| `src/cli/opts.rs` | Run 子命令 input: Option<Vec<String>> + ArgAction::Append | ✓ VERIFIED | 第 75 行 ArgAction::Append，第 78 行字段 |
| `src/main.rs` | Commands::Run { config, input } 解构 + apply_cli_inputs_to_config | ✓ VERIFIED | 第 115 行解构，第 49 行函数，第 117 行注入 |
| `src/cli/init.rs` | CONFIG_TEMPLATE_EN [sqllog] 段使用 inputs 数组 | ✓ VERIFIED | 第 66 行 `inputs = ["sqllogs"]` |
| `tests/integration.rs` | 全部 SqllogConfig 字段引用迁移 + 4 个 e2e 测试 | ✓ VERIFIED | 旧 `path: log_dir.to_str()` 计数 = 0；C1/C2/C3/C4 测试均存在 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| src/cli/run/mod.rs::handle_run | src/parser.rs::SqllogParser::new | cfg.sqllog.inputs.clone() | ✓ WIRED | 第 41 行 |
| src/cli/run/mod.rs 空列表分支 | Error::Parser(ParserError::NoFilesFound) | is_empty() 后 return Err | ✓ WIRED | 第 53–54 行 |
| src/main.rs Commands::Run | cfg.sqllog.inputs | apply_cli_inputs_to_config 覆盖 | ✓ WIRED | 第 115, 117 行 |
| tests/integration.rs 端到端测试 | src/cli/opts.rs --input + path_deprecated 拒绝 | assert_cmd validate 子命令 | ✓ WIRED | C3 test_cli_legacy_path_key_rejected |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| src/parser.rs::log_files() | inputs Vec<String> | SqllogParser::new(cfg.sqllog.inputs.clone()) | 是 — 遍历文件系统路径/glob | ✓ FLOWING |
| src/cli/run/mod.rs::handle_run | log_files Vec<PathBuf> | log_files()? 展开真实文件路径 | 是 — 真实文件路径集合 | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| SqllogConfig inputs 字段存在 | grep "pub inputs: Vec<String>" src/config/sqllog.rs | 1 行匹配 | ✓ PASS |
| path_deprecated 字段存在 | grep "path_deprecated.*Option<toml::Value>" src/config/sqllog.rs | 1 行匹配 | ✓ PASS |
| NoFilesFound 变体存在 | grep "NoFilesFound" src/error.rs | 多行匹配 | ✓ PASS |
| ArgAction::Append 存在 | grep "ArgAction::Append" src/cli/opts.rs | 1 行匹配 | ✓ PASS |
| 旧 path 引用清零 | grep -rn "cfg\.sqllog\.path" src/ | 0 行匹配 | ✓ PASS |
| 全套测试 | cargo test | 226+254+48+1 全部 passed | ✓ PASS |
| clippy | cargo clippy --all-targets -- -D warnings | 无警告 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| INPUT-01 | 49-01, 49-02 | config.toml input 字段支持 glob 模式，自动展开匹配文件列表 | ✓ SATISFIED | SqllogConfig.inputs + SqllogParser::expand_single glob 展开；C2 e2e 测试验证 |
| INPUT-02 | 49-03 | 命令行 --input 参数支持 glob 模式，与配置文件行为一致 | ✓ SATISFIED | ArgAction::Append + apply_cli_inputs_to_config；C1/C2 e2e 测试验证 |

### Anti-Patterns Found

未发现阻塞性反模式。扫描结果：

- 无 TBD/FIXME/XXX 残留（49-01-SUMMARY 提到 opts.rs 的 TODO 注释已由 Plan 03 移除）
- 无 placeholder/stub 实现
- `#[allow(dead_code)]` 已在 Plan 02 中移除（NoFilesFound 被构造后）

### Human Verification Required

无需人工验证项。

### Gaps Summary

无缺口。所有 7 条可观测真相均通过三级（存在/实质/接线）验证；全套 `cargo test`（226 lib + 254 含 main.rs + 48 integration + 1 jemalloc = 529 个测试，全部通过）；`cargo clippy --all-targets -- -D warnings` 无警告；旧 `cfg.sqllog.path` 引用在 src/ 内计数为 0；4 个端到端 CLI 测试（C1/C2/C3/C4）均存在于 tests/integration.rs。

Phase 49 目标已完整实现。

---

_Verified: 2026-06-01T05:30:00Z_
_Verifier: Claude (gsd-verifier)_
