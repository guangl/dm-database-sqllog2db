# 测试目录

所有测试实现与测试专用辅助代码统一放在此目录。

| 位置 | 内容 | 运行方式 |
| --- | --- | --- |
| `unit/` | 按源码模块组织的 Rust 单元测试，包括私有实现测试 | `cargo test --lib` |
| 根目录 `.rs` 文件 | CLI、导出结果等集成测试 | `cargo test --tests` |
| `python/` | 导出内存检查脚本的失败路径测试 | `python3 -B -m unittest discover -s tests/python -p 'test_*.py' -v` |

运行全部 Rust 测试使用 `cargo test --locked`。性能基准保留在根目录 `benches/`，使用 `cargo bench`。

`unit/` 的目录层次对应 `src/`：例如 `unit/config/validate.rs` 测试配置校验，`unit/exporter/csv.rs` 测试 CSV 导出。源码仅通过 `#[cfg(test)]` 和 `#[path = "..."]` 声明对应测试模块。文件虽然位于 `tests/`，仍作为原模块的子模块编译，因此可以测试私有实现，无需扩大生产 API 的可见性，也不会被 Cargo 重复发现为集成测试。

新增单元测试时，将实现放入对应的 `unit/` 子目录；新增独立集成测试时，放在本目录根部。测试专用辅助函数与夹具随测试维护，不放回 `src/`。

单文件测试模块直接使用模块名命名，如 `unit/cli/mod.rs`；只有多个测试文件需要归组时才建立子目录，并用 `mod.rs` 保存该组的公共模块测试。
