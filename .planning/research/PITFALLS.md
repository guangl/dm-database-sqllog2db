# Pitfalls Research

**Domain:** Rust CLI — CI/CD 基础设施、跨平台构建、模块重构、e2e 测试、benchmark 稳定化
**Researched:** 2026-06-02
**Confidence:** HIGH

---

## Critical Pitfalls

### Pitfall 1: rusqlite bundled + cross 跨平台构建在 aarch64-linux 下失败

**What goes wrong:**
`rusqlite` 使用 `features = ["bundled"]` 时，cross-rs 用 Docker 容器构建 `aarch64-unknown-linux-gnu` 目标。cross 的 Docker 镜像虽然包含 aarch64 linker，但默认镜像不一定带 C 编译器工具链的完整头文件（`limits.h` 等）；`libsqlite3-sys` 的 bundled 构建需要用 `cc` crate 在目标架构下编译 C 代码，缺头文件会导致构建立即失败。已有 GitHub Issues 记录此问题（rusqlite#939、rusqlite#871）。

**Why it happens:**
- `bundled` 特性让 `libsqlite3-sys` 在编译时用 `cc` 编译内嵌的 SQLite C 源码，这是跨编译中最脆弱的步骤。
- cross-rs 的官方镜像为常见 Rust-only 项目设计，不一定满足 C 依赖的构建需求。
- release.yaml 当前使用 `use_cross: true` for `aarch64-unknown-linux-gnu`，但没有指定 cross 的自定义 `Cross.toml` 配置。

**How to avoid:**
1. 在项目根创建 `Cross.toml`，为 `aarch64-unknown-linux-gnu` 指定带完整 C 工具链的 Docker 镜像：
   ```toml
   [target.aarch64-unknown-linux-gnu]
   image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main"
   ```
2. 或者在 release.yaml 中为 cross 构建步骤设置 `CROSS_NO_WARNINGS=0`，先观察具体报错再针对性修复。
3. 备选：使用 GitHub Actions 的 `ubuntu-latest` + 安装 `gcc-aarch64-linux-gnu` 原生交叉工具链，替代 cross。当 C 依赖较重时原生工具链比 Docker 更稳定。
4. 在 CI 中先验证 cross 编译，再把它加入 release 门控——不要直接推 tag 触发 release 才发现编译失败。

**Warning signs:**
- `cross build` 失败，错误信息包含 `cc: fatal error: limits.h: No such file or directory`。
- `libsqlite3-sys` 构建失败，`bundled` 字样在错误中出现。
- aarch64 构建 job 超时（cross 拉 Docker 镜像本身耗时较长）。

**Phase to address:**
CD 构建阶段（多平台 Release）。应在 v1.15 正式推 tag 前，先在 CI 中手动触发 aarch64 构建验证。

---

### Pitfall 2: criterion benchmark 在 CI 中产生假性回归报告

**What goes wrong:**
Criterion.rs 的统计分析无法消除 GitHub Actions 虚拟机的基础噪声（通常 ±5-15%）。即使代码没有变化，也会频繁出现 "Performance has regressed" 的报告。如果把 benchmark 结果作为 CI 门控（阻断 merge），会产生大量假阳性，导致开发者开始绕过检查；如果不设门控，benchmark 数据形同虚设。

**Why it happens:**
- GitHub Actions 的 hosted runner 是共享虚拟机，CPU 时钟会因宿主机负载而波动，Criterion 的置信区间无法覆盖宿主机级别的噪声。
- 当前 bench.yml 中已经设置 `continue-on-error: true`——这说明设计上已意识到 benchmark 不能作为硬性门控，但 `cargo bench` 的输出仍会影响开发者判断。
- 项目已有 `collect_bench_results.sh` 收集 estimates.json，但没有对比历史 baseline——每次 CI 只能看到本次数值，缺乏趋势跟踪。

**How to avoid:**
1. **保持 `continue-on-error: true`**，永远不要把 criterion 在 CI 中的结果设为 merge 门控。
2. 真正的性能门控使用集成测试中的 `test_csv_throughput_baseline`（当前已存在于 `tests/integration.rs:464`）——这是一个基于 release 构建的粗粒度门控，容差足够大（500K rec/s），不受 CI 噪声影响。
3. 如果要跟踪趋势，使用 `benchmark-action/github-action-benchmark` 将历史数据存储在独立分支，对 ±15% 以上才告警。
4. 在本地开发时使用 `--baseline` 对比（已在 BENCHMARKS.md 中记录），CI 中只做"编译检查"（`cargo bench --no-run`）。

