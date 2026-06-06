---
status: partial
phase: 69-watch
source: [69-VERIFICATION.md]
started: 2026-06-06T05:00:00Z
updated: 2026-06-06T05:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. 验证 WATCH-05：状态行 last 字段随时间动态更新

expected: 触发一次处理后，状态行的 last 字段在数秒后更新为 "3 seconds ago" 等动态时间（而非永远显示 "just now"）
result: [pending]

**操作步骤：**
1. `cargo build --release`
2. 准备 config.toml（watch 模式），监听一个空目录
3. `cargo run --release -- watch -c config.toml`
4. 向监听目录写入一个 .log 文件触发处理
5. 等待 5-10 秒，观察状态行的 `last` 字段是否更新为动态时间

### 2. 验证 WATCH-02：新建 .log 文件在 2 秒内触发处理

expected: 向监听目录写入 .log 文件后，CSV 输出文件被创建，行数 > 1（含 header）
result: [pending]

**操作步骤：**
1. `cargo build --release`
2. 准备 config.toml（CSV exporter，监听目录 `sqllogs/`）
3. `cargo run --release -- watch -c config.toml`（后台或另一终端）
4. 向 `sqllogs/` 写入一个 .log 文件（内容含至少一条 DM SQL log 格式的行）
5. 观察 CSV 输出文件在 2 秒内被创建，`wc -l output.csv` > 1

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
