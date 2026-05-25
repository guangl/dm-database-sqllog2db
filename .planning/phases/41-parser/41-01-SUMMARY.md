---
phase: 41-parser
plan: 01
subsystem: dependencies
requirements: [REFACTOR-02, PARSER-01]
tags: [rust, dependency-upgrade, parser]
affects:
  - Cargo.toml
  - Cargo.lock
  - src/cli/run/prescan.rs

dependency_graph:
  requires: []
  provides: [dm-database-parser-sqllog@2.0.0, clean-dep-baseline]
  affects: [Phase 42 benches, Phase 43 API adaptation, Phase 44/45 optimization]

tech_stack:
  upgraded:
    - "dm-database-parser-sqllog 1.1.0 -> 2.0.0"
    - "autocfg 1.5.0 -> 1.5.1"
    - "bumpalo 3.20.2 -> 3.20.3"
    - "either 1.15.0 -> 1.16.0"
    - "js-sys 0.3.98 -> 0.3.99"
    - "serde_json 1.0.149 -> 1.0.150"
    - "wasm-bindgen 0.2.121 -> 0.2.122"
    - "web-sys 0.3.98 -> 0.3.99"
  locked:
    - "criterion 0.7.x (rust-version=1.85 约束)"

key_files:
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/cli/run/prescan.rs

decisions:
  - "D-01: 直接修改 Cargo.toml 中的 major 版本号，cargo update 同步间接依赖"
  - "D-02: 2.0.0 公共 API 完全向后兼容，src/ 下零代码改动"
  - "D-03: criterion 锁定 0.7.x，不随 cargo update 升级到 0.8.x"
  - "D-04: 三道质量门禁(build/test/clippy)全部通过"

metrics:
  duration_minutes: 15
  completed_date: "2026-05-24"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
---

# Phase 41 Plan 01: 依赖升级与 dm-database-parser-sqllog 2.0.0 适配 Summary

将 `dm-database-parser-sqllog` 从 1.1.0 升级到 2.0.0 并通过 `cargo update` 同步 10 个间接依赖，公共 API 完全向后兼容，src/ 下零代码改动，三道质量门禁（build/test/clippy）全部通过。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 升级 dm-database-parser-sqllog 到 2.0.0 并同步 cargo update | `32840d9` | Cargo.toml, Cargo.lock |
| 2 | 审查并清理 prescan.rs 中过时的 v1.1.0 注释 | `759c677` | src/cli/run/prescan.rs |

## 依赖版本对比表

| 依赖 | 升级前 | 升级后 | 备注 |
|------|--------|--------|------|
| dm-database-parser-sqllog | 1.1.0 | **2.0.0** | 直接依赖，major 升级 |
| autocfg | 1.5.0 | 1.5.1 | 间接依赖，patch |
| bumpalo | 3.20.2 | 3.20.3 | 间接依赖，patch |
| either | 1.15.0 | 1.16.0 | 间接依赖，minor |
| js-sys | 0.3.98 | 0.3.99 | 间接依赖，patch |
| serde_json | 1.0.149 | 1.0.150 | 间接依赖，patch |
| wasm-bindgen | 0.2.121 | 0.2.122 | 间接依赖，patch |
| wasm-bindgen-macro | 0.2.121 | 0.2.122 | 间接依赖，patch |
| wasm-bindgen-macro-support | 0.2.121 | 0.2.122 | 间接依赖，patch |
| wasm-bindgen-shared | 0.2.121 | 0.2.122 | 间接依赖，patch |
| web-sys | 0.3.98 | 0.3.99 | 间接依赖，patch |
| criterion | 0.7.x | **0.7.x (锁定)** | rust-version=1.85 约束，不升级到 0.8.x |

## 质量门禁运行结果

| 命令 | 结果 | 备注 |
|------|------|------|
| `cargo build --release` | PASS (exit 0) | 0 warnings, 0 errors |
| `cargo test` | PASS (exit 0) | 239 单元 + 33 集成 = **272 passed, 0 failed** |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0) | 无 clippy 警告 |
| `cargo fmt --check` | PASS (exit 0) | 格式无漂移 |

### cargo build --release 输出摘要

```
   Compiling dm-database-parser-sqllog v2.0.0
   Compiling dm-database-sqllog2db v1.9.0
    Finished `release` profile [optimized] target(s) in 33.27s
```