**Warning signs:**
- CI benchmark job 每次都输出 "Performance has regressed"，但没有任何代码性能相关改动。
- 开发者开始跳过 benchmark job 或在 PR 描述中写 "bench noise, not real regression"。
- `bench.yml` 被修改为直接 skip 或 `cargo bench --no-run` 替代实际运行。

**Phase to address:**
CI 基础设施阶段。bench.yml 的 "stabilization" 核心是明确 benchmark 在 CI 中的定位：*信息性，非门控*。

---

### Pitfall 3: cli/run 模块拆分后，单元测试丢失对私有函数的访问权

**What goes wrong:**
`src/cli/run/tests.rs` 使用 `use super::*` 访问同模块下的私有函数（如 `handle_run`、内部 helper）。如果把 `mod.rs` 中的部分函数移动到新的子模块（如 `orchestration.rs`），`tests.rs` 的 `super::*` 只能访问 `mod.rs` 的内容，移走的私有函数立即变得不可见，编译失败。

**Why it happens:**
- Rust 的可见性规则：子模块可以访问父模块的私有项，但 `tests.rs`（通过 `#[path = "tests.rs"]` 或 `mod tests`）只是 `mod.rs` 的子模块，无法直接访问移走到兄弟模块的私有函数。
- `use super::*` 是一个诱人的快捷方式，但它只能拿到直接父模块暴露的项。
- 重构时容易先移动函数、后发现测试编译失败，此时需要决定"把函数改为 `pub(crate)`"还是"把测试移动到更合适的位置"。

**How to avoid:**
1. 重构前先列出 `tests.rs` 中测试的每个函数，标记其来源模块，确认移动计划不会切断访问链。
2. 被单元测试使用的内部 helper 改为 `pub(super)` 或 `pub(crate)` 可见性，而非保持私有。
3. 将单元测试随被测代码一起移动——如果 `process_log_file` 移到 `processor.rs`，对应的 unit test 也移过去，放在 `processor.rs` 底部的 `#[cfg(test)] mod tests { ... }` 块中。
4. 使用 `#[allow(unused_imports)]` 配合 CI clippy `-D warnings`——如果 `use super::*` 引入了不再需要的导入，clippy 会立刻报错提示。

**Warning signs:**
- 重构后 `cargo build` 成功但 `cargo test` 失败，报 `error[E0425]: cannot find function`。
- `tests.rs` 中出现大量 `use crate::cli::run::新模块::函数名` 的手动引入，说明 `super::*` 已不够用。
- 测试通过但 clippy 报 `unused_imports` 警告，说明部分 `super::*` 导入的函数已被移走。

**Phase to address:**
代码重构阶段（cli/run 模块拆分）。重构第一步就应该运行 `cargo test` 而非只跑 `cargo build`。

---

### Pitfall 4: release.yaml 中多个并行 job 同时上传到同一 GitHub Release 时的竞争条件

**What goes wrong:**
release.yaml 有 4 个并行的 matrix job（linux/aarch64/windows/macos），每个 job 都调用 `softprops/action-gh-release` 上传 artifact 并设置 `body_path`。当 4 个 job 几乎同时完成时，会出现：
- 后完成的 job 覆盖之前 job 写入的 release notes（body）。
- 第一个触发 release 创建的 job 和后续 job 的 body 合并出现竞争，导致最终 release body 内容随机。
- v2.5.2 前的版本有已知的 `already_exists` 竞争 bug。

**Why it happens:**
- 每个 matrix job 独立调用 `softprops/action-gh-release`，每次调用都会尝试"创建或更新" release。
- `body_path` 参数在 GitHub API 中是 PATCH 操作，多个并发 PATCH 请求会互相覆盖。
- 当前 `release.yaml` 使用的 `softprops/action-gh-release@v3` 是否包含竞争修复取决于 v3 的具体提交。

