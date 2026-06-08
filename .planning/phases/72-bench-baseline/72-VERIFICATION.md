---
phase: 72-bench-baseline
verified: 2026-06-08T11:18:34Z
status: gaps_found
score: 4/5 must-haves verified
overrides_applied: 0
gaps:
  - truth: "BENCHMARKS.md 包含 hyperfine 冷启动基准段落，说明测量方法、典型延迟数值、与 v1.19 基线的对比（ROADMAP SC-2）"
    status: partial
    reason: "validate 测量结果反映的是失败退出路径（exit 2），而非成功路径。config.toml 的 directory 字段已废弃（v1.12 改名为 inputs），serde 静默丢弃该字段，validate() 立即因 inputs=[] 报错退出。Phase 72 的 2.2ms 与 Phase 9 的 ~2.8ms 来自不同执行路径，-0.6ms 对比数值不可作为冷启动改进的证据（CR-01）。BENCHMARKS.md 未对此偏差作任何标注或脚注。"
    artifacts:
      - path: "benches/BENCHMARKS.md"
        issue: "第 753 行 validate 对比行显示 -0.6ms 改进，但此值来自失败路径，与 Phase 9 成功路径测量不可比。第 777 行有 'Warning: Ignoring non-zero exit code.' 但无解释性注释。"
    missing:
      - "在 validate 对比行添加脚注（Option A）：说明 v1.20 binary 对此 config.toml 非零退出，计时反映失败路径，不可用于断言冷启动改进/回归"
      - "或重新采集 validate 成功路径数据（Option B）：使用含 inputs = [\"sqllogs\"] 的有效 config 重新测量，替换表格中的值"
---

# Phase 72: 基准体系完善（v1.20）Verification Report

**Phase Goal:** 开发者可以用 hyperfine 测量 CLI 冷启动延迟，用 criterion `--save-baseline` 将基准结果存档到 `benches/baselines/`，版本间性能趋势有迹可循
**Verified:** 2026-06-08T11:18:34Z
**Status:** gaps_found
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | 开发者运行 `hyperfine 'sqllog2db --version'` 可得到冷启动延迟数据，结果示例记录在 BENCHMARKS.md | VERIFIED | BENCHMARKS.md 第 755-766 行含 --version 原始 hyperfine 输出（2.1ms），commit fd91d63 |
| SC-2 | BENCHMARKS.md 包含 hyperfine 冷启动基准段落，说明测量方法、典型延迟数值、与 v1.19 基线的对比 | PARTIAL | 段落存在（第 741-780 行），--version 对比有效；但 validate 计时来自失败路径（exit 2），-0.6ms 对比数值误导性，无脚注说明（CR-01，见 Gaps） |
| SC-3 | `cargo bench -- --save-baseline v1.20` 执行成功，基准结果文件保存至 criterion 默认 baseline 目录 | VERIFIED | 19 个 v1.20 目录（`benches/baselines/<group>/<id>/v1.20/`），含 benchmark.json + estimates.json + sample.json + tukey.json；commit 8da9f83 |
| SC-4 | `benches/baselines/` 目录存在且包含可用的 baseline 快照，`cargo bench -- --baseline v1.20` 可加载对比 | VERIFIED | 目录存在，19 个场景覆盖 4 个 bench 文件全部合成场景；冒烟测试通过（"No change in performance detected"） |
| SC-5 | `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，无性能退化 | VERIFIED | clippy 无警告；cargo test 912 通过（395+426+3+87+1+7）0 失败 |

**Score:** 4/5 truths verified (SC-2 PARTIAL — 构成 BLOCKER)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `benches/BENCHMARKS.md` | Phase 72 段落（hyperfine + Criterion 两部分） | VERIFIED | 第 735-803 行，三个子节齐全，checklist BENCH-01 [x] BENCH-02 [x] |
| `benches/baselines/csv_export/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在，含标准 criterion JSON |
| `benches/baselines/parser_throughput/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |
| `benches/baselines/sqlite_export/1000/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |
| `benches/baselines/filters/no_pipeline/v1.20/benchmark.json` | criterion v1.20 baseline 数据 | VERIFIED | 文件存在 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| Phase 72 对比表格 | Phase 9 历史值 ~2.9ms / ~2.8ms | 表格 Phase 9 (v1.9) mean 列 | VERIFIED | 第 750-753 行，列头和值均存在 |
| hyperfine 折叠原始输出块 | 实测 stdout | `<details><summary>hyperfine 原始输出</summary>` | VERIFIED | 第 755-780 行，两个 `<details>` 块，14 个 `<details>` 与 `</details>` 全部配对 |
| How to compare 段落 | v1.20 对比命令 | bash 代码块 `--baseline v1.20` | VERIFIED | 第 38 行，位于 How to compare bash 代码块内 |
| Phase 72 段落 Criterion 小节 | benches/baselines/ v1.20 目录 | `--save-baseline v1.20` 引用 | VERIFIED | 第 787-793 行，存档命令 + 对比命令均在文档中 |

