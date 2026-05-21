# Pitfalls Research

**Domain:** Rust CLI data processing tool — parsing DaMeng SQL logs, streaming to CSV/SQLite
**Researched:** 2026-05-21
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: 错误类型重构时丢失 Fatal vs. Non-fatal 的界限

**What goes wrong:**
在 processor.rs 的热循环中（process_log_file 第 100 行），`exporter_manager.export_one_preparsed(...)?` 使用 `?` 传播错误——写入失败（如磁盘满）会直接终止整个导出流程。当前代码中 parse 错误单独走 `log::warn!` 路径（非致命），而 export 错误走 `?` 路径（致命）。添加"continue-on-error"时，开发者可能不加区分地让所有错误变为非致命，导致磁盘满等致命错误被静默忽略，用户最终得到不完整的输出文件而不自知。

**Why it happens:**
- `#[from]` 自动生成 `From` impl，使得 `Exporter::export_one_preparsed` 返回的 `Result<()>` 能自动转化为 `Error::Export` 变体。热路径上开发者只需写 `?`，容易忘记区分哪些错误是致命的、哪些可以继续。
- 当前 error.rs 的 `Error` 枚举将所有变体（Config/File/Parser/Export/Io）放在同一层级，没有"致命/非致命"分类标记。
- processor.rs 中对 parse 错误的处理是"碰到 Err 就走 log::warn 然后 continue"，而不是返回错误结果——这是两条完全不同的路径。重构时容易把两条路径混用。

**How to avoid:**
1. 在 `Error` 枚举（或通过方法）显式标记哪些变体是 fatal 的：`fn is_fatal(&self) -> bool`。
2. 热循环中单独处理可恢复错误——用 match 而非 `?`：匹配 `Err(Error::Export(ExportError::WriteFailed{..}))` 时记录警告并继续，匹配其他错误时终止。
3. 为"记录级别"错误新建轻量级 `RecordError` 类型，与终止性 `FatalError` 类型分开。
4. 调研 `#[error(transparent)]` 替代 `#[from]`+`#[error("...")]` 组合，获得更清晰的错误链。

**Warning signs:**
- 编译报错说 `?` 无法自动转换错误类型（重构时破坏了 `#[from]` 关系）。
- 新增的错误变体没有 `is_fatal()` 覆盖，导致 `match` 语句出现未处理分支。
- 测试中 `export_one_preparsed` 返回 `Err` 后，预期继续处理但实际终止了。

**Phase to address:**
ERR-01（错误类型细分）和 ERR-02（非致命错误继续处理）必须放在一起做，否则先细分再改 continue 会破坏已有逻辑。建议先设计 fatal/non-fatal 分类，再一次性重构热循环。

---

### Pitfall 2: Stdin 输入与 LogParserBuilder 的文件路径假设冲突

**What goes wrong:**
`dm-database-parser-sqllog` 的 `LogParserBuilder::new(path).build()` 内部调用 `fs::read(&self.path)` 从文件系统读取全部数据到 `Vec<u8>`。它不接受 `std::io::Read` trait 对象。这意味着：
- **直接传入 "-" 不可行**——build() 会尝试打开名为 "-" 的文件。
- **无法流式处理 stdin**——`LogParser` 内部持有 `Vec<u8>` 并基于 `&[u8]` 迭代，不支持分批数据到达。
- **pre-scan 阶段也会崩溃**——`scan_for_trxids_by_transaction_filters` 同样调用 `LogParserBuilder::new(file_path)`，两种输入方式不能走同一条代码路径。

此外，SqllogParser 的 `log_files()` 方法调用 `path.exists()`、`path.is_file()`、`std::fs::read_dir()`——这些对 stdin 全部不适用，会直接 panic 或返回空列表。

**Why it happens:**
- 上游 crate 的设计假设：所有数据通过文件路径加载到内存后再解析。这个假设在 v1.0 使用 mmap 时更强，当前 v1.1 改用 `fs::read` 后仍是"全量加载"模式。
- 当前架构的层层抽象都围绕文件路径展开：`SqllogParser` -> `log_files()` -> `process_log_file(file_path, ...)` -> `LogParserBuilder::new(file_path)`。
- 代码中没有任何一层使用 `io::Read` 抽象——每个函数都接收 `&str` 文件路径。

