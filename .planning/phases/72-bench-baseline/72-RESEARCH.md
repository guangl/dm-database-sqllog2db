# Phase 72: 基准体系完善 - Research

**Researched:** 2026-06-08
**Domain:** Criterion baseline archiving + hyperfine CLI startup measurement
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** 测量命令与 Phase 9 保持一致：`hyperfine --warmup 3 './target/release/sqllog2db --version'` 和 `hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'`
- **D-02:** BENCHMARKS.md 新增 "Phase 72 — 基准体系完善（v1.20）" 段落，记录：测量命令、hyperfine 原始输出（详情折叠）、与 Phase 9（v1.9 ~3ms）的对比数值
- **D-03:** 若 hyperfine 未安装（CI 环境），BENCHMARKS.md 使用占位说明，不阻断 CI；hyperfine 安装方式写入文档（`brew install hyperfine`）
- **D-04:** 使用 `CRITERION_HOME=benches/baselines` 环境变量，baseline 存档至 `benches/baselines/`，纳入 repo
- **D-05:** 运行全部 4 个 bench files：bench_csv、bench_sqlite、bench_filters、bench_parser，命令：
  ```bash
  CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20
  ```
- **D-06:** 更新 BENCHMARKS.md 的 "How to compare against this baseline" 段落，添加 v1.20 对比命令示例
- **D-07:** baselines/.gitignore 已排除 `**/new/` 和 `**/report/`；确认 v1.20 JSON 数据文件不在排除范围内
- **D-08:** 在 BENCHMARKS.md 文件末尾追加新段落，不修改历史段落
- **D-09:** 新段落包含：hyperfine 冷启动数值（含与 Phase 9 对比）、criterion v1.20 baseline 说明、存档命令

### Claude's Discretion
- hyperfine `--warmup` 次数（3 次为历史惯例，可保持）
- 是否额外测量 `sqllog2db run`（I/O bound，不稳定，可跳过）
- criterion 运行样本数（使用 criterion 默认值）

### Deferred Ideas (OUT OF SCOPE)
- hyperfine CI 自动化（在 bench.yml 中加入冷启动测量步骤）
- hyperfine `--export-json` 输出存档
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BENCH-01 | 开发者可以用 hyperfine 测量 CLI 冷启动延迟，结果存入 BENCHMARKS.md | hyperfine 1.20.0 已安装；历史基线（Phase 9 ~3ms）已记录在 BENCHMARKS.md；测量命令确认可用 |
| BENCH-02 | 开发者可以用 `--save-baseline` 将 criterion 结果存档到 `benches/baselines/`，版本间对比有迹可循 | criterion 0.7.0 `--save-baseline` flag 已验证；`CRITERION_HOME=benches/baselines` 模式已存在多个历史 baseline；.gitignore 已正确排除临时文件 |
</phase_requirements>

---

## Summary

本 phase 是纯文档与数据采集任务，无代码改动。目标是为 v1.20 里程碑建立两类基准档案：(1) hyperfine 冷启动延迟测量并记录至 BENCHMARKS.md；(2) criterion `--save-baseline v1.20` 将全部 benchmark 组的 JSON 数据存档至 `benches/baselines/`。

环境已完全就绪：hyperfine 1.20.0 已安装，release binary（v1.16.0）已构建，criterion 0.7.0 `--save-baseline`/`--baseline` flag 已通过现有 baseline（v1.0、phase33、phase44-before/after）验证，`CRITERION_HOME=benches/baselines` 模式是本项目既有惯例，baselines/.gitignore 已正确排除 `**/new/` 和 `**/report/`，v1.20 JSON 数据文件不受排除影响。

本 phase 无需安装任何新包，无代码变更，无 CI 配置改动。两个任务均为线性操作：运行命令 → 采集输出 → 追加文档。

**Primary recommendation:** 先 release build 确保二进制最新，再运行 hyperfine 两条命令采集数值，最后运行 `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` 存档全部 criterion baselines，追加 BENCHMARKS.md 新段落。

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI 冷启动延迟测量 | CLI Binary | — | 直接测量 release binary 的进程启动到退出全链路时间 |
| criterion throughput baseline 存档 | 构建/测试层 | 文件系统 | criterion 写入 benchmark.json/estimates.json/sample.json/tukey.json 到 CRITERION_HOME 指定目录 |
| BENCHMARKS.md 文档更新 | 文档层 | — | 纯追加操作，不触及代码路径 |

