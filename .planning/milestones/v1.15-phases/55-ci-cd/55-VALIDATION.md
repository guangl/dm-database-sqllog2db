---
phase: 55
slug: ci-cd
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-02
---

# Phase 55 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | GitHub Actions workflow lint（无本地 test framework） |
| **Config file** | `.github/workflows/*.yaml` |
| **Quick run command** | `cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check` |
| **Full suite command** | Push PR → 观察 GitHub Actions CI tab |
| **Estimated runtime** | ~5 分钟（GitHub Actions CI run） |

---

## Sampling Rate

- **After every task commit:** Run `cargo clippy --all-targets -- -D warnings`（Rust 代码无变动，YAML/TOML 修改后快速 lint）
- **After every plan wave:** Push PR 到 GitHub，验证 CI 全绿
- **Before `/gsd:verify-work`:** 全部 workflow 文件 YAML 语法检查通过 + GitHub Actions 实际运行绿灯
- **Max feedback latency:** ~300 秒（GitHub Actions CI）

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 55-01 | 01 | 1 | CICD-01/02/03 | — | N/A | smoke | Push PR → Actions tab 全绿 | ✅ ci.yaml（修改后）| ⬜ pending |
| 55-02 | 01 | 1 | CICD-04 | — | N/A | smoke | 文件存在 + cross build 成功 | ❌ Cross.toml（需新建）| ⬜ pending |
| 55-03 | 01 | 1 | CICD-02/03 | — | N/A | smoke | Push tag → release job 成功，4 个产物上传 | ✅ release.yaml（重构后）| ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cross.toml` — aarch64-unknown-linux-gnu 跨编译配置（CICD-04）

*其他 workflow 文件已存在，Wave 0 只需新建 Cross.toml。*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CI 三平台全绿 | CICD-01 | GitHub Actions 无法本地完全模拟 | Push PR，观察 Actions tab，ubuntu/windows/macos 三个 test job 均绿灯 |
| Release 4 平台二进制构建 | CICD-02 | 需要实际 tag push 触发 | Push `v0.0.0-test` tag，观察 release.yaml，4 个 build job 全绿，GitHub Release 创建成功 |
| Release body 内容完整无竞争条件 | CICD-03 | 竞争条件需要并行执行才能暴露 | 检查创建的 GitHub Release body 内容完整，changelog 提取正确 |
| aarch64-linux 跨编译成功 | CICD-04 | Cross 工具链环境依赖 | release.yaml 的 aarch64-linux build job 绿灯，产物正确上传 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