**How to avoid:**
1. **将 release body 的写入与 artifact 上传分离**：新增一个独立的 `create-release` job，在 matrix build 之前运行，只负责创建 release 和写入 changelog；matrix job 只负责 `files:` 上传，不设置 `body_path`。
   ```yaml
   needs: [create-release]
   with:
     body: ""  # 不重写 body
     files: dist/${{ matrix.artifact }}
   ```
2. 或者升级 `softprops/action-gh-release` 到最新 patch 版本，该版本包含并发上传修复。
3. 使用 `gh release upload` CLI 命令替代 action，CLI 只做文件上传，不修改 release metadata。

**Warning signs:**
- 多次发布后发现 GitHub Release 的 body 内容不完整，只包含部分 changelog。
- CI 日志显示 `409 Conflict` 或 `422 Unprocessable Entity` 错误。
- Release artifact 文件有的出现有的不出现（上传中途失败）。

**Phase to address:**
CD 发布阶段。在推出第一个真实 tag 前，先用 `v0.0.0-test` tag 测试完整的 release 流程。

---

### Pitfall 5: actions/checkout 使用 v6 但与自托管 runner / 其他 action 不兼容

**What goes wrong:**
当前所有 workflow 使用 `actions/checkout@v6`。v6 于 2025 年底发布，但引入了一个 breaking change：凭证持久化路径硬编码了 GitHub Actions 的 runner 路径，导致与 Forgejo/Gitea/GitLab 等自托管 runner 完全不兼容。同时，`peter-evans/create-pull-request` 等热门 action 明确标注与 v6 不兼容（2025 年的公开 Issue）。虽然本项目目前只用 GitHub 官方 runner，但其他 action（如 `softprops/action-gh-release`）的内部实现可能假设 checkout v4 的行为。

**Why it happens:**
- v6 在 2025 年底才发布，许多 action 的依赖文档仍指向 v4。
- 版本号递增不代表向后兼容——GitHub action 的语义版本规范较松散。
- CI 在本地测试时通过（checkout 成功），但某些 post-checkout 步骤失败时不容易关联到 checkout 版本。

**How to avoid:**
1. 短期内：将 `actions/checkout@v6` 锁定到当前最新 stable commit SHA，避免 tag 漂移导致自动获取 breaking change：
   ```yaml
   uses: actions/checkout@v4  # 或指定具体 SHA
   ```
2. 长期：使用 commit SHA 固定所有第三方 action（包括 `dtolnay/rust-toolchain`、`Swatinem/rust-cache`、`taiki-e/install-action`），防止供应链攻击和非预期的版本更新。2025 年 3 月的 `tj-actions/changed-files` 事件（23,000+ 仓库受影响）是典型案例。
3. 使用 `dependabot.yml`（已配置）定期更新 action 版本，但审查 changelog 再合并。

**Warning signs:**
- CI 在 checkout 步骤后的 `git config` 或认证步骤失败。
- 升级 checkout 版本后，`softprops/action-gh-release` 或其他 action 的上传步骤报 permission 错误。
- 本地 `act` 测试正常但 GitHub Actions 运行异常。

**Phase to address:**
CI 基础设施阶段。在 workflow 编写时就锁定版本，而非使用浮动 tag。

---

### Pitfall 6: e2e 测试（assert_cmd）在 Windows CI 上因路径分隔符或 stderr 输出格式失败