---

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| hyperfine | 1.20.0 | CLI 冷启动延迟测量（wall-clock time，统计报告） | 已安装 [VERIFIED: local env]；Phase 9 历史惯例 [CITED: benches/BENCHMARKS.md] |
| criterion | 0.7.0 | Rust micro-benchmark 框架，`--save-baseline` 存档 JSON | 已在 Cargo.toml 声明为 dev-dep [VERIFIED: Cargo.toml]；`--save-baseline` flag 已确认 [VERIFIED: criterion CLI help] |
| CRITERION_HOME env var | — | 重定向 criterion 写入路径至 `benches/baselines/` | 本项目既有模式，Phase 4/42/44 均使用 [CITED: benches/BENCHMARKS.md §"How to compare against this baseline"] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| cargo build --release | — | 确保测量对象是最新 release binary | hyperfine 前必须先构建 |
| git add + commit | — | 将 v1.20 baseline JSON 纳入版本管理 | criterion 写入后提交 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `--save-baseline v1.20` | `--save-baseline base`（默认） | default 名称 "base" 无版本语义，不利于历史追溯；v1.20 明确对应里程碑 |
| CRITERION_HOME 重定向 | criterion 默认 `target/criterion/` | 默认目录被 .gitignore 排除，无法持久化；CRITERION_HOME 到 benches/baselines/ 是本项目既有模式 |

**Installation:** 无需安装新包。hyperfine 已就位（`/opt/homebrew/bin/hyperfine`）。

---

## Package Legitimacy Audit

本 phase 不安装任何新外部包。现有依赖（criterion 0.7.0、hyperfine 1.20.0）均已在本项目中使用多个 phase，无需重新审计。

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
[Task 1: hyperfine 冷启动测量]
  cargo build --release
       ↓
  hyperfine --warmup 3 './target/release/sqllog2db --version'
  hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
       ↓
  stdout 输出（mean ± σ, min…max, runs 数）
       ↓
  手动记录至 BENCHMARKS.md 新段落（与 Phase 9 ~3ms 对比）

[Task 2: criterion --save-baseline v1.20]
  CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20
       ↓
  criterion 运行 4 bench files（bench_csv / bench_sqlite / bench_filters / bench_parser）
       ↓
  写入 benches/baselines/<group>/<id>/v1.20/{benchmark,estimates,sample,tukey}.json
       ↓
  git add benches/baselines/ && commit
       ↓
  BENCHMARKS.md 追加 v1.20 对比命令说明

[验证]
  CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
       ↓
  criterion 输出 "Performance has improved / regressed / No change" 信息
```

### Recommended Project Structure

本 phase 不新增目录结构。criterion `--save-baseline v1.20` 会在现有目录下自动创建子目录：

```
benches/
├── baselines/
│   ├── .gitignore                  # 已有：排除 **/new/ 和 **/report/
│   ├── csv_export/<size>/v1.20/    # 新增（由 criterion 自动创建）
│   ├── csv_export_real/real_file/v1.20/
│   ├── csv_format_only/10000/v1.20/
│   ├── filters/<scenario>/v1.20/
│   ├── parser_throughput/<size>/v1.20/
│   ├── sqlite_export/<size>/v1.20/
│   ├── sqlite_export_real/real_file/v1.20/
│   └── sqlite_single_row/<size>/v1.20/
└── BENCHMARKS.md                   # 追加 Phase 72 新段落
```

### Pattern 1: CRITERION_HOME + --save-baseline

**What:** 通过环境变量将 criterion 数据目录重定向到版本管理目录，再用 `--save-baseline <name>` 创建命名快照
**When to use:** 需要持久化 benchmark 基线供跨版本对比时
**Example:**
```bash
# Source: benches/BENCHMARKS.md §"How to compare against this baseline"
# 存档全部 bench groups 的 v1.20 baseline
CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20

