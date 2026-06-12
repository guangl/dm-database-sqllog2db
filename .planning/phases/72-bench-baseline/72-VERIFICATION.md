---
phase: 72-bench-baseline
verified: 2026-06-08T12:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "BENCHMARKS.md validate 行展示的是 v1.20 binary 走完整 validate 成功路径（exit 0）的真实毫秒值（CR-01 已闭合）"
  gaps_remaining: []
  regressions: []
---

# Phase 72: 基准体系完善（v1.20）Verification Report

**Phase Goal:** 采集并存档 v1.20 里程碑的 CLI 冷启动（hyperfine）与 Criterion benchmark 基线，作为 Phase 73-76 性能改进工作的回归判定锚点
**Verified:** 2026-06-08T12:00:00Z
**Status:** passed
**Re-verification:** Yes — after CR-01 gap closure (Plan 72-03)

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | 开发者运行 `hyperfine 'sqllog2db --version'` 可得到冷启动延迟数据，结果示例记录在 BENCHMARKS.md | VERIFIED | BENCHMARKS.md 第 761-763 行含 --version 原始 hyperfine 输出（2.1ms），commit fd91d63 |
| SC-2 | BENCHMARKS.md 包含 hyperfine 冷启动基准段落，说明测量方法、典型延迟数值、与 v1.19 基线的对比 | VERIFIED | 段落存在（第 741-781 行）；validate 行已更新为 benches/hyperfine-validate.toml 成功路径（2.4ms，exit 0），无 "Warning: Ignoring non-zero exit code."，新增脚注说明 fixture 用途（第 755 行）；CR-01 已闭合 |
| SC-3 | `cargo bench -- --save-baseline v1.20` 执行成功，基准结果文件保存至 criterion 默认 baseline 目录 | VERIFIED | 19 个 v1.20 目录（`benches/baselines/<group>/<id>/v1.20/`），含 benchmark.json + estimates.json + sample.json + tukey.json；commit 8da9f83 |
| SC-4 | `benches/baselines/` 目录存在且包含可用的 baseline 快照，`cargo bench -- --baseline v1.20` 可加载对比 | VERIFIED | 目录存在，19 个场景覆盖 4 个 bench 文件全部合成场景；冒烟测试通过 |
| SC-5 | `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，无性能退化 | VERIFIED | clippy 无警告；cargo test 919 通过（395+426+3+87+1+7）0 失败 |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `benches/BENCHMARKS.md` | Phase 72 段落（hyperfine + Criterion 两部分） | VERIFIED | 第 735-804 行，三个子节齐全，checklist BENCH-01 [x] BENCH-02 [x] |
| `benches/hyperfine-validate.toml` | hyperfine validate 子命令专用最小合法 config fixture | VERIFIED | 文件存在，含 `inputs = ["sqllogs"]`，无旧字段 `directory`，validate exit 0 确认；commit 3dcd35d |
| `benches/baselines/csv_export/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在，含标准 criterion JSON |
| `benches/baselines/parser_throughput/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |
| `benches/baselines/sqlite_export/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |
| `benches/baselines/filters/no_pipeline/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| Phase 72 对比表格 | Phase 9 历史值 ~2.9ms / ~2.8ms | 表格 Phase 9 (v1.9) mean 列 | VERIFIED | 第 750-753 行，列头和值均存在 |
| hyperfine 折叠原始输出块 | 实测 stdout | `<details><summary>hyperfine 原始输出</summary>` | VERIFIED | 14 个 `<details>` / `</details>` 全部配对（含 Phase 72 两个块） |
| How to compare 段落 | v1.20 对比命令 | bash 代码块 `--baseline v1.20` | VERIFIED | 第 38 行，位于 How to compare bash 代码块内 |
| Phase 72 段落 validate 命令 | benches/hyperfine-validate.toml fixture | 命令字符串引用 | VERIFIED | 第 747、753、774 行均引用 `benches/hyperfine-validate.toml`；fixture 文件存在 |
| Phase 72 validate 行 | 成功路径数值（exit 0） | `./target/release/sqllog2db validate -c benches/hyperfine-validate.toml` | VERIFIED | 实测 exit=0 确认；stdout 含 "Configuration valid."；原始输出块无 non-zero exit 警告 |
| Phase 72 段落 Criterion 小节 | benches/baselines/ v1.20 目录 | `--save-baseline v1.20` 引用 | VERIFIED | 第 787-794 行，存档命令 + 对比命令均在文档中 |

---

### Data-Flow Trace (Level 4)

不适用 — 本 Phase 产物为静态文档（BENCHMARKS.md）和数据文件（baseline JSON），不涉及动态数据渲染。

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| BENCHMARKS.md Phase 72 段落标题唯一 | `grep -c "^## Phase 72 — 基准体系完善（v1.20）$" benches/BENCHMARKS.md` | 1 | PASS |
| validate fixture 文件存在 | `test -f benches/hyperfine-validate.toml` | exit 0 | PASS |
| fixture 含合法 inputs 字段 | `grep -q 'inputs = ["sqllogs"]' benches/hyperfine-validate.toml` | exit 0 | PASS |
| fixture 不含旧 directory 字段 | `! grep -q '^directory' benches/hyperfine-validate.toml` | exit 0 | PASS |
| validate 走成功路径 | `./target/release/sqllog2db validate -c benches/hyperfine-validate.toml; echo $?` | 0（"Configuration valid."） | PASS |
| Warning 行已彻底移除 | `! grep -q "Warning: Ignoring non-zero exit code." benches/BENCHMARKS.md` | exit 0 | PASS |
| validate 行引用新 fixture | `grep -q "validate -c benches/hyperfine-validate.toml" benches/BENCHMARKS.md` | exit 0（第 747/753/774 行） | PASS |
| 脚注说明 fixture 用途 | `grep -q "validate 行使用 \`benches/hyperfine-validate.toml\`" benches/BENCHMARKS.md` | exit 0（第 755 行） | PASS |
| --version 行未被修改 | `grep -q "| \`--version\` | ~2.9 ms | 2.1 ms |" benches/BENCHMARKS.md` | exit 0（第 752 行） | PASS |
| `<details>` 标签平衡 | `awk '/<details>/{o++} /<\/details>/{c++} END{exit !(o>=2 && o==c)}'` | 14/14 平衡 | PASS |
| BENCH-01 checklist [x] | `grep -q "^\- \[x\] BENCH-01"` | 第 801 行 | PASS |
| BENCH-02 checklist [x] | `grep -q "^\- \[x\] BENCH-02"` | 第 802 行 | PASS |
| cargo clippy | `cargo clippy --all-targets -- -D warnings` | 无警告 | PASS |
| cargo test | `cargo test` | 919 通过 / 0 失败 | PASS |

---

### Probe Execution

不适用 — 本 Phase 无 `scripts/*/tests/probe-*.sh` 探针。

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BENCH-01 | 72-01-PLAN.md (+ 72-03-PLAN.md gap-closure) | 开发者可以用 hyperfine 测量 CLI 冷启动延迟，结果存入 BENCHMARKS.md | SATISFIED | --version 2.1ms + validate 2.4ms（成功路径）均已记录；benches/hyperfine-validate.toml fixture 确保 validate 走 exit 0 完整路径，与 Phase 9 同口径可比；CR-01 已闭合 |
| BENCH-02 | 72-02-PLAN.md | 开发者可以用 `--save-baseline` 将 criterion 结果存档到 `benches/baselines/`，版本间对比有迹可循 | SATISFIED | 19 个 v1.20 目录已纳入 repo，`--baseline v1.20` 冒烟通过 |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| 无 | — | — | — | 上次 BLOCKER（validate 失败路径误导）已由 Plan 72-03 闭合 |

---

### Human Verification Required

无 — 所有关键验证项均可通过代码检查和命令执行完成。

---

## CR-01 闭合确认

初次验证发现的 BLOCKER（validate 计时反映 exit 2 失败路径）已由 Plan 72-03 完整解决：

| 判别项 | 初次验证（gaps_found） | 本次 re-verification（passed） |
|--------|----------------------|-------------------------------|
| `validate -c <config>` 退出码 | exit 2（失败路径） | exit 0（成功路径） |
| hyperfine stdout 含 non-zero exit 警告 | 是（第 777 行） | 否（已彻底移除） |
| 对比表 validate 数据来源 | 失败路径（立即 ConfigError 退出） | 成功路径（完整 validate + 打印） |
| fixture 文件 | 无（引用了旧字段的 config.toml） | `benches/hyperfine-validate.toml`（inputs 非空，v1.12+ 合法字段） |
| 与 Phase 9 可比性 | 不可比 | 同口径可比（均为成功路径） |

---

## Gaps Summary

无 — 初次验证的 1 个 BLOCKER 已完整闭合，Phase 72 全部目标达成。

**BENCH-01 完整达成：** --version（2.1ms）+ validate（2.4ms，成功路径）两条 hyperfine 数值均来自 exit 0 路径，与 Phase 9 历史基线（~2.9ms / ~2.8ms）同口径可比，可作 Phase 73-76 性能改进的回归判定锚点。

**BENCH-02 完整达成：** 19 个 v1.20 criterion baseline 目录存档于 `benches/baselines/`，覆盖 4 个 bench 文件全部合成场景。

---

_Verified: 2026-06-08T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Yes — after Plan 72-03 CR-01 gap closure_
