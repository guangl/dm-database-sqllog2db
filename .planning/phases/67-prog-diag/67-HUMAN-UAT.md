---
status: partial
phase: 67-prog-diag
source: [67-VERIFICATION.md]
started: 2026-06-05T12:00:00Z
updated: 2026-06-05T12:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. 多文件运行时进度条 [N/M] 显示
expected: 进度条以 [1/3]、[2/3]、[3/3] 形式递进，非 TTY 环境无 ANSI 序列
result: [pending]

### 2. records/sec 实时更新
expected: 进度条 message 中出现 'Xk rec/s' 或 'X rec/s' 样式字符串，随记录数增长变化
result: [pending]

### 3. ETA 字段随运行时间变化
expected: 进度条 '| eta X' 字段显示合理的剩余时间估算，随进度增加而减少
result: [pending]

### 4. encoding_error hint 输出到 stderr
expected: stderr 含 'hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030'
result: [pending]

### 5. field_missing hint 输出到 stderr
expected: stderr 含 'hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致'
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