# 后续版本对比（会输出 improved/regressed/no change）
CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
```

criterion 会为每个 benchmark group 写入如下文件（已验证于 baselines/csv_export/1000/v1.0/）：
- `benchmark.json` — benchmark 元数据（group_id, value_str, throughput 配置）
- `estimates.json` — 统计估计（mean, median, std_dev, slope, MAD 置信区间）
- `sample.json` — 原始采样数据
- `tukey.json` — Tukey 围栏离群值分析

### Pattern 2: hyperfine 冷启动测量

**What:** 统计多次运行的 wall-clock time，自动过滤 OS cache 差异（通过 warmup）
**When to use:** 需要量化 CLI 进程启动到退出的端到端延迟时
**Example:**
```bash
# Source: benches/BENCHMARKS.md §Phase 9
# 与 Phase 9 保持相同命令（D-01）
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
```

输出格式（Phase 9 实测参考）：
```
Benchmark 1: ./target/release/sqllog2db --version
  Time (mean ± σ):       2.9 ms ±   0.4 ms    [User: 1.7 ms, System: 0.8 ms]
  Range (min … max):     2.5 ms …   5.9 ms    356 runs
```

### Anti-Patterns to Avoid

- **不要在 debug build 上运行 hyperfine：** debug binary 启动延迟数倍于 release，测量无意义。必须先 `cargo build --release`。
- **不要忘记提交 baseline JSON：** criterion 写入后必须 `git add benches/baselines/`，否则 v1.20 baseline 只在本地，后续 Phase 73-74 的对比会失败。
- **不要修改 BENCHMARKS.md 历史段落：** D-08 明确要求仅追加，历史数值是回归比较的参照基准。
- **不要混淆 `--save-baseline` 和 `--baseline`：** 前者写入快照（此 phase 用），后者读取快照进行对比（后续 Phase 73-74 用）。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI 启动延迟测量 | 自写 Rust 测试计时 | hyperfine | 处理 OS 缓存、进程 fork 开销、统计异常值；shell 内置 `time` 只给一次测量，无统计可靠性 |
| Benchmark baseline 存档格式 | 自定义 JSON 格式 | criterion `--save-baseline` | criterion estimates.json 已包含置信区间、MAD、slope 等统计量；--baseline 对比逻辑由 criterion 内置，无需手写比较脚本 |

---

## Common Pitfalls

### Pitfall 1: release binary 过期

**What goes wrong:** hyperfine 测量的是旧版本 binary，数值与当前代码不对应
**Why it happens:** `cargo build --release` 未在测量前运行，或运行了 `cargo bench` 但忘了 release binary 是单独构建的
**How to avoid:** 任务顺序严格保持：`cargo build --release` → hyperfine 测量
**Warning signs:** binary 的版本号（`--version` 输出）与当前 Cargo.toml version 不一致

### Pitfall 2: CRITERION_HOME 路径错误导致 baseline 写入 target/

**What goes wrong:** 忘记设置 `CRITERION_HOME=benches/baselines`，criterion 写入默认的 `target/criterion/` 目录（被 .gitignore 排除），v1.20 baseline 无法提交
**Why it happens:** 直接运行 `cargo bench -- --save-baseline v1.20` 而未设置环境变量
**How to avoid:** 始终使用 `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20`（D-04/D-05 已锁定）
**Warning signs:** `ls benches/baselines/csv_export/` 找不到 v1.20 子目录

### Pitfall 3: csv_export_real / sqlite_export_real 在本地跳过

**What goes wrong:** `sqllogs/` 目录不存在时，bench_csv 和 bench_sqlite 的 real-file 场景自动跳过（`skip` 分支），baseline 中无 real_file/v1.20/
**Why it happens:** 真实日志文件体积大（538MB×2），不在 repo 中
**How to avoid:** 这是已知设计行为（见 Phase 4 注释），不影响 BENCH-02 验收。只需确认合成 benchmark groups（csv_export、sqlite_export、filters、parser_throughput）的 v1.20 baseline 已写入。
**Warning signs:** criterion 输出 "sqllogs/ not found, skipping xxx benchmark" — 这是正常信息，不是错误

### Pitfall 4: git add 未包含 baselines/ 下的新 JSON 文件

**What goes wrong:** `git add src/` 或 `git add .` 未覆盖 `benches/baselines/`，v1.20 JSON 未入库
**Why it happens:** 提交时遗漏了 benches/ 路径
**How to avoid:** 提交时显式 `git add benches/baselines/` 或 `git add benches/`
**Warning signs:** `git status` 显示 `benches/baselines/.../v1.20/` 为 untracked

---

## Code Examples

### 完整执行序列

```bash
# Source: benches/BENCHMARKS.md + 72-CONTEXT.md §Specific Ideas

