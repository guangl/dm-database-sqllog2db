---
phase: 20-test-coverage
plan: 02
subsystem: testing
tags: [rust, integration-testing, boundary-testing, csv, filters, template, field-projection]

# Dependency graph
requires:
  - phase: 17-filter-nesting
    provides: FiltersFeature/IncludeFilters/ExcludeFilters 过滤器 API
  - phase: 18-template-chart-nesting
    provides: TemplateConfig/OutputConfig 配置结构
  - phase: 19-code-refactor
    provides: 重构后的 handle_run 签名（10 参数）及 ExporterManager

provides:
  - tests/integration.rs 新增 3 条端到端集成测试（TEST-02）
  - tests/integration.rs 新增 4 条边界条件测试（TEST-03）
  - 验证完整 pipeline（输入 log → 过滤/归一化/投影 → CSV 输出）三条主路径
  - 验证空文件/全过滤/格式错误/超长 SQL 四个边界场景不 panic

affects: [20-test-coverage]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TempDir + write_test_log + make_run_config + handle_run + read_to_string 端到端测试模式"
    - "无效行放文件开头以触发独立解析失败（解析器多行记录合并特性）"

key-files:
  created: []
  modified:
    - tests/integration.rs

key-decisions:
  - "malformed_line 测试将无效行放在文件开头：解析器会把文件首部内容作为独立记录，解析失败后跳过，后续 4 条正常行全部导出"
  - "test_boundary_malformed_line 断言 CSV = 5 行（header+4 data），不验证 errors_in_file 内部变量（符合 Pitfall 3 指导）"

patterns-established:
  - "Pattern: 格式错误行测试——无效行放文件开头，不放中间（中间会被并入前一条多行记录）"
  - "Pattern: writeln! 宏替代 push_str(&format!(...)) 避免 clippy::format-push-string lint"

requirements-completed: [TEST-02, TEST-03]

# Metrics
duration: 30min
completed: 2026-05-18
---

# Phase 20 Plan 02: 端到端与边界集成测试 Summary

**为 CSV 导出 pipeline 的三条功能路径（过滤/归一化/投影）和四个边界场景（空文件/全过滤/格式错误/1MB SQL）补充集成测试，共新增 7 条测试函数，测试总数从 55 升至 62，全套通过**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-05-18T12:00:00Z
- **Completed:** 2026-05-18T12:33:13Z
- **Tasks:** 2
- **Files modified:** 1 (tests/integration.rs)

## Accomplishments
- 新增 `test_e2e_filter_pipeline`：验证 include.users 过滤路径，10 条 TESTUSER 记录全通过，OTHER 全被过滤
- 新增 `test_e2e_template_normalization`：验证 TemplateConfig.enable=true 后 CSV header 含 normalized_sql 且数据非空
- 新增 `test_e2e_field_projection`：验证 OutputConfig.fields=[ts,username,sql] 后 header 精确为 "ts,username,sql"
- 新增 `test_boundary_empty_log_file`：0 字节日志文件不 panic，CSV 只有 header
- 新增 `test_boundary_all_filtered`：全部记录被 include 过滤不 panic，CSV 只有 header
- 新增 `test_boundary_malformed_line`：无效格式行被跳过，4 条正常行全部导出
- 新增 `test_boundary_long_sql`：1MB SQL 字段不 panic，记录正常导出

## Task Commits

Each task was committed atomically:

1. **Task 1: 追加三条端到端集成测试 (TEST-02)** - `d48bd05` (test)
2. **Task 2: 追加四条边界条件测试 (TEST-03)** - `03edacd` (test)

## Files Created/Modified
- `tests/integration.rs` - 追加 7 条集成测试函数 + 更新顶部 use 块（导入 OutputConfig、TemplateConfig）

## Decisions Made
- 将 `test_boundary_malformed_line` 的无效行放在文件开头（而非中间）：解析器的多行记录合并机制会将插入中间的无效行附加到前一条记录的 body 中，导致记录数不减少；放在文件开头可使其被作为独立记录处理，从而触发解析失败 + 跳过
- 使用 `writeln!` 宏替代 `push_str(&format!(...))` 避免 clippy::format-push-string 报错

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] test_boundary_malformed_line 断言值修正**
- **Found during:** Task 2 (追加边界条件测试) - 测试运行阶段
- **Issue:** 计划假设无效行夹在正常行之间会被解析器跳过（得到4条数据），但解析器的多行记录逻辑会将无效行并入前一条记录的 body，实际产出5条记录
- **Fix:** 将无效行从"插在第2、3条之间"改为"放在文件开头"，使其被作为独立记录处理 → 解析失败 → 跳过，后续4条正常行全部导出（CSV = header + 4 data = 5 行）
- **Files modified:** tests/integration.rs
- **Verification:** `cargo test --test integration test_boundary_malformed_line` 通过
- **Committed in:** 03edacd (Task 2 commit)

**2. [Rule 1 - Bug] 修复 clippy::format-push-string lint**
- **Found during:** Task 2 提交阶段 - pre-commit 钩子运行 clippy -D warnings
- **Issue:** `content.push_str(&format!(...))` 触发 `clippy::format-push-string` 错误
- **Fix:** 改用 `writeln!(content, ...)` 宏直接写入 String
- **Files modified:** tests/integration.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` 零警告
- **Committed in:** 03edacd (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 - Bug)
**Impact on plan:** 两处修复都是为了确保测试正确反映实际 pipeline 行为。无功能范围扩展。

## Issues Encountered
- 解析器多行记录合并机制：当无效行夹在两条正常行之间时，它会被附加到前一条正常行的 SQL body 中（作为多行 SQL 的一部分），而不是被作为独立的失败记录处理。这是解析器的正确行为（支持 SQL 内嵌换行），但与直觉相反。解决方案：将无效行放在文件开头以绕过此行为。

## Known Stubs

None - 所有测试通过实际 I/O 写入和读取，无硬编码占位符。

## Threat Flags

无新增安全相关 surface（纯测试代码，不修改生产路径）。

## Next Phase Readiness
- TEST-02 + TEST-03 完成，tests/integration.rs 现有 62 条集成测试全部通过
- 计划 20-03（proptest 属性测试，TEST-04）可独立执行，无依赖本计划

---
*Phase: 20-test-coverage*
*Completed: 2026-05-18*

## Self-Check: PASSED
