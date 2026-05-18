# Phase 20: 测试覆盖深化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 20-测试覆盖深化
**Areas discussed:** VERIFICATION.md 范围, Fixture 策略, proptest 设计

---

## VERIFICATION.md 范围

| Option | Description | Selected |
|--------|-------------|----------|
| 仅 12/13/14/16（按 TEST-01） | Phase 17/18 是当前里程碑，已有 REVIEW.md + VALIDATION.md | |
| 12/13/14/15/16 一并补 | v1.3 五个阶段全部补全，一次清零历史欠账 | |
| 12/13/14/15/16/17/18 全部补 | 包含 v1.4 平行阶段，工作量最大，但一次清除所有欠账 | ✓ |

**User's choice:** 12/13/14/15/16/17/18 全部补

---

| Option | Description | Selected |
|--------|-------------|----------|
| 各阶段原目录（指哪就写哪） | 12-16 写到 milestones/v1.3-phases/，17-18 写到 phases/ | ✓ |
| 统一写到 .planning/phases/ | 12-16 需要先新建目录，结构清晰但与归档目录重复 | |

**User's choice:** 各阶段原目录

---

| Option | Description | Selected |
|--------|-------------|----------|
| 标准格式：UAT 标准 + 成功标准 + 实际验证方法 | 参照 Phase 19 VERIFICATION.md 格式 | ✓ |
| 精简版：仅列 Success Criteria 通过结果 | 最小工作量，但对未来审计不够 | |

**User's choice:** 标准格式

---

## Fixture 策略

| Option | Description | Selected |
|--------|-------------|----------|
| 程序生成（现有模式） | 延续 write_test_log() 模式，无需维护外部文件 | ✓ |
| 建立 tests/fixtures/ 目录 | 放一个小的真实 .log 片段文件，更接近真实场景 | |

**User's choice:** 程序生成

---

| Option | Description | Selected |
|--------|-------------|----------|
| CSV 输出（比较简单） | 验证 CSV 内容：行数、具体字段值、header 正确性 | ✓ |
| SQLite 输出 | 验证 SQLite 表结构与记录内容 | |
| CSV + SQLite 两种 | 最全面但工作量翻倍 | |

**User's choice:** CSV 输出

---

| Option | Description | Selected |
|--------|-------------|----------|
| 带过滤器的完整流水线 | 配置 include/exclude 过滤器，验证输出记录正确 | ✓ |
| 模板分析流水线 | enable_template_normalization → 验证 template_key 列 | ✓ |
| 字段投影 | ordered_fields 配置 → 验证输出列顺序正确 | ✓ |

**User's choice:** 三个路径全选

---

## proptest 设计

| Option | Description | Selected |
|--------|-------------|----------|
| 任意 ASCII 字符串（最简单） | proptest 默认 any::<String>()，覆盖面最广 | ✓ |
| SQL 样式字符串（自定义 strategy） | 生成更接近真实 SQL 的字符串，但需要编写自定义 strategy | |

**User's choice:** 任意 ASCII 字符串

---

| Option | Description | Selected |
|--------|-------------|----------|
| 仅 normalize_template（按 TEST-04） | 严格遵守 TEST-04 范围 | ✓ |
| normalize_template + fingerprint | 共享扫描引擎，对称性测试有意义 | |

**User's choice:** 仅 normalize_template

---

| Option | Description | Selected |
|--------|-------------|----------|
| 放入 src/pipeline/fingerprint.rs 的 #[cfg(test)] mod | 与现有 normalize_template 单元测试放在一起 | ✓ |
| 新建 tests/proptest.rs | 属性测试与单元测试分离，但增加文件数 | |

**User's choice:** 放入 fingerprint.rs

---

## Claude's Discretion

- TEST-03 各边界 case 的具体测试函数命名和测试结构
- VERIFICATION.md 中各阶段实际运行命令的具体写法
- proptest cases 数量（使用默认即可）
- proptest 字面量保护不变量的具体表达方式

## Deferred Ideas

- fingerprint() 的属性测试 — 超出 TEST-04 范围，v1.5 可考虑
- SQLite 端到端验证 — 超出 TEST-02 当前范围
- cargo llvm-cov 覆盖率门控 — v1.5 引入