---

### Data-Flow Trace (Level 4)

不适用 — 本 Phase 产物为静态文档（BENCHMARKS.md）和数据文件（baseline JSON），不涉及动态数据渲染。

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| BENCHMARKS.md Phase 72 段落标题唯一 | `grep -c "^## Phase 72 — 基准体系完善（v1.20）$" benches/BENCHMARKS.md` | 1 | PASS |
| Phase 72 段落位于 Phase 56 之后（line > 723） | `grep -n "^## Phase 72" benches/BENCHMARKS.md` | 第 735 行 | PASS |
| 对比表表头包含两列 | `grep -q "Phase 9 (v1.9) mean \| Phase 72 (v1.20) mean"` | 存在（第 750 行） | PASS |
| `<details>` 数量 >= 2 | `grep -c "<details>" benches/BENCHMARKS.md` | 14（其中第 755、768 行属 Phase 72） | PASS |
| BENCH-01 checklist [x] | `grep -q "^\- \[x\] BENCH-01"` | 第 800 行 | PASS |
| BENCH-02 checklist [x] | `grep -q "^\- \[x\] BENCH-02"` | 第 801 行 | PASS |
| --baseline v1.20 在 BENCHMARKS.md 出现 >= 2 次 | `grep -c "baseline v1.20"` | 2（第 38、793 行） | PASS |
| validate 二进制实际退出码 | `./target/release/sqllog2db validate -c config.toml; echo $?` | 2（exit FATAL） | FAIL — 确认 CR-01 根因 |
| cargo clippy | `cargo clippy --all-targets -- -D warnings` | 无警告 | PASS |
| cargo test | `cargo test` | 912 通过 / 0 失败 | PASS |

---

### Probe Execution

不适用 — 本 Phase 无 `scripts/*/tests/probe-*.sh` 探针。

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BENCH-01 | 72-01-PLAN.md | 开发者可以用 hyperfine 测量 CLI 冷启动延迟，结果存入 BENCHMARKS.md | PARTIAL | hyperfine --version 测量有效；validate 测量基于失败路径（CR-01），对比数值不可作为回归基准 |
| BENCH-02 | 72-02-PLAN.md | 开发者可以用 `--save-baseline` 将 criterion 结果存档到 `benches/baselines/`，版本间对比有迹可循 | SATISFIED | 19 个 v1.20 目录已纳入 repo，`--baseline v1.20` 冒烟通过 |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| benches/BENCHMARKS.md | 777 | `Warning: Ignoring non-zero exit code.` — 原始输出中有非零退出警告，但文档无解释 | WARNING | 任何未来阅读此段落的开发者无法判断 validate 2.2ms 是否可信；CR-01 已记录此问题 |
| benches/BENCHMARKS.md | 753 | `validate -c config.toml` 行显示 -0.6ms 改进，但两相对比的执行路径不同 | BLOCKER | 基准文档的核心价值是提供可比较的回归基准；失败路径 vs 成功路径的对比值会误导 Phase 73-76 的性能判断 |

---

### Human Verification Required

无 — 所有关键验证项均可通过代码检查和命令执行完成。

---

## Gaps Summary

**1 个 BLOCKER 阻止完整验收：**

**CR-01: validate 计时反映失败退出路径（gaps_found）**

BENCHMARKS.md 第 753 行记录了 validate -0.6ms 改进，但：
- `config.toml` 含废弃字段 `directory = "sqllogs"`，v1.12 后该字段已改名为 `inputs`
- serde 静默丢弃 `directory`，`inputs` 默认为空数组
- `SqllogConfig::validate()` 立即因 `inputs=[]` 返回 `ConfigError::InvalidValue` 并以 exit 2 退出
- 直接验证：`./target/release/sqllog2db validate -c config.toml` → exit 2
- Phase 9 的 ~2.8ms 是成功路径（完整 validate + print），Phase 72 的 2.2ms 是失败路径（无 validate 逻辑执行）
- 差值 -0.6ms 不代表性能改进，而是执行路径缩短

**可选修复方案（二选一）：**

**Option A（文档标注，低成本）：** 在 validate 对比行添加脚注，说明"v1.20 binary 对此 config.toml 以非零退出（inputs 字段重命名）；计时反映失败路径，不可用于断言冷启动回归/改进"。

**Option B（重新测量，高准确性）：** 用含 `inputs = ["sqllogs"]` 的有效 config 重新运行 hyperfine，记录成功路径数值，替换表格内容。

BENCH-01 的 `--version` 测量（2.1ms）不受影响，保持有效。

---

_Verified: 2026-06-08T11:18:34Z_
_Verifier: Claude (gsd-verifier)_