# Step 1: 确保 release binary 最新
cargo build --release

# Step 2: hyperfine 冷启动测量（D-01）
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'

# Step 3: criterion v1.20 baseline 全套存档（D-05）
CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20

# Step 4: 验证 baseline 已写入
ls benches/baselines/csv_export/1000/v1.20/
ls benches/baselines/filters/no_pipeline/v1.20/
ls benches/baselines/parser_throughput/1000/v1.20/
ls benches/baselines/sqlite_export/1000/v1.20/

# Step 5: 验证 --baseline 可正常加载（D-06 的示例命令验证）
CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
```

### BENCHMARKS.md 新段落模板

```markdown
## Phase 72 — 基准体系完善（v1.20）

**Date:** 2026-06-08
**Goal:** 建立 v1.20 里程碑冷启动基线（BENCH-01）+ criterion throughput baseline 存档（BENCH-02）
**Test environment:** Apple Silicon (Darwin 25.5.0), release build (`opt-level=3`, LTO=fat, strip=symbols, panic=abort)

### CLI 冷启动基线（hyperfine）

测量命令（与 Phase 9 保持一致）：

```bash
hyperfine --warmup 3 './target/release/sqllog2db --version'
hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'
```

| 命令 | Phase 9 (v1.9) mean | Phase 72 (v1.20) mean | 变化 |
|------|--------------------|-----------------------|------|
| `--version` | ~2.9 ms | [测量值] | [差值] |
| `validate -c config.toml` | ~2.8 ms | [测量值] | [差值] |

<details>
<summary>hyperfine 原始输出（--version）</summary>

```
[粘贴 hyperfine 输出]
```

</details>

<details>
<summary>hyperfine 原始输出（validate）</summary>

```
[粘贴 hyperfine 输出]
```

</details>

### Criterion v1.20 Baseline 存档

baseline JSON 已存档至 `benches/baselines/`，可用于 Phase 73–74 性能对比：

```bash
# 存档命令（已执行）
CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20

# 后续版本对比命令
CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
```

criterion 输出将标注 "Performance has improved" / "Performance has regressed" / "No change in performance detected"。
```

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| hyperfine | BENCH-01 冷启动测量 | ✓ | 1.20.0 | D-03：CI 环境无 hyperfine 时 BENCHMARKS.md 使用占位说明，不阻断 |
| cargo / rustc | BENCH-02 criterion | ✓ | cargo 1.94.0 | — |
| release binary | BENCH-01 | ✓ | sqllog2db 1.16.0 | 运行 `cargo build --release` 更新 |
| config.toml | validate 命令测量 | ✓ | 项目根目录存在 | — |
| criterion 0.7.0 | BENCH-02 | ✓ | 0.7.0 (Cargo.lock 确认) | — |
| benches/baselines/ | BENCH-02 baseline 存档 | ✓ | 含多个历史 baseline | — |

**Missing dependencies with no fallback:** 无

**Missing dependencies with fallback:**
- hyperfine（CI 环境）：D-03 已定义占位说明方案，不阻断 CI

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | criterion 0.7.0 |
| Config file | Cargo.toml `[[bench]]` sections |
| Quick run command | `cargo test` |
| Full suite command | `cargo clippy --all-targets -- -D warnings && cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-01 | hyperfine 输出可复现，数值记录在 BENCHMARKS.md | manual | `hyperfine --warmup 3 './target/release/sqllog2db --version'` | ✅ (binary exists) |
| BENCH-02 | `cargo bench -- --save-baseline v1.20` 成功，baseline JSON 存在 | manual + file check | `ls benches/baselines/csv_export/1000/v1.20/benchmark.json` | ❌ Wave 0 需执行后验证 |
| BENCH-02 | `cargo bench -- --baseline v1.20` 可加载并输出对比 | smoke | `CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20` | ❌ Wave 0 需执行后验证 |

> 注：本 phase 无代码改动，成功标准是文件存在性检查 + 命令无错误退出，而非 unit test。

### Sampling Rate

- **Per task commit:** `cargo clippy --all-targets -- -D warnings && cargo test`
- **Per wave merge:** 同上
- **Phase gate:** clippy + test 全绿 + baseline JSON 文件存在性确认

### Wave 0 Gaps