**What goes wrong:**
`assert_cmd` + `predicates` 的 e2e 测试在 macOS/Linux 通过，但在 Windows（`windows-latest` runner）失败：
- 路径字符串中的 `\` vs `/` 导致 `contains("path/to/file")` 匹配失败。
- Windows 上 `Command` 进程的 stderr/stdout 行尾是 `\r\n`，而 `contains("hint: ...")` 断言字符串只有 `\n`。
- Windows 上 SQLite 文件锁定行为不同，可能导致"文件仍被进程持有"类的测试错误。
- 临时目录路径包含空格（Windows 用户名带空格），而 config 文件中的路径未加引号。

**Why it happens:**
- `assert_cmd` 的 `assert().stdout(contains("..."))` 对 Windows CRLF 换行敏感。
- `tempfile::TempDir` 在 Windows 上返回 `C:\Users\RUNNER~1\AppData\Local\Temp\...` 格式路径，而代码中 `to_string_lossy().replace('\\', "/")` 转换不一定覆盖所有路径。
- tests/integration.rs 已经有 `replace('\\', "/")` 模式（第 19 行），但新增的 e2e 测试可能忘记这一步。

**How to avoid:**
1. 断言字符串避免包含路径分隔符——只断言错误 *类型* 而非路径内容：
   ```rust
   .stderr(predicates::str::contains("hint:"))
   // 不要: .stderr(predicates::str::contains("/path/to/config.toml"))
   ```
2. 对需要断言路径的场景，用 `Path::display()` 输出再比较，或使用 `predicates::str::contains` 配合平台无关子串。
3. 所有使用 `to_string_lossy()` 的地方统一加 `.replace('\\', "/")`（已有先例，参考 tests/integration.rs:19）。
4. 在 CI matrix 中尽早加上 `windows-latest`，不要等 e2e 测试全部写完再测 Windows。

**Warning signs:**
- `cargo test` 在 `ubuntu-latest` 全部通过，但 `windows-latest` job 有 5-10 个测试失败。
- 失败信息包含 `panicked at 'assertion failed'` 但 expected/actual 只差一个 `\r`。
- Windows 上 `tempfile::TempDir` 清理失败（`PermissionDenied`），因为 SQLite 文件仍被持有。

**Phase to address:**
e2e 测试阶段。每写一个 e2e 测试就在本地检查 Windows 兼容性（或在 CI 中保持三平台 matrix）。

---

### Pitfall 7: cargo-llvm-cov 覆盖率在重构后假性下降触发 CI 门控

**What goes wrong:**
ci.yaml 设置了 `cargo llvm-cov --fail-under-lines 70` 的硬性门控。重构（模块拆分、函数移动）后，覆盖率可能短暂下降：
- 被移动到新文件的函数，如果原测试通过 `use super::*` 访问，重构后测试可能不再覆盖这些函数。
- 内联优化（LTO=fat，opt-level=3）在 release 构建下会把小函数内联，导致 llvm-cov 认为某些代码行"未执行"。
- `#[cfg(not(target_os = "windows"))]` 条件编译的代码在 Linux CI 上报告 100%，但 Windows 特定代码报告 0%，影响总覆盖率。

**Why it happens:**
- `cargo llvm-cov` 默认在 debug 构建下运行，但 `#[cfg(test)]` 只编译测试时有效的代码；条件编译分支在不匹配平台上根本不参与覆盖率统计。
- 重构导致"私有函数可见性变化"时，原来能测的函数变成不可测，覆盖率下降。
- 70% 的门控阈值在当前 529 个测试的项目中应该很容易达到，但任何测试覆盖率的计算公式变化（如新增大量未测试代码）都可能触发。

**How to avoid:**
1. 重构阶段**临时降低**覆盖率门控（如 60%），重构完成后再补充测试恢复到 70%+。
2. 使用 `--ignore-filename-regex` 排除不易测试的 platform-specific 代码。
3. 重构时优先保证测试能编译并通过，再关注覆盖率。
4. `cargo llvm-cov` 只在 Linux 上运行（已配置），避免跨平台覆盖率统计混乱。

**Warning signs:**
- 重构 PR 中 CI coverage job 失败，但所有 test job 通过。
- 覆盖率从 78% 突然降到 65%，但没有删除任何测试。
- llvm-cov 报告中某个模块显示 0% 覆盖，但该模块明确有测试。

**Phase to address:**
代码重构阶段。重构前确认当前覆盖率基线，重构期间放宽门控，重构后补测试恢复。

---

## 技术债务模式

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| benchmark CI 门控直接用 criterion 数值 | 看起来"严格" | CI 噪声产生假阳性，开发者开始绕过 | 永远不可接受；用集成测试中的粗粒度基准代替 |
| 所有 CI 步骤跑在同一 `ubuntu-latest` job | 节省矩阵配置时间 | Windows/macOS 特有的 bug 到用户那才暴露 | 只有纯算法库才可接受；CLI 工具必须三平台测试 |
| `actions/checkout@v6` 浮动 tag | 自动获得 action 更新 | 供应链攻击风险，v6 本身有已知兼容性问题 | 不可接受；应锁定到 SHA 或经测试的 minor tag |
| release workflow 所有 job 都写 body_path | 少写一个"create release" job | 并发写入竞争，release notes 随机残缺 | 永远不可接受 |
| e2e 测试硬编码路径字符串断言 | 写起来直观 | Windows CI 失败 | 临时本地调试可用，绝不提交 CI |
| 模块重构后只跑 `cargo build` 确认 | 快（几秒） | `cargo test` 可能失败，模块可见性问题只在测试中暴露 | 永远不可接受；build 成功是必要非充分条件 |

