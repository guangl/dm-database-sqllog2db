# Feature Research

**Domain:** Rust CLI CI/CD 与工程质量改进（sqllog2db v1.15）
**Researched:** 2026-06-02
**Confidence:** HIGH

---

## Feature Landscape

### Table Stakes（用户/团队预期必须有）

缺少这些 = 工程质量感觉不完整，贡献者对项目失去信心。

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| CI：push/PR 自动运行 `cargo test` | 每个成熟 Rust 项目标配，缺少则每次提交都是盲区 | LOW | 已有 ci.yaml 骨架（三平台矩阵），需验证正确性 |
| CI：`cargo clippy -- -D warnings` 门控 | Rust 社区约定，lint 不过不能合并 | LOW | 已有 lint job，flag 已正确配置 |
| CI：`cargo fmt --check` 格式门控 | 与 clippy 并列为 Rust CI 两个最基本检查 | LOW | 已有，与 clippy 在同一 job |
| CD：打 tag 触发多平台二进制构建 | CLI 工具用户期望直接下载二进制，不想自行编译 | MEDIUM | release.yaml 已存在，覆盖 4 个 target（含 aarch64-linux cross） |
| CD：上传到 GitHub Releases | 配合 tag 构建，是 CLI 工具发布的标准路径 | LOW | softprops/action-gh-release@v3 已配置 |
| e2e：`run` 子命令主路径（完整文件输入 → CSV 输出 → 退出码 0） | 核心功能链路，没有则不算集成测试 | MEDIUM | 现有测试多为 handle_run 单元测试；缺纯 CLI argv 层面的 run→CSV 验证 |
| e2e：`validate` 成功/失败退出码 | 退出码是 CLI 的契约，未测试则随时可能静默回归 | LOW | test_cli_validate_* 已有基础，可加强断言 |
| e2e：`init` 子命令 CLI 路径（assert_cmd） | init 是用户第一个接触的命令，必须有 e2e 保障 | LOW | 现有测试用 handle_init 直接调用，缺 assert_cmd CLI 层 |
| e2e：错误路径退出码 2（EXIT_FATAL） | 3 级退出码是 v1.10 设计决策，必须 e2e 验证 | LOW | test_cli_error_uses_hint_prefix 已覆盖部分；run→EXIT_FATAL 路径待验证 |
| Cargo.lock 纳入版本控制 | 二进制项目约定，保证 CI 和本地复现构建 | LOW | 项目已有，非变更项 |

### Differentiators（可提升工程竞争力的加分项）

不是必须，但有则显著提升代码质量和维护效率。

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| 覆盖率门控（cargo-llvm-cov ≥70% 行覆盖） | 防止测试盲区扩大，量化覆盖率趋势 | MEDIUM | 已有 coverage job（--fail-under-lines 70），需验证在 CI 中稳定运行 |
| e2e：`run` 子命令 SQLite 输出路径 | CSV/SQLite 双导出器都需要 e2e 保障 | MEDIUM | 现有 test_stats_sqlite_* 针对 stats，run→SQLite 纯 e2e 路径欠缺 |
| e2e：`stats` 子命令含 --from/--to 过滤（边界条件） | v1.14 核心功能，e2e 测试锚定时间段过滤行为 | MEDIUM | test_cli_stats_with_cli_from_and_to_succeeds 已有，可加 edge case |
| e2e：`run --quiet` 输出抑制验证（stderr 真正为空） | --quiet 是 v1.12 特性，需确认 e2e 层面真正静默 | LOW | test_cli_quiet_suppresses_summary 已存在，可加强断言 |
| e2e：`run --verbose` 逐文件输出行为 | verbose 是 v1.12 特性，e2e 验证每文件一行输出 | LOW | test_cli_verbose_prints_processing_line_per_file 已有 |
| e2e：hint: 前缀格式化验证（错误输出契约） | hint: 格式是 v1.12 设计契约，e2e 层面锁定 | LOW | test_cli_error_uses_hint_prefix 已部分覆盖 |
| Benchmark CI 结果收集（JSON artifact 上传） | 使性能趋势可追踪，防止无声性能退化 | MEDIUM | bench.yml 已有 + collect_bench_results.sh，需稳定化 |
| cli/run 模块内函数超 40 行的拆分 | 降低认知负担，符合项目"函数不超过 40 行"规范 | MEDIUM | filter_processor.rs 300 行；mod.rs handle_run 是 260 行的大函数 |
| stats 模块超长函数清理（aggregate.rs 388 行，output.rs 354 行） | 两个文件超过 350 行，内部有超 40 行函数 | MEDIUM | 按照项目规范优先清理，重构前需 e2e 保护 |
| CI：`cargo doc --no-deps` + RUSTDOCFLAGS=-D warnings | 防止 rustdoc 内联链接回归（v1.14 曾出现此问题） | LOW | 已有 documentation check job，需确认稳定性 |
| CI：`cargo bench --no-run` 编译检查 | 确保 bench 代码不因普通代码变更而失去编译 | LOW | 已有 lint job 中的 Compile benchmarks 步骤 |
| 依赖安全审计（cargo audit 定期 schedule 运行） | 捕获新出现的 CVE，rusqlite/bundled 依赖链深 | LOW | 当前无 audit job；可加到 weekly schedule，不阻塞 PR |