**How to avoid:**
1. 在 `LogParserBuilder` 层加 stdin 分支：如果路径是 `"-"`（或配置特殊标记），从 `std::io::stdin()` 读取全部数据到 `Vec<u8>`，然后手动构造 `LogParser { data, encoding }`（LogParser 的字段是 `pub(crate)` 但我们可以直接构造结构体，因为它是上游 crate 中的 pub 类型）。
2. 或者：在 stdin 路径下禁用 pre-scan（事务级过滤需要预扫描，而 stdin 不能预扫描），回退到顺序过滤。
3. 进度条在 stdin 模式下应显示"总字节数未知"的 spinner 风格，而非确定进度条，因为文件大小未知。
4. 确保 `--help` 明确指出 stdin 的限制：不支持事务级过滤、不支持 pre-scan。

**Warning signs:**
- 用户运行 `cat huge.log | sqllog2db run` 后立即报 `No such file or directory: '-'` 或类似的路径错误。
- stdin 模式下 pre-scan 阶段仍然尝试打开文件。
- 进度条在 stdin 模式下卡在 0% 或显示错误的总记录数。

**Phase to address:**
PIPE-01（stdin 管道输入）。必须与 ERR-02（继续处理）协调——stdin 因为不能 pre-scan，事务级过滤会失效，这个限制需要清晰的错误信息告知用户。

---

### Pitfall 3: 进度条更新插入热循环导致性能退化

**What goes wrong:**
当前 processor.rs 的热循环（约 5.2M records/sec）对每次记录做：解析、过滤、normalize、CSV 格式化写入。如果在每次记录后增加进度条更新（`progress_bar.inc(1)`），每调用一次就需要：
- 获取内部锁（Mutex）
- 计算并重新绘制进度条（Terminal I/O to stderr）
- 至少一次系统调用

以 5.2M records/sec 计算，每条记录约 192ns。一次进度条锁获取+I/O 操作大约是 1-10μs 级别——放进来性能会下降 50-100 倍。

**Why it happens:**
- 进度条库（indicatif）的 `inc()` 每次都有内部锁和潜在终端 I/O，不是零开销。
- 开发者看到 `inc()` 很简单，容易直接写在热循环里。
- 当前代码已经有一个 `show_progress: bool` 参数和一个 per-file 级别的进度（`eprintln!("[{i}/{n}] ...")`），性能开销极小。替换成 per-record 进度条会反转这个设计。

**How to avoid:**
1. **保持 per-file 级别的进度显示**：每次打开新文件时更新时间，不在热循环内更新。当前的做法（`eprintln!("[{i}/{n}]")`）已经是最优方案。
2. 如果确实需要 per-record 进度条，限定更新频率：每 N 条记录更新一次，N=1024 或更高（参考 processor.rs 已有的 `trailing_zeros() >= 10` 中断检测模式，复用这个计数器）。
3. 使用 `AtomicU64` 做跨线程计数器，仅在主线程定期检查，避免 rayon 的并行线程互锁竞争。
4. 进度条更新只在文件开关时做，不做 per-record 更新——这是当前架构的最优匹配。

**Warning signs:**
- 添加进度条后基准测试从 5.2M records/sec 下降到 < 1M records/sec。
- 在批量测试中观察到大量 stderr 输出（progress bar 重绘痕迹）。
- CPU profile 显示 `std::sync::Mutex::lock` 或 `write(2, ...)` 系统调用占据大量时间。

**Phase to address:**
UX-01（进度显示）。实现时参考 processor.rs 第 102-108 行的 `records_in_file.trailing_zeros() >= 10` 模式，将进度更新与已有的中断检测对齐。

---

### Pitfall 4: 错误信息过度工程化——错误码系统 vs 内联上下文