- baseline JSON 文件（`benches/baselines/*/v1.20/benchmark.json`）— 需执行 `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` 后生成
- BENCHMARKS.md Phase 72 段落 — 需 hyperfine 测量后填入实测数值

---

## Security Domain

本 phase 无代码变更，无网络请求，无用户输入处理，无新依赖引入。ASVS 所有分类均不适用。

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| criterion 默认写 `target/criterion/`（不入库） | `CRITERION_HOME=benches/baselines` 重定向（入库） | Phase 4（v1.1） | baseline 可跨版本对比 |
| 无 CLI 冷启动测量 | hyperfine `--warmup 3` 多运行统计 | Phase 9（v1.9） | 量化进程启动延迟，与 50ms 后台化门控挂钩 |
| criterion 0.5.x API | criterion 0.7.0（breaking changes in 0.6） | 当前 Cargo.toml | `criterion_group!/criterion_main!` 宏 API 无变化；`--save-baseline` flag 语义不变 |

**Deprecated/outdated:**
- criterion `--baseline-lenient`：加载 baseline 时若某 bench 无对应 baseline 则跳过（非中止）。Phase 72 用 `--save-baseline` 而非此 flag，无需关注。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | criterion `--save-baseline v1.20` 在 criterion 0.7.0 下的路径格式为 `$CRITERION_HOME/<group>/<id>/v1.20/` | Architecture Patterns | [ASSUMED] — 基于现有 baselines 目录结构（v1.0、phase44-before 等）推断；如格式变化，baseline 路径验证步骤会失败，但不影响数据写入 |

> A1 风险极低：现有多个历史 baseline（v1.0、phase33、phase44-before/after）均遵循此路径格式，且 criterion 0.7.0 的 `--save-baseline` flag 通过 `cargo bench -- --help` 已确认存在 [VERIFIED: local env]。

**If this table is empty:** 不适用，A1 为唯一假设项，风险可接受。

---

## Open Questions

无阻塞性问题。以下为信息性记录：

1. **jsonl_export baseline 目录已存在但 Cargo.toml 无对应 bench target**
   - What we know: `benches/baselines/jsonl_export/` 目录存在，但 Cargo.toml 中未注册 `[[bench]]` 名为 `bench_jsonl`
   - What's unclear: 该目录可能来自历史实验性代码，已被移除
   - Recommendation: 本 phase 忽略，`--save-baseline v1.20` 只会写入当前注册的 4 个 bench targets；不需要清理

2. **csv_export_real / sqlite_export_real 的 v1.20 baseline 将缺失**
   - What we know: real-file 场景依赖 `sqllogs/` 目录（538MB 真实日志），不在 repo 中；bench 代码自动 skip
   - What's unclear: 不影响 BENCH-02 验收（合成 benchmark 覆盖已足够）
   - Recommendation: BENCHMARKS.md 新段落注明 real-file baseline 未采集（与 Phase 4 处理方式一致）

---

## Sources

### Primary (HIGH confidence)
- `benches/BENCHMARKS.md` — hyperfine Phase 9 历史数值、CRITERION_HOME 使用模式、历史 baseline 目录结构
- `benches/baselines/.gitignore` — 确认 `**/new/` 和 `**/report/` 被排除，v1.20 JSON 文件不受影响
- `Cargo.toml` — criterion 0.7.0 dev-dep 声明，4 个 bench targets 注册
- `cargo bench --bench bench_csv -- --help` 输出 — `--save-baseline`/`--baseline` flag 存在性确认 [VERIFIED: local env]
- `benches/baselines/csv_export/1000/v1.0/` 目录 — baseline JSON 文件格式确认（benchmark.json、estimates.json、sample.json、tukey.json）[VERIFIED: local env]
- `.planning/phases/72-bench-baseline/72-CONTEXT.md` — 所有锁定决策

### Secondary (MEDIUM confidence)
- `Cargo.lock` — criterion 0.7.0 checksum 确认 [VERIFIED: local file]

### Tertiary (LOW confidence)
- 无

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — hyperfine 和 criterion 均已在本地验证；CRITERION_HOME 模式已有多个历史用例
- Architecture: HIGH — 基于现有 baselines 目录结构直接推断，无需引入新概念
- Pitfalls: HIGH — 全部来自本项目历史 benchmark phase 的实际经验

**Research date:** 2026-06-08
**Valid until:** 2026-07-08（criterion API 稳定，hyperfine CLI 稳定）