### Anti-Features（看似合理、实则有害）

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Windows e2e 测试中测试 stdin pipe 路径 | 覆盖率看上去更高 | Windows 无 `/dev/stdin`，assert_cmd 在 CI Windows runner 中无法可靠 spawn stdin pipe | 用 `#[cfg(not(target_os = "windows"))]` 跳过 stdin 相关 e2e；tests/integration.rs 已有此模式 |
| Benchmark 回归门控（CI 失败阻塞 merge） | 想防止性能退化 | GitHub Actions 共享 runner 噪音约 20%，基准结果不可靠，门控产生大量误报 | `continue-on-error: true`（已有）+ 上传 JSON artifact 供人工比较，不作为 merge 门控 |
| Codecov/Coveralls 第三方覆盖率服务接入 | 有好看的 badge | 需要 token 配置、外部服务依赖，为内部工具引入不必要复杂度 | `cargo-llvm-cov --fail-under-lines 70` 本地门控（已有），足够 |
| trycmd 快照测试替代 assert_cmd | 测试代码量更少 | `.toml` 快照文件与 assert_cmd 混用增加认知负担；项目已有成熟 assert_cmd 模式，迁移收益低 | 继续用 assert_cmd + predicates，在已有 integration.rs 中扩充 |
| 多版本 Rust toolchain 矩阵（stable/beta/nightly） | 最大化兼容性验证 | 项目设 `rust-version = "1.85"`，nightly/beta 测试意义有限，显著增加 CI 时间 | 仅 stable，依赖 dtolnay/rust-toolchain@stable（已有） |
| 独立的 e2e 测试文件（tests/e2e.rs） | 看上去结构更清晰 | tests/integration.rs 已有 1940 行且测试命名良好，新建文件会分裂测试上下文，`cargo test --test` 过滤需改变 | 在 integration.rs 中新增 `mod e2e_run` / `mod e2e_stats` 模块分区，利用已有 helper |
| CI 中运行 criterion 基准并以性能变化百分比门控 | 自动化性能保障 | CI runner 时钟不稳定，跨运行结果不可比较；criterion 需热身 + 多次迭代才能收敛，单次 CI 运行不可信 | 只编译基准（--no-run），运行时上传 JSON，人工或跨多次运行比较 |

---

## Feature Dependencies

```
GitHub Actions CI（test/clippy/fmt）
    └──requires──> cargo test 全部通过（包括现有 e2e）

GitHub Actions CD（多平台构建 + Releases）
    └──requires──> CHANGELOG.md 中存在对应版本条目（extract changelog 步骤）
    └──独立于 CI，在 tag push 时触发

e2e CLI 集成测试（run/stats/validate/init）
    └──requires──> assert_cmd + predicates（已在 dev-dependencies）
    └──enhances──> CI 覆盖率门控（覆盖率随 e2e 增加而提升）
    └──should precede──> cli/run 模块拆分（e2e 是重构的安全网）

cli/run 模块拆分
    └──conflicts with──> 同时修改 e2e 测试（防止重构导致测试变更难以审查）
    └──should follow──> e2e 测试覆盖 run 路径

stats 模块重构整理
    └──enhances──> stats e2e 测试稳定性（重构后函数边界更清晰）
    └──should follow──> stats e2e 测试覆盖

Criterion benchmark 稳定化
    └──requires──> scripts/collect_bench_results.sh 正确运行
    └──enhances──> bench.yml CI artifact 上传
```

### Dependency Notes

- **CD 需要 CHANGELOG.md 版本条目**：release.yaml 的 `Extract changelog` 步骤用 awk 从 CHANGELOG.md 提取版本发布说明，若无对应版本节则回退为通用文本，不会 fail，但发布说明质量差。
- **e2e 测试依赖已有 assert_cmd**：assert_cmd 和 predicates 已在 dev-dependencies，无需添加新依赖。
- **模块重构必须在 e2e 测试之后**：重构改变内部结构，e2e 测试是重构的安全网，两者应在不同 phase。
- **覆盖率门控依赖 e2e 测试增加**：新增 e2e 测试后覆盖率会提升，有助于持续满足 ≥70% 门控。

---

## MVP Definition

### v1.15 Launch With（必须交付）