**What goes wrong:**
添加"更好的错误信息"时，一个常见倾向是设计数值错误码（E001、E002...）、错误目录文档、用户需要"查表"才能理解的模式。对于 sqllog2db 这样的 CLI 工具，用户只想看到一句话告诉我"哪错了+怎么修"，不需要翻文档查错误码。

同时，另一个倾向是过度保留错误链导致信息冗余。当 `#[from]` 生成 `Error::Io(io::Error)`，上层再用 `#[error("IO error")]` 包装时，最终输出可能是 "IO error: IO error: file not found" 这种重复前缀。

**Why it happens:**
- 从大型企业项目迁移过来的经验会自然引入错误码系统。
- `thiserror` 的 `#[from]` 会让内部错误自动参与 Display，而外层再额外加描述就产生重复。
- 开发者希望错误信息"专业"而设计层次过多的错误链。

**How to avoid:**
1. **不要引入数值错误码**。sqllog2db 的领域确定性高（文件 IO、解析、导出），错误种类有限。对每个错误变体写一条清晰的中文/英文描述就够了，error.rs 目前已经做得不错。
2. 错误信息包含三个要素：发生了什么、在哪发生的、怎么修。
   - "发生了什么"：当前已有（如 "Configuration file not found"）
   - "在哪发生的"：当前已有（文件路径、字段名）
   - "怎么修"：**缺失的**。例如 "Configuration file not found: /path/to/config.toml. Run `sqllog2db init -o config.toml` to create a default config."
3. 避免 `#[error("Export error: {0}")]` 这种包装——直接透传。使用 `#[error(transparent)]` 让内部错误直接通过 Display 暴露。
4. 在热路径（parse error 已走 `log::warn!`）中避免不必要的 `Result` 上下文转换——给每条 parse error 添加行号、文件路径、记录摘要就足够了。

**Warning signs:**
- 建立了 `error_codes.rs` 或 `errors.md` 文件来枚举错误码。
- PR 中的错误变更增加了错误链嵌套层数而非减少。
- 用户反馈错误信息太"啰嗦"难以定位真正的问题（如多层 "Caused by:..."）。

**Phase to address:**
UX-04（更好的错误信息）。与 ERR-01 和 ERR-02 放在一起做，因为错误类型细分直接影响错误信息的结构。

---

### Pitfall 5: 清理死代码时遗漏测试引用或配置兼容性

**What goes wrong:**
FIX-01（清理 normalize_template）和 FIX-03（清理 FileError::ReadFailed）表面上是"删代码"，但：
- `normalize_template` 可能在测试模块有引用（先确认 src/ 下已无引用，但 `#[cfg(test)]` 模块可能仍然引用）。
- `FileError::ReadFailed` 被移除后，任何现有的 `match` 表达式如果覆盖了 `FileError` 所有分支就会编译失败。
- `ConfigError` 可能在不经意间被解构使用，移除变体导致 match 编译失败。
- [template] 配置段（FIX-02）静默接受的问题：当前代码有 `template_deprecated: Option<toml::Value>` 字段，如果直接删除这个字段，旧配置仍然会被 serde 静默忽略（未知字段默认行为），起不到"拒绝"的效果。

**Why it happens:**
- Rust 编译器在 `match` 表达式上有穷尽性检查——移除 enum 变体后，所有 match 该 enum 的地方都会编译失败。在多人开发时，其他人可能添加了新的 match 分支。
- serde 的默认行为是静默忽略未知字段——要显式拒绝需要 `#[serde(deny_unknown_fields)]`。
- 至少运行 `cargo test` 才能发现测试中的引用，仅 `cargo build` 不够。

**How to avoid:**
1. **三步确认法**：
   - `cargo build` 确认编译通过
   - `cargo test` 确认测试通过
   - `grep -r "被删除的标识符" src/` 确认无任何残留引用
