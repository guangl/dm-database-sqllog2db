# 发版门禁标准

本标准适用于从 `v*` 标签创建的正式 Release。任何门禁失败、取消或未完成，都不得创建 GitHub Release。标准由仓库工作流执行，不能以人工观察内存或 informational benchmark 代替。

## 必须通过的检查

| 门禁 | 验收条件 |
| --- | --- |
| 功能测试 | Linux、Windows、macOS 的 `cargo test --locked` 全部通过；已有明确标注 ignored 的测试仍按现状处理 |
| 代码质量 | rustfmt、Clippy 零警告、文档零警告、benchmark 编译全部通过 |
| 覆盖率 | 行覆盖率 ≥ 70%，沿用现有 CI 标准 |
| 导出内存 | CSV、SQLite 各测 1、4、16 份日志，每份约 256 MiB；每个场景独立启动进程 |
| 峰值上限 | 每个导出进程峰值 RSS ≤ 128 MiB；不是虚拟内存，也不是退出后的内存差值 |
| 增长上限 | 同格式 4、16 文件场景分别相对 1 文件场景，峰值 RSS 增量 ≤ 32 MiB |
| 数据完整性 | 记录数精确一致；CSV 全文件 SHA-256 与独立构造的预期文件一致；SQLite 所有导出字段符合预期且 `quick_check=ok` |
| 执行可靠性 | 每次导出最多 600 秒；非零退出、超时、缺失/无效 RSS、检查异常均失败，不得跳过后放行 |
| 可追溯性 | JSON 报告包含二进制 SHA-256、CI 提交 SHA、平台、阈值、每场景数据和最终结论 |

内存数据固定为合成 SELECT 日志，参数替换关闭、无事务级过滤，最大输入约 4 GiB、19,701,680 条记录。内存阈值仅约束该标准场景，不代表所有异常超长 SQL、参数缓存或事务过滤场景都有相同内存上限。功能语义、异构数据、追加、字段投影和拆分另由 Rust 测试覆盖。

Windows 必须通过功能测试与发布构建；当前 RSS 门禁仅支持 Linux/macOS，不宣称验证了 Windows 的内存。发布仍需三个受测平台全部通过。

## 自动执行与发布阻断

- PR 和 main 推送：CI 调用 `export-memory.yaml`，在 Linux x86_64、Linux aarch64、macOS aarch64 构建 release 程序并执行相同门禁。
- `v*` 标签：`release.yaml` 的 `verification` 调用完整 CI；`export-memory` 等待发布产物构建完成，在对应原生平台下载并检测**将要发布的实际二进制**。标签 CI 不重复执行源代码构建版内存门禁。
- `publish-crate.needs` 同时依赖 `verification`、`export-memory` 和全部构建任务。标签必须与 `Cargo.toml` 版本一致，且仓库需配置具有该 crate 发布权限的 `CARGO_REGISTRY_TOKEN` secret。门禁通过后执行 `cargo publish --locked`；发布失败会阻止 `create-release` 创建 GitHub Release。
- `create-release` 等待 `publish-crate` 成功。没有 `continue-on-error`，发布步骤没有 `always()` 绕过失败。
- 检测报告以 Actions artifact 保留 90 天；成功发版时三个平台的 JSON 报告同时作为 Release 附件，文件名为 `export-memory-<platform>.json`。
- 普通 Criterion benchmark 仍是参考数据，不作为内存门禁的替代。600 秒是卡死保护，不是吞吐性能目标。

仓库里的工作流修改提交并推送后生效；本文不意味着远端分支保护设置已经修改。即使没有配置 PR 必需状态检查，标签发布流程自身仍会等待这些门禁。

## 本地复现

需要 Rust、Python 3.9+，以及 macOS 的 `/usr/bin/time` 或 Linux 的 GNU time。建议预留至少 10 GiB 临时磁盘空间。

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps
cargo bench --locked --no-run
cargo llvm-cov --locked --fail-under-lines 70
cargo build --release --locked
python3 -B -m unittest discover -s scripts -p 'test_export_memory.py' -v
python3 scripts/check_export_memory.py target/release/sqllog2db \
  --file-mib 256 --files 16 --max-rss-mib 128 --max-growth-mib 32 \
  --timeout-seconds 600 --report export-memory-report.json
```

退出码 0 才是通过。自定义较小输入适合调试，不能代替标准命令；调整阈值或样本规模必须通过正常代码评审并提供各受测平台的新证据，不能为了让失败版本发版而临时跳过。

## 本次本地验收（2026-09-10）

本机 Rust 功能测试 542 项通过（另有 3 项既有 ignored）；rustfmt、Clippy、文档检查及 benchmark 编译全部通过。行覆盖率为 89.71%，高于 70% 门槛。

macOS 修复版执行上述完整内存矩阵，6/6 场景通过：

| 格式 | 1 文件峰值 MiB | 4 文件峰值 MiB | 16 文件峰值 MiB |
| --- | ---: | ---: | ---: |
| CSV | 18.8 | 9.7 | 9.6 |
| SQLite | 29.1 | 29.4 | 29.3 |

SQLite 多文件增长不足 1 MiB；所有场景数据完整性通过。门禁自身的 9 项失败路径测试通过，覆盖峰值/增长超限、记录数/内容错误、无效/缺失 RSS、非零退出、超时和异常报告。

修复前程序使用 1、4 份同规模日志做负向验证：SQLite 4 文件峰值约 876 MiB，同时超过峰值和增长上限；脚本退出码为 1，确认门禁能拦截此次回归。该负向测试只需复现失败，不作为修复版 16 文件验收的替代。

原始证据：[修复版通过报告](test-results/export-memory-macos.json)、[旧版被拒报告](test-results/export-memory-before-rejected.json)。本地尚未执行远端 Linux/Windows runner；跨平台结果以推送后对应 GitHub Actions 为准。