这些是 v1.15 里程碑"工程质量全面提升"必须交付的项目。

- [ ] CI workflow 稳定运行（test/clippy/fmt 全部绿色，三平台） — 工程基础，无此则后续 PR 无保障
- [ ] CD workflow 稳定运行（4 个 target 构建 + GitHub Releases） — 交付物门控
- [ ] e2e：`run` 子命令全链路（CSV 输出、多文件、边界条件） — 核心功能链路
- [ ] e2e：`stats` 子命令含时间段过滤（--from/--to 组合，edge cases） — v1.14 特性的 e2e 锁定
- [ ] e2e：`validate` 成功/失败路径退出码强化断言 — 已有基础，补全
- [ ] e2e：`init` CLI 路径（assert_cmd 而非 handle_init 直接调用） — 补完 init 的 e2e 层

### Add After Validation（v1.15 后期阶段）

- [ ] cli/run 模块内超 40 行函数的拆分 — 触发条件：e2e 全部通过提供安全网之后
- [ ] stats 模块超 40 行函数的清理 — 同上，有测试保障后进行
- [ ] cargo audit 定期 schedule job — 触发条件：基础 CI/CD 稳定后附加

### Future Consideration（v1.16+）

- [ ] bench CI 历史对比（跨 PR 比较 JSON artifact） — 需要额外脚本/服务，当前收集已够
- [ ] 覆盖率 badge（本地生成 SVG，不依赖外部服务） — 需要 GitHub Pages 集成

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| CI test/clippy/fmt 门控（稳定化） | HIGH | LOW（已有骨架） | P1 |
| CD 多平台构建 + Releases（稳定化） | HIGH | LOW（已有骨架） | P1 |
| e2e run 子命令全链路 | HIGH | MEDIUM | P1 |
| e2e stats + 时间段过滤 | HIGH | MEDIUM | P1 |
| e2e init CLI 路径 | MEDIUM | LOW | P1 |
| e2e validate 退出码强化 | MEDIUM | LOW | P1 |
| 覆盖率门控（≥70%，验证稳定） | MEDIUM | LOW（已有） | P2 |
| cli/run 模块超长函数拆分 | MEDIUM | MEDIUM | P2 |
| stats 模块超长函数清理 | MEDIUM | MEDIUM | P2 |
| benchmark 稳定化（artifact 上传） | LOW | MEDIUM | P2 |
| cargo audit schedule | LOW | LOW | P3 |

**Priority key:**
- P1: v1.15 里程碑必须交付
- P2: v1.15 里程碑尽量交付，不影响发布
- P3: 后续版本

---

## Ecosystem Reference（工程模式对比）

以下为 Rust CLI 项目 CI/CD 的行业参考，非竞争者分析。

| Practice | 行业惯例（ripgrep/fd 等） | sqllog2db v1.14 现状 | v1.15 目标 |
|---------|--------------------------|---------------------|-----------|
| 三平台 CI | ubuntu/windows/macos | 已有 ✓ | 确认稳定 |
| 覆盖率门控 | 许多项目无，有则 ≥60-80% | ≥70% 行覆盖 ✓ | 确认稳定运行 |
| 多平台 CD | 含 musl、arm 等 target | 4 个 target（含 aarch64-linux cross） ✓ | 确认稳定 |
| 基准追踪 | criterion + artifact 上传 | bench.yml + collect_bench_results.sh | 需稳定化 |
| assert_cmd e2e | 标准做法 | 已用，覆盖 stats/validate/verbose/quiet 路径 | 补全 run/init e2e 层 |
| cargo doc 编译检查 | 常见 CI 步骤 | 已有 RUSTDOCFLAGS=-D warnings ✓ | 确认稳定 |
| 依赖安全审计 | cargo audit 定期运行 | 无 | 可选附加 |

---

## Sources

- assert_cmd crate 文档: https://docs.rs/assert_cmd
- predicates crate 文档: https://docs.rs/predicates
- alexwlchan 2025 assert_cmd 实践: https://alexwlchan.net/2025/testing-rust-cli-apps-with-assert-cmd/
- rust-cli book testing 章节: https://rust-cli.github.io/book/tutorial/testing.html
- cargo-llvm-cov: https://github.com/taiki-e/cargo-llvm-cov
- cross-rs GitHub Actions: https://blog.ediri.io/how-to-cross-compile-your-rust-applications-using-cross-rs-and-github-actions
- actions-rust-lang/audit: https://github.com/actions-rust-lang/audit
- 项目现有工作流: .github/workflows/ci.yaml, release.yaml, bench.yml
- 项目现有集成测试: tests/integration.rs（65 个 #[test]，24 处 assert_cmd 用法，1940 行）

---

*Feature research for: Rust CLI CI/CD 与工程质量改进*
*Researched: 2026-06-02*