2. 移除 `FileError::ReadFailed` 前，先确认没有任何 match `FileError` 的分支（因为其他变体没有匹配 ReadFailed 的情况）。直接从 enum 中删除，让编译器指出所有需要修改的位置。
3. 对 `[template]` 配置段：不删除 `template_deprecated` 字段，而是在 `validate()` 中检查该字段并返回显式错误信息。或者为整个 `Config` 结构体添加 `#[serde(deny_unknown_fields)]`，但这需要确保所有字段都通过 serde 反序列化。
4. 如果 enum 是公开的（pub），移除变体是公共 API 的破坏性变更——需要注意语义版本控制。

**Warning signs:**
- `cargo build` 成功后 `cargo test` 失败（测试中的引用）。
- 用户报告使用 `[template]` 配置时静默无警告（说明拒绝机制没生效）。
- `cargo clippy` 报 `used_underscore_binding` 警告（`template_deprecated` 字段被编译器诊断为未使用）。

**Phase to address:**
FIX-01/FIX-02/FIX-03。应该在 v1.10 项目一开始就做——这些是"低 hanging fruit"，清理完跑一遍全测试链就能验证。

---

### Pitfall 6: 在热路径中新增错误分类时破坏内联和零成本抽象

**What goes wrong:**
当前热路径（processor.rs:58-128、csv/writer.rs:22-209）经过精心优化：
- `pipeline.is_empty()` 快速路径避免虚表调用
- `BufWriter::with_capacity(2MB)` 减少系统调用
- `itoa::Buffer` 复用避免分配
- `line_buf: Vec<u8>` 复用避免 per-record 分配
- `ns_scratch: Vec<u8>` 复用避免 per-record 分配

如果在热路径中增加"记录每个错误的详细信息到 error log"、或"收集统计计数"、或"更新进度条"，这些看似小的改动可能会破坏内联、增加分支、增加分配或增加锁竞争，导致性能从 5.2M records/sec 显著下降。

**Why it happens:**
- 热路径优化的特点是"不做什么"——不做日志、不做锁、不做分配。新功能往往需要做这些事。
- `#[inline]` 标注的函数（csv/writer.rs 第 22 行 `write_record_preparsed`）如果增加复杂度，编译器可能无法再内联。
- 在多条 error 路径上增加分支会影响分支预测。

**How to avoid:**
1. 在 processor.rs 中使用 `log::debug!` 而非 `log::warn!` 或 `log::info!` 来记录处理中的内部状态——debug 日志在默认日志级别下会被编译器优化掉（如果使用了 `log` crate 的条件编译）。
2. 错误统计使用 `AtomicUsize`（无锁原子操作）而非 `Mutex<usize>`。
3. 所有新增的"记录级别"处理只走 `match Err(e) => { log::warn!; continue }` 路径，不走 exporter 路径，避免锁/IO 开销。
4. 在合并 PR 前执行 `cargo bench`（criterion 基准），确认性能退化 < 5%。
5. CSV writer 的 `line_buf` 现在有一个 `clear()` -> `extend` -> `write_all` 模式。不要在热路径中复制 `line_buf` 或使用 `String`（UTF-8 验证开销）。

**Warning signs:**
- CI 中基准测试结果出现显著下降。
- `perf` 或 `flamegraph` 显示新函数（如 `collect_error_stats`）占据大量 CPU 时间。
- 热路径中出现 `String::new()`、`format!()`、`clone()`、`Mutex::lock()`、`HashMap::insert()`。

**Phase to address:**
ERR-02（继续处理）和 UX-04（错误信息）。实现时先画清楚"这条 path 是否在 hot loop 内"，hot loop 外的处理可以随意，hot loop 内的必须逐行审查。

## 技术债务模式

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `#[from]` 过度使用：为所有子错误添加 `#[from]` 属性 | 编写时只需写 `?`，template code 最少 | 丢失错误类型区分度；重构时 `From` 冲突；上层无法区分致命/非致命 | 错误类型少且不会扩展时可用，v1.10 扩展后应逐步替换为手动 From |
| `toml::Value` 字段捕获废弃配置（template_deprecated、pipeline_deprecated） | 以最小成本检测旧配置 | 字段永远留在 struct 中，序列化时可能泄漏 | 临时迁移期（一个版本），不应长期保留。v1.10 应改为 validate() 中显式拒绝 |
| `#[allow(clippy::...)]` 在热路径上 | 避免了 pedantic lint 误报 | 累积后掩盖真实问题 | 可接受，但每个 allow 应注释理由。Cargo.toml 中的全局 allow 比局部 allow 更差 |