无任何 `warning:` 或 `error:` 行（`grep -cE '^(warning|error):'` 返回 0）。

### cargo test 输出摘要

```
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## 依赖树变化摘要

`cargo tree -p dm-database-parser-sqllog --depth 1` 输出：

```
dm-database-parser-sqllog v2.0.0
├── atoi v2.0.0
├── encoding v0.2.33
├── memchr v2.8.0
└── thiserror v2.0.18
```

与 1.1.0 基线（atoi/encoding/memchr/thiserror）完全一致，**无新增传递依赖**。
2.0.0 新增的 `AsyncLogParser`/`FilterBuilder` 功能使用 feature flags 隔离，不影响默认依赖树（T-41-04 威胁已缓解）。

## 代码改动统计

```
Cargo.lock             | 44 ++++++++++++++++++++++----------------------
Cargo.toml             |  2 +-
src/cli/run/prescan.rs |  2 +-
3 files changed, 24 insertions(+), 24 deletions(-)
```

- `src/cli/run/prescan.rs`：仅注释行变化（comment-only diff），函数签名/逻辑/use 语句均未变动
- `git diff milestone/v1.10 -- src/ | wc -l`：2（仅 prescan.rs 注释行的 2 行变化）

## Task 2: 注释清理详情

**改动内容：**

prescan.rs 第 24 行，将版本锁定措辞改为版本无关措辞：

```diff
-    // 收集到 Vec 再并行处理（v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator）
+    // 收集到 Vec 再并行处理（LogParser 未实现 rayon 的 IntoParallelRefIterator，需先 collect 到 Vec 再并行）
```

**理由：** 原注释将行为绑定到 "v1.1.0"，在升级到 2.0.0 后变为误导性描述。实际上该行为（LogParser 未实现 `IntoParallelRefIterator`）在 2.0.0 中仍然成立，因此保留 WHY，仅移除版本号引用。

## Deviations from Plan

None — 计划执行完全符合预期：
- 2.0.0 公共 API 向后兼容，无需任何代码适配（D-02 验证成功）
- cargo update 升级的间接依赖与 RESEARCH §Standard Stack 描述一致
- criterion 0.7.x 约束未被违反
- prescan.rs 注释改动严格限制在注释范围内（comment-only diff）

## Notes for Phase 43

以下位置在 Phase 43 中可利用 dm-database-parser-sqllog 2.0.0 新 API 进行重构：

| 文件 | 位置 | 可用新 API | 说明 |
|------|------|-----------|------|
| `src/cli/run/processor.rs` | L53 | `FilterBuilder` | 可在解析阶段提前过滤，减少后续 pipeline 工作量 |
| `src/cli/run/prescan.rs` | L16-25 | `FilterBuilder` | 预扫描可利用 builder 直接配置过滤条件 |
| `src/cli/run/processor.rs` | L53 | `from_reader` | 若需要从非文件路径解析可使用 |
| 各处 `par_iter()` | 整个 prescan | `AsyncLogParser` | Phase 45 并行扩展时可评估异步解析路径 |

Phase 43 任务 PARSER-02 应以这些调用点为起点，评估 `FilterBuilder` 能否在热路径中减少无效记录解析。

## Threat Surface Scan

无新网络端点、认证路径、文件访问模式变化。T-41-01 已缓解：
- dm-database-parser-sqllog 2.0.0 为项目作者自有库，已在 RESEARCH §Package Legitimacy Audit 验证
- Cargo.lock 锁定具体版本 hash，防止后续拉取漂移
- T-41-04 缓解确认：依赖树深度 1 扫描未发现新传递依赖

## Self-Check: PASSED

| Item | Status |
|------|--------|
| SUMMARY.md 文件存在 | FOUND |
| commit 32840d9 (Task 1) 存在 | FOUND |
| commit 759c677 (Task 2) 存在 | FOUND |
| Cargo.toml dm-database-parser-sqllog = "2.0.0" | VERIFIED |
| Cargo.lock version = "2.0.0" | VERIFIED |
| criterion 保持 0.7.x | VERIFIED |
| prescan.rs 无 v1.1.0 引用 | VERIFIED |
| src/ comment-only diff | VERIFIED (2 lines only) |
| 272 tests passed | VERIFIED |