---

## 集成陷阱

| 集成点 | 常见错误 | 正确做法 |
|--------|----------|----------|
| `rusqlite bundled` + `cross` aarch64 | 假设 cross 默认镜像包含完整 C 工具链 | 添加 `Cross.toml` 指定含完整 gcc toolchain 的镜像，或改用原生交叉编译 |
| `softprops/action-gh-release` 并行 job | 每个 matrix job 都设置 `body_path` | 分离"create release"（serial）和"upload artifact"（parallel）两个阶段 |
| `cargo llvm-cov` + `cfg(test)` | 期望条件编译代码参与覆盖率计算 | 用 `--ignore-filename-regex` 排除平台特定代码，或接受这部分代码的低覆盖率 |
| `assert_cmd` + Windows | 假设路径分隔符和行尾与 Unix 相同 | 统一用 `replace('\\', "/")` + 避免在断言中包含平台相关路径 |
| `Swatinem/rust-cache` + matrix build | 所有平台共享同一 cache key | `rust-cache@v2` 默认已按 OS + toolchain 分 key，但要确认 `key:` 不被手动覆盖为固定值 |

---

## 性能陷阱

| 陷阱 | 症状 | 预防 | 何时爆发 |
|------|------|------|----------|
| CI benchmark 作为 merge 门控 | 每次 PR 都有假性 regression 报告 | 保持 `continue-on-error: true`，用集成测试做真正门控 | 第一次推代码时 |
| `cargo bench --no-run` 不检测 benchmark 代码编译错误 | benchmark 代码改变后 CI 通过但本地 `cargo bench` 失败 | 定期在本地完整跑一次 bench | 当 bench 代码引用被重构的模块时 |
| cross compile + LTO fat 构建时间 | release job 超过 30 分钟 | 对 cross 构建禁用 LTO（release profile 只在 native 构建时用 fat LTO）| 首次设置 CI CD 时 |
| Windows 上 SQLite 文件锁 | e2e 测试偶发性失败，`tempfile::TempDir` drop 报 PermissionDenied | 测试结束前确保所有 Connection 已 drop，或用 `defer` 模式 | Windows 测试 CI 中 |

---

## "Looks Done But Isn't" 检查清单

- [ ] **CI 三平台 test job**: 不只是 `cargo clippy`/`cargo fmt` 跑在 Linux——`cargo test` 必须覆盖 ubuntu/windows/macos 三平台，因为 e2e 测试、路径处理、stdin 行为都有平台差异。
- [ ] **release workflow 测试**: 不只是 workflow 文件语法正确——必须推一个 `v0.0.0-test` tag 验证完整流程，包括 cross 构建、artifact 上传、release notes 生成。
- [ ] **benchmark CI 定位**: 不只是 `cargo bench` 能跑完——必须明确 bench 是信息性（不阻断 merge）还是门控性（可阻断 merge），并在 workflow 中用 `continue-on-error` 体现。
- [ ] **cli/run 重构后测试**: 不只是 `cargo build` 通过——`cargo test` 全量、`cargo clippy` 无 warnings、`cargo test --test integration` 跨平台全部通过。
- [ ] **e2e 测试 Windows 兼容**: 不只是 macOS/Linux 通过——在 CI matrix 中加 `windows-latest` 并在第一天就运行，不要等所有测试写完才加。
- [ ] **crates.io publish**: 不只是 `cargo publish` 命令存在——必须有 `CARGO_REGISTRY_TOKEN` secret、版本号与 git tag 一致的验证，以及 dry-run 测试（`cargo publish --dry-run`）。

---

## 恢复策略