## 集成陷阱

| 集成点 | 常见错误 | 正确做法 |
|--------|----------|----------|
| dm-database-parser-sqllog (LogParserBuilder) | 假设其 API 支持 stdin/stream | 它是全量加载模型，不支持流式。stdin 支持需要直接构造 `LogParser` 或 fork 修改上游 |
| rayon 并行池 | pre-scan 和 parallel CSV 使用不同线程池（都是独立创建的），可能导致资源竞争 | 两个阶段共享线程池或明确 Serialize 执行顺序——当前设计已隔离，但需要注意 |
| rusqlite + mmap_size (sqlite/mod.rs:37) | stdin 模式下 sqlite 还是写本地文件，没有问题。但如果未来输出到 stdout 会有冲突 | stdout 和 sqlite 互斥，当前设计已经限制为一种导出器，保持此限制 |
| ctrlc crate (中断处理) | stdin 模式下 Ctrl+C 处理需要额外小心——stdin 关闭后可能要 flush 已导出数据 | 在 stdin EOF 后正常 finalize，中断信号只应中止读取阶段 |

## 性能陷阱

| 陷阱 | 症状 | 预防 | 何时爆发 |
|------|------|------|----------|
| 热循环内调用 progress_bar.inc() | 50-100 倍性能下降 | 每 1024 条更新一次（复用已有计数模式） | 在 5M records/sec 合成基准测试中立即暴露 |
| 每个记录都写 error log | 大量 IO 写入，HDD 场景尤其严重 | 使用 `log::warn!`（默认写入文件）且限频率，或在内存中累积到阈值再写入 | 在包含大量解析错误的日志上 |
| `String::from_utf8_lossy` 在热路径中 | 每条记录检查 UTF-8 合法性 | `Vec<u8>` 直接操作，仅在最后输出时验证 | 始终存在 |
| 在 `export_one_preparsed` 中做额外的克隆或分配 | CSV writer 中 `line_buf` 使用 `clear`+`extend` 无分配 | 预分配 capacity、复用 buffer | 始终存在，但合成基准测试中放大 |
| 在热路径中使用 `thiserror` 的 Display（`#[error("...")]`） | 错误构造时 format 分配 | 预先分配错误字符串或使用 Cow | 仅在错误路径触发时，不频繁则无害 |

## UX 陷阱

| 陷阱 | 用户影响 | 更好的做法 |
|------|----------|------------|
| stdin 模式下 without progress 或无输入反馈 | 用户不知道程序是否卡住 | stdout 非交互时，eprintln 输出一行 "Processing stdin..." 即可 |
| 错误信息没有"解决办法" | 用户知道哪错了但不知道如何修正 | 每条用户可见错误都附带建议，如 "Run `sqllog2db init -o config.toml` to create a default config" |
| --help 文档与实际支持的 --stdin 等新参数不同步 | 用户不知道新功能存在 | 在帮助文本中明确标注 stdin 支持及其限制 |
| 进度显示与日志输出混在一起 | stderr 被进度条和 log 信息同时占用，显示混乱 | 进度条独占 stderr 的"尾行"，日志走 log 文件；或进度条仅在非 quiet 且 stdout 不是 pipe 时显示 |
| 对错误码的依赖 | 用户需要查表理解错误 | 直接写人类可读的错误信息，不要使用 E001/E002 等编码 |

## "Looks Done But Isn't" 检查清单

