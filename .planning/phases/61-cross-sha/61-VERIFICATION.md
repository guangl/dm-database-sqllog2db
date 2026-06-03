---
phase: 61-cross-sha
verified: 2026-06-03T14:00:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
re_verification: false
---

# Phase 61: Cross.toml SHA256 Digest 固定 — 验证报告

**Phase Goal:** 将 Cross.toml 中 aarch64-linux 构建镜像的 `:edge` 浮动标签替换为固定 SHA256 digest，任意时刻执行 `cross build` 都使用相同镜像层，构建结果可复现。
**Verified:** 2026-06-03T14:00:00Z
**Status:** passed
**Re-verification:** No — initial verification (补写于 milestone 完成前)

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Cross.toml 镜像引用格式为 `@sha256:<digest>`，不含 `:edge` 或其他浮动标签 | VERIFIED | `grep "@sha256:" Cross.toml` 返回 1 行非注释行；`grep -v "^#" Cross.toml \| grep ":edge"` 返回空 |
| 2 | SHA256 digest 为有效 64 位十六进制字符串，注释记录了来源信息和查询时间 | VERIFIED | digest `de04c9cd16fb41658de2eb0177481cb2fc717128b784d565bafcb000250508d7`（64 hex chars）；Cross.toml 含 5 行注释块，含镜像名、标签、SHA、查询时间 2026-06-03 |
| 3 | `tests/cross_config.rs` 三项断言全部通过：SHA 存在、:edge 不存在、digest 为有效 64 hex | VERIFIED | `cargo test --test cross_config`：3/3 通过（`cross_toml_has_exactly_one_sha256_image_reference` / `cross_toml_has_no_floating_edge_tag` / `cross_toml_sha256_digest_is_valid_64_hex_chars`） |
| 4 | 质量门禁全部通过，Cross.toml 变更不影响本机 Rust 编译 | VERIFIED | `cargo clippy --all-targets -- -D warnings`：0 warnings；`cargo test`：68 passed；`cargo fmt --check`：exit 0 |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cross.toml` | `image = "...@sha256:<64-hex>"`，无 `:edge` | VERIFIED | 见 Truth 1/2 |
| `tests/cross_config.rs` | 3 个自动化断言覆盖 CROSS-01 三项成功标准 | VERIFIED | 文件存在，3 个 `#[test]` 函数全部绿 |
| `.planning/phases/61-cross-sha/61-01-SUMMARY.md` | 记录 SHA、实现细节、质量验证结果 | VERIFIED | 文件存在，含 SHA digest、Before/After diff、Task 1-3 完成确认 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Cross.toml` image 行 | `ghcr.io/cross-rs/aarch64-unknown-linux-gnu@sha256:de04c9...` | `@sha256:` 格式引用 | WIRED | `grep -v "^#" Cross.toml` 确认唯一非注释镜像行使用 SHA digest |
| `tests/cross_config.rs` | `Cross.toml` | `fs::read_to_string("Cross.toml")` | WIRED | 测试直接读取 Cross.toml 进行格式断言 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| SHA 格式正确 | `cargo test --test cross_config` | 3 passed | PASS |
| clippy 无警告 | `cargo clippy --all-targets -- -D warnings` | exit 0，0 warnings | PASS |
| 全部测试通过 | `cargo test` | 68 passed, 0 failed | PASS |
| fmt 格式干净 | `cargo fmt --check` | exit 0 | PASS |
| Cross.toml 无浮动标签 | `grep -v "^#" Cross.toml \| grep ":edge"` | 空（0 行） | PASS |

### Manual-Only Verifications

| Behavior | Requirement | Why Manual | Status |
|----------|-------------|------------|--------|
| SHA256 digest 从 ghcr.io registry 查询获取 | CROSS-01 | 一次性 live registry 查询，无法在 CI 中可重现地重放 | JUSTIFIED — 结果已永久固定在 Cross.toml 注释中（查询时间 2026-06-03） |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CROSS-01 | 61-01-PLAN.md | Cross.toml SHA digest 固定，替换 edge 浮动标签 | SATISFIED | SHA 格式验证测试 3/3 通过；Cross.toml 非注释行无 `:edge`；注释含可追溯来源信息 |

---

## Gaps Summary

**1 项 partial（justified）：**
- Task 1（SHA digest 从 live registry 查询）为 manual-only：属一次性外部 registry 操作，结果已固定在代码中，无需自动化重放。

无其他 gaps，所有可自动化的成功标准均通过测试覆盖。

---

_Verified: 2026-06-03T14:00:00Z_
_Verifier: Claude (manual VERIFICATION.md —补写于 milestone complete 前，基于 61-01-SUMMARY.md 和 tests/cross_config.rs 证据)_
