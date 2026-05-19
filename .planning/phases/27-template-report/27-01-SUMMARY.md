# 27-01: TemplateReporter CSV + 配置 + handle_run 集成

**Status:** Complete
**Tasks:** 3/3
**Self-Check:** PASSED

## What Was Built

创建了独立模板报告系统的基础设施：
- `TemplatesReportConfig` 配置段 + `[templates]` TOML 支持
- `TemplateReporter` struct 含 `write_csv()` 完整实现
- `write_sqlite()` stub（由 27-02 完成）
- `derive_template_report_paths()` 自动文件名派生
- `templates_report_enabled()` 配置开关
- `handle_run()` 顺序和并行路径均已集成

## Key Files

| File | Action |
|------|--------|
| src/pipeline/mod.rs | 添加 TemplatesReportConfig + derive functions |
| src/pipeline/template_reporter.rs | 新建，TemplateReporter + write_csv + write_sqlite stub |
| src/config/mod.rs | Config.templates 字段 |
| src/cli/run/mod.rs | 集成 TemplateReporter（顺序+并行路径） |
| src/exporter/csv/mod.rs | writer module 改为 pub(crate) |

## Verification

- `cargo clippy --all-targets -- -D warnings` 通过
- `cargo test --lib` 430 测试全部通过（含 4 个新测试）
- 向后兼容：旧 `[template].output_csv_path` 在 `[templates]` 未配置时正常工作
