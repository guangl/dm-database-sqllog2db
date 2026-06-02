# Research Summary: sqllog2db v1.15 工程质量全面提升

*Synthesized from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Research date: 2026-06-02*

## Executive Summary

sqllog2db v1.15 的核心任务是"让已经存在的工程基础设施真正可靠地运行"，而不是新建功能。项目已有完整的 `.github/workflows/` 骨架（ci.yaml / release.yaml / bench.yml）、覆盖率门控、65 个集成测试和成熟的 assert_cmd e2e 模式，但存在若干已知问题导致这套基础设施实际上无法无误运行：`actions/checkout@v6` 版本不存在（当前最新是 v4）、`actions/upload-artifact@v7` 引入了 ESM 破坏性变更、release workflow 4 个并行 job 同时写入 release body 存在竞争条件。修复这些问题是 v1.15 的第一优先级，其余工作都依赖 CI/CD 首先稳定。

测试覆盖层面，现有 `tests/integration.rs` 对 `stats` / `validate` / `quiet` / `verbose` 子命令有良好的 assert_cmd 覆盖，但 `run` 子命令的 CLI 级别全链路 e2e（argv → CSV/SQLite 输出 → 退出码）和 `init` 子命令的 assert_cmd 路径存在明显缺口。由于 `run` 是项目的核心数据路径，缺乏 e2e 保障意味着任何后续重构都在没有安全网的情况下进行。**e2e 测试补全必须先于任何代码重构**，这是研究得出的最关键排序结论。

架构层面，`cli/run/mod.rs` 当前仅 263 行且已经经过多轮精简，不需要拆分；stats 模块职责边界清晰，只需删除遗留的 `warn!` 占位符并检查 `output.rs` 中是否有超 40 行的子函数。最高风险点是 CD 构建中的 `rusqlite bundled + cross aarch64-linux`——cross-rs 默认 Docker 镜像不保证包含完整 C 工具链头文件，需要 `Cross.toml` 配置。

---

## Stack Additions

| Tool | Version | Purpose |
|------|---------|---------|
| `actions/checkout` | `@v4` | 修正：当前 workflow 错误使用 @v6（不存在） |
| `actions/upload-artifact` | `@v4` | 修正：v7 引入 ESM 破坏性变更；v3 已于 2025-01 下线 |
| `softprops/action-gh-release` | `@v2` | Node 20 LTS，比 v3 Node 24 更稳定 |
| `Swatinem/rust-cache` | `@v2` | 当前 v2.9.1，保持不变 |
| `taiki-e/install-action` | `@v2` | 安装 cargo-llvm-cov + cross，保持不变 |
| `Cross.toml` | — | 新建，解决 aarch64-linux 跨编译缺头文件问题 |

**明确不使用：**
- `x86_64-unknown-linux-musl` — rusqlite bundled 在 musl 下有已知 segfault
- `cargo-dist` / `release-plz` — 对单维护者 CLI 项目过度工程化
- criterion 作为 CI merge 门控 — runner 噪声 ±15%，只能作信息性用途

---

## Feature Table Stakes vs Differentiators

### P1 — 必须交付（Table Stakes）

| Feature | Notes |
|---------|-------|
| CI workflow 稳定运行 | 三平台 test/clippy/fmt/coverage 全绿 |
| CD workflow 稳定运行 | 4 个 target + GitHub Releases |
| e2e: `run` 子命令全链路 | CSV 输出、SQLite 输出、退出码 |
| e2e: `init` 子命令 assert_cmd | 当前只有 handle_init 直调，无 CLI 层测试 |
| e2e: `stats` 时间段边界条件 | --from/--to 边界值、无效格式 |
| stats 模块小幅清理 | 删 warn! 占位符，检查 output.rs 函数长度 |

### P2 — 工程改进（Differentiators）

| Feature | Notes |
|---------|-------|
| cli/run handle_run 内部函数提取 | 仅超 40 行的函数才拆分，不做大规模重构 |
| benchmark 稳定化 | 确认 collect_bench_results.sh 存在，明确信息性定位 |
| release workflow 竞争条件修复 | 分离 create-release 和 upload-artifact job |

### Anti-features（明确不做）

- benchmark CI 门控（噪声太高）
- trycmd 迁移（现有 assert_cmd 模式已成熟）
- Windows stdin e2e（/dev/stdin 不存在）
- 第三方覆盖率服务（codecov 等）

---

## Architecture Findings

**cli/run 模块：不需要拆分**
- `mod.rs` 263 行，所有可提取逻辑已在子模块（processor.rs / prescan.rs / parallel.rs 等）
- 强行拆分只增加复杂度

**stats 模块：仅小幅清理**
- `src/stats/`（5 文件，1354 行）+ `src/cli/stats/`（1 文件，147 行）职责分离清晰
- 待处理：删除 cli/stats/mod.rs 遗留 warn! 占位符，检查 stats/output.rs（354 行）子函数长度

**新增/修改文件：**
- `.github/workflows/ci.yaml` — 修正 action 版本
- `.github/workflows/release.yaml` — 修正版本 + 修复 body_path 竞争
- `.github/workflows/bench.yml` — 修正 action 版本
- `Cross.toml` — aarch64-linux 跨编译配置（新建）
- `tests/integration.rs` — 新增 e2e_run / e2e_init 测试块

**测试策略（双层）：**
- handler 直调（`handle_run()`）→ 业务逻辑验证
- assert_cmd → CLI 参数解析、退出码、stderr 格式验证

---

## Top 5 Pitfalls

| # | Pitfall | Prevention |
|---|---------|------------|
| 1 | rusqlite bundled + cross aarch64-linux 构建失败 | 创建 Cross.toml；用测试 tag 在正式发布前验证 |
| 2 | release.yaml 4 并行 job 竞争写 release body | 新增独立 create-release job，matrix job 只上传 artifact |
| 3 | criterion 不能作 CI merge 门控 | 保持 continue-on-error；用 test_csv_throughput_baseline 做真正门控 |
| 4 | actions/checkout@v6 不存在 | 统一改为 @v4 |
| 5 | e2e 必须先于重构 | 正确顺序：e2e 覆盖 → 重构 → 覆盖率验证 |

---

## Roadmap Implications

建议 5 个阶段（严格遵循依赖顺序）：

| Phase | Name | Dependency | Risk |
|-------|------|-----------|------|
| 55 | CI/CD 基础设施修复 | 无（最先） | HIGH（阻塞性） |
| 56 | stats 模块清理 | 可与 55 并行 | LOW |
| 57 | e2e 测试扩展 | 55 完成后 | MEDIUM |
| 58 | cli/run 函数清理 | 57 完成后 | LOW |
| 59 | Benchmark 稳定化 | 可独立推进 | LOW |

**关键约束：Phase 57 必须先于 Phase 58**（e2e 是重构的安全网）

---

## Open Questions

1. `scripts/collect_bench_results.sh` 是否已存在（Phase 59 开始时确认）
2. `CHANGELOG.md` 格式是否满足 release.yaml awk 提取（推测试 tag 时验证）
3. 当前精确覆盖率基线（Phase 58 开始前运行 `cargo llvm-cov` 获取）

---

*Research confidence: HIGH — 全部基于项目代码直接阅读 + GitHub Action 版本验证*
