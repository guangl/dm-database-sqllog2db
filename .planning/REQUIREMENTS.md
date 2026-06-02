# Requirements: sqllog2db

**Defined:** 2026-06-02
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

---

## Milestone v1.15 — 工程质量全面提升

### CI/CD 基础设施

- [ ] **CICD-01**: 用户推送 PR/branch 时，GitHub Actions CI 自动运行 test/clippy/fmt 全绿（三平台：ubuntu/windows/macos）
- [ ] **CICD-02**: 用户推送 tag 时，CD workflow 成功构建 4 个平台的二进制并创建 GitHub Release
- [ ] **CICD-03**: CD workflow 在 4 个 job 并行运行时正确创建 release notes（无竞争条件，发布内容完整）
- [ ] **CICD-04**: 项目包含 Cross.toml，aarch64-linux 跨编译构建无需手动干预

### e2e 测试

- [ ] **TEST-01**: run 子命令 CLI 全链路测试——给定真实输入文件，验证 CSV 输出内容与退出码；给定真实输入文件，验证 SQLite 输出与退出码
- [ ] **TEST-02**: init 子命令 assert_cmd 测试——验证生成 config.toml 的 CLI 行为、文件存在与退出码
- [ ] **TEST-03**: stats 子命令 --from/--to 边界条件 e2e 测试（空范围、边界值、无效格式拒绝）

### 代码清理

- [ ] **CLEAN-01**: stats 模块删除遗留 warn! 占位符，stats/output.rs 所有函数不超过 40 行
- [ ] **CLEAN-02**: cli/run 模块中超 40 行的函数提取为私有函数（仅拆分确实超长的，不做预防性拆分）

### Benchmark 稳定化

- [ ] **BENCH-01**: 确认 scripts/collect_bench_results.sh 存在（或补充创建），bench.yml 以信息性（non-blocking，continue-on-error）方式运行

---

## Future Requirements（延后）

| Requirement | Milestone | Notes |
|-------------|-----------|-------|
| benchmark CI 门控（regression 自动拒绝） | 未定 | 需要稳定的基准基线，v1.15 先信息性收集 |
| cargo audit 定时扫描 | 未定 | 安全扫描，低优先级 |
| 多平台 e2e matrix（Windows + Linux） | 未定 | v1.15 先本地通过，后续 CI 加平台 |

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| x86_64-unknown-linux-musl | rusqlite bundled 在 musl 下有已知 segfault |
| cargo-dist / release-plz | 对单维护者项目过度工程化 |
| 第三方覆盖率服务（Codecov 等） | 项目已有 cargo-llvm-cov，不需要外部服务 |
| trycmd 迁移 | assert_cmd 模式已成熟，迁移无收益 |
| criterion 作为 merge 门控 | CI runner 噪声 ±15%，不适合门控 |

---

## Traceability

*To be filled by roadmapper.*

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| CICD-01 | — | |
| CICD-02 | — | |
| CICD-03 | — | |
| CICD-04 | — | |
| TEST-01 | — | |
| TEST-02 | — | |
| TEST-03 | — | |
| CLEAN-01 | — | |
| CLEAN-02 | — | |
| BENCH-01 | — | |