| Pitfall | 恢复成本 | 恢复步骤 |
|---------|----------|----------|
| cross aarch64 构建失败 | MEDIUM | 添加 `Cross.toml` 指定正确镜像；或临时从 release matrix 中移除 aarch64 先发布其他平台 |
| benchmark 假性回归 | LOW | 加 `continue-on-error: true`；改用 `cargo bench --no-run` 做编译检查 |
| 模块重构后测试失败 | LOW-MEDIUM | 编译器错误信息精确定位失败点；调整可见性（`pub(super)`/`pub(crate)`）或移动测试位置 |
| release body 被覆盖 | LOW | 重新手动编辑 GitHub Release notes；修复 workflow 分离 create/upload 步骤 |
| Windows e2e 测试失败 | LOW | 识别 CRLF 或路径分隔符问题，加 `.replace('\\', "/")` 或用平台无关断言 |
| 覆盖率门控触发 | LOW | 临时在 ci.yaml 中降低阈值（60%），重构完补测试恢复 |
| cargo publish 意外发布错误版本 | HIGH | 立刻 `cargo yank --version X.Y.Z`；注意 yank 不删除代码，但阻止新依赖；下一步修复版本号再发布正确版本 |

---

## 阶段映射

| Pitfall | 预防阶段 | 验证方式 |
|---------|----------|----------|
| Pitfall 1: rusqlite cross aarch64 构建失败 | CD 构建阶段（多平台 Release） | 推测试 tag 触发完整构建，4 个 matrix job 全部绿 |
| Pitfall 2: criterion CI 假性回归 | CI 基础设施阶段（benchmark stabilization） | bench.yml 有 `continue-on-error: true`；merge 门控只依赖 integration test 的 `test_csv_throughput_baseline` |
| Pitfall 3: 模块重构后测试访问私有函数失败 | 代码重构阶段（cli/run 模块拆分） | 重构每一步后运行 `cargo test` + `cargo clippy -- -D warnings` |
| Pitfall 4: release job 并发竞争 | CD 发布阶段 | 测试 tag 验证 release notes 完整，4 个 artifact 全部出现 |
| Pitfall 5: actions/checkout v6 兼容性 | CI 基础设施阶段（首次 CI setup） | 检查所有 action 版本；workflow 在推 PR 时全部通过 |
| Pitfall 6: e2e 测试 Windows 兼容 | e2e 测试阶段 | CI matrix 含 `windows-latest`，`cargo test` 全部通过 |
| Pitfall 7: 覆盖率假性下降阻断重构 | 代码重构阶段 | 重构前记录覆盖率基线；重构期间放宽门控；重构后补测试恢复 |

---

## Sources

- [Cross-compiling Rust on GitHub Actions](https://obviy.us/blog/cross-compiling-rust-on-gha/) — cross-rs 配置模式
- [rusqlite #939: Cross compiling rusqlite failing with docker](https://github.com/rusqlite/rusqlite/issues/939) — bundled + cross 已知问题
- [CI for performance: Reliable benchmarking in noisy environments](https://pythonspeed.com/articles/consistent-benchmarking-in-ci/) — criterion CI 噪声分析
- [Criterion.rs FAQ](https://bheisler.github.io/criterion.rs/book/faq.html) — 官方承认 CI 噪声问题
- [Pinning GitHub Actions for Enhanced Security](https://www.stepsecurity.io/blog/pinning-github-actions-for-enhanced-security-a-complete-guide) — SHA 固定最佳实践
- [softprops/action-gh-release releases](https://github.com/softprops/action-gh-release/releases) — v2.5.2 并发上传竞争修复
- [GitHub Releases API Race Condition](https://devactivity.com/insights/mastering-github-releases-avoiding-race-conditions-for-enhanced-engineering-productivity/) — release body 竞争分析
- [How to test Rust CLI apps with assert_cmd](https://alexwlchan.net/2025/testing-rust-cli-apps-with-assert-cmd/) — assert_cmd 测试模式
- [Test Organization — The Rust Book](https://doc.rust-lang.org/book/ch11-03-test-organization.html) — `#[cfg(test)]` 与模块可见性
- [Move unit tests into separate files — rust-lang/rust #61097](https://github.com/rust-lang/rust/issues/61097) — `#[path]` 测试文件方案
- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov) — 覆盖率工具已知问题
- 项目代码审查：ci.yaml、release.yaml、bench.yml、tests/integration.rs、src/cli/run/tests.rs、Cargo.toml

---
*Pitfalls research for: sqllog2db v1.15 CI/CD + 工程质量提升*
*Researched: 2026-06-02*