- [ ] **stdin 支持**: 不只是在 parser 层支持，pre-scan (prescan.rs:52) 也必须处理 stdin 不可 pre-scan 的情况——事务级过滤会静默失效，是否有警告？
- [ ] **非致命错误继续**: 不只是修改错误类型，热循环 (processor.rs:58-128) 中的 `?` 必须改为 `match` + continue。导出器 (exporter/csv/mod.rs:203) 的 `write_all` 也需要捕获 Err 而非传播。
- [ ] **进度显示**: `show_progress` 现在只是一个 bool，传入 process_log_file 后只在文件完成时输出。改为真实进度条后，需要确保在 Ctrl+C 中断时进度条正确关闭。
- [ ] **错误信息改进**: 新增的错误变体必须在所有 `match Error` 的位置添加分支。当前 `handle_run` (cli/run/mod.rs:152) 只处理 `Error::Interrupted`，其他错误自然传播到 main——main 中的错误格式化需要同步更新。
- [ ] **死代码清理**: 删除代码后运行 `cargo clippy --all-targets -- -D warnings`，不只是 `cargo build`。clippy 会检测未使用的导出项。
- [ ] **配置验证**: `[template]` 拒绝逻辑需要在 `Config::validate()` 中实现，同时保持 `template_deprecated` 字段用于 serde 反序列化时的检测，才能给出友好的迁移提示。

## 恢复策略

| Pitfall | 恢复成本 | 恢复步骤 |
|---------|----------|----------|
| 错误类型重构导致匹配失败 | LOW | 编译器会明确显示哪些 match 缺少分支，逐处修复即可 |
| 性能退化 | MEDIUM | 恢复之前版本的热路径，用 `git bisect` 定位退化提交，重新设计 |
| stdin 路径下遗漏 pre-scan 警告 | LOW | 加一行 `warn!("stdin mode: transaction-level filters disabled")` |
| 进度条导致性能崩溃 | MEDIUM | 移除 per-record 更新，改为每 1024 条或基于时间的更新 |
| [template] 配置段仍然静默接受 | LOW | 在 `validate()` 中检查并报错；或在 struct 上添加 `#[serde(deny_unknown_fields)]` |

## 阶段映射

| Pitfall | 预防阶段 | 验证方式 |
|---------|----------|----------|
| Pitfall 1: 致命/非致命界限模糊 | ERR-01 + ERR-02（必须同阶段做） | 单元测试验证注入 IO 错误后程序继续处理。冒烟测试验证磁盘满时给出清晰错误 |
| Pitfall 2: stdin 与文件路径假设冲突 | PIPE-01 | stdin 模式下 `cargo test` 通过。手动测试 `echo "2025..." | cargo run -- run` 能正常工作 |
| Pitfall 3: 热循环进度条性能退化 | UX-01 | `cargo bench` 对比 baseline，退化 < 5% |
| Pitfall 4: 错误码过度工程化 | UX-04 + ERR-01 | PR 审查确保无数值错误码设计 |
| Pitfall 5: 死代码清理遗漏测试/配置兼容 | FIX-01/02/03 | 三步骤：`cargo build` + `cargo test` + `grep -r 标识符 src/` |
| Pitfall 6: 热路径破坏内联和零抽象 | ERR-02 + UX-01 | `cargo bench` + 代码审查确认热路径无分配/锁 |

## Sources

- 代码分析：当前 processor.rs 热循环、error.rs 错误类型定义、LogParserBuilder 内部实现（source code at `~/.cargo/registry/src/.../dm-database-parser-sqllog-1.1.0/src/parser.rs`）
- v1.7 审计报告：`.planning/milestones/v1.7-MILESTONE-AUDIT.md`（具体引用 normalize_template、FileError::ReadFailed、[template] 配置段三个已知债务）
- thiserror 文档分析：`#[from]` 陷阱——对同一类型只能有一个 `#[from]` 变体；`#[from]` 生成自动 `From` impl；过度使用 `#[from]` 增加重构时冲突风险
- indicatif 性能特征：内部锁机制 + 终端 I/O 开销，per-record 更新在高吞吐场景不可行
- 当前性能基线：5.2M records/sec（合成基准），1.55M records/sec（1.1GB 真实文件）——来自 project CLAUDE.md

---
*Pitfalls research for: sqllog2db v1.10 quality improvements*
*Researched: 2026-05-21*
