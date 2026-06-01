# Milestones: sqllog2db

## v1.12 — CLI 体验全面提升

**Shipped:** 2026-06-01  
**Phases:** 46–49 | **Plans:** 8 | **Commits:** 57  
**Duration:** 2026-05-31 → 2026-06-01 (~1 day)  
**Code changes:** 38 files, +3,307 / -432 lines  
**Tests:** 529 total (226 lib + 48+ integration + 1 jemalloc), all passing

### Delivered

全面改善用户与 sqllog2db 的交互体验：错误信息结构化并附带 `hint:` 修复建议、配置模板带注释、validate 命令输出清晰、`--verbose`/`--quiet` 运行控制、`inputs: Vec<String>` glob 展开支持。

### Key Accomplishments

1. 致命错误统一 `hint:` 前缀格式，`format_error_output` 纯函数可单元测试（ERROR-01/02）
2. `validate` 静默通过 `Configuration valid.` / 失败输出 `[FAIL] reason\n  hint: ...`（CONFIG-02）
3. `init` 模板所有 exporter 字段补全行内注释（CONFIG-01）
4. `--verbose` 逐文件 `Processing:` 输出，`--quiet` 完全抑制进度与摘要（LOG-01/02/03）
5. `inputs: Vec<String>` 替代 `path: String`，config 和 `--input` CLI flag 均支持 glob 展开（INPUT-01/02）
6. `assert_cmd`/`predicates` 加入 dev-dependencies，e2e CLI 测试覆盖率大幅提升

### Archives

- `.planning/milestones/v1.12-ROADMAP.md` — 完整 Phase 细节
- `.planning/milestones/v1.12-REQUIREMENTS.md` — 需求归档（9/9 满足）
- `.planning/milestones/v1.12-MILESTONE-AUDIT.md` — 审计报告

---

*Previous milestones tracked in git history. See `.planning/milestones/` for v1.7, v1.10, v1.11 archives.*
